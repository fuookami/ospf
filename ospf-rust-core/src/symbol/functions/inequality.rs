//! 不等式函数符号 / Inequality function symbol

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    auto_intermediate_symbol_name, next_auto_intermediate_symbol_id,
};
use super::big_m::infer_linear_shifted_abs_bound_from_tokens;
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{BinaryVariableItem, VariableId, new_group_id};
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;

const MIN_BIG_M: f64 = 1.0;
const INDICATOR_TOLERANCE: f64 = 1.0e-10;
const INDICATOR_STRICT_BOUNDARY: f64 = INDICATOR_TOLERANCE * 16.0 + f64::EPSILON * 16.0;

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

/// 不等式类型。
/// Inequality kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InequalityKind {
    /// 小于等于 / Less than or equal
    LessEqual,
    /// 大于等于 / Greater than or equal
    GreaterEqual,
    /// 严格小于 / Strictly less than
    Less,
    /// 严格大于 / Strictly greater than
    Greater,
    /// 等于 / Equal
    Equal,
    /// 不等于 / Not equal
    NotEqual,
}

/// 不等式指示函数，返回 0 或 1。
/// Represents an inequality condition, returns 0 or 1.
#[derive(Debug, Clone)]
pub struct InequalityFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    left: Linear<V>,
    right: V,
    kind: InequalityKind,
    result_var: BinaryVariableItem,
    side_var: Option<BinaryVariableItem>,
    big_m: V,
    declared_dependency_ids: Vec<u64>,
}

impl<V> InequalityFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建不等式指示函数。
    /// Create a new inequality indicator function.
    pub fn new(
        id: u64,
        name: &str,
        left: Linear<V>,
        right: V,
        kind: InequalityKind,
        big_m: V,
    ) -> Self {
        let group_id = new_group_id();
        let result_var = BinaryVariableItem::create(VariableId::new(group_id, 0), name);
        let side_var = if matches!(kind, InequalityKind::Equal | InequalityKind::NotEqual) {
            Some(BinaryVariableItem::create(
                VariableId::new(group_id, 1),
                &format!("{}_side", name),
            ))
        } else {
            None
        };

        Self {
            id: IntermediateSymbolId::new(id, name),
            left,
            right,
            kind,
            result_var,
            side_var,
            big_m,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 使用自动 ID 与调用方提供的名称创建不等式指示函数。
    /// Create an inequality indicator function with an auto id and caller-provided name.
    pub fn named(
        name: impl AsRef<str>,
        left: Linear<V>,
        right: V,
        kind: InequalityKind,
        big_m: V,
    ) -> Self {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            left,
            right,
            kind,
            big_m,
        )
    }

    /// 使用自动 ID 与自动名称创建不等式指示函数。
    /// Create an inequality indicator function with an auto id and auto-generated name.
    pub fn auto(left: Linear<V>, right: V, kind: InequalityKind, big_m: V) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("inequality", id);
        Self::new(id, &name, left, right, kind, big_m)
    }

    /// 创建 `<=` 指示函数。
    /// Create a `<=` indicator function.
    pub fn less_equal(id: u64, name: &str, left: Linear<V>, right: V, big_m: V) -> Self {
        Self::new(id, name, left, right, InequalityKind::LessEqual, big_m)
    }

    /// 使用自动 ID 与调用方提供的名称创建 `<=` 指示函数。
    /// Create a `<=` indicator function with an auto id and caller-provided name.
    pub fn named_less_equal(name: impl AsRef<str>, left: Linear<V>, right: V, big_m: V) -> Self {
        Self::less_equal(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            left,
            right,
            big_m,
        )
    }

    /// 使用自动 ID 与自动名称创建 `<=` 指示函数。
    /// Create a `<=` indicator function with an auto id and auto-generated name.
    pub fn auto_less_equal(left: Linear<V>, right: V, big_m: V) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("less_equal", id);
        Self::less_equal(id, &name, left, right, big_m)
    }

    /// 创建 `>=` 指示函数。
    /// Create a `>=` indicator function.
    pub fn greater_equal(id: u64, name: &str, left: Linear<V>, right: V, big_m: V) -> Self {
        Self::new(id, name, left, right, InequalityKind::GreaterEqual, big_m)
    }

    /// 使用自动 ID 与调用方提供的名称创建 `>=` 指示函数。
    /// Create a `>=` indicator function with an auto id and caller-provided name.
    pub fn named_greater_equal(name: impl AsRef<str>, left: Linear<V>, right: V, big_m: V) -> Self {
        Self::greater_equal(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            left,
            right,
            big_m,
        )
    }

    /// 使用自动 ID 与自动名称创建 `>=` 指示函数。
    /// Create a `>=` indicator function with an auto id and auto-generated name.
    pub fn auto_greater_equal(left: Linear<V>, right: V, big_m: V) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("greater_equal", id);
        Self::greater_equal(id, &name, left, right, big_m)
    }

    /// 设置声明的依赖符号 ID / Set declared dependency symbol IDs.
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub(crate) fn with_left_polynomial(&self, left: Linear<V>) -> Self {
        let mut cloned = self.clone();
        cloned.left = left;
        cloned
    }

    pub(crate) fn with_big_m_value(&self, big_m: V) -> Self {
        let mut cloned = self.clone();
        cloned.big_m = big_m;
        cloned
    }

    /// 获取结果变量 / Get the result variable.
    pub fn result_variable(&self) -> &BinaryVariableItem {
        &self.result_var
    }

    /// 获取左侧多项式 / Get the left-hand polynomial.
    pub fn left_polynomial(&self) -> &Linear<V> {
        &self.left
    }

    /// 获取右侧值 / Get the right-hand value.
    pub fn right_value(&self) -> &V {
        &self.right
    }

    /// 获取不等式类型 / Get the inequality kind.
    pub fn inequality_kind(&self) -> InequalityKind {
        self.kind
    }

    /// 获取 Big-M 值 / Get the Big-M value.
    pub fn big_m(&self) -> &V {
        &self.big_m
    }
}

