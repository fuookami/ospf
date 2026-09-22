//! 绝对值函数符号 / Abs function symbol

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    auto_intermediate_symbol_name, next_auto_intermediate_symbol_id,
};
use super::big_m::{BigMPolicy, infer_linear_bounds_from_tokens};
use crate::error::{ModelError, Result};
use crate::model::intermediate::{
    ConstraintSource, DOMAIN_PROOF_LOST, DOMAIN_PROOF_WIDENED, DeferredFunctionStructure,
    InputDomainProof, StructureUsageBinding, fingerprint_float,
};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{
    BinaryVariableItem, ContinuousVariableItem, VariableId, VariableRange, new_group_id,
};
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

const DEFAULT_BIG_M: f64 = 1_000_000.0;
const BIG_M_POLICY: BigMPolicy = BigMPolicy::new(DEFAULT_BIG_M, 1.0);

/// 绝对值两条分支约束使用的非对称 Big-M。
///
/// `y = |x|` 的精确图关系由四条约束表达：
/// `y - x >= 0`、`y + x >= 0`、`y - x + M_pos * b <= M_pos`、
/// `y + x - M_neg * b <= 0`。两条含 `M` 的行只在选择器指向另一侧时起松弛作用，
/// 因此只需覆盖该松弛方向上真实解的取值，不需要统一的 `2 * |x|` 上界。
///
/// Asymmetric Big-M pair used by the two absolute-value branch rows.
///
/// The exact graph relation of `y = |x|` is expressed by `y - x >= 0`,
/// `y + x >= 0`, `y - x + M_pos * b <= M_pos`, and `y + x - M_neg * b <= 0`.
/// Each row containing `M` only relaxes when the selector points at the other side,
/// so it must cover the true solution over that relaxation direction rather than a
/// unified `2 * |x|` upper bound.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AbsBranchBigM {
    /// 正分支行 `y - x + M * b <= M` 的松弛系数，需覆盖 `max(y - x) = max(0, -2 * lower)`。
    /// Relaxation coefficient of the positive branch row `y - x + M * b <= M`,
    /// which must cover `max(y - x) = max(0, -2 * lower)`.
    pub positive_branch: f64,
    /// 负分支行 `y + x - M * b <= 0` 的松弛系数，需覆盖 `max(y + x) = max(0, 2 * upper)`。
    /// Relaxation coefficient of the negative branch row `y + x - M * b <= 0`,
    /// which must cover `max(y + x) = max(0, 2 * upper)`.
    pub negative_branch: f64,
}

impl AbsBranchBigM {
    /// 使用回退 Big-M 构造对称取值 / Build the symmetric pair from the policy fallback.
    pub fn fallback() -> Self {
        Self {
            positive_branch: BIG_M_POLICY.resolve(None),
            negative_branch: BIG_M_POLICY.resolve(None),
        }
    }

    /// 由输入多项式的有限值域推导非对称 Big-M。
    ///
    /// 对 `x ∈ [lower, upper]`：`y - x` 在真实解上的最大值为 `max(0, -2 * lower)`，
    /// `y + x` 的最大值为 `max(0, 2 * upper)`。两者都只覆盖真实解，因此比统一上界更紧，
    /// 同时不会裁掉任何可行分支。
    ///
    /// Derive the asymmetric Big-M pair from a finite input domain.
    ///
    /// For `x ∈ [lower, upper]` the true solution maximizes `y - x` at
    /// `max(0, -2 * lower)` and `y + x` at `max(0, 2 * upper)`. Both values only cover
    /// true solutions, so they are tighter than a unified bound without cutting off any
    /// feasible branch.
    pub fn from_input_bounds(lower: f64, upper: f64) -> Option<Self> {
        if !lower.is_finite() || !upper.is_finite() || lower > upper {
            return None;
        }
        let positive_branch = if lower < 0.0 { -2.0 * lower } else { 0.0 };
        let negative_branch = if upper > 0.0 { 2.0 * upper } else { 0.0 };
        if !positive_branch.is_finite() || !negative_branch.is_finite() {
            return None;
        }
        Some(Self {
            positive_branch: clamp_branch_big_m(positive_branch),
            negative_branch: clamp_branch_big_m(negative_branch),
        })
    }
}

/// 把推断的分支 Big-M 限制到策略最小值；非有限值回退到策略回退值。
/// Clamp an inferred branch Big-M to the policy minimum; non-finite values fall back.
fn clamp_branch_big_m(value: f64) -> f64 {
    if value.is_finite() {
        value.max(BIG_M_POLICY.min())
    } else {
        BIG_M_POLICY.fallback()
    }
}

/// 绝对值函数 / Abs Function
///
/// 数学形式 / Mathematical Form:
/// - result = |x|
#[derive(Debug, Clone)]
pub struct AbsFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Linear<V>,
    result_var: ContinuousVariableItem,
    side_var: BinaryVariableItem,
}

impl<V> AbsFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建新的绝对值函数 / Create new abs function
    pub fn new(id: u64, name: &str, input: Linear<V>) -> Self {
        let group_id = new_group_id();
        let result_var =
            ContinuousVariableItem::create(VariableId::new(group_id, 0), &format!("{}_abs", name));
        let side_var =
            BinaryVariableItem::create(VariableId::new(group_id, 1), &format!("{}_side", name));

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            result_var,
            side_var,
        }
    }

    /// 使用自动 ID 与调用方提供的名称创建绝对值函数。
    /// Create an abs function with an auto id and caller-provided name.
    pub fn named(name: impl AsRef<str>, input: Linear<V>) -> Self {
        Self::new(next_auto_intermediate_symbol_id(), name.as_ref(), input)
    }

    /// 使用自动 ID 与自动名称创建绝对值函数。
    /// Create an abs function with an auto id and auto-generated name.
    pub fn auto(input: Linear<V>) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("abs", id);
        Self::new(id, &name, input)
    }

    /// 获取输入线性多项式 / Get the input linear polynomial.
    pub fn input_polynomial(&self) -> &Linear<V> {
        &self.input
    }

    /// 获取绝对值结果变量 / Get the absolute-value result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 获取符号辅助变量 / Get the sign auxiliary variable.
    pub fn side_variable(&self) -> &BinaryVariableItem {
        &self.side_var
    }

    fn infer_big_m_from_tokens(&self, tokens: &[Token<V>]) -> Option<AbsBranchBigM>
    where
        V: ToPrimitive,
    {
        let (lower, upper) = self.infer_input_bounds(tokens)?;
        AbsBranchBigM::from_input_bounds(lower, upper)
    }

    /// 从令牌边界推断输入多项式的有限值域 / Infer the finite input domain from token bounds.
    pub fn infer_input_bounds(&self, tokens: &[Token<V>]) -> Option<(f64, f64)>
    where
        V: ToPrimitive,
    {
        infer_linear_bounds_from_tokens(&self.input, tokens)
    }
}

