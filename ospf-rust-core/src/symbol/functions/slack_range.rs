//! 范围松弛函数符号 / Slack-range function symbol
//!
//! - `SlackRangeFunction`：到闭区间的距离 / distance to a closed interval

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
use super::max::MaxFunction;
use crate::error::{ModelError, Result};
use crate::model::LinearConstraint;
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::ContinuousVariableItem;
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;

fn evaluate_linear<V>(
    poly: &Linear<V>,
    token_table: &dyn TokenList<V>,
    zero_if_none: bool,
) -> Option<V>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V> + Zero,
{
    let mut value = poly.constant_term().clone();
    for monomial in poly.monomials() {
        let term_value = match token_table
            .find_by_index(monomial.var_index())
            .and_then(|token| token.get_result())
        {
            Some(v) => v,
            None if zero_if_none => V::zero(),
            None => return None,
        };
        value = value + monomial.coefficient().clone() * term_value;
    }
    Some(value)
}

fn to_f64<V>(value: &V) -> Option<f64>
where
    V: ToPrimitive,
{
    value.to_f64()
}

fn from_f64<V>(value: f64) -> Option<V>
where
    V: FromPrimitive,
{
    V::from_f64(value)
}

/// 范围松弛函数 / Slack-range function
///
/// 到闭区间 `[lower, upper]` 的距离：`max(lower - x, x - upper, 0)`。
/// Distance to a closed interval `[lower, upper]`: `max(lower - x, x - upper, 0)`.
///
/// 此实现内部复用精确最大值约束。
/// This implementation reuses exact-max constraints internally.
#[derive(Debug, Clone)]
pub struct SlackRangeFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 符号 ID / Symbol ID
    id: IntermediateSymbolId,
    /// 输入多项式 / Input polynomial
    input: Linear<V>,
    /// 下界 / Lower bound
    lower: V,
    /// 上界 / Upper bound
    upper: V,
    /// 内部最大值函数 / Inner maximum function
    inner: MaxFunction<V>,
    /// Optional explicit Big-M value. `None` delegates to the shared default
    /// or token-derived policy, while `Some` is used verbatim after validation.
    /// 显式 Big-M 值；为空时使用共享默认/令牌推导策略。
    big_m: Option<V>,
    /// 声明的依赖 ID / Declared dependency IDs
    declared_dependency_ids: Vec<u64>,
}

