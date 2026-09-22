//! 逻辑函数符号 / Logic function symbols

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    LogicFunctionSymbol, auto_intermediate_symbol_name, next_auto_intermediate_symbol_id,
};
use super::big_m::{BigMPolicy, infer_big_m_for_polynomials, infer_linear_abs_bound_from_tokens};
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{BinaryVariableItem, VariableId, VariableType, new_group_id};
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

fn as_binary(value: f64) -> f64 {
    if value.abs() <= f64::EPSILON {
        0.0
    } else {
        1.0
    }
}

const DEFAULT_BIG_M: f64 = 1_000_000.0;
const BIG_M_POLICY: BigMPolicy = BigMPolicy::new(DEFAULT_BIG_M, 1.0);
/// Shared zero-band width used by Kotlin and Rust nonzero indicators.
pub(crate) const NONZERO_TOLERANCE: f64 = 1e-10;
/// Shared strict boundary used by Kotlin and Rust nonzero indicators.
pub(crate) const STRICT_NONZERO_BOUNDARY: f64 = NONZERO_TOLERANCE * 16.0 + f64::EPSILON * 16.0;

/// 判断多项式是否为"系数 1、常数 0"的单项式，并返回其变量索引。
///
/// 该结构判定不检查令牌类型，供缺少令牌上下文时与注册阶段保持一致。
///
/// Determine whether a polynomial is a single monomial with coefficient 1 and constant 0,
/// returning its variable index. This structural check does not inspect the token type; it
/// keeps the constraint builder consistent with registration when no token context exists.
fn direct_monomial_index<V>(polynomial: &Linear<V>) -> Option<usize>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    if polynomial.monomials().len() != 1 {
        return None;
    }
    if to_f64(polynomial.constant_term())? != 0.0 {
        return None;
    }
    let monomial = polynomial.monomials().first()?;
    if to_f64(monomial.coefficient())? != 1.0 {
        return None;
    }
    Some(monomial.var_index())
}

/// 判断多项式是否为直接二值变量输入，并返回其求解器索引。
///
/// 只有"系数 1、常数 0 的单项式"且对应令牌确实是二值变量时才成立。直接二值输入可以
/// 使用 AND/OR/NOT 的紧凑 hull，不再需要非零指示变量与正负侧辅助变量。
///
/// Determine whether a polynomial is a direct binary variable input, returning its solver
/// index. This holds only for a single monomial with coefficient 1 and constant 0 whose
/// token really is a binary variable. Direct binary inputs can use the compact AND/OR/NOT
/// hull and no longer need nonzero indicator or positive/negative side helper variables.
fn direct_binary_input_index<V>(polynomial: &Linear<V>, tokens: &[Token<V>]) -> Option<usize>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    let index = direct_monomial_index(polynomial)?;
    let token = tokens.iter().find(|token| token.solver_index == index)?;
    if token.var_type() != VariableType::Binary {
        return None;
    }
    Some(index)
}

fn nonzero_indicator_inequalities<V>(
    polynomial: &Linear<V>,
    indicator_index: usize,
    side_index: usize,
    big_m: f64,
    name_prefix: &str,
) -> Result<Vec<(LinearInequality<V>, String)>>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    nonzero_indicator_inequalities_with_policy(
        polynomial,
        indicator_index,
        side_index,
        big_m,
        NONZERO_TOLERANCE,
        STRICT_NONZERO_BOUNDARY,
        name_prefix,
    )
}

fn nonzero_indicator_inequalities_with_policy<V>(
    polynomial: &Linear<V>,
    indicator_index: usize,
    side_index: usize,
    big_m: f64,
    tolerance: f64,
    strict_boundary: f64,
    name_prefix: &str,
) -> Result<Vec<(LinearInequality<V>, String)>>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    if !tolerance.is_finite() || tolerance < 0.0 {
        return Err(ModelError::InvalidConstraint(format!(
            "logic `{}` tolerance must be finite and non-negative",
            name_prefix
        ))
        .into());
    }
    if !strict_boundary.is_finite() || strict_boundary <= 0.0 {
        return Err(ModelError::InvalidConstraint(format!(
            "logic `{}` strict boundary must be finite and positive",
            name_prefix
        ))
        .into());
    }
    if !big_m.is_finite() || big_m <= 0.0 {
        return Err(ModelError::InvalidConstraint(format!(
            "logic `{}` Big-M must be finite and positive",
            name_prefix
        ))
        .into());
    }
    // The input bound is the base magnitude.  Add the corresponding
    // transition margin to both branches so an exactly inferred bound still
    // admits the strict boundary (for example, |x| = 1 with M = 1).
    let band_m = big_m + tolerance;
    let out_m = big_m + strict_boundary;
    if !band_m.is_finite() || !out_m.is_finite() {
        return Err(ModelError::InvalidConstraint(format!(
            "logic `{}` Big-M is too large for the configured margins",
            name_prefix
        ))
        .into());
    }
    let mut constraints = Vec::with_capacity(4);
    let mut base_monomials = Vec::with_capacity(polynomial.monomials().len());
    for monomial in polynomial.monomials() {
        let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "logic `{}` input coefficient cannot be converted to f64",
                name_prefix
            ))
        })?;
        base_monomials.push((coefficient, monomial.var_index()));
    }
    let constant = to_f64(polynomial.constant_term()).ok_or_else(|| {
        ModelError::InvalidConstraint(format!(
            "logic `{}` input constant cannot be converted to f64",
            name_prefix
        ))
    })?;

    let mut ub_monomials = Vec::with_capacity(polynomial.monomials().len() + 1);
    for (coefficient, index) in &base_monomials {
        ub_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(*coefficient, "logic input coefficient")?,
            *index,
        ));
    }
    ub_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(-band_m, "logic indicator coefficient")?,
        indicator_index,
    ));
    constraints.push((
        LinearInequality::new(
            Linear::new(
                ub_monomials,
                convert_f64_to_v::<V>(constant, "logic input constant")?,
            ),
            ConstraintRelation::LessEqual,
            convert_f64_to_v::<V>(tolerance, "logic rhs")?,
        ),
        format!("{}_band_ub", name_prefix),
    ));

    let mut lb_monomials = Vec::with_capacity(polynomial.monomials().len() + 1);
    for (coefficient, index) in &base_monomials {
        lb_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(*coefficient, "logic input coefficient")?,
            *index,
        ));
    }
    lb_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(band_m, "logic indicator coefficient")?,
        indicator_index,
    ));
    constraints.push((
        LinearInequality::new(
            Linear::new(
                lb_monomials,
                convert_f64_to_v::<V>(constant, "logic input constant")?,
            ),
            ConstraintRelation::GreaterEqual,
            convert_f64_to_v::<V>(-tolerance, "logic rhs")?,
        ),
        format!("{}_band_lb", name_prefix),
    ));

    let mut out_lb_monomials = Vec::with_capacity(polynomial.monomials().len() + 2);
    for (coefficient, index) in &base_monomials {
        out_lb_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(*coefficient, "logic input coefficient")?,
            *index,
        ));
    }
    out_lb_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(-out_m, "logic indicator coefficient")?,
        indicator_index,
    ));
    out_lb_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(-out_m, "logic side coefficient")?,
        side_index,
    ));
    constraints.push((
        LinearInequality::new(
            Linear::new(
                out_lb_monomials,
                convert_f64_to_v::<V>(constant, "logic input constant")?,
            ),
            ConstraintRelation::GreaterEqual,
            convert_f64_to_v::<V>(strict_boundary - 2.0 * out_m, "logic rhs")?,
        ),
        format!("{}_out_lb", name_prefix),
    ));

    let mut out_ub_monomials = Vec::with_capacity(polynomial.monomials().len() + 2);
    for (coefficient, index) in &base_monomials {
        out_ub_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(*coefficient, "logic input coefficient")?,
            *index,
        ));
    }
    out_ub_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(out_m, "logic indicator coefficient")?,
        indicator_index,
    ));
    out_ub_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(-out_m, "logic side coefficient")?,
        side_index,
    ));
    constraints.push((
        LinearInequality::new(
            Linear::new(
                out_ub_monomials,
                convert_f64_to_v::<V>(constant, "logic input constant")?,
            ),
            ConstraintRelation::LessEqual,
            convert_f64_to_v::<V>(-strict_boundary + out_m, "logic rhs")?,
        ),
        format!("{}_out_ub", name_prefix),
    ));

    Ok(constraints)
}

/// 与函数 / And Function
///
/// 当所有输入多项式均非零时结果为 1，否则为 0。
/// Result is 1 when all input polynomials are nonzero, 0 otherwise.
#[derive(Debug, Clone)]
pub struct AndFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    polynomials: Vec<Linear<V>>,
    result_var: BinaryVariableItem,
    /// 非零指示二值变量列表 / Nonzero indicator binary variables
    indicator_vars: Vec<BinaryVariableItem>,
    /// 辅助松弛二值变量列表 / Auxiliary side binary variables
    side_vars: Vec<BinaryVariableItem>,
}

impl<V> AndFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建逻辑与函数 / Create a logical AND function.
    pub fn new(id: u64, name: &str, polynomials: Vec<Linear<V>>) -> Self {
        let n = polynomials.len();
        let group_id = new_group_id();
        let result_var =
            BinaryVariableItem::create(VariableId::new(group_id, 0), &format!("{}_and", name));
        let indicator_vars: Vec<_> = (0..n)
            .map(|i| {
                BinaryVariableItem::create(
                    VariableId::new(group_id, i + 1),
                    &format!("{}_and_nz{}", name, i),
                )
            })
            .collect();
        let side_vars: Vec<_> = (0..n)
            .map(|i| {
                BinaryVariableItem::create(
                    VariableId::new(group_id, n + i + 1),
                    &format!("{}_and_side{}", name, i),
                )
            })
            .collect();

        Self {
            id: IntermediateSymbolId::new(id, name),
            polynomials,
            result_var,
            indicator_vars,
            side_vars,
        }
    }

    /// 使用自动 ID 与调用方提供的名称创建 and 函数。
    /// Create an and function with an auto id and caller-provided name.
    pub fn named(name: impl AsRef<str>, polynomials: Vec<Linear<V>>) -> Self {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            polynomials,
        )
    }

    /// 使用自动 ID 与自动名称创建 and 函数。
    /// Create an and function with an auto id and auto-generated name.
    pub fn auto(polynomials: Vec<Linear<V>>) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("and", id);
        Self::new(id, &name, polynomials)
    }

    /// 获取输入多项式 / Get the input polynomials.
    pub fn polynomials(&self) -> &[Linear<V>] {
        &self.polynomials
    }

    /// 获取结果变量 / Get the result variable.
    pub fn result_variable(&self) -> &BinaryVariableItem {
        &self.result_var
    }

    /// 获取非零指示变量 / Get the nonzero indicator variables.
    pub fn indicator_variables(&self) -> &[BinaryVariableItem] {
        &self.indicator_vars
    }

    /// 获取符号辅助变量 / Get the sign auxiliary variables.
    pub fn side_variables(&self) -> &[BinaryVariableItem] {
        &self.side_vars
    }
}

