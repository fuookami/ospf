//! 最小值/最大值函数符号 / Min/max function symbols

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
use super::big_m::{
    BigMPolicy, infer_big_m_for_polynomials, infer_extremum_candidate_big_ms,
    infer_extremum_result_bounds, tighten_token_range,
};
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{BinaryVariableItem, ContinuousVariableItem, VariableId, new_group_id};
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::collections::{HashMap, HashSet};
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

const DEFAULT_BIG_M: f64 = 1_000_000.0;
const BIG_M_POLICY: BigMPolicy = BigMPolicy::new(DEFAULT_BIG_M, 1.0);

fn convert_f64_to_v<V>(value: f64, context: &str) -> Result<V>
where
    V: FromPrimitive,
{
    from_f64(value).ok_or_else(|| {
        ModelError::InvalidConstraint(format!(
            "failed to convert `{}` value {} from f64 into model value type",
            context, value
        ))
        .into()
    })
}

/// 最小值函数 / Minimum function
///
/// 表示 min(p1, p2, ..., pn)。
/// Represents min(p1, p2, ..., pn).
#[derive(Debug, Clone)]
pub struct MinFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 符号 ID / Symbol ID
    id: IntermediateSymbolId,
    /// 输入多项式列表 / Input polynomials
    polynomials: Vec<Linear<V>>,
    /// 结果连续变量 / Result continuous variable
    result_var: ContinuousVariableItem,
    /// 选择器二值变量列表（精确模式） / Selector binary variables (exact mode)
    binary_vars: Option<Vec<BinaryVariableItem>>,
    /// 声明的依赖 ID / Declared dependency IDs
    declared_dependency_ids: Vec<u64>,
}

impl<V> MinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建最小值函数 / Create a minimum function.
    pub fn new(id: u64, name: &str, polynomials: Vec<Linear<V>>, exact: bool) -> Self {
        let group_id = new_group_id();
        let result_var =
            ContinuousVariableItem::create(VariableId::new(group_id, 0), &format!("{}_min", name));

        let binary_vars = if exact {
            Some(
                (0..polynomials.len())
                    .map(|i| {
                        BinaryVariableItem::create(
                            VariableId::new(group_id, i + 1),
                            &format!("{}_u{}", name, i),
                        )
                    })
                    .collect(),
            )
        } else {
            None
        };

        Self {
            id: IntermediateSymbolId::new(id, name),
            polynomials,
            result_var,
            binary_vars,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置声明的依赖符号 ID / Set declared dependency symbol IDs.
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub(crate) fn with_polynomials(&self, polynomials: Vec<Linear<V>>) -> Self {
        let mut cloned = self.clone();
        cloned.polynomials = polynomials;
        cloned
    }

    pub(crate) fn mechanism_constraints_with_big_m(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<LinearConstraint<V>>>
    where
        V: Add<Output = V> + Mul<Output = V> + Zero + ToPrimitive + FromPrimitive,
        f64: IntoValue<V>,
    {
        let big_ms = vec![big_m; self.polynomials.len()];
        self.build_mechanism_constraints(symbol_to_index, &big_ms)
    }

    /// 获取结果变量 / Get the result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 获取精确模式的选择器变量；非精确模式返回 `None`。
    /// Get the selector variables in exact mode, or `None` in non-exact mode.
    pub fn selector_variables(&self) -> Option<&[BinaryVariableItem]> {
        self.binary_vars.as_deref()
    }

    /// 获取输入多项式 / Get the input polynomials.
    pub fn polynomials(&self) -> &[Linear<V>] {
        &self.polynomials
    }

    /// 是否启用精确选择约束 / Whether exact selector constraints are enabled.
    pub fn exact(&self) -> bool {
        self.binary_vars.is_some()
    }
}

impl<V> MinFunction<V>
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
    fn infer_big_m_from_tokens(&self, tokens: &[Token<V>]) -> Option<f64> {
        infer_big_m_for_polynomials(&self.polynomials, tokens, BIG_M_POLICY.min())
    }

    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_ms: &[f64],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "min result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        let mut constraints = Vec::new();
        for (i, polynomial) in self.polynomials.iter().enumerate() {
            let mut upper_monomials = Vec::with_capacity(polynomial.monomials().len() + 1);
            upper_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(1.0, "min result upper coefficient")?,
                result_index,
            ));
            for monomial in polynomial.monomials() {
                let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                    ModelError::InvalidConstraint(format!(
                        "min `{}` polynomial coefficient cannot be converted to f64",
                        self.id.name
                    ))
                })?;
                let polynomial_index = monomial.var_index();
                upper_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(-coefficient, "min polynomial upper coefficient")?,
                    polynomial_index,
                ));
            }
            let constant = to_f64(polynomial.constant_term()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "min `{}` polynomial constant cannot be converted to f64",
                    self.id.name
                ))
            })?;
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        upper_monomials,
                        convert_f64_to_v::<V>(-constant, "min upper constant")?,
                    ),
                    ConstraintRelation::LessEqual,
                    convert_f64_to_v::<V>(0.0, "min upper rhs")?,
                ),
                &format!("{}_min_ub_{}", self.id.name, i),
                Arc::new(self.clone()),
            ));

            if let Some(binary_vars) = &self.binary_vars {
                let selector_index = symbol_to_index
                    .get(&(binary_vars[i].id().unique_id() as usize))
                    .copied()
                    .ok_or_else(|| {
                        ModelError::SymbolNotRegistered(format!(
                            "min selector variable id {}",
                            binary_vars[i].id().unique_id()
                        ))
                    })?;
                let mut lower_monomials = Vec::with_capacity(polynomial.monomials().len() + 2);
                lower_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(1.0, "min result lower coefficient")?,
                    result_index,
                ));
                for monomial in polynomial.monomials() {
                    let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                        ModelError::InvalidConstraint(format!(
                            "min `{}` polynomial coefficient cannot be converted to f64",
                            self.id.name
                        ))
                    })?;
                    let polynomial_index = monomial.var_index();
                    lower_monomials.push(LinearMonomial::new(
                        convert_f64_to_v::<V>(-coefficient, "min polynomial lower coefficient")?,
                        polynomial_index,
                    ));
                }
                lower_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(-big_ms[i], "min selector lower coefficient")?,
                    selector_index,
                ));
                constraints.push(LinearConstraint::from_symbol(
                    LinearInequality::new(
                        Linear::new(
                            lower_monomials,
                            convert_f64_to_v::<V>(big_ms[i] - constant, "min lower constant")?,
                        ),
                        ConstraintRelation::GreaterEqual,
                        convert_f64_to_v::<V>(0.0, "min lower rhs")?,
                    ),
                    &format!("{}_min_lb_{}", self.id.name, i),
                    Arc::new(self.clone()),
                ));
            }
        }

        if let Some(binary_vars) = &self.binary_vars {
            let mut selector_sum_monomials = Vec::with_capacity(binary_vars.len());
            for selector in binary_vars {
                let selector_index = symbol_to_index
                    .get(&(selector.id().unique_id() as usize))
                    .copied()
                    .ok_or_else(|| {
                        ModelError::SymbolNotRegistered(format!(
                            "min selector variable id {}",
                            selector.id().unique_id()
                        ))
                    })?;
                selector_sum_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(1.0, "min selector sum coefficient")?,
                    selector_index,
                ));
            }
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        selector_sum_monomials,
                        convert_f64_to_v::<V>(0.0, "min selector sum constant")?,
                    ),
                    ConstraintRelation::Equal,
                    convert_f64_to_v::<V>(1.0, "min selector sum rhs")?,
                ),
                &format!("{}_min_selector_sum", self.id.name),
                Arc::new(self.clone()),
            ));
        }

        Ok(constraints)
    }
}