impl<V> InequalityFunction<V>
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
    fn configured_big_m(&self) -> Result<f64> {
        let big_m = to_f64(&self.big_m).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "inequality `{}` big-M cannot be converted to f64",
                self.id.name
            ))
        })?;
        if !big_m.is_finite() || big_m <= 0.0 {
            return Err(ModelError::InvalidConstraint(format!(
                "inequality `{}` requires positive finite big-M for mechanism constraint injection",
                self.id.name
            ))
            .into());
        }
        Ok(big_m)
    }

    fn infer_big_m_from_tokens(&self, tokens: &[Token<V>]) -> Option<f64> {
        infer_linear_shifted_abs_bound_from_tokens(&self.left, &self.right, tokens)
            .map(|big_m| big_m.max(MIN_BIG_M))
    }

    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let result_symbol_id = self.result_var.id().unique_id() as usize;
        let result_index = symbol_to_index
            .get(&result_symbol_id)
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "inequality result variable id {}",
                    result_symbol_id
                ))
            })?;

        let right = to_f64(&self.right).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "inequality `{}` rhs cannot be converted to f64",
                self.id.name
            ))
        })?;
        let tolerance = INDICATOR_TOLERANCE;
        let strict_boundary = INDICATOR_STRICT_BOUNDARY;

        let mut monomials = Vec::with_capacity(self.left.monomials().len() + 1);
        for monomial in self.left.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "inequality `{}` lhs coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            let left_index = monomial.var_index();
            monomials.push((coefficient, left_index));
        }
        let left_constant = to_f64(self.left.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "inequality `{}` lhs constant cannot be converted to f64",
                self.id.name
            ))
        })?;
        let shifted_constant = left_constant - right;

        let side_index = self
            .side_var
            .as_ref()
            .map(|var| {
                symbol_to_index
                    .get(&(var.id().unique_id() as usize))
                    .copied()
                    .ok_or_else(|| {
                        ModelError::SymbolNotRegistered(format!(
                            "inequality side variable id {}",
                            var.id().unique_id()
                        ))
                    })
            })
            .transpose()?;

        let build_constraint = |name_suffix: &str,
                                relation: ConstraintRelation,
                                rhs: f64,
                                y_coefficient: f64,
                                side_coefficient: Option<f64>|
         -> Result<LinearConstraint<V>> {
            let mut linear_monomials = Vec::with_capacity(monomials.len() + 2);
            for (coefficient, index) in &monomials {
                linear_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(*coefficient, "inequality lhs coefficient")?,
                    *index,
                ));
            }
            linear_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(y_coefficient, "inequality y coefficient")?,
                result_index,
            ));

            if let Some(side_coefficient) = side_coefficient {
                let side_index = side_index.ok_or_else(|| {
                    ModelError::InvalidConstraint(format!(
                        "inequality `{}` kind {:?} requires auxiliary side variable for mechanism injection",
                        self.id.name, self.kind
                    ))
                })?;
                linear_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(side_coefficient, "inequality side coefficient")?,
                    side_index,
                ));
            }

            let polynomial = Linear::new(
                linear_monomials,
                convert_f64_to_v::<V>(shifted_constant, "inequality constant")?,
            );
            Ok(LinearConstraint::from_symbol(
                LinearInequality::new(
                    polynomial,
                    relation,
                    convert_f64_to_v::<V>(rhs, "inequality rhs")?,
                ),
                &format!("{}_{}", self.id.name, name_suffix),
                Arc::new(self.clone()),
            ))
        };

        match self.kind {
            InequalityKind::LessEqual => Ok(vec![
                build_constraint(
                    "ineq_lb",
                    ConstraintRelation::GreaterEqual,
                    tolerance,
                    big_m,
                    None,
                )?,
                build_constraint("ineq_ub", ConstraintRelation::LessEqual, big_m, big_m, None)?,
            ]),
            InequalityKind::GreaterEqual => Ok(vec![
                build_constraint(
                    "ineq_lb",
                    ConstraintRelation::GreaterEqual,
                    -big_m,
                    -big_m,
                    None,
                )?,
                build_constraint(
                    "ineq_ub",
                    ConstraintRelation::LessEqual,
                    -tolerance,
                    -big_m,
                    None,
                )?,
            ]),
            InequalityKind::Less => Ok(vec![
                build_constraint(
                    "ineq_lb",
                    ConstraintRelation::GreaterEqual,
                    0.0,
                    big_m,
                    None,
                )?,
                build_constraint(
                    "ineq_ub",
                    ConstraintRelation::LessEqual,
                    big_m - tolerance,
                    big_m,
                    None,
                )?,
            ]),
            InequalityKind::Greater => Ok(vec![
                build_constraint(
                    "ineq_lb",
                    ConstraintRelation::GreaterEqual,
                    tolerance - big_m,
                    -big_m,
                    None,
                )?,
                build_constraint("ineq_ub", ConstraintRelation::LessEqual, 0.0, -big_m, None)?,
            ]),
            InequalityKind::Equal => Ok(vec![
                build_constraint(
                    "ineq_eq_band_ub",
                    ConstraintRelation::LessEqual,
                    tolerance + big_m,
                    big_m,
                    None,
                )?,
                build_constraint(
                    "ineq_eq_band_lb",
                    ConstraintRelation::GreaterEqual,
                    -tolerance - big_m,
                    -big_m,
                    None,
                )?,
                build_constraint(
                    "ineq_eq_out_lb",
                    ConstraintRelation::GreaterEqual,
                    strict_boundary - big_m,
                    big_m,
                    Some(-big_m),
                )?,
                build_constraint(
                    "ineq_eq_out_ub",
                    ConstraintRelation::LessEqual,
                    -strict_boundary,
                    -big_m,
                    Some(-big_m),
                )?,
            ]),
            InequalityKind::NotEqual => Ok(vec![
                build_constraint(
                    "ineq_neq_band_ub",
                    ConstraintRelation::LessEqual,
                    tolerance,
                    -big_m,
                    None,
                )?,
                build_constraint(
                    "ineq_neq_band_lb",
                    ConstraintRelation::GreaterEqual,
                    -tolerance,
                    big_m,
                    None,
                )?,
                build_constraint(
                    "ineq_neq_out_lb",
                    ConstraintRelation::GreaterEqual,
                    strict_boundary - 2.0 * big_m,
                    -big_m,
                    Some(-big_m),
                )?,
                build_constraint(
                    "ineq_neq_out_ub",
                    ConstraintRelation::LessEqual,
                    -strict_boundary + big_m,
                    big_m,
                    Some(-big_m),
                )?,
            ]),
        }
    }
}