impl<V> AndFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    fn infer_big_m_from_tokens(&self, tokens: &[Token<V>]) -> Option<f64> {
        infer_big_m_for_polynomials(&self.polynomials, tokens, BIG_M_POLICY.min())
    }
}

impl<V> AndFunction<V>
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
    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<LinearConstraint<V>>> {
        if self.polynomials.len() != self.indicator_vars.len()
            || self.polynomials.len() != self.side_vars.len()
        {
            return Err(ModelError::InvalidConstraint(format!(
                "and function `{}` internal auxiliary-variable size mismatch",
                self.id.name
            ))
            .into());
        }

        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "and result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        let source = Arc::new(self.clone());
        let mut constraints = Vec::new();

        if let Some(input_indices) = self.binary_hull_input_indices(symbol_to_index) {
            return self.build_binary_hull_constraints(result_index, &input_indices, source);
        }

        let mut indicator_indices = Vec::with_capacity(self.indicator_vars.len());

        for (i, polynomial) in self.polynomials.iter().enumerate() {
            let indicator_index = symbol_to_index
                .get(&(self.indicator_vars[i].id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "and indicator variable id {}",
                        self.indicator_vars[i].id().unique_id()
                    ))
                })?;
            let side_index = symbol_to_index
                .get(&(self.side_vars[i].id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "and side variable id {}",
                        self.side_vars[i].id().unique_id()
                    ))
                })?;
            indicator_indices.push(indicator_index);

            for (inequality, name) in nonzero_indicator_inequalities(
                polynomial,
                indicator_index,
                side_index,
                big_m,
                &format!("{}_and_nz_{}", self.id.name, i),
            )? {
                constraints.push(LinearConstraint::from_symbol(
                    inequality,
                    &name,
                    source.clone(),
                ));
            }
        }

        if indicator_indices.is_empty() {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "and result coefficient")?,
                            result_index,
                        )],
                        convert_f64_to_v::<V>(0.0, "and constant")?,
                    ),
                    ConstraintRelation::Equal,
                    convert_f64_to_v::<V>(1.0, "and rhs")?,
                ),
                &format!("{}_and_empty", self.id.name),
                source,
            ));
            return Ok(constraints);
        }

        for (i, indicator_index) in indicator_indices.iter().copied().enumerate() {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(1.0, "and result coefficient")?,
                                result_index,
                            ),
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(-1.0, "and indicator coefficient")?,
                                indicator_index,
                            ),
                        ],
                        convert_f64_to_v::<V>(0.0, "and constant")?,
                    ),
                    ConstraintRelation::LessEqual,
                    convert_f64_to_v::<V>(0.0, "and rhs")?,
                ),
                &format!("{}_and_link_ub_{}", self.id.name, i),
                Arc::new(self.clone()),
            ));
        }

        let mut lb_monomials = Vec::with_capacity(indicator_indices.len() + 1);
        lb_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "and result coefficient")?,
            result_index,
        ));
        for indicator_index in &indicator_indices {
            lb_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-1.0, "and indicator coefficient")?,
                *indicator_index,
            ));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(lb_monomials, convert_f64_to_v::<V>(0.0, "and constant")?),
                ConstraintRelation::GreaterEqual,
                convert_f64_to_v::<V>(1.0 - indicator_indices.len() as f64, "and rhs")?,
            ),
            &format!("{}_and_link_lb", self.id.name),
            Arc::new(self.clone()),
        ));

        Ok(constraints)
    }

    /// 直接二值输入时的 hull 输入索引。
    ///
    /// 当辅助指示变量未被注册，且每个输入都是"系数 1、常数 0"的单项式时返回其索引；
    /// 这种情况说明注册阶段已经按直接二值输入处理。
    ///
    /// Hull input indices for direct binary inputs.
    ///
    /// Returns the indices when the auxiliary indicator variables were not registered and
    /// every input is a single monomial with coefficient 1 and constant 0, which means the
    /// registration step already treated the inputs as direct binary variables.
    fn binary_hull_input_indices(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Option<Vec<usize>> {
        if self.polynomials.is_empty() {
            return None;
        }
        if self
            .indicator_vars
            .iter()
            .any(|var| symbol_to_index.contains_key(&(var.id().unique_id() as usize)))
        {
            return None;
        }
        self.polynomials
            .iter()
            .map(direct_monomial_index::<V>)
            .collect()
    }

    /// 生成 AND 的紧凑二值 hull：`result <= input_i` 与 `sum(input) - result <= n - 1`。
    ///
    /// 这里刻意不照抄 Kotlin `_binary_sum` 的 `sum - n * result >= 1 - n`：该形式对
    /// `result = 0` 恒成立，会丢掉 "全部输入为 1 时结果必须为 1" 的精确图关系。迁移原则
    /// 要求保留精确关系，因此使用标准的双侧 hull。
    ///
    /// Build the compact AND binary hull: `result <= input_i` plus
    /// `sum(input) - result <= n - 1`.
    ///
    /// The Kotlin `_binary_sum` row `sum - n * result >= 1 - n` is deliberately not
    /// copied: it always holds for `result = 0` and therefore drops the exact graph
    /// relation "the result must be 1 when every input is 1". The migration policy requires
    /// preserving exact relations, so the standard two-sided hull is used instead.
    fn build_binary_hull_constraints(
        &self,
        result_index: usize,
        input_indices: &[usize],
        source: Arc<dyn IntermediateSymbol<V>>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mut constraints = Vec::with_capacity(input_indices.len() + 1);
        for (i, input_index) in input_indices.iter().copied().enumerate() {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(1.0, "and result coefficient")?,
                                result_index,
                            ),
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(-1.0, "and input coefficient")?,
                                input_index,
                            ),
                        ],
                        convert_f64_to_v::<V>(0.0, "and constant")?,
                    ),
                    ConstraintRelation::LessEqual,
                    convert_f64_to_v::<V>(0.0, "and rhs")?,
                ),
                &format!("{}_binary_le_{}", self.id.name, i),
                source.clone(),
            ));
        }

        let count = input_indices.len() as f64;
        let mut sum_monomials = Vec::with_capacity(input_indices.len() + 1);
        for input_index in input_indices {
            sum_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(1.0, "and input coefficient")?,
                *input_index,
            ));
        }
        sum_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-1.0, "and result coefficient")?,
            result_index,
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(sum_monomials, convert_f64_to_v::<V>(0.0, "and constant")?),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(count - 1.0, "and rhs")?,
            ),
            &format!("{}_binary_sum", self.id.name),
            source,
        ));

        Ok(constraints)
    }
}

impl<V> Display for AndFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "and({})", self.id.name)
    }
}

impl<V> DynSymbol for AndFunction<V>
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

impl<V> Symbol for AndFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for AndFunction<V>
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

    fn flush(&self, _force: bool) {}

    fn register_auxiliary_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        <Self as FunctionSymbol<V>>::register_tokens(self, tokens)
    }

    fn register_auxiliary_tokens_with_context(
        &self,
        tokens: &mut Vec<Token<V>>,
        registered: &[Token<V>],
    ) -> Result<()> {
        // 输入全为直接二值变量时，紧凑 hull 只需要结果列。
        // When every input is a direct binary variable the compact hull only needs the
        // result column.
        if self.polynomials.is_empty() {
            return <Self as FunctionSymbol<V>>::register_tokens(self, tokens);
        }
        let binary_inputs: Option<Vec<usize>> = self
            .polynomials
            .iter()
            .map(|polynomial| direct_binary_input_index(polynomial, registered))
            .collect();
        if binary_inputs.is_some() {
            tokens.push(Token::from_generic(
                self.result_var.clone(),
                self.result_var.index(),
            ));
            return Ok(());
        }
        <Self as FunctionSymbol<V>>::register_tokens(self, tokens)
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.build_mechanism_constraints(symbol_to_index, BIG_M_POLICY.fallback())
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let big_m = BIG_M_POLICY.resolve(self.infer_big_m_from_tokens(tokens));
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
        format!("and({})", self.id.name)
    }

    fn deferred_structure_with_tokens(
        &self,
        tokens: &[Token<V>],
    ) -> Option<Arc<dyn crate::model::intermediate::DeferredFunctionStructure<V>>> {
        // 与注册阶段同一判定：输入全为直接二值变量（紧凑 hull）时才提供延迟结构，一般数值
        // 多项式继续即时展开，不暴露只覆盖一部分模型的原生能力。
        // Same rule as registration: expose a deferred structure only when every input is a direct
        // binary variable (compact hull); general numeric polynomials keep eager expansion so a
        // native path can never cover just part of a model.
        if self.polynomials.is_empty() {
            return None;
        }
        let all_direct_binary = self
            .polynomials
            .iter()
            .all(|polynomial| direct_binary_input_index(polynomial, tokens).is_some());
        if !all_direct_binary {
            return None;
        }
        Some(Arc::new(AndStructure::new(
            self.id.name.clone(),
            Arc::new(self.clone()),
            BIG_M_POLICY.resolve(self.infer_big_m_from_tokens(tokens)),
        )))
    }
}

/// AND 的求解器无关结构描述 / Solver-neutral structure description of AND
///
/// 只覆盖「所有输入都是直接二值变量」的紧凑 hull 形态。结构与 NOT 采用同一模式：持有产生它的
/// 符号（`Arc`），物化时直接调用手写路径的同一个公式生成器，因此延迟物化与 EAGER 展开逐列
/// 一致；一般数值多项式不提供结构，继续即时展开。
///
/// Covers only the compact-hull form where every input is a direct binary variable. It follows the
/// same pattern as NOT: the structure holds the symbol that produced it (an `Arc`) and materializes
/// by calling the very same formula generator as the handwritten eager path, so deferred
/// materialization stays column-identical to eager expansion; general numeric polynomials expose no
/// structure and keep eager expansion.
#[derive(Debug)]
pub struct AndStructure<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 函数名称 / Function name
    name: String,
    /// 产生本结构的符号 / Symbol that produced this structure
    symbol: Arc<AndFunction<V>>,
    /// 结果列 / Result column
    result: crate::variable::VariableId,
    /// 推导出的 Big-M（紧凑 hull 不依赖它，保留以便与 EAGER 路径参数一致）
    /// Inferred Big-M (the compact hull does not depend on it; kept so the eager path's arguments
    /// stay identical)
    big_m: f64,
}