impl<V> Display for MinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "min({})", self.id.name)
    }
}

impl<V> DynSymbol for MinFunction<V>
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

impl<V> Symbol for MinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for MinFunction<V>
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

    fn refine_auxiliary_tokens(
        &self,
        auxiliary: &mut [Token<V>],
        tokens: &[Token<V>],
    ) -> Result<()> {
        // 非精确 MIN 只保留 hypograph，结果可以低于真实最小值，不能收紧下界。
        // The non-exact MIN keeps only the hypograph, so its result may stay below the
        // true minimum and the lower bound must not be tightened.
        if self.binary_vars.is_none() {
            return Ok(());
        }
        let Some((lower, upper)) =
            infer_extremum_result_bounds(&self.polynomials, tokens, true)
        else {
            return Ok(());
        };
        for token in auxiliary.iter_mut() {
            if token.id() == self.result_var.id() {
                tighten_token_range(token, lower, upper)?;
            }
        }
        Ok(())
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let big_ms = vec![BIG_M_POLICY.fallback(); self.polynomials.len()];
        self.build_mechanism_constraints(symbol_to_index, &big_ms)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let big_ms = infer_extremum_candidate_big_ms(
            &self.polynomials,
            tokens,
            true,
            BIG_M_POLICY.min(),
        )
        .unwrap_or_else(|| {
            vec![BIG_M_POLICY.resolve(self.infer_big_m_from_tokens(tokens)); self.polynomials.len()]
        });
        self.build_mechanism_constraints(symbol_to_index, &big_ms)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &std::collections::HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_var.index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("min({})", self.id.name)
    }

    fn deferred_structure_with_tokens(
        &self,
        tokens: &[Token<V>],
    ) -> Option<Arc<dyn crate::model::intermediate::DeferredFunctionStructure<V>>> {
        // 只对「带选择器的精确极值」暴露延迟结构：非精确 hypograph 形态语义不同
        // （不收紧结果列），继续即时展开。
        // Expose a deferred structure only for the exact selector form; the non-exact hypograph
        // form has different semantics (it does not tighten the result column) and keeps eager
        // expansion.
        if self.polynomials.is_empty() || self.binary_vars.is_none() {
            return None;
        }
        let big_ms = infer_extremum_candidate_big_ms(
            &self.polynomials,
            tokens,
            true,
            BIG_M_POLICY.min(),
        )
        .unwrap_or_else(|| {
            vec![BIG_M_POLICY.resolve(self.infer_big_m_from_tokens(tokens)); self.polynomials.len()]
        });
        Some(Arc::new(MinStructure::new(
            self.id.name.clone(),
            Arc::new(self.clone()),
            big_ms,
        )))
    }
}

/// MIN 的求解器无关结构描述 / Solver-neutral structure description of MIN
///
/// 与 [`MaxStructure`] 对称：只覆盖「带选择器列的精确极值」形态，持有产生它的符号与创建时按令牌
/// 边界推断出的每候选非对称 Big-M 数组，物化时回调手写路径的同一个公式生成器并传入同一组 M，
/// 因此延迟物化与 EAGER 展开逐行一致。
///
/// Symmetric to [`MaxStructure`]: it covers only the exact selector form and holds the symbol that
/// produced it together with the per-candidate asymmetric Big-M array inferred from token bounds at
/// creation time, materializing through the very same formula generator as the handwritten eager
/// path with that same array, so deferred materialization matches eager expansion row by row.
#[derive(Debug)]
pub struct MinStructure<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 函数名称 / Function name
    name: String,
    /// 产生本结构的符号 / Symbol that produced this structure
    symbol: Arc<MinFunction<V>>,
    /// 结果列 / Result column
    result: crate::variable::VariableId,
    /// 选择器辅助列 / Selector helper columns
    selectors: Vec<crate::variable::VariableId>,
    /// 每候选非对称 Big-M / Per-candidate asymmetric Big-M values
    big_ms: Vec<f64>,
}