impl<V> SlackRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    fn validate_bounds(lower: &V, upper: &V, name: &str) {
        let lower_f64 = to_f64(lower).expect("convert lower to f64");
        let upper_f64 = to_f64(upper).expect("convert upper to f64");
        assert!(
            lower_f64.is_finite() && upper_f64.is_finite(),
            "SlackRangeFunction `{name}` requires finite bounds"
        );
        assert!(
            lower_f64 <= upper_f64,
            "SlackRangeFunction `{name}` requires lower <= upper"
        );
    }

    fn validate_big_m(big_m: &V) {
        let value = to_f64(big_m).expect("convert slack-range big-M to f64");
        assert!(
            value.is_finite() && value > 0.0,
            "SlackRangeFunction requires a positive finite big-M"
        );
    }

    fn validate_input(input: &Linear<V>) {
        assert!(
            to_f64(input.constant_term())
                .map(|value| value.is_finite())
                .unwrap_or(false)
                && input.monomials().iter().all(|monomial| {
                    to_f64(monomial.coefficient())
                        .map(|value| value.is_finite())
                        .unwrap_or(false)
                }),
            "SlackRangeFunction input polynomial must contain finite values"
        );
    }

    fn validate_derived_candidates(input: &Linear<V>, lower: &V, upper: &V) {
        let lower_f64 = to_f64(lower).expect("convert lower to f64");
        let upper_f64 = to_f64(upper).expect("convert upper to f64");
        let input_constant = to_f64(input.constant_term()).expect("convert input constant to f64");
        assert!(
            (lower_f64 - input_constant).is_finite()
                && (input_constant - upper_f64).is_finite()
                && input.monomials().iter().all(|monomial| {
                    to_f64(monomial.coefficient())
                        .map(|coefficient| (-coefficient).is_finite())
                        .unwrap_or(false)
                }),
            "SlackRangeFunction derived candidates must contain finite values"
        );
    }

    fn build_polynomials(input: &Linear<V>, lower: &V, upper: &V) -> Vec<Linear<V>> {
        let lower_f64 = to_f64(lower).expect("convert lower to f64");
        let upper_f64 = to_f64(upper).expect("convert upper to f64");
        let input_constant = to_f64(input.constant_term()).expect("convert input constant to f64");

        let mut lower_minus_input_terms = Vec::with_capacity(input.monomials().len());
        let mut input_minus_upper_terms = Vec::with_capacity(input.monomials().len());
        for monomial in input.monomials() {
            let coefficient =
                to_f64(monomial.coefficient()).expect("convert input coefficient to f64");
            lower_minus_input_terms.push(LinearMonomial::new(
                from_f64(-coefficient).expect("convert lower-input coefficient"),
                monomial.var_index(),
            ));
            input_minus_upper_terms.push(LinearMonomial::new(
                from_f64(coefficient).expect("convert input-upper coefficient"),
                monomial.var_index(),
            ));
        }

        let lower_minus_input = Linear::new(
            lower_minus_input_terms,
            from_f64(lower_f64 - input_constant).expect("convert lower-input constant"),
        );
        let input_minus_upper = Linear::new(
            input_minus_upper_terms,
            from_f64(input_constant - upper_f64).expect("convert input-upper constant"),
        );
        let zero_poly = Linear::new(vec![], from_f64(0.0).expect("convert zero"));

        vec![lower_minus_input, input_minus_upper, zero_poly]
    }

    fn build_inner(id: u64, name: &str, input: &Linear<V>, lower: &V, upper: &V) -> MaxFunction<V> {
        MaxFunction::new(id, name, Self::build_polynomials(input, lower, upper), true)
    }

    /// 创建新的范围松弛函数 / Create a new slack-range function
    pub fn new(id: u64, name: &str, input: Linear<V>, lower: V, upper: V) -> Self {
        Self::validate_bounds(&lower, &upper, name);
        Self::validate_input(&input);
        Self::validate_derived_candidates(&input, &lower, &upper);
        let inner = Self::build_inner(id, name, &input, &lower, &upper);
        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            lower,
            upper,
            inner,
            big_m: None,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 创建使用显式 Big-M 的范围松弛函数。
    /// Create a range-slack function with an explicit Big-M value.
    pub fn with_big_m(id: u64, name: &str, input: Linear<V>, lower: V, upper: V, big_m: V) -> Self {
        Self::validate_bounds(&lower, &upper, name);
        Self::validate_input(&input);
        Self::validate_derived_candidates(&input, &lower, &upper);
        Self::validate_big_m(&big_m);
        let inner = Self::build_inner(id, name, &input, &lower, &upper);
        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            lower,
            upper,
            inner,
            big_m: Some(big_m),
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub(crate) fn with_input_polynomial(&self, input: Linear<V>) -> Self {
        Self::validate_input(&input);
        Self::validate_derived_candidates(&input, &self.lower, &self.upper);
        let mut cloned = self.clone();
        cloned.input = input;
        cloned.inner = self.inner.with_polynomials(Self::build_polynomials(
            &cloned.input,
            &self.lower,
            &self.upper,
        ));
        cloned
    }

    /// 在保持结果/选择器变量 ID 不变的前提下覆盖 Big-M。
    /// Override Big-M while preserving result/selector variable IDs.
    pub(crate) fn with_big_m_value(&self, big_m: V) -> Self {
        let mut cloned = self.clone();
        cloned.big_m = Some(big_m);
        cloned
    }

    /// 获取输入多项式 / Get the input polynomial
    pub fn input_polynomial(&self) -> &Linear<V> {
        &self.input
    }

    /// 获取下界 / Get the lower bound
    pub fn lower_bound(&self) -> &V {
        &self.lower
    }

    /// 获取上界 / Get the upper bound
    pub fn upper_bound(&self) -> &V {
        &self.upper
    }

    /// 获取显式 Big-M；未指定时返回 `None`。
    /// Return the explicit Big-M, or `None` when the shared policy is used.
    pub fn big_m(&self) -> Option<&V> {
        self.big_m.as_ref()
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for SlackRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "slack_range({})", self.id.name)
    }
}