impl<V> AbsFunction<V>
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
        big_m: AbsBranchBigM,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "abs result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;
        let side_index = symbol_to_index
            .get(&(self.side_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "abs side variable id {}",
                    self.side_var.id().unique_id()
                ))
            })?;

        generate_abs_constraints(
            &self.id.name,
            &self.input,
            result_index,
            side_index,
            big_m,
            Arc::new(self.clone()),
        )
    }
}

/// 绝对值函数的通用 fallback 公式生成器。
///
/// EAGER 展开与延迟结构的 fallback 物化共用本函数，保证两条路径产生逐列相同的稀疏行：
/// `y - x >= 0`、`y + x >= 0`、`y - x + M_pos * b <= M_pos`、`y + x - M_neg * b <= 0`。
///
/// Shared generic-fallback formula generator of the absolute-value function.
///
/// Eager expansion and deferred fallback materialization both call this function so the two
/// paths produce identical sparse rows: `y - x >= 0`, `y + x >= 0`,
/// `y - x + M_pos * b <= M_pos`, and `y + x - M_neg * b <= 0`.
pub(crate) fn generate_abs_constraints<V>(
    name: &str,
    input: &Linear<V>,
    result_index: usize,
    side_index: usize,
    big_m: AbsBranchBigM,
    source: Arc<dyn IntermediateSymbol<V>>,
) -> Result<Vec<LinearConstraint<V>>>
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
{
    let mut constraints = Vec::new();
    let mut input_monomials = Vec::with_capacity(input.monomials().len());
    for monomial in input.monomials() {
        let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "abs `{}` input coefficient cannot be converted to f64",
                name
            ))
        })?;
        input_monomials.push((coefficient, monomial.var_index()));
    }
    let input_constant = to_f64(input.constant_term()).ok_or_else(|| {
        ModelError::InvalidConstraint(format!(
            "abs `{}` input constant cannot be converted to f64",
            name
        ))
    })?;

    // y - x >= 0
    let mut ge_x_monomials = Vec::with_capacity(input.monomials().len() + 1);
    ge_x_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(1.0, "abs result coefficient")?,
        result_index,
    ));
    for (coefficient, index) in &input_monomials {
        ge_x_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-*coefficient, "abs input coefficient")?,
            *index,
        ));
    }
    constraints.push(LinearConstraint::from_symbol(
        LinearInequality::new(
            Linear::new(
                ge_x_monomials,
                convert_f64_to_v::<V>(-input_constant, "abs input constant")?,
            ),
            ConstraintRelation::GreaterEqual,
            convert_f64_to_v::<V>(0.0, "abs rhs")?,
        ),
        &format!("{}_abs_ge_x", name),
        source.clone(),
    ));

    // y + x >= 0
    let mut ge_neg_x_monomials = Vec::with_capacity(input.monomials().len() + 1);
    ge_neg_x_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(1.0, "abs result coefficient")?,
        result_index,
    ));
    for (coefficient, index) in &input_monomials {
        ge_neg_x_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(*coefficient, "abs input coefficient")?,
            *index,
        ));
    }
    constraints.push(LinearConstraint::from_symbol(
        LinearInequality::new(
            Linear::new(
                ge_neg_x_monomials,
                convert_f64_to_v::<V>(input_constant, "abs input constant")?,
            ),
            ConstraintRelation::GreaterEqual,
            convert_f64_to_v::<V>(0.0, "abs rhs")?,
        ),
        &format!("{}_abs_ge_neg_x", name),
        source.clone(),
    ));

    // y - x + M * b <= M
    let mut le_pos_branch_monomials = Vec::with_capacity(input.monomials().len() + 2);
    le_pos_branch_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(1.0, "abs result coefficient")?,
        result_index,
    ));
    for (coefficient, index) in &input_monomials {
        le_pos_branch_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-*coefficient, "abs input coefficient")?,
            *index,
        ));
    }
    le_pos_branch_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(big_m.positive_branch, "abs side coefficient")?,
        side_index,
    ));
    constraints.push(LinearConstraint::from_symbol(
        LinearInequality::new(
            Linear::new(
                le_pos_branch_monomials,
                convert_f64_to_v::<V>(-input_constant, "abs input constant")?,
            ),
            ConstraintRelation::LessEqual,
            convert_f64_to_v::<V>(big_m.positive_branch, "abs rhs")?,
        ),
        &format!("{}_abs_pos_branch", name),
        source.clone(),
    ));

    // y + x - M * b <= 0
    let mut le_neg_branch_monomials = Vec::with_capacity(input.monomials().len() + 2);
    le_neg_branch_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(1.0, "abs result coefficient")?,
        result_index,
    ));
    for (coefficient, index) in &input_monomials {
        le_neg_branch_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(*coefficient, "abs input coefficient")?,
            *index,
        ));
    }
    le_neg_branch_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(-big_m.negative_branch, "abs side coefficient")?,
        side_index,
    ));
    constraints.push(LinearConstraint::from_symbol(
        LinearInequality::new(
            Linear::new(
                le_neg_branch_monomials,
                convert_f64_to_v::<V>(input_constant, "abs input constant")?,
            ),
            ConstraintRelation::LessEqual,
            convert_f64_to_v::<V>(0.0, "abs rhs")?,
        ),
        &format!("{}_abs_neg_branch", name),
        source,
    ));

    Ok(constraints)
}