impl<V> AndStructure<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建结构描述 / Create a structure description.
    pub fn new(name: impl Into<String>, symbol: Arc<AndFunction<V>>, big_m: f64) -> Self {
        let result = symbol.result_var.id();
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

    /// 获取推导出的 Big-M / Get the inferred Big-M.
    pub fn big_m(&self) -> f64 {
        self.big_m
    }
}

impl<V> AndStructure<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    /// 获取紧凑 hull 的操作数列下标（只读；`None` 表示输入不是直接二值单项式）
    /// Read-only access to the compact hull's operand column indices (`None` when some input is not a
    /// direct binary monomial).
    ///
    /// 用途：原生 writer 要用 `result = AND(inputs)` 的 SDK 一般约束替换即时展开的两族 hull 行
    /// （`result <= input_i` 与 `sum(input) - result <= n - 1`），因此必须拿到与即时展开**完全相同**
    /// 的操作数列。本访问器复用即时路径的同一个 `direct_monomial_index` 判定，不复制公式，也不暴露
    /// 可变状态。
    ///
    /// Purpose: a native writer replaces the eager two hull row families (`result <= input_i` and
    /// `sum(input) - result <= n - 1`) with the SDK's `result = AND(inputs)` general constraint, so it
    /// must obtain the **very same** operand columns. This reuses the eager path's own
    /// `direct_monomial_index` check, copies no formula and exposes no mutable state.
    pub fn operand_indices(&self) -> Option<Vec<usize>> {
        if self.symbol.polynomials.is_empty() {
            return None;
        }
        self.symbol
            .polynomials
            .iter()
            .map(direct_monomial_index::<V>)
            .collect()
    }
}

impl<V> crate::model::intermediate::DeferredFunctionStructure<V> for AndStructure<V>
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
        // 紧凑 hull 只注册结果列；输入是模型既有列，不属于本结构的辅助列。
        // The compact hull registers only the result column; the inputs are existing model columns
        // and not helpers of this structure.
        Some(crate::model::intermediate::StructureUsageBinding::new(
            self.symbol.id.id,
            self.result.clone(),
            Vec::new(),
        ))
    }

    fn fingerprint(&self) -> Option<String> {
        Some(format!(
            "and|{}|{}|{}",
            self.name,
            self.symbol.id.id,
            self.result.unique_id()
        ))
    }

    fn materialize(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        // 复用即时展开的同一份生成器，保证两条路径逐列一致。
        // Reuse the eager path's generator so both paths stay column-identical.
        self.symbol
            .build_mechanism_constraints(symbol_to_index, self.big_m)
    }
}

impl<V> FunctionSymbol<V> for AndFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero        + ToPrimitive
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        for indicator_var in &self.indicator_vars {
            tokens.push(Token::from_generic(
                indicator_var.clone(),
                indicator_var.index(),
            ));
        }
        for side_var in &self.side_vars {
            tokens.push(Token::from_generic(side_var.clone(), side_var.index()));
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        for polynomial in &self.polynomials {
            let value = evaluate_linear(polynomial, token_table, zero_if_none)?;
            if as_binary(to_f64(&value)?) == 0.0 {
                return from_f64(0.0);
            }
        }
        from_f64(1.0)
    }
}

impl<V> LinearIntermediateSymbol<V> for AndFunction<V>
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

impl<V> LogicFunctionSymbol<V> for AndFunction<V>
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
}

/// 或函数 / Or Function
#[derive(Debug, Clone)]
pub struct OrFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    polynomials: Vec<Linear<V>>,
    result_var: BinaryVariableItem,
    indicator_vars: Vec<BinaryVariableItem>,
    side_vars: Vec<BinaryVariableItem>,
}

impl<V> OrFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建逻辑或函数 / Create a logical OR function.
    pub fn new(id: u64, name: &str, polynomials: Vec<Linear<V>>) -> Self {
        let n = polynomials.len();
        let group_id = new_group_id();
        let result_var =
            BinaryVariableItem::create(VariableId::new(group_id, 0), &format!("{}_or", name));
        let indicator_vars: Vec<_> = (0..n)
            .map(|i| {
                BinaryVariableItem::create(
                    VariableId::new(group_id, i + 1),
                    &format!("{}_or_nz{}", name, i),
                )
            })
            .collect();
        let side_vars: Vec<_> = (0..n)
            .map(|i| {
                BinaryVariableItem::create(
                    VariableId::new(group_id, n + i + 1),
                    &format!("{}_or_side{}", name, i),
                )
            })
            .collect();

        Self {
            id: IntermediateSymbolId::new(id, name),
            polynomials,
            result_var,
            indicator_vars,
            side_vars,
        }
    }

    /// 使用自动 ID 与调用方提供的名称创建 or 函数。
    /// Create an or function with an auto id and caller-provided name.
    pub fn named(name: impl AsRef<str>, polynomials: Vec<Linear<V>>) -> Self {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            polynomials,
        )
    }

    /// 使用自动 ID 与自动名称创建 or 函数。
    /// Create an or function with an auto id and auto-generated name.
    pub fn auto(polynomials: Vec<Linear<V>>) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("or", id);
        Self::new(id, &name, polynomials)
    }

    /// 获取输入多项式 / Get the input polynomials.
    pub fn polynomials(&self) -> &[Linear<V>] {
        &self.polynomials
    }

    /// 获取结果变量 / Get the result variable.
    pub fn result_variable(&self) -> &BinaryVariableItem {
        &self.result_var
    }

    /// 获取非零指示变量 / Get the nonzero indicator variables.
    pub fn indicator_variables(&self) -> &[BinaryVariableItem] {
        &self.indicator_vars
    }

    /// 获取符号辅助变量 / Get the sign auxiliary variables.
    pub fn side_variables(&self) -> &[BinaryVariableItem] {
        &self.side_vars
    }
}

impl<V> OrFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    fn infer_big_m_from_tokens(&self, tokens: &[Token<V>]) -> Option<f64> {
        infer_big_m_for_polynomials(&self.polynomials, tokens, BIG_M_POLICY.min())
    }
}

impl<V> OrFunction<V>
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
    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<LinearConstraint<V>>> {
        if self.polynomials.len() != self.indicator_vars.len()
            || self.polynomials.len() != self.side_vars.len()
        {
            return Err(ModelError::InvalidConstraint(format!(
                "or function `{}` internal auxiliary-variable size mismatch",
                self.id.name
            ))
            .into());
        }

        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "or result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        let source = Arc::new(self.clone());
        let mut constraints = Vec::new();

        if let Some(input_indices) = self.binary_hull_input_indices(symbol_to_index) {
            return self.build_binary_hull_constraints(result_index, &input_indices, source);
        }

        let mut indicator_indices = Vec::with_capacity(self.indicator_vars.len());

        for (i, polynomial) in self.polynomials.iter().enumerate() {
            let indicator_index = symbol_to_index
                .get(&(self.indicator_vars[i].id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "or indicator variable id {}",
                        self.indicator_vars[i].id().unique_id()
                    ))
                })?;
            let side_index = symbol_to_index
                .get(&(self.side_vars[i].id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "or side variable id {}",
                        self.side_vars[i].id().unique_id()
                    ))
                })?;
            indicator_indices.push(indicator_index);

            for (inequality, name) in nonzero_indicator_inequalities(
                polynomial,
                indicator_index,
                side_index,
                big_m,
                &format!("{}_or_nz_{}", self.id.name, i),
            )? {
                constraints.push(LinearConstraint::from_symbol(
                    inequality,
                    &name,
                    source.clone(),
                ));
            }
        }

        if indicator_indices.is_empty() {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "or result coefficient")?,
                            result_index,
                        )],
                        convert_f64_to_v::<V>(0.0, "or constant")?,
                    ),
                    ConstraintRelation::Equal,
                    convert_f64_to_v::<V>(0.0, "or rhs")?,
                ),
                &format!("{}_or_empty", self.id.name),
                source,
            ));
            return Ok(constraints);
        }

        for (i, indicator_index) in indicator_indices.iter().copied().enumerate() {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(1.0, "or result coefficient")?,
                                result_index,
                            ),
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(-1.0, "or indicator coefficient")?,
                                indicator_index,
                            ),
                        ],
                        convert_f64_to_v::<V>(0.0, "or constant")?,
                    ),
                    ConstraintRelation::GreaterEqual,
                    convert_f64_to_v::<V>(0.0, "or rhs")?,
                ),
                &format!("{}_or_link_lb_{}", self.id.name, i),
                Arc::new(self.clone()),
            ));
        }

        let mut ub_monomials = Vec::with_capacity(indicator_indices.len() + 1);
        ub_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "or result coefficient")?,
            result_index,
        ));
        for indicator_index in &indicator_indices {
            ub_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-1.0, "or indicator coefficient")?,
                *indicator_index,
            ));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(ub_monomials, convert_f64_to_v::<V>(0.0, "or constant")?),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(0.0, "or rhs")?,
            ),
            &format!("{}_or_link_ub", self.id.name),
            Arc::new(self.clone()),
        ));

        Ok(constraints)
    }

    /// 直接二值输入时的 hull 输入索引。
    ///
    /// Hull input indices for direct binary inputs.
    fn binary_hull_input_indices(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Option<Vec<usize>> {
        if self.polynomials.is_empty() {
            return None;
        }
        if self
            .indicator_vars
            .iter()
            .any(|var| symbol_to_index.contains_key(&(var.id().unique_id() as usize)))
        {
            return None;
        }
        self.polynomials
            .iter()
            .map(direct_monomial_index::<V>)
            .collect()
    }

    /// 生成 OR 的紧凑二值 hull：`result >= input_i` 与 `sum(input) - result >= 0`。
    ///
    /// 这里刻意不照抄 Kotlin `_binary_sum` 的 `sum - result <= 0`：该形式只给出下界，
    /// 会允许"全部输入为 0 时结果仍为 1"，丢掉精确图关系。迁移原则要求保留精确关系，
    /// 因此使用标准的双侧 hull。
    ///
    /// Build the compact OR binary hull: `result >= input_i` plus
    /// `sum(input) - result >= 0`.
    ///
    /// The Kotlin `_binary_sum` row `sum - result <= 0` is deliberately not copied: it only
    /// provides a lower bound and would allow a result of 1 while every input is 0, dropping
    /// the exact graph relation. The migration policy requires preserving exact relations,
    /// so the standard two-sided hull is used instead.
    fn build_binary_hull_constraints(
        &self,
        result_index: usize,
        input_indices: &[usize],
        source: Arc<dyn IntermediateSymbol<V>>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mut constraints = Vec::with_capacity(input_indices.len() + 1);
        for (i, input_index) in input_indices.iter().copied().enumerate() {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(1.0, "or result coefficient")?,
                                result_index,
                            ),
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(-1.0, "or input coefficient")?,
                                input_index,
                            ),
                        ],
                        convert_f64_to_v::<V>(0.0, "or constant")?,
                    ),
                    ConstraintRelation::GreaterEqual,
                    convert_f64_to_v::<V>(0.0, "or rhs")?,
                ),
                &format!("{}_binary_ge_{}", self.id.name, i),
                source.clone(),
            ));
        }

        let mut sum_monomials = Vec::with_capacity(input_indices.len() + 1);
        for input_index in input_indices {
            sum_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(1.0, "or input coefficient")?,
                *input_index,
            ));
        }
        sum_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-1.0, "or result coefficient")?,
            result_index,
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(sum_monomials, convert_f64_to_v::<V>(0.0, "or constant")?),
                ConstraintRelation::GreaterEqual,
                convert_f64_to_v::<V>(0.0, "or rhs")?,
            ),
            &format!("{}_binary_sum", self.id.name),
            source,
        ));

        Ok(constraints)
    }
}