impl<V> DynSymbol for SlackRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn name(&self) -> &str {
        &self.id.name
    }

    fn display_name(&self) -> &str {
        &self.id.name
    }

    fn dyn_id(&self) -> SymbolDynId<'_> {
        SymbolDynId::standalone(self.id.id as usize)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<V> Symbol for SlackRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for SlackRangeFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + ToPrimitive
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn category(&self) -> Category {
        Category::Linear
    }

    fn cached(&self) -> bool {
        false
    }

    fn dependencies(&self) -> HashSet<Arc<dyn IntermediateSymbol<V>>> {
        HashSet::new()
    }

    fn declared_dependency_ids(&self) -> Vec<u64> {
        self.declared_dependency_ids.clone()
    }

    fn flush(&self, _force: bool) {}

    fn register_auxiliary_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        <Self as FunctionSymbol<V>>::register_tokens(self, tokens)
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &std::collections::HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        match self.big_m.as_ref() {
            Some(big_m) => {
                let value = to_f64(big_m).ok_or_else(|| {
                    ModelError::InvalidConstraint(format!(
                        "slack-range `{}` big-M cannot be converted to f64",
                        self.id.name
                    ))
                })?;
                if !value.is_finite() || value <= 0.0 {
                    return Err(ModelError::InvalidConstraint(format!(
                        "slack-range `{}` requires positive finite big-M",
                        self.id.name
                    ))
                    .into());
                }
                self.inner
                    .mechanism_constraints_with_big_m(symbol_to_index, value)
            }
            None => self.inner.mechanism_constraints(symbol_to_index),
        }
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &std::collections::HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        match self.big_m.as_ref() {
            Some(big_m) => {
                let value = to_f64(big_m).ok_or_else(|| {
                    ModelError::InvalidConstraint(format!(
                        "slack-range `{}` big-M cannot be converted to f64",
                        self.id.name
                    ))
                })?;
                if !value.is_finite() || value <= 0.0 {
                    return Err(ModelError::InvalidConstraint(format!(
                        "slack-range `{}` requires positive finite big-M",
                        self.id.name
                    ))
                    .into());
                }
                self.inner
                    .mechanism_constraints_with_big_m(symbol_to_index, value)
            }
            None => self
                .inner
                .mechanism_constraints_with_tokens(symbol_to_index, tokens),
        }
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &std::collections::HashMap<usize, V>) -> Option<V> {
        values.get(&self.inner.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("slack_range({})", self.id.name)
    }

    fn deferred_structure_with_tokens(
        &self,
        tokens: &[Token<V>],
    ) -> Option<Arc<dyn crate::model::intermediate::DeferredFunctionStructure<V>>> {
        // Big-M 与即时路径逐字一致：显式配置值原样使用；未指定时快照即时展开实际会用的那组每候选
        // 非对称 M（与内部 MAX 共用同一份推断，含推断不可用时的统一回退值）。两种来源都只保存
        // `mechanism_constraints_with_tokens` 真正会用到的 M，所以物化与即时展开逐行一致；配置值
        // 非法时不提供结构，让即时展开照旧报出配置错误，而不是把错误推迟到物化阶段。
        // The Big-M matches the eager path verbatim: an explicit configured value is used as is, and
        // otherwise the per-candidate asymmetric M group eager expansion would actually use is
        // snapshotted (sharing one inference with the inner MAX, including the unified fallback value
        // when inference is unavailable). Both sources keep exactly the M group
        // `mechanism_constraints_with_tokens` uses, so materialization matches eager expansion row by
        // row; an invalid configured value withholds the structure so eager expansion keeps reporting
        // the configuration error instead of deferring it to materialization.
        let big_ms = match self.big_m.as_ref() {
            Some(big_m) => {
                let value = to_f64(big_m)?;
                if !value.is_finite() || value <= 0.0 {
                    return None;
                }
                vec![value; self.inner.polynomials().len()]
            }
            None => self.inner.eager_candidate_big_ms(tokens),
        };
        Some(Arc::new(SlackRangeStructure::new(
            self.id.name.clone(),
            Arc::new(self.clone()),
            big_ms,
        )))
    }
}