/// 绝对值函数的求解器无关结构描述 / Solver-neutral structure description of the ABS function
///
/// 只保留 fallback 公式所需的数据：函数名、输入多项式快照、结果列与侧列的令牌 ID、非对称
/// Big-M 及其来源符号。字段对模型各层保持只读，不持有求解器 SDK 对象。
///
/// Keeps only the data the fallback formula needs: function name, input polynomial snapshot,
/// token IDs of the result and side columns, the asymmetric Big-M pair, and the source symbol for
/// reporting. Every field is read-only for downstream layers and no solver SDK object is held.
#[derive(Debug, Clone)]
pub struct AbsStructure<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 函数名称 / Function name
    name: String,
    /// 输入多项式快照 / Input polynomial snapshot
    input: Linear<V>,
    /// 结果列 / Result column
    result: VariableId,
    /// 侧列 / Side column
    side: VariableId,
    /// 来源符号 ID，其关系行不计入外部引用 / Source symbol ID whose rows are not external references
    source_symbol_id: u64,
    /// 非对称分支 Big-M / Asymmetric branch Big-M pair
    big_m: AbsBranchBigM,
    /// 推导该 Big-M 所依据的输入域证明 / Input-domain proof the Big-M was derived from
    input_domain: Option<InputDomainProof>,
    /// 约束来源符号 / Constraint source symbol
    source: ConstraintSource<V>,
}

impl<V> AbsStructure<V>
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
{
    /// 创建结构描述 / Create a structure description.
    pub fn new(
        name: impl Into<String>,
        input: Linear<V>,
        result: VariableId,
        side: VariableId,
        big_m: AbsBranchBigM,
        source: ConstraintSource<V>,
    ) -> Self {
        let source_symbol_id = source.id().id;
        Self {
            name: name.into(),
            input,
            result,
            side,
            source_symbol_id,
            big_m,
            input_domain: None,
            source,
        }
    }

    /// 附上推导 Big-M 所依据的输入域证明 / Attach the input-domain proof the Big-M was derived from.
    pub fn with_input_domain(mut self, input_domain: Option<InputDomainProof>) -> Self {
        self.input_domain = input_domain;
        self
    }

    /// 获取非对称分支 Big-M / Get the asymmetric branch Big-M pair.
    pub fn big_m(&self) -> AbsBranchBigM {
        self.big_m
    }

    /// 获取输入域证明 / Get the input-domain proof.
    pub fn input_domain(&self) -> Option<InputDomainProof> {
        self.input_domain
    }

    /// 获取函数名称 / Get the function name.
    pub fn function_name(&self) -> &str {
        &self.name
    }

    /// 获取输入多项式快照 / Get the input polynomial snapshot.
    pub fn input(&self) -> &Linear<V> {
        &self.input
    }

    /// 获取结果列 / Get the result column.
    pub fn result(&self) -> &VariableId {
        &self.result
    }

    /// 获取侧列 / Get the side column.
    pub fn side(&self) -> &VariableId {
        &self.side
    }

    /// 校验一组 Big-M 是否被本结构的输入域证明覆盖。
    ///
    /// 有证明时必须不小于证明所要求的分支 M：更小会裁掉可行解，因此返回错误；更大只是收紧，
    /// 允许通过。没有证明时只能使用策略回退值，此时仅要求两个分支 M 为有限的非负值。
    ///
    /// Validate that a Big-M pair is covered by this structure's input-domain proof.
    ///
    /// With a proof, both branches must be at least the required value: a smaller M would cut off
    /// feasible solutions and is rejected, while a larger M is merely a tightening and passes.
    /// Without a proof only the policy fallback is admissible, so the check merely requires both
    /// branches to be finite and non-negative.
    pub fn validate_big_m(&self, big_m: AbsBranchBigM) -> Result<()> {
        if !big_m.positive_branch.is_finite()
            || !big_m.negative_branch.is_finite()
            || big_m.positive_branch < 0.0
            || big_m.negative_branch < 0.0
        {
            return Err(ModelError::InvalidConstraint(format!(
                "abs `{}` branch big-M must be finite and non-negative, got [{}, {}]",
                self.name, big_m.positive_branch, big_m.negative_branch
            ))
            .into());
        }

        let Some(proof) = self.input_domain else {
            return Ok(());
        };
        // 证明边界已在构造时校验为有限，因此推断必定成功；万一失败则拒绝而不是静默放过。
        // Proof bounds are validated as finite at construction, so inference must succeed; should
        // it fail, reject instead of silently passing.
        let Some(required) = AbsBranchBigM::from_input_bounds(proof.lower(), proof.upper()) else {
            return Err(ModelError::InvalidConstraint(format!(
                "abs `{}` cannot derive the required branch big-M from the input domain proof [{}, {}]",
                self.name,
                proof.lower(),
                proof.upper()
            ))
            .into());
        };
        if big_m.positive_branch + f64::EPSILON < required.positive_branch
            || big_m.negative_branch + f64::EPSILON < required.negative_branch
        {
            return Err(ModelError::InvalidConstraint(format!(
                "abs `{}` branch big-M [{}, {}] is smaller than the value [{}, {}] required by the input domain proof [{}, {}] and would cut off feasible solutions",
                self.name,
                big_m.positive_branch,
                big_m.negative_branch,
                required.positive_branch,
                required.negative_branch,
                proof.lower(),
                proof.upper()
            ))
            .into());
        }
        Ok(())
    }

    /// 用一个新的（只允许更紧的）输入域证明重建结构，并据此重算分支 Big-M。
    ///
    /// 新证明比原证明更宽时返回错误；原证明存在而新证明缺失时同样返回错误，避免自动 M 的域
    /// 依据被静默丢失。原证明缺失时允许补充证明，这属于收紧而非扩大。
    ///
    /// Rebuild the structure with a new input-domain proof that may only be tighter, recomputing
    /// the branch Big-M accordingly.
    ///
    /// A wider proof is rejected, and dropping an existing proof is rejected as well so the domain
    /// basis of an automatic M cannot be lost silently. Supplying a proof where none existed is
    /// allowed because it tightens rather than widens.
    pub fn with_domain(&self, input_domain: Option<InputDomainProof>) -> Result<Self> {
        match (self.input_domain, input_domain) {
            (Some(existing), None) => Err(ModelError::InvalidConstraint(format!(
                "abs `{}`: {DOMAIN_PROOF_LOST}; existing proof is [{}, {}]",
                self.name,
                existing.lower(),
                existing.upper()
            ))
            .into()),
            (Some(existing), Some(candidate)) if !candidate.is_contained_in(&existing) => {
                Err(ModelError::InvalidConstraint(format!(
                    "abs `{}`: {DOMAIN_PROOF_WIDENED}; [{}, {}] is not contained in [{}, {}]",
                    self.name,
                    candidate.lower(),
                    candidate.upper(),
                    existing.lower(),
                    existing.upper()
                ))
                .into())
            }
            (_, Some(candidate)) => {
                let Some(big_m) =
                    AbsBranchBigM::from_input_bounds(candidate.lower(), candidate.upper())
                else {
                    return Err(ModelError::InvalidConstraint(format!(
                        "abs `{}` cannot derive a branch big-M from the new input domain proof [{}, {}]",
                        self.name,
                        candidate.lower(),
                        candidate.upper()
                    ))
                    .into());
                };
                Ok(Self {
                    big_m,
                    input_domain: Some(candidate),
                    ..self.clone()
                })
            }
            (None, None) => Ok(self.clone()),
        }
    }
}