impl<V> Display for OrFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "or({})", self.id.name)
    }
}

impl<V> DynSymbol for OrFunction<V>
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

impl<V> Symbol for OrFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for OrFunction<V>
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

    fn flush(&self, _force: bool) {}

    fn register_auxiliary_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        <Self as FunctionSymbol<V>>::register_tokens(self, tokens)
    }

    fn register_auxiliary_tokens_with_context(
        &self,
        tokens: &mut Vec<Token<V>>,
        registered: &[Token<V>],
    ) -> Result<()> {
        // 输入全为直接二值变量时，紧凑 hull 只需要结果列。
        // When every input is a direct binary variable the compact hull only needs the
        // result column.
        if self.polynomials.is_empty() {
            return <Self as FunctionSymbol<V>>::register_tokens(self, tokens);
        }
        let binary_inputs: Option<Vec<usize>> = self
            .polynomials
            .iter()
            .map(|polynomial| direct_binary_input_index(polynomial, registered))
            .collect();
        if binary_inputs.is_some() {
            tokens.push(Token::from_generic(
                self.result_var.clone(),
                self.result_var.index(),
            ));
            return Ok(());
        }
        <Self as FunctionSymbol<V>>::register_tokens(self, tokens)
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.build_mechanism_constraints(symbol_to_index, BIG_M_POLICY.fallback())
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let big_m = BIG_M_POLICY.resolve(self.infer_big_m_from_tokens(tokens));
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
        format!("or({})", self.id.name)
    }

    fn deferred_structure_with_tokens(
        &self,
        tokens: &[Token<V>],
    ) -> Option<Arc<dyn crate::model::intermediate::DeferredFunctionStructure<V>>> {
        // 与注册阶段同一判定：输入全为直接二值变量（紧凑 hull）时才提供延迟结构，一般数值
        // 多项式继续即时展开，不暴露只覆盖一部分模型的原生能力。
        // Same rule as registration: expose a deferred structure only when every input is a direct
        // binary variable (compact hull); general numeric polynomials keep eager expansion so a
        // native path can never cover just part of a model.
        if self.polynomials.is_empty() {
            return None;
        }
        let all_direct_binary = self
            .polynomials
            .iter()
            .all(|polynomial| direct_binary_input_index(polynomial, tokens).is_some());
        if !all_direct_binary {
            return None;
        }
        Some(Arc::new(OrStructure::new(
            self.id.name.clone(),
            Arc::new(self.clone()),
            BIG_M_POLICY.resolve(self.infer_big_m_from_tokens(tokens)),
        )))
    }
}

/// OR 的求解器无关结构描述 / Solver-neutral structure description of OR
///
/// 只覆盖「所有输入都是直接二值变量」的紧凑 hull 形态。结构与 NOT/AND 采用同一模式：持有产生
/// 它的符号（`Arc`），物化时直接调用手写路径的同一个公式生成器，因此延迟物化与 EAGER 展开逐列
/// 一致；一般数值多项式不提供结构，继续即时展开。
///
/// Covers only the compact-hull form where every input is a direct binary variable. It follows the
/// same pattern as NOT and AND: the structure holds the symbol that produced it (an `Arc`) and
/// materializes by calling the very same formula generator as the handwritten eager path, so
/// deferred materialization stays column-identical to eager expansion; general numeric polynomials
/// expose no structure and keep eager expansion.
#[derive(Debug)]
pub struct OrStructure<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 函数名称 / Function name
    name: String,
    /// 产生本结构的符号 / Symbol that produced this structure
    symbol: Arc<OrFunction<V>>,
    /// 结果列 / Result column
    result: crate::variable::VariableId,
    /// 推导出的 Big-M（紧凑 hull 不依赖它，保留以便与 EAGER 路径参数一致）
    /// Inferred Big-M (the compact hull does not depend on it; kept so the eager path's arguments
    /// stay identical)
    big_m: f64,
}

impl<V> OrStructure<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建结构描述 / Create a structure description.
    pub fn new(name: impl Into<String>, symbol: Arc<OrFunction<V>>, big_m: f64) -> Self {
        let result = symbol.result_var.id();
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

    /// 获取推导出的 Big-M / Get the inferred Big-M.
    pub fn big_m(&self) -> f64 {
        self.big_m
    }
}

impl<V> OrStructure<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    /// 获取紧凑 hull 的操作数列下标（只读；`None` 表示输入不是直接二值单项式）
    /// Read-only access to the compact hull's operand column indices (`None` when some input is not a
    /// direct binary monomial).
    ///
    /// 用途与 [`AndStructure::operand_indices`] 相同：原生 writer 用 `result = OR(inputs)` 的 SDK
    /// 一般约束替换即时展开的 `result >= input_i` 与 `sum(input) - result >= 0` 两族行，因此必须拿到
    /// 与即时展开完全相同的操作数列；本访问器复用即时路径的同一个判定，不复制公式。
    ///
    /// The purpose matches [`AndStructure::operand_indices`]: a native writer replaces the eager
    /// `result >= input_i` and `sum(input) - result >= 0` row families with the SDK's
    /// `result = OR(inputs)` general constraint, so it must obtain exactly the same operand columns;
    /// this reuses the eager path's own check and copies no formula.
    pub fn operand_indices(&self) -> Option<Vec<usize>> {
        if self.symbol.polynomials.is_empty() {
            return None;
        }
        self.symbol
            .polynomials
            .iter()
            .map(direct_monomial_index::<V>)
            .collect()
    }
}

impl<V> crate::model::intermediate::DeferredFunctionStructure<V> for OrStructure<V>
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
        // 紧凑 hull 只注册结果列；输入是模型既有列，不属于本结构的辅助列。
        // The compact hull registers only the result column; the inputs are existing model columns
        // and not helpers of this structure.
        Some(crate::model::intermediate::StructureUsageBinding::new(
            self.symbol.id.id,
            self.result.clone(),
            Vec::new(),
        ))
    }

    fn fingerprint(&self) -> Option<String> {
        Some(format!(
            "or|{}|{}|{}",
            self.name,
            self.symbol.id.id,
            self.result.unique_id()
        ))
    }

    fn materialize(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        // 复用即时展开的同一份生成器，保证两条路径逐列一致。
        // Reuse the eager path's generator so both paths stay column-identical.
        self.symbol
            .build_mechanism_constraints(symbol_to_index, self.big_m)
    }
}

impl<V> FunctionSymbol<V> for OrFunction<V>
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
        for indicator_var in &self.indicator_vars {
            tokens.push(Token::from_generic(
                indicator_var.clone(),
                indicator_var.index(),
            ));
        }
        for side_var in &self.side_vars {
            tokens.push(Token::from_generic(side_var.clone(), side_var.index()));
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        for polynomial in &self.polynomials {
            let value = evaluate_linear(polynomial, token_table, zero_if_none)?;
            if as_binary(to_f64(&value)?) > 0.0 {
                return from_f64(1.0);
            }
        }
        from_f64(0.0)
    }
}

impl<V> LinearIntermediateSymbol<V> for OrFunction<V>
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

impl<V> LogicFunctionSymbol<V> for OrFunction<V>
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
}

/// 非函数 / Not Function
#[derive(Debug, Clone)]
pub struct NotFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    polynomial: Linear<V>,
    result_var: BinaryVariableItem,
    indicator_var: BinaryVariableItem,
    side_var: BinaryVariableItem,
}

impl<V> NotFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建逻辑非函数 / Create a logical NOT function.
    pub fn new(id: u64, name: &str, polynomial: Linear<V>) -> Self {
        let group_id = new_group_id();
        let result_var =
            BinaryVariableItem::create(VariableId::new(group_id, 0), &format!("{}_not", name));
        let indicator_var =
            BinaryVariableItem::create(VariableId::new(group_id, 1), &format!("{}_not_nz", name));
        let side_var =
            BinaryVariableItem::create(VariableId::new(group_id, 2), &format!("{}_not_side", name));

        Self {
            id: IntermediateSymbolId::new(id, name),
            polynomial,
            result_var,
            indicator_var,
            side_var,
        }
    }

    /// 使用自动 ID 与调用方提供的名称创建 not 函数。
    /// Create a not function with an auto id and caller-provided name.
    pub fn named(name: impl AsRef<str>, polynomial: Linear<V>) -> Self {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            polynomial,
        )
    }

    /// 使用自动 ID 与自动名称创建 not 函数。
    /// Create a not function with an auto id and auto-generated name.
    pub fn auto(polynomial: Linear<V>) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("not", id);
        Self::new(id, &name, polynomial)
    }

    /// 获取结果变量 / Get the result variable.
    pub fn result_variable(&self) -> &BinaryVariableItem {
        &self.result_var
    }

    /// 获取非零指示变量 / Get the nonzero indicator variable.
    pub fn indicator_variable(&self) -> &BinaryVariableItem {
        &self.indicator_var
    }

    /// 获取符号辅助变量 / Get the sign auxiliary variable.
    pub fn side_variable(&self) -> &BinaryVariableItem {
        &self.side_var
    }
}

impl<V> NotFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    /// 获取输入多项式 / Get the input polynomial.
    pub fn polynomial(&self) -> &Linear<V> {
        &self.polynomial
    }

    fn infer_big_m_from_tokens(&self, tokens: &[Token<V>]) -> Option<f64> {
        infer_linear_abs_bound_from_tokens(&self.polynomial, tokens)
            .map(|bound| bound.max(BIG_M_POLICY.min()))
    }
}