/// 范围松弛的求解器无关结构描述 / Solver-neutral structure description of slack-range
///
/// 范围松弛的机制约束转发给内部精确 MAX，因此结构与 [`MaxStructure`] 采用同一模式：持有产生它的
/// 符号（`Arc`）、结果列、选择器列与创建时快照的每候选非对称 Big-M 数组，物化时回调内部 MAX 手写
/// 路径的同一个公式生成器并传入同一组 M，所以延迟物化与 EAGER 展开逐行一致（含 M 取值）。
///
/// 与 MINMAX/MAXMIN 这类纯转发符号不同，范围松弛的即时展开会读取令牌边界：没有显式 Big-M 时每候选
/// M 由「输入与上下界推出的候选多项式范围」决定，因此结构必须把推断出的 M 一起快照，否则物化阶段
/// 无法复现同一组松弛。选择器列与 MAX 一样属于本结构的辅助列。
///
/// [`MaxStructure`]: crate::symbol::functions::max::MaxStructure
///
/// Slack-range's mechanism constraints forward to the inner exact MAX, so the structure follows the
/// same pattern as [`MaxStructure`]: it holds the symbol that produced it (an `Arc`), the result
/// column, the selector columns and the per-candidate asymmetric Big-M array snapshotted at creation
/// time, and it materializes through the very same formula generator as the handwritten inner-MAX
/// path with that same M group, so deferred materialization matches eager expansion row by row,
/// including the M values.
///
/// Unlike pure forwarding symbols such as MINMAX/MAXMIN, slack-range's eager expansion does read token
/// bounds: without an explicit Big-M each candidate M follows from the candidate polynomial ranges
/// implied by the input and the interval bounds, so the structure must snapshot the inferred M group
/// or materialization could not reproduce the same relaxation. Like MAX, the selector columns are
/// helpers of this structure.
#[derive(Debug)]
pub struct SlackRangeStructure<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 函数名称 / Function name
    name: String,
    /// 产生本结构的符号 / Symbol that produced this structure
    symbol: Arc<SlackRangeFunction<V>>,
    /// 结果列 / Result column
    result: crate::variable::VariableId,
    /// 选择器辅助列 / Selector helper columns
    selectors: Vec<crate::variable::VariableId>,
    /// 创建时快照的每候选非对称 Big-M / Per-candidate asymmetric Big-M snapshotted at creation time
    big_ms: Vec<f64>,
}

impl<V> SlackRangeStructure<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    /// 创建结构描述 / Create a structure description.
    pub fn new(
        name: impl Into<String>,
        symbol: Arc<SlackRangeFunction<V>>,
        big_ms: Vec<f64>,
    ) -> Self {
        let result = symbol.result_variable().id();
        let selectors = symbol
            .inner
            .selector_variables()
            .map(|selectors| selectors.iter().map(|selector| selector.id()).collect())
            .unwrap_or_default();
        Self {
            name: name.into(),
            symbol,
            result,
            selectors,
            big_ms,
        }
    }

    /// 获取函数名称 / Get the function name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取结果列 / Get the result column.
    pub fn result(&self) -> &crate::variable::VariableId {
        &self.result
    }

    /// 获取选择器辅助列 / Get the selector helper columns.
    pub fn selectors(&self) -> &[crate::variable::VariableId] {
        &self.selectors
    }

    /// 获取创建时快照的每候选非对称 Big-M / Get the per-candidate asymmetric Big-M snapshotted at creation.
    pub fn big_ms(&self) -> &[f64] {
        &self.big_ms
    }
}