impl<V> DeferredFunctionStructure<V> for AbsStructure<V>
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
{
    fn function_name(&self) -> &str {
        &self.name
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn usage_binding(&self) -> Option<StructureUsageBinding> {
        Some(StructureUsageBinding::new(
            self.source_symbol_id,
            self.result.clone(),
            vec![self.side.clone()],
        ))
    }

    fn fingerprint(&self) -> Option<String> {
        // 指纹覆盖所有影响 ABS 语义的字段：函数名、来源符号、两列、分支 M、输入域证明与输入
        // 多项式。任一变化都必须改变指纹，恢复阶段才能发现"结构已经和原生写入时不一样"。
        // The fingerprint covers every field that affects ABS semantics: function name, source
        // symbol, both columns, branch M, input-domain proof and input polynomial. Any change must
        // change the fingerprint so recovery notices a structure that no longer matches the
        // native write.
        let mut monomials: Vec<String> = self
            .input
            .monomials()
            .iter()
            .map(|monomial| {
                format!(
                    "{}:{}",
                    monomial.var_index(),
                    fingerprint_float(to_f64(monomial.coefficient()).unwrap_or(f64::NAN))
                )
            })
            .collect();
        monomials.sort();
        let input_constant = fingerprint_float(to_f64(self.input.constant_term()).unwrap_or(f64::NAN));
        let domain = match self.input_domain {
            Some(proof) => format!(
                "{}..{}",
                fingerprint_float(proof.lower()),
                fingerprint_float(proof.upper())
            ),
            None => "unbounded".to_string(),
        };
        Some(format!(
            "abs|{}|{}|{}|{}|{}|{}|{}|{}",
            self.name,
            self.source_symbol_id,
            self.result.unique_id(),
            self.side.unique_id(),
            fingerprint_float(self.big_m.positive_branch),
            fingerprint_float(self.big_m.negative_branch),
            domain,
            format!("{input_constant}({})", monomials.join(",")),
        ))
    }

    fn materialize(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        // 物化前重新校验域证明：分支 M 不得小于证明所要求的取值。
        // Re-validate the domain proof before materializing: neither branch M may be smaller than
        // the value the proof requires.
        self.validate_big_m(self.big_m)?;

        let result_id = self.result.unique_id() as usize;
        let side_id = self.side.unique_id() as usize;
        let result_index = symbol_to_index.get(&result_id).copied().ok_or_else(|| {
            ModelError::SymbolNotRegistered(format!("abs result variable id {}", result_id))
        })?;
        let side_index = symbol_to_index.get(&side_id).copied().ok_or_else(|| {
            ModelError::SymbolNotRegistered(format!("abs side variable id {}", side_id))
        })?;
        generate_abs_constraints(
            &self.name,
            &self.input,
            result_index,
            side_index,
            self.big_m,
            self.source.clone(),
        )
    }
}

impl<V> Display for AbsFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "abs({})", self.id.name)
    }
}

impl<V> DynSymbol for AbsFunction<V>
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