impl<V> Display for InequalityFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ineq({})", self.id.name)
    }
}

impl<V> DynSymbol for InequalityFunction<V>
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

impl<V> Symbol for InequalityFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for InequalityFunction<V>
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
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.build_mechanism_constraints(symbol_to_index, self.configured_big_m()?)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let big_m = match self.infer_big_m_from_tokens(tokens) {
            Some(inferred) => inferred,
            None => self.configured_big_m()?,
        };
        self.build_mechanism_constraints(symbol_to_index, big_m)
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
        format!("ineq({})", self.id.name)
    }

    fn deferred_structure_with_tokens(
        &self,
        tokens: &[Token<V>],
    ) -> Option<Arc<dyn crate::model::intermediate::DeferredFunctionStructure<V>>> {
        // Big-M 必须在结构创建时固定：优先令牌边界推断值，取不到时用配置值；两者都不可用时
        // 不提供结构，让即时展开给出配置错误，而不是把错误推迟到物化阶段。
        // The Big-M must be fixed at creation time: the token-inferred value first, the configured
        // value second; when neither is available no structure is offered so eager expansion
        // surfaces the configuration error instead of deferring it to materialization.
        let big_m = match self.infer_big_m_from_tokens(tokens) {
            Some(inferred) => inferred,
            None => self.configured_big_m().ok()?,
        };
        Some(Arc::new(InequalityStructure::new(
            self.id.name.clone(),
            Arc::new(self.clone()),
            big_m,
        )))
    }
}