impl<V> crate::model::intermediate::DeferredFunctionStructure<V> for SlackRangeStructure<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + ToPrimitive
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn function_name(&self) -> &str {
        &self.name
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn usage_binding(&self) -> Option<crate::model::intermediate::StructureUsageBinding> {
        // 选择器列属于本结构的辅助列，参与「是否被外部引用 / 是否可省略」的判定。
        // Selector columns are helpers of this structure and take part in the
        // externally-referenced and omittable analysis.
        Some(crate::model::intermediate::StructureUsageBinding::new(
            self.symbol.id.id,
            self.result.clone(),
            self.selectors.clone(),
        ))
    }

    fn fingerprint(&self) -> Option<String> {
        // Big-M 数组进入指纹：M 的取值变化同样必须让旧记录失效。
        // The Big-M array is part of the fingerprint: a change in M must invalidate old records too.
        let selectors = self
            .selectors
            .iter()
            .map(|selector| selector.unique_id().to_string())
            .collect::<Vec<_>>()
            .join(",");
        let big_ms = self
            .big_ms
            .iter()
            .map(|value| crate::model::intermediate::fingerprint_float(*value))
            .collect::<Vec<_>>()
            .join(",");
        Some(format!(
            "slack_range|{}|{}|{}|{}|{}",
            self.name,
            self.symbol.id.id,
            self.result.unique_id(),
            selectors,
            big_ms
        ))
    }

    fn materialize(
        &self,
        symbol_to_index: &std::collections::HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        // 复用内部 MAX 即时展开的同一份生成器与同一组快照 M，保证两条路径逐行一致。
        // Reuse the inner MAX eager generator and the same snapshotted M group so both paths stay
        // row-identical.
        self.symbol
            .inner
            .build_mechanism_constraints(symbol_to_index, &self.big_ms)
    }
}

impl<V> FunctionSymbol<V> for SlackRangeFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + ToPrimitive
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        self.inner.register_tokens(tokens)
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let x = to_f64(&evaluate_linear(&self.input, token_table, zero_if_none)?)?;
        let lower = to_f64(&self.lower)?;
        let upper = to_f64(&self.upper)?;
        if x < lower {
            from_f64(lower - x)
        } else if x > upper {
            from_f64(x - upper)
        } else {
            from_f64(0.0)
        }
    }
}