impl<V> MinStructure<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建结构描述 / Create a structure description.
    pub fn new(name: impl Into<String>, symbol: Arc<MinFunction<V>>, big_ms: Vec<f64>) -> Self {
        let result = symbol.result_variable().id();
        let selectors = symbol
            .binary_vars
            .as_ref()
            .map(|vars| vars.iter().map(|var| var.id()).collect())
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

    /// 获取每候选非对称 Big-M / Get the per-candidate asymmetric Big-M values.
    pub fn big_ms(&self) -> &[f64] {
        &self.big_ms
    }

    /// 获取原始候选线性多项式 / Get the raw candidate linear polynomials.
    ///
    /// 供求解器无关的准入判定使用：原生 writer 的 planner 需要看到候选的真实形状，才能决定能否
    /// 写成 `result = min(operands) + constant`。
    /// Used by solver-neutral admission: a native writer's planner needs the candidates' real shape to
    /// decide whether `result = min(operands) + constant` can be written.
    pub fn candidate_polynomials(&self) -> &[Linear<V>] {
        &self.symbol.polynomials
    }
}

impl<V> crate::model::intermediate::DeferredFunctionStructure<V> for MinStructure<V>
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
        let big_ms = self
            .big_ms
            .iter()
            .map(|value| crate::model::intermediate::fingerprint_float(*value))
            .collect::<Vec<_>>()
            .join(",");
        let selectors = self
            .selectors
            .iter()
            .map(|selector| selector.unique_id().to_string())
            .collect::<Vec<_>>()
            .join(",");
        Some(format!(
            "min|{}|{}|{}|{}|{}",
            self.name,
            self.symbol.id.id,
            self.result.unique_id(),
            selectors,
            big_ms
        ))
    }

    fn materialize(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        // 复用即时展开的同一份生成器与同一组 Big-M，保证两条路径逐行一致。
        // Reuse the eager path's generator and the same Big-M values so both paths stay
        // row-identical.
        self.symbol
            .build_mechanism_constraints(symbol_to_index, &self.big_ms)
    }
}

impl<V> FunctionSymbol<V> for MinFunction<V>
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
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        if let Some(ref binary_vars) = self.binary_vars {
            for var in binary_vars {
                tokens.push(Token::from_generic(var.clone(), var.index()));
            }
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let mut min_value: Option<f64> = None;
        for polynomial in &self.polynomials {
            let value = to_f64(&evaluate_linear(polynomial, token_table, zero_if_none)?)?;
            min_value = Some(match min_value {
                Some(current) => current.min(value),
                None => value,
            });
        }
        match min_value {
            Some(v) => from_f64(v),
            None if zero_if_none => from_f64(0.0),
            None => None,
        }
    }
}

impl<V> LinearIntermediateSymbol<V> for MinFunction<V>
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
        Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                self.result_var.index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        )
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