impl<V> Symbol for AbsFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for AbsFunction<V>
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

    fn refine_auxiliary_tokens(
        &self,
        auxiliary: &mut [Token<V>],
        tokens: &[Token<V>],
    ) -> Result<()> {
        // 输入域有限时把 `0 <= y <= max(|lower|, |upper|)` 传播到结果变量。
        // Propagate `0 <= y <= max(|lower|, |upper|)` to the result variable when the
        // input domain is finite.
        let Some((lower, upper)) = infer_linear_bounds_from_tokens(&self.input, tokens) else {
            return Ok(());
        };
        let abs_bound = lower.abs().max(upper.abs());
        if !abs_bound.is_finite() {
            return Ok(());
        }

        for token in auxiliary.iter_mut() {
            if token.id() != self.result_var.id() {
                continue;
            }
            let mut result_lower = 0.0_f64;
            let mut result_upper = abs_bound;
            if let Some(existing) = token.variable.lower_bound().and_then(|value| value.to_f64()) {
                result_lower = result_lower.max(existing);
            }
            if let Some(existing) = token.variable.upper_bound().and_then(|value| value.to_f64()) {
                result_upper = result_upper.min(existing);
            }
            if result_lower > result_upper {
                return Err(ModelError::InvalidConstraint(format!(
                    "abs `{}` finite input domain [{}, {}] contradicts the declared result range [{}, {}]",
                    self.id.name, lower, upper, result_lower, result_upper
                ))
                .into());
            }
            let (Some(lower_bound), Some(upper_bound)) =
                (from_f64::<V>(result_lower), from_f64::<V>(result_upper))
            else {
                continue;
            };
            token.set_range(VariableRange::bounded(lower_bound, upper_bound));
        }
        Ok(())
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.build_mechanism_constraints(symbol_to_index, AbsBranchBigM::fallback())
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let big_m = self
            .infer_big_m_from_tokens(tokens)
            .unwrap_or_else(AbsBranchBigM::fallback);
        self.build_mechanism_constraints(symbol_to_index, big_m)
    }

    fn deferred_structure_with_tokens(
        &self,
        tokens: &[Token<V>],
    ) -> Option<Arc<dyn DeferredFunctionStructure<V>>> {
        // 结构快照携带与即时展开完全相同的推断 Big-M、输入多项式与输入域证明，保证两条路径
        // 等价且物化阶段可以重新校验 M 的域依据。
        // The snapshot carries exactly the same inferred Big-M, input polynomial and input-domain
        // proof as eager expansion so both paths stay equivalent and materialization can
        // re-validate the domain basis of the M values.
        let big_m = self
            .infer_big_m_from_tokens(tokens)
            .unwrap_or_else(AbsBranchBigM::fallback);
        let input_domain = infer_linear_bounds_from_tokens(&self.input, tokens)
            .and_then(|(lower, upper)| InputDomainProof::from_optional_bounds(Some(lower), Some(upper)));
        Some(Arc::new(
            AbsStructure::new(
                self.id.name.clone(),
                self.input.clone(),
                self.result_var.id(),
                self.side_var.id(),
                big_m,
                Arc::new(self.clone()),
            )
            .with_input_domain(input_domain),
        ))
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
        format!("abs({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for AbsFunction<V>
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
        tokens.push(Token::from_generic(
            self.side_var.clone(),
            self.side_var.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let value = evaluate_linear(&self.input, token_table, zero_if_none)?;
        from_f64(to_f64(&value)?.abs())
    }
}

impl<V> LinearIntermediateSymbol<V> for AbsFunction<V>
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
    use crate::model::intermediate::{NativeWriteRecord, verify_native_write};
    use crate::token::{MutableTokenList, Token, VecTokenList};
    use crate::variable::{ContinuousVariableItem, VariableRange};

    #[test]
    fn abs_function_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(-3.0);
        tokens.add_token(tx);

        let poly = Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0);
        let abs = AbsFunction::new(100, "abs_x", poly);
        let value = <AbsFunction as FunctionSymbol>::calculate_value(&abs, &tokens, false);
        assert_eq!(value, Some(5.0));
    }

    #[test]
    fn abs_function_supports_f32_values() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::<f32>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(-3.0_f32);
        tokens.add_token(tx);

        let abs: AbsFunction<f32> = AbsFunction::new(
            1001,
            "abs_f32",
            Linear::new(vec![LinearMonomial::new(2.0_f32, 0)], 1.0_f32),
        );
        let value =
            <AbsFunction<f32> as FunctionSymbol<f32>>::calculate_value(&abs, &tokens, false);
        assert_eq!(value, Some(5.0_f32));

        let result_id = abs.result_variable().id().unique_id() as usize;
        let side_id = abs.side_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (side_id, 2usize)]);
        let constraints = abs
            .mechanism_constraints(&symbol_to_index)
            .expect("f32 abs mechanism constraints should be generated");

        assert_eq!(constraints.len(), 4);
    }

    #[test]
    fn abs_function_zero_if_none() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tokens.add_token(tx);

        let poly = Linear::new(vec![LinearMonomial::new(2.0, 0)], -1.0);
        let abs = AbsFunction::new(101, "abs_x", poly);
        let value = <AbsFunction as FunctionSymbol>::calculate_value(&abs, &tokens, true);
        assert_eq!(value, Some(1.0));
    }

    #[test]
    fn abs_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let abs: AbsFunction<f64> = AbsFunction::new(
            102,
            "abs_bound",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
        );

        let result_id = abs.result_variable().id().unique_id() as usize;
        let side_id = abs.side_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (side_id, 2usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(abs.result_variable().clone(), 1),
            Token::from_generic(abs.side_variable().clone(), 2),
        ];

        let constraints = abs
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("abs mechanism constraints should be generated");
        let pos_branch = constraints
            .iter()
            .find(|constraint| constraint.name == "abs_bound_abs_pos_branch")
            .expect("positive branch constraint should exist");
        let side_term = pos_branch
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 2)
            .expect("side variable term should exist");

        // 2x + 1 且 x ∈ [-2, 3] => 取值范围 [-3, 7]，绝对值上界为 7。
        // 2x + 1 with x in [-2, 3] => range [-3, 7], abs bound = 7.
        // 正分支行需覆盖 max(y - x) = -2 * lower = 6；负分支行需覆盖 max(y + x) = 2 * upper = 14。
        // The positive branch row must cover max(y - x) = -2 * lower = 6 while the
        // negative branch row must cover max(y + x) = 2 * upper = 14.
        assert!((pos_branch.inequality.rhs - 6.0).abs() <= 1e-9);
        assert!((*side_term.coefficient() - 6.0).abs() <= 1e-9);

        let neg_branch = constraints
            .iter()
            .find(|constraint| constraint.name == "abs_bound_abs_neg_branch")
            .expect("negative branch constraint should exist");
        let neg_side_term = neg_branch
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 2)
            .expect("side variable term should exist");
        assert!((*neg_side_term.coefficient() + 14.0).abs() <= 1e-9);
    }

    #[test]
    fn abs_function_propagates_finite_input_bounds_to_result_variable() {
        let x: ContinuousVariableItem = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(-3.0, 7.0),
        );
        let abs: AbsFunction<f64> = AbsFunction::new(
            104,
            "abs_bounds",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );

        let input_tokens = vec![Token::from_generic(x, 0)];
        let mut auxiliary_tokens = Vec::new();
        abs.register_auxiliary_tokens(&mut auxiliary_tokens)
            .expect("abs auxiliary tokens should register");
        abs.refine_auxiliary_tokens(&mut auxiliary_tokens, &input_tokens)
            .expect("abs auxiliary bounds should refine");

        let result_token = auxiliary_tokens
            .iter()
            .find(|token| token.id() == abs.result_variable().id())
            .expect("result token should exist");
        assert_eq!(result_token.variable.lower_bound(), Some(0.0));
        assert_eq!(result_token.variable.upper_bound(), Some(7.0));
    }

    #[test]
    fn abs_function_keeps_result_bounds_without_finite_input_domain() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let abs: AbsFunction<f64> = AbsFunction::new(
            105,
            "abs_unbounded",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );

        let input_tokens = vec![Token::from_generic(x, 0)];
        let mut auxiliary_tokens = Vec::new();
        abs.register_auxiliary_tokens(&mut auxiliary_tokens)
            .expect("abs auxiliary tokens should register");
        abs.refine_auxiliary_tokens(&mut auxiliary_tokens, &input_tokens)
            .expect("missing bounds must not fail refinement");

        let result_token = auxiliary_tokens
            .iter()
            .find(|token| token.id() == abs.result_variable().id())
            .expect("result token should exist");
        assert_eq!(result_token.variable.lower_bound(), None);
        assert_eq!(result_token.variable.upper_bound(), None);
    }

    #[test]
    fn abs_branch_big_m_covers_only_true_solutions() {
        // x ∈ [2, 5]：输入恒为正，正分支行不再需要松弛量。
        // x in [2, 5]: the input is always positive, so the positive branch row needs no relaxation.
        let positive = AbsBranchBigM::from_input_bounds(2.0, 5.0)
            .expect("finite domain should produce branch Big-M");
        assert_eq!(positive.positive_branch, 1.0);
        assert!((positive.negative_branch - 10.0).abs() <= 1e-9);

        // x ∈ [-4, -1]：输入恒为负，负分支行不再需要松弛量。
        // x in [-4, -1]: the input is always negative, so the negative branch row needs no relaxation.
        let negative = AbsBranchBigM::from_input_bounds(-4.0, -1.0)
            .expect("finite domain should produce branch Big-M");
        assert!((negative.positive_branch - 8.0).abs() <= 1e-9);
        assert_eq!(negative.negative_branch, 1.0);

        // x ∈ [-3, 7]：两侧都需要覆盖真实解。
        // x in [-3, 7]: both sides must cover the true solution.
        let mixed = AbsBranchBigM::from_input_bounds(-3.0, 7.0)
            .expect("finite domain should produce branch Big-M");
        assert!((mixed.positive_branch - 6.0).abs() <= 1e-9);
        assert!((mixed.negative_branch - 14.0).abs() <= 1e-9);

        assert!(AbsBranchBigM::from_input_bounds(3.0, -3.0).is_none());
        assert!(AbsBranchBigM::from_input_bounds(f64::NEG_INFINITY, 1.0).is_none());
    }

    #[test]
    fn abs_function_handles_signed_zero_and_a_degenerate_domain() {
        // 固定为零的输入：结果域退化为 [0, 0]，两条分支只保留策略最小值。
        // An input fixed at zero: the result domain degenerates to [0, 0] and both branch
        // relaxations keep only the policy minimum.
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::fixed(0.0),
        );
        let abs: AbsFunction<f64> = AbsFunction::new(
            106,
            "abs_degenerate",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );

        let input_tokens = vec![Token::from_generic(x.clone(), 0)];
        let mut auxiliary = Vec::new();
        abs.register_auxiliary_tokens(&mut auxiliary)
            .expect("abs auxiliary tokens should register");
        abs.refine_auxiliary_tokens(&mut auxiliary, &input_tokens)
            .expect("abs auxiliary bounds should refine");
        let result_token = auxiliary
            .iter()
            .find(|token| token.id() == abs.result_variable().id())
            .expect("result token should exist");
        assert_eq!(result_token.variable.lower_bound(), Some(0.0));
        assert_eq!(result_token.variable.upper_bound(), Some(0.0));

        let big_m = AbsBranchBigM::from_input_bounds(0.0, 0.0)
            .expect("degenerate domain should still produce branch Big-M");
        assert_eq!(big_m.positive_branch, 1.0);
        assert_eq!(big_m.negative_branch, 1.0);

        // 正负零都求值为零 / positive and negative zero both evaluate to zero.
        for signed_zero in [0.0_f64, -0.0_f64] {
            let zero_token = Token::from_generic(x.clone(), 0);
            zero_token.set_result(signed_zero);
            let mut tokens = VecTokenList::new();
            tokens.add_token(zero_token);
            let value = <AbsFunction as FunctionSymbol>::calculate_value(&abs, &tokens, false);
            assert_eq!(value, Some(0.0));
        }
    }

    #[test]
    fn abs_structure_materializes_the_same_rows_as_eager_expansion() {
        fn normalize(constraints: &[LinearConstraint<f64>]) -> Vec<String> {
            let mut rows = constraints
                .iter()
                .map(|constraint| {
                    let mut monomials = constraint
                        .inequality
                        .polynomial
                        .monomials()
                        .iter()
                        .map(|monomial| {
                            format!("{}:{:.12}", monomial.var_index(), monomial.coefficient())
                        })
                        .collect::<Vec<_>>();
                    monomials.sort();
                    format!(
                        "{}|{:?}|{:.12}|{:.12}|{}",
                        constraint.name,
                        constraint.inequality.relation,
                        constraint.inequality.rhs,
                        constraint.inequality.polynomial.constant_term(),
                        monomials.join(",")
                    )
                })
                .collect::<Vec<_>>();
            rows.sort();
            rows
        }

        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let abs: AbsFunction<f64> = AbsFunction::new(
            107,
            "abs_shared_generator",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
        );
        let result_id = abs.result_variable().id().unique_id() as usize;
        let side_id = abs.side_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (side_id, 2usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(abs.result_variable().clone(), 1),
            Token::from_generic(abs.side_variable().clone(), 2),
        ];

        let eager = abs
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("eager abs constraints should be generated");
        let structure = abs
            .deferred_structure_with_tokens(&tokens)
            .expect("abs should expose a deferred structure");
        assert_eq!(structure.function_name(), "abs_shared_generator");
        let deferred = structure
            .materialize(&symbol_to_index)
            .expect("abs structure should materialize");

        // EAGER 展开与延迟 fallback 物化必须复用同一公式生成器，逐列一致。
        // Eager expansion and deferred fallback materialization must reuse the same formula
        // generator and stay column-identical.
        assert_eq!(normalize(&eager), normalize(&deferred));
    }

    #[test]
    fn abs_structure_records_the_input_domain_proof_behind_the_inferred_big_m() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(-3.0, 4.0),
        );
        let abs: AbsFunction<f64> = AbsFunction::new(
            108,
            "abs_domain_proof",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(abs.result_variable().clone(), 1),
            Token::from_generic(abs.side_variable().clone(), 2),
        ];

        let structure = abs
            .deferred_structure_with_tokens(&tokens)
            .expect("abs should expose a deferred structure");
        let structure = structure
            .as_any()
            .downcast_ref::<AbsStructure<f64>>()
            .expect("deferred structure should be an abs structure");

        // 有限输入域被记录为证明，且分支 M 与域一一对应：M_pos = 2*3 = 6，M_neg = 2*4 = 8。
        // The finite input domain is recorded as a proof and each branch M follows from it:
        // M_pos = 2*3 = 6 and M_neg = 2*4 = 8.
        let proof = structure.input_domain().expect("domain proof should be recorded");
        assert_eq!((proof.lower(), proof.upper()), (-3.0, 4.0));
        assert_eq!(structure.big_m().positive_branch, 6.0);
        assert_eq!(structure.big_m().negative_branch, 8.0);

        // 小于域要求的 M 会裁掉可行解，必须被拒绝；更大的 M 只是收紧，允许通过。
        // An M smaller than the domain requires would cut off feasible solutions and must be
        // rejected; a larger M is merely a tightening and passes.
        assert!(
            structure
                .validate_big_m(AbsBranchBigM {
                    positive_branch: 5.0,
                    negative_branch: 8.0,
                })
                .is_err()
        );
        assert!(
            structure
                .validate_big_m(AbsBranchBigM {
                    positive_branch: 6.0,
                    negative_branch: 7.5,
                })
                .is_err()
        );
        assert!(
            structure
                .validate_big_m(AbsBranchBigM {
                    positive_branch: 12.0,
                    negative_branch: 12.0,
                })
                .is_ok()
        );
    }

    #[test]
    fn abs_domain_proof_may_be_tightened_but_not_widened_or_dropped() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(-3.0, 4.0),
        );
        let abs: AbsFunction<f64> = AbsFunction::new(
            109,
            "abs_domain_tightening",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(abs.result_variable().clone(), 1),
            Token::from_generic(abs.side_variable().clone(), 2),
        ];
        let structure = abs
            .deferred_structure_with_tokens(&tokens)
            .expect("abs should expose a deferred structure")
            .as_any()
            .downcast_ref::<AbsStructure<f64>>()
            .expect("deferred structure should be an abs structure")
            .clone();

        // 收紧：[-3, 4] -> [-1, 2]，重算得到更小的 M_pos = 2、M_neg = 4。
        // Tightening: [-3, 4] -> [-1, 2] recomputes the smaller pair M_pos = 2, M_neg = 4.
        let tightened = structure
            .with_domain(Some(InputDomainProof::new(-1.0, 2.0).unwrap()))
            .expect("tightening the domain proof should succeed");
        assert_eq!(tightened.big_m().positive_branch, 2.0);
        assert_eq!(tightened.big_m().negative_branch, 4.0);
        assert!(tightened.validate_big_m(tightened.big_m()).is_ok());

        // 扩大：[-3, 4] -> [-5, 4] 会让 M_pos 失效，必须拒绝。
        // Widening: [-3, 4] -> [-5, 4] invalidates M_pos and must be rejected.
        let widened = structure.with_domain(Some(InputDomainProof::new(-5.0, 4.0).unwrap()));
        assert!(widened.is_err());
        assert!(
            widened
                .unwrap_err()
                .to_string()
                .contains(DOMAIN_PROOF_WIDENED)
        );

        // 丢失：已有证明却传 None，必须拒绝。
        // Loss: passing None while a proof exists must be rejected.
        let dropped = structure.with_domain(None);
        assert!(dropped.is_err());
        assert!(dropped.unwrap_err().to_string().contains(DOMAIN_PROOF_LOST));

        // 原本没有证明时补充证明属于收紧，允许。
        // Supplying a proof where none existed is a tightening and is allowed.
        let without_proof = AbsStructure::new(
            "abs_without_proof",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            VariableId::standalone(7_100),
            VariableId::standalone(7_101),
            AbsBranchBigM::fallback(),
            Arc::new(AbsFunction::<f64>::new(
                111,
                "abs_without_proof_source",
                Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            )),
        );
        assert!(without_proof.input_domain().is_none());
        assert!(without_proof.with_domain(None).is_ok());
        let gained = without_proof
            .with_domain(Some(InputDomainProof::new(-2.0, 0.0).unwrap()))
            .expect("adding a domain proof should succeed");
        assert_eq!(gained.big_m().positive_branch, 4.0);
    }

    #[test]
    fn abs_structure_rejects_unusable_domain_proofs_on_materialization() {
        // 手工构造一个与证明不一致的结构：M 小于域要求，物化必须原子失败。
        // Hand-build a structure whose M is smaller than its proof requires; materialization must
        // fail atomically.
        let abs: AbsFunction<f64> = AbsFunction::new(
            110,
            "abs_inconsistent",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );
        let result_id = abs.result_variable().id().unique_id() as usize;
        let side_id = abs.side_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (side_id, 2usize)]);
        let structure = AbsStructure::new(
            "abs_inconsistent",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            abs.result_variable().id(),
            abs.side_variable().id(),
            AbsBranchBigM {
                positive_branch: 1.0,
                negative_branch: 1.0,
            },
            Arc::new(abs),
        )
        .with_input_domain(Some(InputDomainProof::new(-3.0, 4.0).unwrap()));

        assert!(structure.materialize(&symbol_to_index).is_err());
        // 证明本身也拒绝非法区间。
        // The proof itself rejects invalid intervals.
        assert!(InputDomainProof::new(4.0, -3.0).is_err());
        assert!(InputDomainProof::new(f64::NEG_INFINITY, 4.0).is_err());
        assert_eq!(
            InputDomainProof::from_optional_bounds(None, Some(4.0)),
            None
        );
        assert_eq!(
            InputDomainProof::new(-3.0, 4.0)
                .unwrap()
                .intersect(&InputDomainProof::new(0.0, 1.0).unwrap())
                .map(|proof| (proof.lower(), proof.upper())),
            Some((0.0, 1.0))
        );
        assert!(
            InputDomainProof::new(-3.0, -2.0)
                .unwrap()
                .intersect(&InputDomainProof::new(1.0, 2.0).unwrap())
                .is_none()
        );
    }

    #[test]
    fn abs_fingerprint_tracks_its_semantic_content() {
        fn structure(
            name: &str,
            result: VariableId,
            side: VariableId,
            big_m: AbsBranchBigM,
            domain: Option<InputDomainProof>,
        ) -> AbsStructure<f64> {
            AbsStructure::new(
                name,
                Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
                result,
                side,
                big_m,
                Arc::new(AbsFunction::<f64>::new(
                    112,
                    "abs_fingerprint_source",
                    Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
                )),
            )
            .with_input_domain(domain)
        }

        let result = VariableId::standalone(7_200);
        let side = VariableId::standalone(7_201);
        let big_m = AbsBranchBigM {
            positive_branch: 6.0,
            negative_branch: 8.0,
        };
        let domain = InputDomainProof::new(-3.0, 4.0).unwrap();

        let baseline = structure("abs_fp", result.clone(), side.clone(), big_m, Some(domain))
            .fingerprint()
            .expect("abs should provide a fingerprint");

        // 相同内容必须给出相同指纹（跨实例稳定）。
        // Identical content must give an identical fingerprint across instances.
        let identical = structure("abs_fp", result.clone(), side.clone(), big_m, Some(domain))
            .fingerprint()
            .unwrap();
        assert_eq!(baseline, identical);

        // 任一语义字段变化都必须改变指纹。
        // Any semantic field change must change the fingerprint.
        let different_name = structure(
            "abs_fp_other",
            result.clone(),
            side.clone(),
            big_m,
            Some(domain),
        )
        .fingerprint()
        .unwrap();
        assert_ne!(baseline, different_name);

        let different_columns = structure(
            "abs_fp",
            result.clone(),
            VariableId::standalone(7_202),
            big_m,
            Some(domain),
        )
        .fingerprint()
        .unwrap();
        assert_ne!(baseline, different_columns);

        let different_big_m = structure(
            "abs_fp",
            result.clone(),
            side.clone(),
            AbsBranchBigM {
                positive_branch: 6.0,
                negative_branch: 9.0,
            },
            Some(domain),
        )
        .fingerprint()
        .unwrap();
        assert_ne!(baseline, different_big_m);

        // 输入域证明被收紧或丢失同样改变指纹。
        // Tightening or dropping the input-domain proof changes the fingerprint too.
        let tightened = structure(
            "abs_fp",
            result.clone(),
            side.clone(),
            big_m,
            Some(InputDomainProof::new(-1.0, 2.0).unwrap()),
        )
        .fingerprint()
        .unwrap();
        assert_ne!(baseline, tightened);

        let without_domain = structure("abs_fp", result, side, big_m, None)
            .fingerprint()
            .unwrap();
        assert_ne!(baseline, without_domain);

        // 恢复校验：指纹一致才通过；任一侧缺失都视为不可校验。
        // Recovery validation: only matching fingerprints pass, and a missing side never does.
        let current = structure("abs_fp", VariableId::standalone(7_200), VariableId::standalone(7_201), big_m, Some(domain));
        assert!(verify_native_write(Some(&baseline), &current));
        assert!(!verify_native_write(Some(&tightened), &current));
        assert!(!verify_native_write(None, &current));

        // 记录级入口：恢复阶段用持久化记录直接校验当前结构。
        // Record-level entry: recovery validates the current structure straight from the persisted
        // record.
        let matching = NativeWriteRecord::new("gurobi_abs", "functions-abs-1")
            .with_fingerprint(Some(baseline.clone()));
        assert!(matching.matches_structure(&current));
        let stale = NativeWriteRecord::new("gurobi_abs", "functions-abs-1")
            .with_fingerprint(Some(tightened.clone()));
        assert!(!stale.matches_structure(&current));
        let without_fingerprint = NativeWriteRecord::new("gurobi_abs", "functions-abs-1");
        assert!(!without_fingerprint.matches_structure(&current));
    }

    #[test]
    fn abs_function_falls_back_to_default_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let abs: AbsFunction<f64> = AbsFunction::new(
            103,
            "abs_default_m",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );

        let result_id = abs.result_variable().id().unique_id() as usize;
        let side_id = abs.side_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (side_id, 2usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(abs.result_variable().clone(), 1),
            Token::from_generic(abs.side_variable().clone(), 2),
        ];

        let constraints = abs
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("abs mechanism constraints should be generated");
        let pos_branch = constraints
            .iter()
            .find(|constraint| constraint.name == "abs_default_m_abs_pos_branch")
            .expect("positive branch constraint should exist");
        let side_term = pos_branch
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 2)
            .expect("side variable term should exist");

        assert!((pos_branch.inequality.rhs - DEFAULT_BIG_M).abs() <= 1e-9);
        assert!((*side_term.coefficient() - DEFAULT_BIG_M).abs() <= 1e-9);
    }
}