impl<V> NotFunction<V>
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
    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "not result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        let source = Arc::new(self.clone());
        let mut constraints = Vec::new();

        // 辅助指示变量未注册时说明注册阶段已判定输入为直接二值变量。
        // A missing auxiliary indicator variable means the registration step already
        // classified the input as a direct binary variable.
        let indicator_registered = symbol_to_index
            .contains_key(&(self.indicator_var.id().unique_id() as usize));
        if !indicator_registered {
            if let Some(input_index) = direct_monomial_index::<V>(&self.polynomial) {
                // 直接二值输入：`result + input = 1` 即可，无需非零指示与符号辅助变量。
                // Direct binary input: `result + input = 1` suffices, with no nonzero
                // indicator or sign helper variables.
                constraints.push(LinearConstraint::from_symbol(
                    LinearInequality::new(
                        Linear::new(
                            vec![
                                LinearMonomial::new(
                                    convert_f64_to_v::<V>(1.0, "not result coefficient")?,
                                    result_index,
                                ),
                                LinearMonomial::new(
                                    convert_f64_to_v::<V>(1.0, "not input coefficient")?,
                                    input_index,
                                ),
                            ],
                            convert_f64_to_v::<V>(0.0, "not constant")?,
                        ),
                        ConstraintRelation::Equal,
                        convert_f64_to_v::<V>(1.0, "not rhs")?,
                    ),
                    &format!("{}_binary_result", self.id.name),
                    source,
                ));
                return Ok(constraints);
            }
        }

        let indicator_index = symbol_to_index
            .get(&(self.indicator_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "not indicator variable id {}",
                    self.indicator_var.id().unique_id()
                ))
            })?;
        let side_index = symbol_to_index
            .get(&(self.side_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "not side variable id {}",
                    self.side_var.id().unique_id()
                ))
            })?;

        for (inequality, name) in nonzero_indicator_inequalities(
            &self.polynomial,
            indicator_index,
            side_index,
            big_m,
            &format!("{}_not_nz", self.id.name),
        )? {
            constraints.push(LinearConstraint::from_symbol(
                inequality,
                &name,
                source.clone(),
            ));
        }

        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    vec![
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "not result coefficient")?,
                            result_index,
                        ),
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "not indicator coefficient")?,
                            indicator_index,
                        ),
                    ],
                    convert_f64_to_v::<V>(0.0, "not constant")?,
                ),
                ConstraintRelation::Equal,
                convert_f64_to_v::<V>(1.0, "not rhs")?,
            ),
            &format!("{}_not_link", self.id.name),
            source,
        ));

        Ok(constraints)
    }
}

impl<V> Display for NotFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "not({})", self.id.name)
    }
}

impl<V> DynSymbol for NotFunction<V>
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

impl<V> Symbol for NotFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for NotFunction<V>
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

    fn flush(&self, _force: bool) {}

    fn register_auxiliary_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        <Self as FunctionSymbol<V>>::register_tokens(self, tokens)
    }

    fn register_auxiliary_tokens_with_context(
        &self,
        tokens: &mut Vec<Token<V>>,
        registered: &[Token<V>],
    ) -> Result<()> {
        // 输入为直接二值变量时，`result = 1 - input` 只需要结果列。
        // When the input is a direct binary variable `result = 1 - input` only needs the
        // result column.
        if direct_binary_input_index(&self.polynomial, registered).is_some() {
            tokens.push(Token::from_generic(
                self.result_var.clone(),
                self.result_var.index(),
            ));
            return Ok(());
        }
        <Self as FunctionSymbol<V>>::register_tokens(self, tokens)
    }

    fn deferred_structure_with_tokens(
        &self,
        tokens: &[Token<V>],
    ) -> Option<Arc<dyn crate::model::intermediate::DeferredFunctionStructure<V>>> {
        // 只有直接二值输入（紧凑 hull）才提供延迟结构：一般数值多项式继续即时展开，
        // 不把「原生路径只能覆盖一部分模型」的半成品能力暴露出去。
        // Only a direct binary input (compact hull) exposes a deferred structure; general numeric
        // polynomials keep eager expansion so a native path can never cover just part of a model.
        if direct_binary_input_index(&self.polynomial, tokens).is_none() {
            return None;
        }
        Some(Arc::new(NotStructure::new(
            self.id.name.clone(),
            Arc::new(self.clone()),
            BIG_M_POLICY.resolve(self.infer_big_m_from_tokens(tokens)),
        )))
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.build_mechanism_constraints(symbol_to_index, BIG_M_POLICY.fallback())
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let big_m = BIG_M_POLICY.resolve(self.infer_big_m_from_tokens(tokens));
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
        format!("not({})", self.id.name)
    }
}

/// NOT 的求解器无关结构描述 / Solver-neutral structure description of NOT
///
/// 只覆盖「输入是直接二值变量」的紧凑 hull 形态。结构持有产生它的符号（`Arc`，与 ABS 存
/// `Arc::new(self.clone())` 的做法一致），物化时直接调用手写路径的同一个公式生成器，因此
/// 延迟物化与 EAGER 展开保证逐列一致；一般数值多项式不提供结构，继续即时展开。
///
/// Covers only the compact-hull form where the input is a direct binary variable. The structure
/// holds the symbol that produced it (an `Arc`, exactly like ABS storing `Arc::new(self.clone())`)
/// and materializes by calling the very same formula generator as the handwritten eager path, so
/// deferred materialization stays column-identical to eager expansion; general numeric polynomials
/// expose no structure and keep eager expansion.
#[derive(Debug)]
pub struct NotStructure<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 函数名称 / Function name
    name: String,    /// 产生本结构的符号 / Symbol that produced this structure
    symbol: Arc<NotFunction<V>>,
    /// 结果列 / Result column
    result: crate::variable::VariableId,
    /// 推导出的 Big-M（紧凑 hull 不依赖它，保留以便与 EAGER 路径参数一致）
    /// Inferred Big-M (the compact hull does not depend on it; kept so the eager path's arguments
    /// stay identical)
    big_m: f64,
}

impl<V> NotStructure<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 获取符号句柄 / Get the symbol handle.
    pub fn symbol(&self) -> &Arc<NotFunction<V>> {
        &self.symbol
    }

    /// 创建结构描述 / Create a structure description.
    pub fn new(name: impl Into<String>, symbol: Arc<NotFunction<V>>, big_m: f64) -> Self {
        let result = symbol.result_var.id();
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

    /// 获取推导出的 Big-M / Get the inferred Big-M.
    pub fn big_m(&self) -> f64 {
        self.big_m
    }
}

impl<V> crate::model::intermediate::DeferredFunctionStructure<V> for NotStructure<V>
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
        // 紧凑 hull 只注册结果列，输入是模型既有列、不属于本结构的辅助列。
        // The compact hull registers only the result column; the input is an existing model column
        // and not a helper of this structure.
        Some(crate::model::intermediate::StructureUsageBinding::new(
            self.symbol.id.id,
            self.result.clone(),
            Vec::new(),
        ))
    }

    fn fingerprint(&self) -> Option<String> {
        Some(format!(
            "not|{}|{}|{}",
            self.name,
            self.symbol.id.id,
            self.result.unique_id()
        ))
    }

    fn materialize(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        // 复用即时展开的同一份生成器，保证两条路径逐列一致。
        // Reuse the eager path's generator so both paths stay column-identical.
        self.symbol
            .build_mechanism_constraints(symbol_to_index, self.big_m)
    }
}

impl<V> FunctionSymbol<V> for NotFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero        + ToPrimitive
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        tokens.push(Token::from_generic(
            self.indicator_var.clone(),
            self.indicator_var.index(),
        ));
        tokens.push(Token::from_generic(
            self.side_var.clone(),
            self.side_var.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let value = evaluate_linear(&self.polynomial, token_table, zero_if_none)?;
        if as_binary(to_f64(&value)?) == 0.0 {
            from_f64(1.0)
        } else {
            from_f64(0.0)
        }
    }
}

impl<V> LinearIntermediateSymbol<V> for NotFunction<V>
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

impl<V> LogicFunctionSymbol<V> for NotFunction<V>
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
}

/// 异或函数 / Xor Function
#[derive(Debug, Clone)]
pub struct XorFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    polynomials: Vec<Linear<V>>,
    result_var: BinaryVariableItem,
    indicator_vars: Vec<BinaryVariableItem>,
    side_vars: Vec<BinaryVariableItem>,
    explicit_big_m: Option<f64>,
    tolerance: f64,
    strict_boundary: f64,
}

impl<V> XorFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建逻辑异或函数 / Create a logical XOR function.
    pub fn new(id: u64, name: &str, polynomials: Vec<Linear<V>>) -> Self {
        assert!(
            !polynomials.is_empty(),
            "XorFunction requires at least one input polynomial.",
        );
        let n = polynomials.len();
        let group_id = new_group_id();
        let result_var =
            BinaryVariableItem::create(VariableId::new(group_id, 0), &format!("{}_xor", name));
        let indicator_vars: Vec<_> = (0..n)
            .map(|i| {
                BinaryVariableItem::create(
                    VariableId::new(group_id, i + 1),
                    &format!("{}_xor_nz{}", name, i),
                )
            })
            .collect();
        let side_vars: Vec<_> = (0..n)
            .map(|i| {
                BinaryVariableItem::create(
                    VariableId::new(group_id, n + i + 1),
                    &format!("{}_xor_side{}", name, i),
                )
            })
            .collect();

        Self {
            id: IntermediateSymbolId::new(id, name),
            polynomials,
            result_var,
            indicator_vars,
            side_vars,
            explicit_big_m: None,
            tolerance: NONZERO_TOLERANCE,
            strict_boundary: STRICT_NONZERO_BOUNDARY,
        }
    }

    /// Set an explicit Big-M value for all input indicators.
    pub fn with_big_m(mut self, big_m: f64) -> Self {
        assert!(
            big_m.is_finite() && big_m > 0.0,
            "xor Big-M must be finite and positive"
        );
        self.explicit_big_m = Some(big_m);
        self
    }

    /// Set the shared zero-band tolerance used by input indicators.
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        assert!(
            tolerance.is_finite() && tolerance >= 0.0 && tolerance < self.strict_boundary,
            "xor tolerance must be finite, non-negative, and below the strict boundary"
        );
        self.tolerance = tolerance;
        self
    }

    /// Set the strict nonzero boundary used by input indicators.
    pub fn with_strict_boundary(mut self, strict_boundary: f64) -> Self {
        assert!(
            strict_boundary.is_finite() && strict_boundary > self.tolerance,
            "xor strict boundary must be finite and exceed the tolerance"
        );
        self.strict_boundary = strict_boundary;
        self
    }

    /// Configure Big-M, zero-band tolerance, and strict boundary together.
    pub fn with_parameters(
        mut self,
        big_m: Option<f64>,
        tolerance: f64,
        strict_boundary: f64,
    ) -> Self {
        if let Some(big_m) = big_m {
            self = self.with_big_m(big_m);
        }
        assert!(
            tolerance.is_finite()
                && tolerance >= 0.0
                && strict_boundary.is_finite()
                && strict_boundary > tolerance,
            "xor strict boundary must be finite and exceed the finite non-negative tolerance"
        );
        self.tolerance = tolerance;
        self.strict_boundary = strict_boundary;
        self
    }

    /// 使用自动 ID 与调用方提供的名称创建 xor 函数。
    /// Create a xor function with an auto id and caller-provided name.
    pub fn named(name: impl AsRef<str>, polynomials: Vec<Linear<V>>) -> Self {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            polynomials,
        )
    }

    /// 使用自动 ID 与自动名称创建 xor 函数。
    /// Create a xor function with an auto id and auto-generated name.
    pub fn auto(polynomials: Vec<Linear<V>>) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("xor", id);
        Self::new(id, &name, polynomials)
    }

    /// 获取输入多项式 / Get the input polynomials.
    pub fn polynomials(&self) -> &[Linear<V>] {
        &self.polynomials
    }

    /// 获取结果变量 / Get the result variable.
    pub fn result_variable(&self) -> &BinaryVariableItem {
        &self.result_var
    }

    /// 获取非零指示变量 / Get the nonzero indicator variables.
    pub fn indicator_variables(&self) -> &[BinaryVariableItem] {
        &self.indicator_vars
    }

    /// 获取符号辅助变量 / Get the sign auxiliary variables.
    pub fn side_variables(&self) -> &[BinaryVariableItem] {
        &self.side_vars
    }

    /// Get the explicit Big-M override, if configured.
    pub fn big_m(&self) -> Option<f64> {
        self.explicit_big_m
    }

    /// Get the zero-band tolerance.
    pub fn tolerance(&self) -> f64 {
        self.tolerance
    }

    /// Get the strict nonzero boundary.
    pub fn strict_boundary(&self) -> f64 {
        self.strict_boundary
    }
}