/// Represents max(p1, p2, ..., pn).
#[derive(Debug, Clone)]
pub struct MaxFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    polynomials: Vec<Linear<V>>,
    result_var: ContinuousVariableItem,
    binary_vars: Option<Vec<BinaryVariableItem>>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> MaxFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建最大值函数 / Create a maximum function.
    pub fn new(id: u64, name: &str, polynomials: Vec<Linear<V>>, exact: bool) -> Self {
        let group_id = new_group_id();
        let result_var =
            ContinuousVariableItem::create(VariableId::new(group_id, 0), &format!("{}_max", name));

        let binary_vars = if exact {
            Some(
                (0..polynomials.len())
                    .map(|i| {
                        BinaryVariableItem::create(
                            VariableId::new(group_id, i + 1),
                            &format!("{}_u{}", name, i),
                        )
                    })
                    .collect(),
            )
        } else {
            None
        };

        Self {
            id: IntermediateSymbolId::new(id, name),
            polynomials,
            result_var,
            binary_vars,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置声明的依赖符号 ID / Set declared dependency symbol IDs.
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub(crate) fn with_polynomials(&self, polynomials: Vec<Linear<V>>) -> Self {
        let mut cloned = self.clone();
        cloned.polynomials = polynomials;
        cloned
    }

    pub(crate) fn mechanism_constraints_with_big_m(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<LinearConstraint<V>>>
    where
        V: Add<Output = V> + Mul<Output = V> + Zero + ToPrimitive + FromPrimitive,
        f64: IntoValue<V>,
    {
        let big_ms = vec![big_m; self.polynomials.len()];
        self.build_mechanism_constraints(symbol_to_index, &big_ms)
    }

    /// 获取结果变量 / Get the result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 获取精确模式的选择器变量；非精确模式返回 `None`。
    /// Get the selector variables in exact mode, or `None` in non-exact mode.
    pub fn selector_variables(&self) -> Option<&[BinaryVariableItem]> {
        self.binary_vars.as_deref()
    }

    /// 获取输入多项式 / Get the input polynomials.
    pub fn polynomials(&self) -> &[Linear<V>] {
        &self.polynomials
    }

    /// 是否启用精确选择约束 / Whether exact selector constraints are enabled.
    pub fn exact(&self) -> bool {
        self.binary_vars.is_some()
    }
}

impl<V> MaxFunction<V>
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
    fn infer_big_m_from_tokens(&self, tokens: &[Token<V>]) -> Option<f64> {
        infer_big_m_for_polynomials(&self.polynomials, tokens, BIG_M_POLICY.min())
    }

    /// 推断即时展开使用的每候选非对称 Big-M。
    ///
    /// 这是即时展开与延迟结构共用的唯一一份 M 计算：先按可见令牌边界推断每候选 M，任一候选缺少
    /// 有限域时退回统一回退值。转发到 MAX 的符号（例如范围松弛）在创建延迟结构时快照本方法的
    /// 返回值，物化阶段直接使用同一组 M，因此两条路径不可能各自解析出不同的 M。
    ///
    /// Infer the per-candidate asymmetric Big-M values eager expansion uses.
    ///
    /// This is the single M computation shared by eager expansion and deferred structures: it first
    /// infers a per-candidate M from the visible token bounds and falls back to the unified fallback
    /// value when any candidate lacks a finite domain. A symbol forwarding to MAX (for example
    /// slack-range) snapshots the returned group when it creates its deferred structure and reuses
    /// that exact group at materialization, so the two paths can never resolve different M values.
    pub(crate) fn eager_candidate_big_ms(&self, tokens: &[Token<V>]) -> Vec<f64> {
        infer_extremum_candidate_big_ms(&self.polynomials, tokens, false, BIG_M_POLICY.min())
            .unwrap_or_else(|| {
                vec![
                    BIG_M_POLICY.resolve(self.infer_big_m_from_tokens(tokens));
                    self.polynomials.len()
                ]
            })
    }

    /// 使用给定的每候选 Big-M 生成机制约束。
    ///
    /// 延迟结构在创建时快照了那组 M，物化时通过本方法复用与即时展开完全相同的公式生成器，
    /// 因此延迟物化与 EAGER 展开逐行一致（含 M 取值）。
    ///
    /// Build mechanism constraints from the given per-candidate Big-M values.
    ///
    /// A deferred structure snapshots the M group at creation time and reuses exactly the same formula
    /// generator as eager expansion through this method at materialization, so deferred
    /// materialization matches eager expansion row by row, including the M values.
    pub(crate) fn build_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_ms: &[f64],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "max result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        let mut constraints = Vec::new();
        for (i, polynomial) in self.polynomials.iter().enumerate() {
            let mut lower_monomials = Vec::with_capacity(polynomial.monomials().len() + 1);
            lower_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(1.0, "max result lower coefficient")?,
                result_index,
            ));
            for monomial in polynomial.monomials() {
                let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                    ModelError::InvalidConstraint(format!(
                        "max `{}` polynomial coefficient cannot be converted to f64",
                        self.id.name
                    ))
                })?;
                let polynomial_index = monomial.var_index();
                lower_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(-coefficient, "max polynomial lower coefficient")?,
                    polynomial_index,
                ));
            }
            let constant = to_f64(polynomial.constant_term()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "max `{}` polynomial constant cannot be converted to f64",
                    self.id.name
                ))
            })?;
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        lower_monomials,
                        convert_f64_to_v::<V>(-constant, "max lower constant")?,
                    ),
                    ConstraintRelation::GreaterEqual,
                    convert_f64_to_v::<V>(0.0, "max lower rhs")?,
                ),
                &format!("{}_max_lb_{}", self.id.name, i),
                Arc::new(self.clone()),
            ));

            if let Some(binary_vars) = &self.binary_vars {
                let selector_index = symbol_to_index
                    .get(&(binary_vars[i].id().unique_id() as usize))
                    .copied()
                    .ok_or_else(|| {
                        ModelError::SymbolNotRegistered(format!(
                            "max selector variable id {}",
                            binary_vars[i].id().unique_id()
                        ))
                    })?;
                let mut upper_monomials = Vec::with_capacity(polynomial.monomials().len() + 2);
                upper_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(1.0, "max result upper coefficient")?,
                    result_index,
                ));
                for monomial in polynomial.monomials() {
                    let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                        ModelError::InvalidConstraint(format!(
                            "max `{}` polynomial coefficient cannot be converted to f64",
                            self.id.name
                        ))
                    })?;
                    let polynomial_index = monomial.var_index();
                    upper_monomials.push(LinearMonomial::new(
                        convert_f64_to_v::<V>(-coefficient, "max polynomial upper coefficient")?,
                        polynomial_index,
                    ));
                }
                upper_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(big_ms[i], "max selector upper coefficient")?,
                    selector_index,
                ));
                constraints.push(LinearConstraint::from_symbol(
                    LinearInequality::new(
                        Linear::new(
                            upper_monomials,
                            convert_f64_to_v::<V>(-big_ms[i] - constant, "max upper constant")?,
                        ),
                        ConstraintRelation::LessEqual,
                        convert_f64_to_v::<V>(0.0, "max upper rhs")?,
                    ),
                    &format!("{}_max_ub_{}", self.id.name, i),
                    Arc::new(self.clone()),
                ));
            }
        }

        if let Some(binary_vars) = &self.binary_vars {
            let mut selector_sum_monomials = Vec::with_capacity(binary_vars.len());
            for selector in binary_vars {
                let selector_index = symbol_to_index
                    .get(&(selector.id().unique_id() as usize))
                    .copied()
                    .ok_or_else(|| {
                        ModelError::SymbolNotRegistered(format!(
                            "max selector variable id {}",
                            selector.id().unique_id()
                        ))
                    })?;
                selector_sum_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(1.0, "max selector sum coefficient")?,
                    selector_index,
                ));
            }
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        selector_sum_monomials,
                        convert_f64_to_v::<V>(0.0, "max selector sum constant")?,
                    ),
                    ConstraintRelation::Equal,
                    convert_f64_to_v::<V>(1.0, "max selector sum rhs")?,
                ),
                &format!("{}_max_selector_sum", self.id.name),
                Arc::new(self.clone()),
            ));
        }

        Ok(constraints)
    }
}

impl<V> Display for MaxFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "max({})", self.id.name)
    }
}

impl<V> DynSymbol for MaxFunction<V>
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