impl<V> LinearIntermediateSymbol<V> for SlackRangeFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + ToPrimitive
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn to_linear_polynomial(&self) -> Linear<V> {
        self.inner.to_linear_polynomial()
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        self.inner.to_quadratic_polynomial()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::model::{FunctionExpansionPolicy, MetaModel};
    use crate::symbol::flatten::LinearMonomial;
    use crate::variable::{VariableId, VariableRange};

    /// 构造列映射：输入列、选择器辅助列、结果列。
    /// Build the column map: input column, selector helper columns and the result column.
    fn symbol_to_index_for(
        function: &SlackRangeFunction<f64>,
        input_id: VariableId,
    ) -> HashMap<usize, usize> {
        let selectors = function
            .inner
            .selector_variables()
            .expect("slack-range inner max should be exact")
            .to_vec();
        let mut symbol_to_index = HashMap::new();
        symbol_to_index.insert(input_id.unique_id() as usize, 0);
        for (offset, selector) in selectors.iter().enumerate() {
            symbol_to_index.insert(selector.id().unique_id() as usize, 1 + offset);
        }
        symbol_to_index.insert(
            function.result_variable().id().unique_id() as usize,
            1 + selectors.len(),
        );
        symbol_to_index
    }

    /// 断言即时行与延迟行逐行一致 / Assert eager and deferred rows agree row by row.
    fn assert_rows_match(eager: &[LinearConstraint<f64>], deferred: &[LinearConstraint<f64>]) {
        assert!(!eager.is_empty(), "eager rows must not be empty");
        assert_eq!(eager.len(), deferred.len());
        for (eager_row, deferred_row) in eager.iter().zip(deferred.iter()) {
            assert_eq!(eager_row.name, deferred_row.name);
            assert_eq!(eager_row.inequality.relation, deferred_row.inequality.relation);
            assert_eq!(eager_row.inequality.rhs, deferred_row.inequality.rhs);
            assert_eq!(
                eager_row.inequality.polynomial.constant_term(),
                deferred_row.inequality.polynomial.constant_term()
            );
        }
    }

    /// 读取结构描述里快照的每候选 Big-M / Read the snapshot per-candidate Big-M of a structure.
    fn snapshot_big_ms(
        structure: &Arc<dyn crate::model::intermediate::DeferredFunctionStructure<f64>>,
    ) -> Vec<f64> {
        structure
            .as_any()
            .downcast_ref::<SlackRangeStructure<f64>>()
            .expect("structure should downcast to the slack-range structure")
            .big_ms()
            .to_vec()
    }

    #[test]
    fn slack_range_structure_materializes_the_same_rows_as_eager_expansion() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(96_300),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let input = Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0);
        let tokens = vec![Token::from_generic(x.clone(), 0)];

        // 第一种 Big-M 来源：无显式值时按令牌边界推断每候选 M。
        // First Big-M source: with no explicit value the per-candidate M is inferred from token
        // bounds.
        let inferred = SlackRangeFunction::new(31010, "slack_range_inferred", input.clone(), 1.0, 3.0);
        let structure = inferred
            .deferred_structure_with_tokens(&tokens)
            .expect("slack-range should expose a deferred structure");
        assert_eq!(structure.function_name(), "slack_range_inferred");
        let binding = structure
            .usage_binding()
            .expect("slack-range structure should expose a usage binding");
        assert_eq!(binding.result, inferred.result_variable().id());
        assert_eq!(binding.helpers.len(), 3);
        assert!(structure.fingerprint().is_some());
        let concrete = structure
            .as_any()
            .downcast_ref::<SlackRangeStructure<f64>>()
            .expect("structure should downcast to the slack-range structure");
        assert_eq!(*concrete.result(), inferred.result_variable().id());
        assert_eq!(concrete.selectors(), binding.helpers.as_slice());

        // 2x + 1 且 x ∈ [-2, 3]：候选 [-6, 4]、[-6, 4]、[0, 0] => M = [10, 10, 4]。
        // 2x + 1 with x in [-2, 3]: candidates [-6, 4], [-6, 4], [0, 0] => M = [10, 10, 4].
        let big_ms = snapshot_big_ms(&structure);
        assert_eq!(big_ms.len(), 3);
        assert!((big_ms[0] - 10.0).abs() <= 1e-9, "big_ms={big_ms:?}");
        assert!((big_ms[1] - 10.0).abs() <= 1e-9, "big_ms={big_ms:?}");
        assert!((big_ms[2] - 4.0).abs() <= 1e-9, "big_ms={big_ms:?}");

        let symbol_to_index = symbol_to_index_for(&inferred, x.id());
        let eager = inferred
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("eager slack-range constraints should be generated");
        let deferred = structure
            .materialize(&symbol_to_index)
            .expect("slack-range structure should materialize");
        assert_eq!(eager.len(), 7);
        assert_rows_match(&eager, &deferred);

        // 第二种 Big-M 来源：显式配置值原样进入结构与即时展开。
        // Second Big-M source: an explicit configured value enters both the structure and eager
        // expansion verbatim.
        let explicit = SlackRangeFunction::with_big_m(
            31011,
            "slack_range_explicit",
            input.clone(),
            1.0,
            3.0,
            13.0,
        );
        let structure = explicit
            .deferred_structure_with_tokens(&tokens)
            .expect("slack-range should expose a deferred structure");
        assert_eq!(snapshot_big_ms(&structure), vec![13.0, 13.0, 13.0]);
        let symbol_to_index = symbol_to_index_for(&explicit, x.id());
        let eager = explicit
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("eager slack-range constraints should be generated");
        let eager_default = <SlackRangeFunction<f64> as IntermediateSymbol<f64>>::mechanism_constraints(
            &explicit,
            &symbol_to_index,
        )
        .expect("default eager slack-range constraints should be generated");
        let deferred = structure
            .materialize(&symbol_to_index)
            .expect("slack-range structure should materialize");
        assert_rows_match(&eager, &deferred);
        assert_rows_match(&eager_default, &deferred);

        // 第三种 Big-M 来源：没有令牌边界时退回统一回退 Big-M，两条路径仍一致。
        // Third Big-M source: with no token bounds the unified fallback Big-M is used and both paths
        // still agree.
        let free = ContinuousVariableItem::create(VariableId::standalone(96_301), "x_free");
        let fallback = SlackRangeFunction::new(31012, "slack_range_fallback", input, 1.0, 3.0);
        let fallback_tokens = vec![Token::from_generic(free.clone(), 0)];
        let structure = fallback
            .deferred_structure_with_tokens(&fallback_tokens)
            .expect("slack-range should fall back to the shared default big-M");
        let big_ms = snapshot_big_ms(&structure);
        assert_eq!(big_ms.len(), 3);
        assert!((big_ms[0] - 1_000_000.0).abs() <= 1e-9, "big_ms={big_ms:?}");
        let symbol_to_index = symbol_to_index_for(&fallback, free.id());
        let eager = fallback
            .mechanism_constraints_with_tokens(&symbol_to_index, &fallback_tokens)
            .expect("eager slack-range constraints should be generated");
        let deferred = structure
            .materialize(&symbol_to_index)
            .expect("slack-range structure should materialize");
        assert_rows_match(&eager, &deferred);
    }

    #[test]
    fn slack_range_defers_through_the_model_pipeline() {
        fn rows(policy: FunctionExpansionPolicy) -> Vec<String> {
            let mut model = MetaModel::<f64>::new("slack_range_deferred_pipeline");
            model.set_function_expansion_policy(policy);
            let x = ContinuousVariableItem::with_range(
                VariableId::standalone(95_700),
                "x",
                VariableRange::bounded(-2.0, 3.0),
            );
            let x_index = model.register_variable(x).unwrap();
            let slack_range: SlackRangeFunction<f64> = SlackRangeFunction::new(
                994,
                "slack_range_pipeline",
                Linear::new(vec![LinearMonomial::new(2.0, x_index)], 1.0),
                1.0,
                3.0,
            );
            model.add_symbol(Arc::new(slack_range)).unwrap();

            let mechanism = model.try_into_mechanism_model().unwrap();
            if policy.is_deferred() {
                // 范围松弛在延迟策略下不写即时行，但保留结构描述。
                // Slack-range writes no eager row under a deferred policy while keeping its structure
                // description.
                assert!(mechanism.as_basic().constraints().is_empty());
                assert_eq!(mechanism.as_basic().deferred_functions().len(), 1);
                assert_eq!(
                    mechanism.as_basic().deferred_functions()[0].function_name(),
                    "slack_range_pipeline"
                );
            }

            let linear = mechanism.into_linear_triad_model();
            let mut names = linear.basic.constraint_names.clone();
            names.sort();
            names
        }

        let eager = rows(FunctionExpansionPolicy::Eager);
        // 3 个候选各一条下界与一条上界，加上选择器和式（转换时拆成两行）。
        // Each of the three candidates contributes a lower and an upper row, plus the selector sum,
        // which the conversion splits into two rows.
        assert!(eager.len() > 5, "eager rows: {eager:?}");
        // 延迟路径经物化后必须与 EAGER 得到同一批行，且 Big-M 取同一组推断值。
        // The deferred path must produce the same rows as eager expansion once materialized, using
        // the same inferred Big-M group.
        assert_eq!(eager, rows(FunctionExpansionPolicy::DeferredNativeFirst));
    }
}