impl<V> XorFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    fn infer_big_m_from_tokens(&self, tokens: &[Token<V>]) -> Option<f64> {
        infer_big_m_for_polynomials(&self.polynomials, tokens, BIG_M_POLICY.min())
    }

    fn resolve_big_m(&self, inferred: Option<f64>) -> f64 {
        BIG_M_POLICY.resolve(self.explicit_big_m.or(inferred))
    }
}

impl<V> XorFunction<V>
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
    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<LinearConstraint<V>>> {
        if self.polynomials.len() != self.indicator_vars.len()
            || self.polynomials.len() != self.side_vars.len()
        {
            return Err(ModelError::InvalidConstraint(format!(
                "xor function `{}` internal auxiliary-variable size mismatch",
                self.id.name
            ))
            .into());
        }

        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "xor result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        let source = Arc::new(self.clone());
        let mut constraints = Vec::new();
        let mut indicator_indices = Vec::with_capacity(self.indicator_vars.len());

        for (i, polynomial) in self.polynomials.iter().enumerate() {
            let indicator_index = symbol_to_index
                .get(&(self.indicator_vars[i].id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "xor indicator variable id {}",
                        self.indicator_vars[i].id().unique_id()
                    ))
                })?;
            let side_index = symbol_to_index
                .get(&(self.side_vars[i].id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "xor side variable id {}",
                        self.side_vars[i].id().unique_id()
                    ))
                })?;
            indicator_indices.push(indicator_index);
            for (inequality, name) in nonzero_indicator_inequalities_with_policy(
                polynomial,
                indicator_index,
                side_index,
                big_m,
                self.tolerance,
                self.strict_boundary,
                &format!("{}_xor_nz_{}", self.id.name, i),
            )? {
                constraints.push(LinearConstraint::from_symbol(
                    inequality,
                    &name,
                    source.clone(),
                ));
            }
        }

        let mut sum_monomials = Vec::with_capacity(indicator_indices.len() + 1);
        sum_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "xor result coefficient")?,
            result_index,
        ));
        for indicator_index in &indicator_indices {
            sum_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-1.0, "xor indicator coefficient")?,
                *indicator_index,
            ));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(sum_monomials, convert_f64_to_v::<V>(0.0, "xor constant")?),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(0.0, "xor rhs")?,
            ),
            &format!("{}_xor_sum_ub", self.id.name),
            source.clone(),
        ));

        // y >= a_i - sum_{j != i}(a_j).  Exactly one selected indicator forces y = 1.
        for i in 0..indicator_indices.len() {
            let mut single_monomials = Vec::with_capacity(indicator_indices.len() + 1);
            single_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(1.0, "xor result coefficient")?,
                result_index,
            ));
            for (j, indicator_index) in indicator_indices.iter().enumerate() {
                single_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(
                        if i == j { -1.0 } else { 1.0 },
                        "xor single-indicator coefficient",
                    )?,
                    *indicator_index,
                ));
            }
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        single_monomials,
                        convert_f64_to_v::<V>(0.0, "xor constant")?,
                    ),
                    ConstraintRelation::GreaterEqual,
                    convert_f64_to_v::<V>(0.0, "xor rhs")?,
                ),
                &format!("{}_xor_single_{}", self.id.name, i),
                source.clone(),
            ));
        }

        // Any selected pair forces y = 0: y + a_i + a_j <= 2.
        for i in 0..indicator_indices.len() {
            for j in (i + 1)..indicator_indices.len() {
                constraints.push(LinearConstraint::from_symbol(
                    LinearInequality::new(
                        Linear::new(
                            vec![
                                LinearMonomial::new(
                                    convert_f64_to_v::<V>(1.0, "xor result coefficient")?,
                                    result_index,
                                ),
                                LinearMonomial::new(
                                    convert_f64_to_v::<V>(1.0, "xor left indicator coefficient")?,
                                    indicator_indices[i],
                                ),
                                LinearMonomial::new(
                                    convert_f64_to_v::<V>(1.0, "xor right indicator coefficient")?,
                                    indicator_indices[j],
                                ),
                            ],
                            convert_f64_to_v::<V>(0.0, "xor constant")?,
                        ),
                        ConstraintRelation::LessEqual,
                        convert_f64_to_v::<V>(2.0, "xor rhs")?,
                    ),
                    &format!("{}_xor_pair_{}_{}", self.id.name, i, j),
                    source.clone(),
                ));
            }
        }

        Ok(constraints)
    }
}

impl<V> Display for XorFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "xor({})", self.id.name)
    }
}

impl<V> DynSymbol for XorFunction<V>
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

impl<V> Symbol for XorFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for XorFunction<V>
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

    fn flush(&self, _force: bool) {}

    fn register_auxiliary_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        <Self as FunctionSymbol<V>>::register_tokens(self, tokens)
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.build_mechanism_constraints(symbol_to_index, self.resolve_big_m(None))
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let big_m = self.resolve_big_m(self.infer_big_m_from_tokens(tokens));
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
        format!("xor({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for XorFunction<V>
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
        for indicator_var in &self.indicator_vars {
            tokens.push(Token::from_generic(
                indicator_var.clone(),
                indicator_var.index(),
            ));
        }
        for side_var in &self.side_vars {
            tokens.push(Token::from_generic(side_var.clone(), side_var.index()));
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let mut non_zero_count = 0usize;

        for polynomial in &self.polynomials {
            let value = evaluate_linear(polynomial, token_table, zero_if_none)?;
            let magnitude = to_f64(&value)?.abs();
            if magnitude > self.tolerance && magnitude < self.strict_boundary {
                return None;
            }
            if magnitude >= self.strict_boundary {
                non_zero_count += 1;
            }
        }

        from_f64(if non_zero_count == 1 { 1.0 } else { 0.0 })
    }
}

impl<V> LinearIntermediateSymbol<V> for XorFunction<V>
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