impl<V> Symbol for MaxFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for MaxFunction<V>
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

    fn refine_auxiliary_tokens(
        &self,
        auxiliary: &mut [Token<V>],
        tokens: &[Token<V>],
    ) -> Result<()> {
        // 非精确 MAX 只保留 epigraph，结果可以高于真实最大值，不能收紧上界。
        // The non-exact MAX keeps only the epigraph, so its result may stay above the
        // true maximum and the upper bound must not be tightened.
        if self.binary_vars.is_none() {
            return Ok(());
        }
        let Some((lower, upper)) =
            infer_extremum_result_bounds(&self.polynomials, tokens, false)
        else {
            return Ok(());
        };
        for token in auxiliary.iter_mut() {
            if token.id() == self.result_var.id() {
                tighten_token_range(token, lower, upper)?;
            }
        }
        Ok(())
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let big_ms = vec![BIG_M_POLICY.fallback(); self.polynomials.len()];
        self.build_mechanism_constraints(symbol_to_index, &big_ms)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let big_ms = self.eager_candidate_big_ms(tokens);
        self.build_mechanism_constraints(symbol_to_index, &big_ms)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &std::collections::HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_var.index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("max({})", self.id.name)
    }

    fn deferred_structure_with_tokens(
        &self,
        tokens: &[Token<V>],
    ) -> Option<Arc<dyn crate::model::intermediate::DeferredFunctionStructure<V>>> {
        // 只对「带选择器的精确极值」暴露延迟结构：非精确 epigraph/hypograph 形态语义不同
        // （不收紧结果列），继续即时展开。
        // Expose a deferred structure only for the exact selector form; the non-exact
        // epigraph/hypograph form has different semantics (it does not tighten the result column)
        // and keeps eager expansion.
        if self.polynomials.is_empty() || self.binary_vars.is_none() {
            return None;
        }
        let big_ms = self.eager_candidate_big_ms(tokens);
        Some(Arc::new(MaxStructure::new(
            self.id.name.clone(),
            Arc::new(self.clone()),
            big_ms,
        )))
    }
}

/// MAX 的求解器无关结构描述 / Solver-neutral structure description of MAX
///
/// 只覆盖「带选择器列的精确极值」形态。结构持有产生它的符号（`Arc`）与创建时按令牌边界推断出的
/// **每候选非对称 Big-M 数组**，物化时回调手写路径的同一个公式生成器并传入同一组 Big-M，因此
/// 延迟物化与 EAGER 展开逐行一致——包括 M 的取值本身，而不是仅结构相同。
///
/// Covers only the exact selector form. The structure holds the symbol that produced it (an `Arc`)
/// together with the per-candidate asymmetric Big-M array inferred from token bounds at creation
/// time, and materializes by calling the very same formula generator as the handwritten eager path
/// with that same array. Deferred materialization therefore matches eager expansion row by row,
/// including the M values themselves rather than only the structure.
#[derive(Debug)]
pub struct MaxStructure<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 函数名称 / Function name
    name: String,
    /// 产生本结构的符号 / Symbol that produced this structure
    symbol: Arc<MaxFunction<V>>,
    /// 结果列 / Result column
    result: crate::variable::VariableId,
    /// 选择器辅助列 / Selector helper columns
    selectors: Vec<crate::variable::VariableId>,
    /// 每候选非对称 Big-M / Per-candidate asymmetric Big-M values
    big_ms: Vec<f64>,
}

impl<V> MaxStructure<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建结构描述 / Create a structure description.
    pub fn new(name: impl Into<String>, symbol: Arc<MaxFunction<V>>, big_ms: Vec<f64>) -> Self {
        let result = symbol.result_variable().id();
        let selectors = symbol
            .binary_vars
            .as_ref()
            .map(|vars| vars.iter().map(|var| var.id()).collect())
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

    /// 获取每候选非对称 Big-M / Get the per-candidate asymmetric Big-M values.
    pub fn big_ms(&self) -> &[f64] {
        &self.big_ms
    }

    /// 获取原始候选线性多项式 / Get the raw candidate linear polynomials.
    ///
    /// 供求解器无关的准入判定使用：原生 writer 的 planner 需要看到候选的真实形状，才能决定能否
    /// 写成 `result = max(operands) + constant`。
    /// Used by solver-neutral admission: a native writer's planner needs the candidates' real shape to
    /// decide whether `result = max(operands) + constant` can be written.
    pub fn candidate_polynomials(&self) -> &[Linear<V>] {
        &self.symbol.polynomials
    }
}

impl<V> crate::model::intermediate::DeferredFunctionStructure<V> for MaxStructure<V>
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
        let big_ms = self
            .big_ms
            .iter()
            .map(|value| crate::model::intermediate::fingerprint_float(*value))
            .collect::<Vec<_>>()
            .join(",");
        let selectors = self
            .selectors
            .iter()
            .map(|selector| selector.unique_id().to_string())
            .collect::<Vec<_>>()
            .join(",");
        Some(format!(
            "max|{}|{}|{}|{}|{}",
            self.name,
            self.symbol.id.id,
            self.result.unique_id(),
            selectors,
            big_ms
        ))
    }

    fn materialize(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        // 复用即时展开的同一份生成器与同一组 Big-M，保证两条路径逐行一致。
        // Reuse the eager path's generator and the same Big-M values so both paths stay
        // row-identical.
        self.symbol
            .build_mechanism_constraints(symbol_to_index, &self.big_ms)
    }
}

impl<V> FunctionSymbol<V> for MaxFunction<V>
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
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        if let Some(ref binary_vars) = self.binary_vars {
            for var in binary_vars {
                tokens.push(Token::from_generic(var.clone(), var.index()));
            }
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let mut max_value: Option<f64> = None;
        for polynomial in &self.polynomials {
            let value = to_f64(&evaluate_linear(polynomial, token_table, zero_if_none)?)?;
            max_value = Some(match max_value {
                Some(current) => current.max(value),
                None => value,
            });
        }
        match max_value {
            Some(v) => from_f64(v),
            None if zero_if_none => from_f64(0.0),
            None => None,
        }
    }
}