/// 关系指示（不等式/等式）的求解器无关结构描述
/// Solver-neutral structure description of the relation indicator
///
/// 与二值化采用同一模式：持有产生它的符号与创建时固定的 Big-M，物化时回调手写路径的同一个公式
/// 生成器并传入同一个 M，因此延迟物化与 EAGER 展开逐行一致（含 M 取值）。关系指示没有辅助列，
/// 结构只涉及结果列。
///
/// Follows the same pattern as binaryzation: the structure holds the symbol that produced it and the
/// Big-M fixed at creation time, materializing through the very same formula generator as the
/// handwritten eager path with that same M, so deferred materialization matches eager expansion row
/// by row, including the M value. The relation indicator has no helper columns, so the structure
/// only involves the result column.
#[derive(Debug)]
pub struct InequalityStructure<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 函数名称 / Function name
    name: String,
    /// 产生本结构的符号 / Symbol that produced this structure
    symbol: Arc<InequalityFunction<V>>,
    /// 结果列 / Result column
    result: crate::variable::VariableId,
    /// 创建时固定的 Big-M / Big-M fixed at creation time
    big_m: f64,
}

impl<V> InequalityStructure<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建结构描述 / Create a structure description.
    pub fn new(name: impl Into<String>, symbol: Arc<InequalityFunction<V>>, big_m: f64) -> Self {
        let result = symbol.result_variable().id();
        Self {
            name: name.into(),
            symbol,
            result,
            big_m,
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

    /// 获取固定的 Big-M / Get the fixed Big-M.
    pub fn big_m(&self) -> f64 {
        self.big_m
    }
}

impl<V> crate::model::intermediate::DeferredFunctionStructure<V> for InequalityStructure<V>
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
        // 关系指示没有辅助列，只上报结果列。
        // The relation indicator has no helper columns, so only the result column is reported.
        Some(crate::model::intermediate::StructureUsageBinding::new(
            self.symbol.id.id,
            self.result.clone(),
            Vec::new(),
        ))
    }

    fn fingerprint(&self) -> Option<String> {
        Some(format!(
            "inequality|{}|{}|{}|{}",
            self.name,
            self.symbol.id.id,
            self.result.unique_id(),
            crate::model::intermediate::fingerprint_float(self.big_m)
        ))
    }

    fn materialize(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        // 复用即时展开的同一份生成器与同一个 Big-M，保证两条路径逐行一致。
        // Reuse the eager path's generator and the same Big-M so both paths stay row-identical.
        self.symbol
            .build_mechanism_constraints(symbol_to_index, self.big_m)
    }
}