impl<V> LogicFunctionSymbol<V> for XorFunction<V>
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
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::token::{MutableTokenList, Token, VecTokenList};
    use crate::variable::{BinaryVariableItem, ContinuousVariableItem, VariableRange};

    fn bool_poly(var_index: usize) -> Linear<f64> {
        Linear::new(vec![LinearMonomial::new(1.0, var_index)], 0.0)
    }

    fn make_binary_tokens(x: f64, y: f64) -> VecTokenList<f64> {
        let x_var = BinaryVariableItem::create(VariableId::standalone(0), "x");
        let y_var = BinaryVariableItem::create(VariableId::standalone(1), "y");

        let tx = Token::from_generic(x_var, 0);
        tx.set_result(x);
        let ty = Token::from_generic(y_var, 1);
        ty.set_result(y);

        let mut tokens = VecTokenList::new();
        tokens.add_token(tx);
        tokens.add_token(ty);
        tokens
    }

    #[test]
    fn and_or_not_xor_calculate_values() {
        let tokens = make_binary_tokens(1.0, 0.0);
        let p0 = bool_poly(0);
        let p1 = bool_poly(1);

        let and_fn = AndFunction::new(100, "and_xy", vec![p0.clone(), p1.clone()]);
        let or_fn = OrFunction::new(101, "or_xy", vec![p0.clone(), p1.clone()]);
        let not_fn = NotFunction::new(102, "not_y", p1.clone());
        let xor_fn = XorFunction::new(103, "xor_xy", vec![p0, p1]);

        assert_eq!(
            <AndFunction as FunctionSymbol>::calculate_value(&and_fn, &tokens, false),
            Some(0.0)
        );
        assert_eq!(
            <OrFunction as FunctionSymbol>::calculate_value(&or_fn, &tokens, false),
            Some(1.0)
        );
        assert_eq!(
            <NotFunction as FunctionSymbol>::calculate_value(&not_fn, &tokens, false),
            Some(1.0)
        );
        assert_eq!(
            <XorFunction as FunctionSymbol>::calculate_value(&xor_fn, &tokens, false),
            Some(1.0)
        );
    }

    #[test]
    fn logic_functions_support_f32_values() {
        let x_var = BinaryVariableItem::create(VariableId::standalone(0), "x");
        let y_var = BinaryVariableItem::create(VariableId::standalone(1), "y");
        let tx = Token::from_generic(x_var, 0);
        tx.set_result(1.0_f32);
        let ty = Token::from_generic(y_var, 1);
        ty.set_result(0.0_f32);
        let mut tokens = VecTokenList::<f32>::new();
        tokens.add_token(tx);
        tokens.add_token(ty);

        let p0 = Linear::new(vec![LinearMonomial::new(1.0_f32, 0)], 0.0_f32);
        let p1 = Linear::new(vec![LinearMonomial::new(1.0_f32, 1)], 0.0_f32);
        let and_fn: AndFunction<f32> =
            AndFunction::new(1001, "and_f32", vec![p0.clone(), p1.clone()]);
        let or_fn: OrFunction<f32> = OrFunction::new(1002, "or_f32", vec![p0.clone(), p1.clone()]);
        let not_fn: NotFunction<f32> = NotFunction::new(1003, "not_f32", p1.clone());
        let xor_fn: XorFunction<f32> = XorFunction::new(1004, "xor_f32", vec![p0, p1]);

        assert_eq!(
            <AndFunction<f32> as FunctionSymbol<f32>>::calculate_value(&and_fn, &tokens, false),
            Some(0.0_f32)
        );
        assert_eq!(
            <OrFunction<f32> as FunctionSymbol<f32>>::calculate_value(&or_fn, &tokens, false),
            Some(1.0_f32)
        );
        assert_eq!(
            <NotFunction<f32> as FunctionSymbol<f32>>::calculate_value(&not_fn, &tokens, false),
            Some(1.0_f32)
        );
        assert_eq!(
            <XorFunction<f32> as FunctionSymbol<f32>>::calculate_value(&xor_fn, &tokens, false),
            Some(1.0_f32)
        );

        let mut aux_tokens = Vec::new();
        <XorFunction<f32> as FunctionSymbol<f32>>::register_tokens(&xor_fn, &mut aux_tokens)
            .expect("f32 xor tokens should be registered");
        let symbol_to_index: HashMap<_, _> = aux_tokens
            .iter()
            .map(|token| (token.id().unique_id() as usize, token.solver_index))
            .collect();
        let constraints = xor_fn
            .mechanism_constraints(&symbol_to_index)
            .expect("f32 xor mechanism constraints should be generated");

        assert!(!constraints.is_empty());
    }

    #[test]
    fn xor_all_equal_is_zero() {
        let tokens = make_binary_tokens(1.0, 1.0);
        let xor_fn = XorFunction::new(104, "xor_equal", vec![bool_poly(0), bool_poly(1)]);
        assert_eq!(
            <XorFunction as FunctionSymbol>::calculate_value(&xor_fn, &tokens, false),
            Some(0.0)
        );
    }

    #[test]
    fn and_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(80_000),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let and_fn = AndFunction::new(
            2000,
            "and_bound",
            vec![Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0)],
        );

        let mut aux_tokens = Vec::new();
        <AndFunction as FunctionSymbol>::register_tokens(&and_fn, &mut aux_tokens)
            .expect("and tokens should be registered");
        let mut symbol_to_index = HashMap::new();
        for token in &aux_tokens {
            symbol_to_index.insert(token.id().unique_id() as usize, token.solver_index);
        }
        let tokens = vec![Token::from_generic(x, 0)];

        let constraints = and_fn
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("and constraints should be generated");
        let band_ub = constraints
            .iter()
            .find(|constraint| constraint.name == "and_bound_and_nz_0_band_ub")
            .expect("and band upper constraint should exist");
        let indicator_index = *symbol_to_index
            .get(&(and_fn.indicator_variables()[0].id().unique_id() as usize))
            .expect("indicator index should exist");
        let indicator_term = band_ub
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == indicator_index)
            .expect("indicator term should exist");

        assert!((*indicator_term.coefficient() + 7.0).abs() <= 1e-9);
    }

    #[test]
    fn and_function_keeps_exact_inferred_bound_feasible_at_strict_boundary() {
        let and_fn = AndFunction::new(
            2002,
            "and_exact_bound",
            vec![Linear::new(vec![], 1.0), Linear::new(vec![], 0.0)],
        );

        let mut aux_tokens = Vec::new();
        <AndFunction as FunctionSymbol>::register_tokens(&and_fn, &mut aux_tokens)
            .expect("and tokens should be registered");
        let symbol_to_index = aux_tokens
            .iter()
            .map(|token| (token.id().unique_id() as usize, token.solver_index))
            .collect::<HashMap<_, _>>();
        let constraints = and_fn
            .mechanism_constraints_with_tokens(&symbol_to_index, &[])
            .expect("and constraints should be generated");
        let feasible = HashMap::from([
            (
                *symbol_to_index
                    .get(&(and_fn.result_variable().id().unique_id() as usize))
                    .expect("result index should exist"),
                0.0,
            ),
            (
                *symbol_to_index
                    .get(&(and_fn.indicator_variables()[0].id().unique_id() as usize))
                    .expect("first indicator index should exist"),
                1.0,
            ),
            (
                *symbol_to_index
                    .get(&(and_fn.indicator_variables()[1].id().unique_id() as usize))
                    .expect("second indicator index should exist"),
                0.0,
            ),
            (
                *symbol_to_index
                    .get(&(and_fn.side_variables()[0].id().unique_id() as usize))
                    .expect("first side index should exist"),
                1.0,
            ),
            (
                *symbol_to_index
                    .get(&(and_fn.side_variables()[1].id().unique_id() as usize))
                    .expect("second side index should exist"),
                0.0,
            ),
        ]);

        for constraint in &constraints {
            let polynomial = &constraint.inequality.polynomial;
            let lhs = *polynomial.constant_term()
                + polynomial
                    .monomials()
                    .iter()
                    .map(|monomial| {
                        *monomial.coefficient()
                            * feasible.get(&monomial.var_index()).copied().unwrap_or(0.0)
                    })
                    .sum::<f64>();
            let rhs = constraint.inequality.rhs;
            let satisfied = match constraint.inequality.relation {
                ConstraintRelation::LessEqual => lhs <= rhs + 1e-12,
                ConstraintRelation::Equal => (lhs - rhs).abs() <= 1e-12,
                ConstraintRelation::GreaterEqual => lhs + 1e-12 >= rhs,
            };
            assert!(
                satisfied,
                "{} is not satisfied by the exact-bound fixture: lhs={}, rhs={}",
                constraint.name, lhs, rhs
            );
        }
    }

    #[test]
    fn and_function_falls_back_to_default_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(80_010), "x");
        let and_fn = AndFunction::new(
            2001,
            "and_default",
            vec![Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0)],
        );

        let mut aux_tokens = Vec::new();
        <AndFunction as FunctionSymbol>::register_tokens(&and_fn, &mut aux_tokens)
            .expect("and tokens should be registered");
        let mut symbol_to_index = HashMap::new();
        for token in &aux_tokens {
            symbol_to_index.insert(token.id().unique_id() as usize, token.solver_index);
        }
        let tokens = vec![Token::from_generic(x, 0)];

        let constraints = and_fn
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("and constraints should be generated");
        let band_ub = constraints
            .iter()
            .find(|constraint| constraint.name == "and_default_and_nz_0_band_ub")
            .expect("and band upper constraint should exist");
        let indicator_index = *symbol_to_index
            .get(&(and_fn.indicator_variables()[0].id().unique_id() as usize))
            .expect("indicator index should exist");
        let indicator_term = band_ub
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == indicator_index)
            .expect("indicator term should exist");

        assert!((*indicator_term.coefficient() + DEFAULT_BIG_M).abs() <= 1e-9);
    }

    #[test]
    fn not_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(81_000),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let not_fn = NotFunction::new(
            2100,
            "not_bound",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
        );

        let mut aux_tokens = Vec::new();
        <NotFunction as FunctionSymbol>::register_tokens(&not_fn, &mut aux_tokens)
            .expect("not tokens should be registered");
        let mut symbol_to_index = HashMap::new();
        for token in &aux_tokens {
            symbol_to_index.insert(token.id().unique_id() as usize, token.solver_index);
        }
        let tokens = vec![Token::from_generic(x, 0)];

        let constraints = not_fn
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("not constraints should be generated");
        let band_ub = constraints
            .iter()
            .find(|constraint| constraint.name == "not_bound_not_nz_band_ub")
            .expect("not band upper constraint should exist");
        let indicator_index = *symbol_to_index
            .get(&(not_fn.indicator_variable().id().unique_id() as usize))
            .expect("indicator index should exist");
        let indicator_term = band_ub
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == indicator_index)
            .expect("indicator term should exist");

        assert!((*indicator_term.coefficient() + 7.0).abs() <= 1e-9);
    }

    #[test]
    fn not_function_falls_back_to_default_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(81_010), "x");
        let not_fn = NotFunction::new(
            2101,
            "not_default",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );

        let mut aux_tokens = Vec::new();
        <NotFunction as FunctionSymbol>::register_tokens(&not_fn, &mut aux_tokens)
            .expect("not tokens should be registered");
        let mut symbol_to_index = HashMap::new();
        for token in &aux_tokens {
            symbol_to_index.insert(token.id().unique_id() as usize, token.solver_index);
        }
        let tokens = vec![Token::from_generic(x, 0)];

        let constraints = not_fn
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("not constraints should be generated");
        let band_ub = constraints
            .iter()
            .find(|constraint| constraint.name == "not_default_not_nz_band_ub")
            .expect("not band upper constraint should exist");
        let indicator_index = *symbol_to_index
            .get(&(not_fn.indicator_variable().id().unique_id() as usize))
            .expect("indicator index should exist");
        let indicator_term = band_ub
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == indicator_index)
            .expect("indicator term should exist");

        assert!((*indicator_term.coefficient() + DEFAULT_BIG_M).abs() <= 1e-9);
    }

    fn constraint_holds(constraint: &LinearConstraint<f64>, values: &HashMap<usize, f64>) -> bool {
        let mut lhs = *constraint.inequality.polynomial.constant_term();
        for monomial in constraint.inequality.polynomial.monomials() {
            lhs += *monomial.coefficient()
                * values.get(&monomial.var_index()).copied().unwrap_or(0.0);
        }
        let rhs = constraint.inequality.rhs;
        match constraint.inequality.relation {
            ConstraintRelation::LessEqual => lhs <= rhs + 1e-9,
            ConstraintRelation::GreaterEqual => lhs + 1e-9 >= rhs,
            ConstraintRelation::Equal => (lhs - rhs).abs() <= 1e-9,
        }
    }

    fn binary_pair() -> (
        Vec<Token<f64>>,
        BinaryVariableItem,
        BinaryVariableItem,
    ) {
        let x = BinaryVariableItem::create(VariableId::standalone(90_000), "x");
        let y = BinaryVariableItem::create(VariableId::standalone(90_001), "y");
        let tokens = vec![
            Token::from_generic(x.clone(), 0),
            Token::from_generic(y.clone(), 1),
        ];
        (tokens, x, y)
    }

    #[test]
    fn direct_binary_inputs_use_the_compact_and_hull() {
        let (registered, _x, _y) = binary_pair();
        let and_fn = AndFunction::new(3000, "and_hull", vec![bool_poly(0), bool_poly(1)]);

        let mut auxiliary = Vec::new();
        and_fn
            .register_auxiliary_tokens_with_context(&mut auxiliary, &registered)
            .expect("and auxiliary tokens should register");
        // 直接二值输入只注册结果列，不再产生非零指示与侧辅助变量。
        // Direct binary inputs register the result column only, with no nonzero indicator
        // or side helper variables.
        assert_eq!(auxiliary.len(), 1);
        assert_eq!(auxiliary[0].id(), and_fn.result_variable().id());

        let mut tokens = registered.clone();
        let result_index = tokens.len();
        tokens.push(auxiliary[0].clone());
        let symbol_to_index = HashMap::from([(
            and_fn.result_variable().id().unique_id() as usize,
            result_index,
        )]);

        let constraints = and_fn
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("and hull constraints should be generated");
        let names = constraints
            .iter()
            .map(|constraint| constraint.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "and_hull_binary_le_0",
                "and_hull_binary_le_1",
                "and_hull_binary_sum"
            ]
        );

        for x_value in [0.0_f64, 1.0] {
            for y_value in [0.0_f64, 1.0] {
                let expected = if x_value == 1.0 && y_value == 1.0 {
                    1.0
                } else {
                    0.0
                };
                for result in [0.0_f64, 1.0] {
                    let values = HashMap::from([
                        (0usize, x_value),
                        (1usize, y_value),
                        (result_index, result),
                    ]);
                    let feasible = constraints
                        .iter()
                        .all(|constraint| constraint_holds(constraint, &values));
                    assert_eq!(
                        feasible,
                        result == expected,
                        "and hull mismatch for x={x_value}, y={y_value}, result={result}"
                    );
                }
            }
        }
    }

    #[test]
    fn direct_binary_inputs_use_the_compact_or_hull() {
        let (registered, _x, _y) = binary_pair();
        let or_fn = OrFunction::new(3010, "or_hull", vec![bool_poly(0), bool_poly(1)]);

        let mut auxiliary = Vec::new();
        or_fn
            .register_auxiliary_tokens_with_context(&mut auxiliary, &registered)
            .expect("or auxiliary tokens should register");
        assert_eq!(auxiliary.len(), 1);
        assert_eq!(auxiliary[0].id(), or_fn.result_variable().id());

        let mut tokens = registered.clone();
        let result_index = tokens.len();
        tokens.push(auxiliary[0].clone());
        let symbol_to_index = HashMap::from([(
            or_fn.result_variable().id().unique_id() as usize,
            result_index,
        )]);

        let constraints = or_fn
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("or hull constraints should be generated");
        let names = constraints
            .iter()
            .map(|constraint| constraint.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "or_hull_binary_ge_0",
                "or_hull_binary_ge_1",
                "or_hull_binary_sum"
            ]
        );

        for x_value in [0.0_f64, 1.0] {
            for y_value in [0.0_f64, 1.0] {
                let expected = if x_value == 1.0 || y_value == 1.0 {
                    1.0
                } else {
                    0.0
                };
                for result in [0.0_f64, 1.0] {
                    let values = HashMap::from([
                        (0usize, x_value),
                        (1usize, y_value),
                        (result_index, result),
                    ]);
                    let feasible = constraints
                        .iter()
                        .all(|constraint| constraint_holds(constraint, &values));
                    assert_eq!(
                        feasible,
                        result == expected,
                        "or hull mismatch for x={x_value}, y={y_value}, result={result}"
                    );
                }
            }
        }
    }

    #[test]
    fn direct_binary_input_uses_the_compact_not_hull() {
        let (registered, _x, _y) = binary_pair();
        let not_fn = NotFunction::new(3020, "not_hull", bool_poly(1));

        let mut auxiliary = Vec::new();
        not_fn
            .register_auxiliary_tokens_with_context(&mut auxiliary, &registered)
            .expect("not auxiliary tokens should register");
        assert_eq!(auxiliary.len(), 1);
        assert_eq!(auxiliary[0].id(), not_fn.result_variable().id());

        let mut tokens = registered.clone();
        let result_index = tokens.len();
        tokens.push(auxiliary[0].clone());
        let symbol_to_index = HashMap::from([(
            not_fn.result_variable().id().unique_id() as usize,
            result_index,
        )]);

        let constraints = not_fn
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("not hull constraints should be generated");
        assert_eq!(constraints.len(), 1);
        assert_eq!(constraints[0].name, "not_hull_binary_result");

        for y_value in [0.0_f64, 1.0] {
            let expected = 1.0 - y_value;
            for result in [0.0_f64, 1.0] {
                let values = HashMap::from([
                    (0usize, 0.0),
                    (1usize, y_value),
                    (result_index, result),
                ]);
                let feasible = constraint_holds(&constraints[0], &values);
                assert_eq!(
                    feasible,
                    result == expected,
                    "not hull mismatch for y={y_value}, result={result}"
                );
            }
        }
    }

    #[test]
    fn not_structure_materializes_the_same_row_as_eager_expansion() {
        let (registered, _x, _y) = binary_pair();
        let not_fn = NotFunction::new(3021, "not_deferred", bool_poly(1));

        let mut tokens = registered.clone();
        let structure = not_fn
            .deferred_structure_with_tokens(&tokens)
            .expect("direct binary input should expose a deferred structure");
        assert_eq!(structure.function_name(), "not_deferred");
        let binding = structure
            .usage_binding()
            .expect("not structure should expose a usage binding");
        assert_eq!(binding.helpers.len(), 0);
        assert_eq!(binding.result, not_fn.result_variable().id());
        assert!(structure.fingerprint().is_some());

        let mut auxiliary = Vec::new();
        not_fn
            .register_auxiliary_tokens_with_context(&mut auxiliary, &tokens)
            .expect("not auxiliary tokens should register");
        tokens.push(auxiliary[0].clone());
        let result_index = tokens.len() - 1;
        let symbol_to_index = HashMap::from([(
            not_fn.result_variable().id().unique_id() as usize,
            result_index,
        )]);

        let eager = not_fn
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("eager not constraints should be generated");
        let deferred = structure
            .materialize(&symbol_to_index)
            .expect("not structure should materialize");
        assert_eq!(eager.len(), 1);
        assert_eq!(deferred.len(), 1);
        assert_eq!(eager[0].name, deferred[0].name);
        assert_eq!(eager[0].inequality.relation, deferred[0].inequality.relation);
        assert_eq!(eager[0].inequality.rhs, deferred[0].inequality.rhs);
        assert_eq!(
            eager[0].inequality.polynomial.constant_term(),
            deferred[0].inequality.polynomial.constant_term()
        );
    }

    #[test]
    fn and_structure_materializes_the_same_rows_as_eager_expansion() {
        let (registered, _x, _y) = binary_pair();
        let and_fn = AndFunction::new(3030, "and_deferred", vec![bool_poly(0), bool_poly(1)]);

        let mut tokens = registered.clone();
        let structure = and_fn
            .deferred_structure_with_tokens(&tokens)
            .expect("direct binary inputs should expose a deferred structure");
        assert_eq!(structure.function_name(), "and_deferred");
        let binding = structure
            .usage_binding()
            .expect("and structure should expose a usage binding");
        assert_eq!(binding.helpers.len(), 0);
        assert_eq!(binding.result, and_fn.result_variable().id());
        assert!(structure.fingerprint().is_some());

        let mut auxiliary = Vec::new();
        and_fn
            .register_auxiliary_tokens_with_context(&mut auxiliary, &tokens)
            .expect("and auxiliary tokens should register");
        assert_eq!(auxiliary.len(), 1);
        tokens.push(auxiliary[0].clone());
        let result_index = tokens.len() - 1;
        let symbol_to_index = HashMap::from([(
            and_fn.result_variable().id().unique_id() as usize,
            result_index,
        )]);

        let eager = and_fn
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("eager and constraints should be generated");
        let deferred = structure
            .materialize(&symbol_to_index)
            .expect("and structure should materialize");
        assert!(eager.len() >= 2);
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

    #[test]
    fn or_structure_materializes_the_same_rows_as_eager_expansion() {
        let (registered, _x, _y) = binary_pair();
        let or_fn = OrFunction::new(3040, "or_deferred", vec![bool_poly(0), bool_poly(1)]);

        let mut tokens = registered.clone();
        let structure = or_fn
            .deferred_structure_with_tokens(&tokens)
            .expect("direct binary inputs should expose a deferred structure");
        assert_eq!(structure.function_name(), "or_deferred");
        let binding = structure
            .usage_binding()
            .expect("or structure should expose a usage binding");
        assert_eq!(binding.helpers.len(), 0);
        assert_eq!(binding.result, or_fn.result_variable().id());
        assert!(structure.fingerprint().is_some());

        let mut auxiliary = Vec::new();
        or_fn
            .register_auxiliary_tokens_with_context(&mut auxiliary, &tokens)
            .expect("or auxiliary tokens should register");
        assert_eq!(auxiliary.len(), 1);
        tokens.push(auxiliary[0].clone());
        let result_index = tokens.len() - 1;
        let symbol_to_index = HashMap::from([(
            or_fn.result_variable().id().unique_id() as usize,
            result_index,
        )]);

        let eager = or_fn
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("eager or constraints should be generated");
        let deferred = structure
            .materialize(&symbol_to_index)
            .expect("or structure should materialize");
        assert!(eager.len() >= 2);
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

    #[test]
    fn general_numeric_inputs_keep_indicator_helpers() {
        let (registered, _x, _y) = binary_pair();

        // 系数不为 1 的二值输入不是"直接二值变量"，仍使用通用非零指示路径。
        // A binary input whose coefficient is not 1 is not a direct binary variable and
        // still uses the general nonzero indicator path.
        let scaled = AndFunction::new(
            3030,
            "and_scaled",
            vec![Linear::new(vec![LinearMonomial::new(2.0, 0)], 0.0)],
        );
        let mut auxiliary = Vec::new();
        scaled
            .register_auxiliary_tokens_with_context(&mut auxiliary, &registered)
            .expect("scaled and auxiliary tokens should register");
        assert_eq!(auxiliary.len(), 3);

        // 连续输入同样保留非零指示与侧辅助变量。
        // A continuous input likewise keeps the nonzero indicator and side helpers.
        let continuous = vec![Token::from_generic(
            ContinuousVariableItem::with_range(
                VariableId::standalone(90_010),
                "z",
                VariableRange::bounded(0.0, 5.0),
            ),
            0,
        )];
        let numeric = OrFunction::new(3031, "or_numeric", vec![bool_poly(0)]);
        let mut auxiliary = Vec::new();
        numeric
            .register_auxiliary_tokens_with_context(&mut auxiliary, &continuous)
            .expect("numeric or auxiliary tokens should register");
        assert_eq!(auxiliary.len(), 3);
    }
}