impl<V> LinearIntermediateSymbol<V> for MaxFunction<V>
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
        Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                self.result_var.index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        )
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::variable::{ContinuousVariableItem, VariableRange};

    #[test]
    fn min_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(70_000),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let f: MinFunction<f64> = MinFunction::new(
            8000,
            "min_bound",
            vec![Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0)],
            true,
        );

        let selector = f
            .binary_vars
            .as_ref()
            .expect("exact min should have binary vars")[0]
            .clone();
        let result_id = f.result_variable().id().unique_id() as usize;
        let selector_id = selector.id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 2usize), (selector_id, 1usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(selector, 1),
            Token::from_generic(f.result_variable().clone(), 2),
        ];

        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("min constraints should be generated");
        let lower = constraints
            .iter()
            .find(|constraint| constraint.name == "min_bound_min_lb_0")
            .expect("min lower constraint should exist");
        let selector_term = lower
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 1)
            .expect("selector term should exist");

        // 2x + 1 且 x ∈ [-2, 3] => 取值范围 [-3, 7]。
        // MIN 的每候选 Big-M 为 upper_i - min(lower) = 7 - (-3) = 10。
        // 2x + 1 with x in [-2, 3] => range [-3, 7].
        // The per-candidate MIN Big-M is upper_i - min(lower) = 7 - (-3) = 10.
        assert!((*selector_term.coefficient() + 10.0).abs() <= 1e-9);
    }

    #[test]
    fn min_function_falls_back_to_default_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(70_010), "x");
        let f: MinFunction<f64> = MinFunction::new(
            8001,
            "min_default",
            vec![Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0)],
            true,
        );

        let selector = f
            .binary_vars
            .as_ref()
            .expect("exact min should have binary vars")[0]
            .clone();
        let result_id = f.result_variable().id().unique_id() as usize;
        let selector_id = selector.id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 2usize), (selector_id, 1usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(selector, 1),
            Token::from_generic(f.result_variable().clone(), 2),
        ];

        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("min constraints should be generated");
        let lower = constraints
            .iter()
            .find(|constraint| constraint.name == "min_default_min_lb_0")
            .expect("min lower constraint should exist");
        let selector_term = lower
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 1)
            .expect("selector term should exist");

        assert!((*selector_term.coefficient() + DEFAULT_BIG_M).abs() <= 1e-9);
    }

    #[test]
    fn max_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(71_000),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let f: MaxFunction<f64> = MaxFunction::new(
            8100,
            "max_bound",
            vec![Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0)],
            true,
        );

        let selector = f
            .binary_vars
            .as_ref()
            .expect("exact max should have binary vars")[0]
            .clone();
        let result_id = f.result_variable().id().unique_id() as usize;
        let selector_id = selector.id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 2usize), (selector_id, 1usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(selector, 1),
            Token::from_generic(f.result_variable().clone(), 2),
        ];

        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("max constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "max_bound_max_ub_0")
            .expect("max upper constraint should exist");
        let selector_term = upper
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 1)
            .expect("selector term should exist");

        // 2x + 1 且 x ∈ [-2, 3] => 取值范围 [-3, 7]。
        // MAX 的每候选 Big-M 为 max(upper) - lower_i = 7 - (-3) = 10。
        // 2x + 1 with x in [-2, 3] => range [-3, 7].
        // The per-candidate MAX Big-M is max(upper) - lower_i = 7 - (-3) = 10.
        assert!((*selector_term.coefficient() - 10.0).abs() <= 1e-9);
    }

    #[test]
    fn max_function_falls_back_to_default_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(71_010), "x");
        let f: MaxFunction<f64> = MaxFunction::new(
            8101,
            "max_default",
            vec![Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0)],
            true,
        );

        let selector = f
            .binary_vars
            .as_ref()
            .expect("exact max should have binary vars")[0]
            .clone();
        let result_id = f.result_variable().id().unique_id() as usize;
        let selector_id = selector.id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 2usize), (selector_id, 1usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(selector, 1),
            Token::from_generic(f.result_variable().clone(), 2),
        ];

        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("max constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "max_default_max_ub_0")
            .expect("max upper constraint should exist");
        let selector_term = upper
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 1)
            .expect("selector term should exist");

        assert!((*selector_term.coefficient() - DEFAULT_BIG_M).abs() <= 1e-9);
    }

    #[test]
    fn exact_extremum_propagates_finite_input_bounds_to_result_variable() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(72_000),
            "x",
            VariableRange::bounded(-10.0, -4.0),
        );
        let y = ContinuousVariableItem::with_range(
            VariableId::standalone(72_001),
            "y",
            VariableRange::bounded(-6.0, -2.0),
        );
        let tokens = vec![Token::from_generic(x, 0), Token::from_generic(y, 1)];

        let maximum: MaxFunction<f64> = MaxFunction::new(
            8200,
            "max_bounds",
            vec![
                Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
                Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0),
            ],
            true,
        );
        let mut auxiliary = Vec::new();
        maximum
            .register_auxiliary_tokens(&mut auxiliary)
            .expect("max auxiliary tokens should register");
        maximum
            .refine_auxiliary_tokens(&mut auxiliary, &tokens)
            .expect("max auxiliary bounds should refine");
        let result_token = auxiliary
            .iter()
            .find(|token| token.id() == maximum.result_variable().id())
            .expect("max result token should exist");
        // max([-10, -4], [-6, -2]) => [-6, -2]
        assert_eq!(result_token.variable.lower_bound(), Some(-6.0));
        assert_eq!(result_token.variable.upper_bound(), Some(-2.0));

        let minimum: MinFunction<f64> = MinFunction::new(
            8201,
            "min_bounds",
            vec![
                Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
                Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0),
            ],
            true,
        );
        let mut auxiliary = Vec::new();
        minimum
            .register_auxiliary_tokens(&mut auxiliary)
            .expect("min auxiliary tokens should register");
        minimum
            .refine_auxiliary_tokens(&mut auxiliary, &tokens)
            .expect("min auxiliary bounds should refine");
        let result_token = auxiliary
            .iter()
            .find(|token| token.id() == minimum.result_variable().id())
            .expect("min result token should exist");
        // min([-10, -4], [-6, -2]) => [-10, -4]
        assert_eq!(result_token.variable.lower_bound(), Some(-10.0));
        assert_eq!(result_token.variable.upper_bound(), Some(-4.0));
    }

    #[test]
    fn non_exact_extremum_keeps_result_bounds_untouched() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(72_010),
            "x",
            VariableRange::bounded(-10.0, -4.0),
        );
        let tokens = vec![Token::from_generic(x, 0)];

        let maximum: MaxFunction<f64> = MaxFunction::new(
            8300,
            "max_hypograph",
            vec![Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0)],
            false,
        );
        let mut auxiliary = Vec::new();
        maximum
            .register_auxiliary_tokens(&mut auxiliary)
            .expect("max auxiliary tokens should register");
        maximum
            .refine_auxiliary_tokens(&mut auxiliary, &tokens)
            .expect("non-exact max refinement must not fail");
        let result_token = auxiliary
            .iter()
            .find(|token| token.id() == maximum.result_variable().id())
            .expect("max result token should exist");
        assert_eq!(result_token.variable.lower_bound(), None);
        assert_eq!(result_token.variable.upper_bound(), None);

        let minimum: MinFunction<f64> = MinFunction::new(
            8301,
            "min_epigraph",
            vec![Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0)],
            false,
        );
        let mut auxiliary = Vec::new();
        minimum
            .register_auxiliary_tokens(&mut auxiliary)
            .expect("min auxiliary tokens should register");
        minimum
            .refine_auxiliary_tokens(&mut auxiliary, &tokens)
            .expect("non-exact min refinement must not fail");
        let result_token = auxiliary
            .iter()
            .find(|token| token.id() == minimum.result_variable().id())
            .expect("min result token should exist");
        assert_eq!(result_token.variable.lower_bound(), None);
        assert_eq!(result_token.variable.upper_bound(), None);
    }

    #[test]
    fn max_candidate_big_m_covers_the_candidate_spread() {
        let low = ContinuousVariableItem::with_range(
            VariableId::standalone(72_020),
            "low",
            VariableRange::fixed(-1.0),
        );
        let high = ContinuousVariableItem::with_range(
            VariableId::standalone(72_021),
            "high",
            VariableRange::fixed(1.0),
        );
        let f: MaxFunction<f64> = MaxFunction::new(
            8400,
            "max_spread",
            vec![
                Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
                Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0),
            ],
            true,
        );

        let selectors = f
            .binary_vars
            .as_ref()
            .expect("exact max should have binary vars")
            .clone();
        let result_id = f.result_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([
            (result_id, 3usize),
            (selectors[0].id().unique_id() as usize, 2usize),
            (selectors[1].id().unique_id() as usize, 4usize),
        ]);
        let tokens = vec![
            Token::from_generic(low, 0),
            Token::from_generic(high, 1),
            Token::from_generic(selectors[0].clone(), 2),
            Token::from_generic(f.result_variable().clone(), 3),
            Token::from_generic(selectors[1].clone(), 4),
        ];

        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("max constraints should be generated");
        let candidate_big_m = |name: &str| {
            constraints
                .iter()
                .find(|constraint| constraint.name == name)
                .expect("max upper constraint should exist")
                .inequality
                .polynomial
                .monomials()
                .iter()
                .find(|monomial| {
                    monomial.var_index() == 2usize || monomial.var_index() == 4usize
                })
                .map(|monomial| *monomial.coefficient())
                .expect("selector term should exist")
        };

        // 未选中的候选行必须覆盖 max(upper) - lower_i：-1 的候选需要 M = 1 - (-1) = 2。
        // The unselected candidate row must cover max(upper) - lower_i: the -1 candidate
        // needs M = 1 - (-1) = 2.
        assert!((candidate_big_m("max_spread_max_ub_0") - 2.0).abs() <= 1e-9);
        assert!((candidate_big_m("max_spread_max_ub_1") - 1.0).abs() <= 1e-9);
    }

    #[test]
    fn tied_extremum_inputs_keep_every_selector_feasible() {
        let first = ContinuousVariableItem::with_range(
            VariableId::standalone(73_000),
            "first",
            VariableRange::fixed(2.0),
        );
        let second = ContinuousVariableItem::with_range(
            VariableId::standalone(73_001),
            "second",
            VariableRange::fixed(2.0),
        );
        let tokens = vec![Token::from_generic(first, 0), Token::from_generic(second, 1)];

        // 并列极值时结果范围是一个点，任一候选都可以被选为极值来源。
        // With tied inputs the result range is a single point and either candidate may be
        // selected as the extremum source.
        let maximum: MaxFunction<f64> = MaxFunction::new(
            8500,
            "max_tie",
            vec![
                Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
                Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0),
            ],
            true,
        );
        let mut auxiliary = Vec::new();
        maximum
            .register_auxiliary_tokens(&mut auxiliary)
            .expect("max auxiliary tokens should register");
        maximum
            .refine_auxiliary_tokens(&mut auxiliary, &tokens)
            .expect("max auxiliary bounds should refine");
        let result_token = auxiliary
            .iter()
            .find(|token| token.id() == maximum.result_variable().id())
            .expect("max result token should exist");
        assert_eq!(result_token.variable.lower_bound(), Some(2.0));
        assert_eq!(result_token.variable.upper_bound(), Some(2.0));

        let minimum: MinFunction<f64> = MinFunction::new(
            8501,
            "min_tie",
            vec![
                Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
                Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0),
            ],
            true,
        );
        let mut auxiliary = Vec::new();
        minimum
            .register_auxiliary_tokens(&mut auxiliary)
            .expect("min auxiliary tokens should register");
        minimum
            .refine_auxiliary_tokens(&mut auxiliary, &tokens)
            .expect("min auxiliary bounds should refine");
        let result_token = auxiliary
            .iter()
            .find(|token| token.id() == minimum.result_variable().id())
            .expect("min result token should exist");
        assert_eq!(result_token.variable.lower_bound(), Some(2.0));
        assert_eq!(result_token.variable.upper_bound(), Some(2.0));
    }

    #[test]
    fn max_structure_materializes_the_same_rows_as_eager_expansion() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(96_000),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let max_fn: MaxFunction<f64> = MaxFunction::new(
            8601,
            "max_deferred",
            vec![Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0)],
            true,
        );
        let result_id = max_fn.result_variable().id().unique_id() as usize;
        let selector_ids: Vec<usize> = max_fn
            .selector_variables()
            .expect("exact max should have selectors")
            .iter()
            .map(|var| var.id().unique_id() as usize)
            .collect();
        let mut tokens = vec![Token::from_generic(x, 0)];
        let mut symbol_to_index = HashMap::new();
        for (offset, id) in selector_ids.iter().enumerate() {
            let var = max_fn.selector_variables().unwrap()[offset].clone();
            symbol_to_index.insert(*id, tokens.len());
            tokens.push(Token::from_generic(var, tokens.len()));
        }
        symbol_to_index.insert(result_id, tokens.len());
        tokens.push(Token::from_generic(
            max_fn.result_variable().clone(),
            tokens.len(),
        ));

        let structure = max_fn
            .deferred_structure_with_tokens(&tokens)
            .expect("exact max should expose a deferred structure");
        assert_eq!(structure.function_name(), "max_deferred");
        let binding = structure
            .usage_binding()
            .expect("max structure should expose a usage binding");
        assert_eq!(binding.result, max_fn.result_variable().id());
        assert_eq!(binding.helpers.len(), selector_ids.len());
        assert!(structure.fingerprint().is_some());

        let eager = max_fn
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("eager max constraints should be generated");
        let deferred = structure
            .materialize(&symbol_to_index)
            .expect("max structure should materialize");
        assert!(!eager.is_empty());
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

        // 非精确 epigraph 形态不暴露结构，继续即时展开。
        // The non-exact epigraph form exposes no structure and keeps eager expansion.
        let epigraph: MaxFunction<f64> = MaxFunction::new(
            8602,
            "max_epigraph",
            vec![Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0)],
            false,
        );
        assert!(
            epigraph
                .deferred_structure_with_tokens(&[Token::from_generic(
                    ContinuousVariableItem::with_range(
                        VariableId::standalone(96_001),
                        "x",
                        VariableRange::bounded(-2.0, 3.0),
                    ),
                    0,
                )])
                .is_none()
        );
    }

    #[test]
    fn min_structure_materializes_the_same_rows_as_eager_expansion() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(96_100),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let min_fn: MinFunction<f64> = MinFunction::new(
            8701,
            "min_deferred",
            vec![Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0)],
            true,
        );
        let result_id = min_fn.result_variable().id().unique_id() as usize;
        let selectors = min_fn
            .selector_variables()
            .expect("exact min should have selectors")
            .to_vec();
        let mut tokens = vec![Token::from_generic(x, 0)];
        let mut symbol_to_index = HashMap::new();
        for selector in &selectors {
            symbol_to_index.insert(selector.id().unique_id() as usize, tokens.len());
            tokens.push(Token::from_generic(selector.clone(), tokens.len()));
        }
        symbol_to_index.insert(result_id, tokens.len());
        tokens.push(Token::from_generic(
            min_fn.result_variable().clone(),
            tokens.len(),
        ));

        let structure = min_fn
            .deferred_structure_with_tokens(&tokens)
            .expect("exact min should expose a deferred structure");
        assert_eq!(structure.function_name(), "min_deferred");
        let binding = structure
            .usage_binding()
            .expect("min structure should expose a usage binding");
        assert_eq!(binding.result, min_fn.result_variable().id());
        assert_eq!(binding.helpers.len(), selectors.len());
        assert!(structure.fingerprint().is_some());

        let eager = min_fn
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("eager min constraints should be generated");
        let deferred = structure
            .materialize(&symbol_to_index)
            .expect("min structure should materialize");
        assert!(!eager.is_empty());
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

        // 非精确 hypograph 形态不暴露结构，继续即时展开。
        // The non-exact hypograph form exposes no structure and keeps eager expansion.
        let hypograph: MinFunction<f64> = MinFunction::new(
            8702,
            "min_hypograph",
            vec![Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0)],
            false,
        );
        assert!(
            hypograph
                .deferred_structure_with_tokens(&[Token::from_generic(
                    ContinuousVariableItem::with_range(
                        VariableId::standalone(96_101),
                        "x",
                        VariableRange::bounded(-2.0, 3.0),
                    ),
                    0,
                )])
                .is_none()
        );
    }

    #[test]
    fn explicit_extremum_big_m_is_used_verbatim() {
        let f: MaxFunction<f64> = MaxFunction::new(
            8600,
            "max_explicit",
            vec![Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0)],
            true,
        );

        let selector = f
            .binary_vars
            .as_ref()
            .expect("exact max should have binary vars")[0]
            .clone();
        let result_id = f.result_variable().id().unique_id() as usize;
        let selector_id = selector.id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 2usize), (selector_id, 1usize)]);

        // 显式小 M 必须原样使用，不被自动推断静默放大。
        // An explicit small M must be used verbatim and never silently widened by the
        // automatic inference.
        let constraints = f
            .mechanism_constraints_with_big_m(&symbol_to_index, 5.0)
            .expect("explicit max constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "max_explicit_max_ub_0")
            .expect("max upper constraint should exist");
        let selector_term = upper
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 1)
            .expect("selector term should exist");
        assert!((*selector_term.coefficient() - 5.0).abs() <= 1e-9);
    }
}