impl<V> FunctionSymbol<V> for InequalityFunction<V>
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
        if let Some(side_var) = &self.side_var {
            tokens.push(Token::from_generic(side_var.clone(), side_var.index()));
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let left = to_f64(&evaluate_linear(&self.left, token_table, zero_if_none)?)?;
        let right = to_f64(&self.right)?;
        let eps = INDICATOR_TOLERANCE;

        let satisfied = match self.kind {
            InequalityKind::LessEqual => left <= right + eps,
            InequalityKind::GreaterEqual => left + eps >= right,
            InequalityKind::Less => left < right - eps,
            InequalityKind::Greater => left > right + eps,
            InequalityKind::Equal => (left - right).abs() <= eps,
            InequalityKind::NotEqual => (left - right).abs() > eps,
        };
        from_f64(if satisfied { 1.0 } else { 0.0 })
    }
}

impl<V> LinearIntermediateSymbol<V> for InequalityFunction<V>
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
    use crate::model::{ConstraintRelation, LinearConstraint};
    use crate::token::{MutableTokenList, Token, VecTokenList};
    use crate::variable::{ContinuousVariableItem, VariableRange};

    fn constraint_lhs(constraint: &LinearConstraint<f64>, values: &HashMap<usize, f64>) -> f64 {
        let mut lhs = *constraint.inequality.polynomial.constant_term();
        for monomial in constraint.inequality.polynomial.monomials() {
            lhs += *monomial.coefficient()
                * values
                    .get(&monomial.var_index())
                    .copied()
                    .unwrap_or_default();
        }
        lhs
    }

    fn satisfies(constraint: &LinearConstraint<f64>, values: &HashMap<usize, f64>) -> bool {
        let lhs = constraint_lhs(constraint, values);
        match constraint.inequality.relation {
            ConstraintRelation::LessEqual => lhs <= constraint.inequality.rhs + 1e-9,
            ConstraintRelation::Equal => (lhs - constraint.inequality.rhs).abs() <= 1e-9,
            ConstraintRelation::GreaterEqual => lhs + 1e-9 >= constraint.inequality.rhs,
        }
    }

    #[test]
    fn inequality_structure_materializes_the_same_rows_as_eager_expansion() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(40_100),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let f: InequalityFunction<f64> = InequalityFunction::less_equal(
            5001,
            "ineq_deferred",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
            0.0,
            100.0,
        );
        let result_id = f.result_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(f.result_variable().clone(), 1),
        ];

        let structure = f
            .deferred_structure_with_tokens(&tokens)
            .expect("inequality should expose a deferred structure");
        assert_eq!(structure.function_name(), "ineq_deferred");
        let binding = structure
            .usage_binding()
            .expect("inequality structure should expose a usage binding");
        assert_eq!(binding.result, f.result_variable().id());
        assert!(binding.helpers.is_empty());
        assert!(structure.fingerprint().is_some());

        let eager = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("eager inequality constraints should be generated");
        let deferred = structure
            .materialize(&symbol_to_index)
            .expect("inequality structure should materialize");
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

        // 没有令牌边界时退回配置 Big-M，两条路径仍一致。
        // Without token bounds the configured Big-M is used and both paths still agree.
        let no_tokens_structure = f
            .deferred_structure_with_tokens(&[])
            .expect("inequality should fall back to the configured big-M");
        let eager_default =
            <InequalityFunction<f64> as IntermediateSymbol<f64>>::mechanism_constraints(
                &f,
                &symbol_to_index,
            )
            .expect("eager inequality constraints should be generated");
        let deferred_default = no_tokens_structure
            .materialize(&symbol_to_index)
            .expect("inequality structure should materialize");
        assert_eq!(eager_default.len(), deferred_default.len());
        assert_eq!(eager_default[0].name, deferred_default[0].name);
    }

    #[test]
    fn inequality_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(40_000),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let f: InequalityFunction<f64> = InequalityFunction::less_equal(
            5000,
            "ineq_bound",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
            0.0,
            100.0,
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(f.result_variable().clone(), 1),
        ];

        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("inequality constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "ineq_bound_ineq_ub")
            .expect("upper inequality constraint should exist");
        let y_term = upper
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 1)
            .expect("inequality y term should exist");

        // 2x + 1 with x in [-2, 3] => range [-3, 7], rhs=0 => M = 7.
        assert!((upper.inequality.rhs - 7.0).abs() <= 1e-9);
        assert!((*y_term.coefficient() - 7.0).abs() <= 1e-9);
    }

    #[test]
    fn inequality_function_falls_back_to_configured_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(40_010), "x");
        let f: InequalityFunction<f64> = InequalityFunction::less_equal(
            5001,
            "ineq_default",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            0.0,
            11.0,
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(f.result_variable().clone(), 1),
        ];

        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("inequality constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "ineq_default_ineq_ub")
            .expect("upper inequality constraint should exist");
        let y_term = upper
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 1)
            .expect("inequality y term should exist");

        assert!((upper.inequality.rhs - 11.0).abs() <= 1e-9);
        assert!((*y_term.coefficient() - 11.0).abs() <= 1e-9);
    }

    #[test]
    fn equality_indicator_treats_zero_difference_as_satisfied() {
        let x = ContinuousVariableItem::create(VariableId::standalone(40_020), "x");
        let f: InequalityFunction<f64> = InequalityFunction::new(
            5002,
            "ineq_eq_zero",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            0.0,
            InequalityKind::Equal,
            10.0,
        );

        let tx = Token::from_generic(x, 0);
        tx.set_result(0.0);
        let mut tokens = VecTokenList::new();
        tokens.add_token(tx);
        assert_eq!(
            <InequalityFunction as FunctionSymbol>::calculate_value(&f, &tokens, false),
            Some(1.0)
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let side_id = f
            .side_var
            .as_ref()
            .expect("equal inequality should have a side variable")
            .id()
            .unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (side_id, 2usize)]);
        let constraints = f
            .mechanism_constraints(&symbol_to_index)
            .expect("equal inequality constraints should be generated");

        let satisfied = HashMap::from([(0usize, 0.0), (1usize, 1.0), (2usize, 0.0)]);
        assert!(
            constraints
                .iter()
                .all(|constraint| satisfies(constraint, &satisfied))
        );

        let zero_result_side0 = HashMap::from([(0usize, 0.0), (1usize, 0.0), (2usize, 0.0)]);
        let zero_result_side1 = HashMap::from([(0usize, 0.0), (1usize, 0.0), (2usize, 1.0)]);
        assert!(
            !constraints
                .iter()
                .all(|constraint| satisfies(constraint, &zero_result_side0))
        );
        assert!(
            !constraints
                .iter()
                .all(|constraint| satisfies(constraint, &zero_result_side1))
        );
    }

    #[test]
    fn equality_indicator_treats_nonzero_difference_as_violated() {
        let x = ContinuousVariableItem::create(VariableId::standalone(40_030), "x");
        let f: InequalityFunction<f64> = InequalityFunction::new(
            5003,
            "ineq_eq_nonzero",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            0.0,
            InequalityKind::Equal,
            10.0,
        );

        let tx = Token::from_generic(x, 0);
        tx.set_result(1.0e-6);
        let mut tokens = VecTokenList::new();
        tokens.add_token(tx);
        assert_eq!(
            <InequalityFunction as FunctionSymbol>::calculate_value(&f, &tokens, false),
            Some(0.0)
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let side_id = f
            .side_var
            .as_ref()
            .expect("equal inequality should have a side variable")
            .id()
            .unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (side_id, 2usize)]);
        let constraints = f
            .mechanism_constraints(&symbol_to_index)
            .expect("equal inequality constraints should be generated");

        let violated = HashMap::from([(0usize, 1.0e-6), (1usize, 0.0), (2usize, 1.0)]);
        assert!(
            constraints
                .iter()
                .all(|constraint| satisfies(constraint, &violated))
        );

        let wrong_result = HashMap::from([(0usize, 1.0e-6), (1usize, 1.0), (2usize, 0.0)]);
        assert!(
            !constraints
                .iter()
                .all(|constraint| satisfies(constraint, &wrong_result))
        );
    }
}
