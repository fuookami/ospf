//! If-in 函数符号 / If-in function symbol

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    auto_intermediate_symbol_name, next_auto_intermediate_symbol_id,
};
use super::ConditionalIndicatorFunction;
use super::big_m::infer_linear_bounds_from_tokens;
use super::conditional::{IfInRangeFunction, TruthValue};
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{BinaryVariableItem, VariableId, VariableRange, new_group_id};
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
    V: FromPrimitive + ToPrimitive,
{
    if !value.is_finite() {
        return Err(ModelError::InvalidConstraint(format!(
            "`{context}` value must be finite, got {value}"
        ))
        .into());
    }
    let converted = from_f64(value).ok_or_else(|| {
        ModelError::InvalidConstraint(format!(
            "failed to convert `{}` value {} from f64 into model value type",
            context, value
        ))
    })?;
    let roundtrip = to_f64(&converted).ok_or_else(|| {
        ModelError::InvalidConstraint(format!(
            "failed to inspect converted `{context}` value {value}"
        ))
    })?;
    if !roundtrip.is_finite() {
        return Err(ModelError::InvalidConstraint(format!(
            "converted `{context}` value must be finite, got {roundtrip}"
        ))
        .into());
    }
    Ok(converted)
}

const MIN_BIG_M: f64 = 1.0;

/// IF-IN 即时展开的 band 容差 / Band tolerance of the IF-IN eager expansion
///
/// 候选值的指示列取真（`b_i = 1`）时，即时展开的 `band_ub` / `band_lb` 两条核心行把平移量
/// `s_i = input - values[i]` 夹在 `[-STEP_EPSILON, +STEP_EPSILON]` 内。原生 writer 必须复用本常量，
/// 绝不能另取一个容差：两条路径的可行域差异只有 1e-8 量级。
///
/// When a candidate value's indicator is true (`b_i = 1`), the eager `band_ub` / `band_lb` core rows
/// clamp the shift `s_i = input - values[i]` into `[-STEP_EPSILON, +STEP_EPSILON]`. A native writer
/// must reuse this very constant and must never pick another tolerance: the two paths' feasible sets
/// differ only at the 1e-8 scale.
pub const STEP_EPSILON: f64 = 1e-8;

/// IF-IN 即时展开的严格边界 / Strict boundary of the IF-IN eager expansion
///
/// `s_i` 落在 band 之外（即候选值指示列为假）时，即时展开要求它离候选值至少这么远
/// （`out_ub` / `out_lb` 两条核心行给出 `s_i <= -STRICT_BOUNDARY` 或 `s_i >= STRICT_BOUNDARY`）。
/// 它等于两倍 band 容差，因此 band 与「不在集合内」之间存在一段**刻意留出的间隙**；原生 writer
/// 必须复用同一常量，否则间隙宽度会变。
///
/// When `s_i` lies outside the band (that is, the candidate value's indicator is false), eager
/// expansion requires it to stay at least this far from the candidate value (the `out_ub` / `out_lb`
/// core rows give `s_i <= -STRICT_BOUNDARY` or `s_i >= STRICT_BOUNDARY`). It equals twice the band
/// tolerance, so a **deliberate gap** sits between the band and "not in the set"; a native writer must
/// reuse the same constant or the gap width changes.
pub const STRICT_BOUNDARY: f64 = STEP_EPSILON + STEP_EPSILON;

/// IF-IN 每个候选值在平移量 `s_i = input - values[i]` 上的核心关系与 Big-M 松弛量
/// Core relations and Big-M relaxations of one IF-IN candidate value on the shift
/// `s_i = input - values[i]`
///
/// 「核心关系」是即时展开里**不含 Big-M 项**的那些行（对每个 `(b_i, side_i)` 取值组合而言）；
/// 「松弛量」是同一批行在其它组合下只剩 Big-M 的形状：`s_i >= relaxed_lower_rhs - M` 或
/// `s_i <= M - relaxed_upper_rhs`（band 行的松弛还要用到 `band_tolerance`）。原生 writer 只写核心
/// 关系，因此必须证明这些松弛行在输入的实际盒上成立，证明所需的 ε 全部取自本表。
///
/// The "core relations" are the eager rows that carry **no Big-M term** (for each `(b_i, side_i)`
/// assignment); the "relaxations" are what the same rows reduce to for the other assignments, namely
/// `s_i >= relaxed_lower_rhs - M` or `s_i <= M - relaxed_upper_rhs` (band relaxations additionally use
/// `band_tolerance`). A native writer writes only the core relations, so it must prove those relaxed
/// rows hold on the input's actual box, and every ε the proof needs comes from this table.
#[derive(Debug, Clone, PartialEq)]
pub struct IfInValueCoreRelations {
    /// 候选值指示列取真时的核心关系：`s_i <= tol` 与 `s_i >= -tol`
    /// Core relations for `indicator = 1`: `s_i <= tol` and `s_i >= -tol`
    pub when_indicator_true: Vec<(ConstraintRelation, f64)>,
    /// side 列取真（且指示列为假）时的核心关系：`s_i >= strict_boundary`
    /// Core relation for `side = 1` (with the indicator false): `s_i >= strict_boundary`
    pub when_side_true: (ConstraintRelation, f64),
    /// side 列取假（且指示列为假）时的核心关系：`s_i <= -strict_boundary`
    /// Core relation for `side = 0` (with the indicator false): `s_i <= -strict_boundary`
    pub when_side_false: (ConstraintRelation, f64),
    /// 需要证明的下侧松弛 `M + s_i_min >= relaxed_lower_rhs`
    /// Lower relaxation to prove: `M + s_i_min >= relaxed_lower_rhs`
    pub relaxed_lower_rhs: f64,
    /// 需要证明的上侧松弛 `M - s_i_max >= relaxed_upper_rhs`
    /// Upper relaxation to prove: `M - s_i_max >= relaxed_upper_rhs`
    pub relaxed_upper_rhs: f64,
    /// band 行使用的容差（`s_i` 取真时的核心关系即由它给出）
    /// Tolerance the band rows use (it also gives the core relations for `indicator = 1`)
    pub band_tolerance: f64,
    /// 「不在集合内」一侧与候选值之间的最小距离 / Minimum distance from a candidate when outside the set
    pub strict_boundary: f64,
}

/// 返回 IF-IN 即时展开的核心关系与 Big-M 松弛量（所有候选值共用同一张表）
/// Core relations and Big-M relaxations of the IF-IN eager expansion (one table for every candidate)
///
/// 逐条对应 `build_mechanism_constraints` 里每个候选值的四行：`band_ub` / `band_lb` 在 `b_i = 1`
/// 时给出 band 核心关系、在 `b_i = 0` 时只剩 `s_i <= tol + M` 与 `s_i >= -tol - M`；`out_lb` 只在
/// `(b_i, side_i) = (0, 1)` 时给出 `s_i >= STRICT_BOUNDARY`，`out_ub` 只在 `(b_i, side_i) = (0, 0)`
/// 时给出 `s_i <= -STRICT_BOUNDARY`，其余组合都只剩 Big-M 松弛。`STRICT_BOUNDARY` 同时是两侧松弛
/// 的右端（`s_i >= sb - M` 与 `s_i <= M - sb`）。
///
/// Each entry mirrors one of the four rows `build_mechanism_constraints` emits per candidate:
/// `band_ub` / `band_lb` give the band core relations at `b_i = 1` and reduce to `s_i <= tol + M` and
/// `s_i >= -tol - M` at `b_i = 0`; `out_lb` gives `s_i >= STRICT_BOUNDARY` only at
/// `(b_i, side_i) = (0, 1)` while `out_ub` gives `s_i <= -STRICT_BOUNDARY` only at
/// `(b_i, side_i) = (0, 0)`, every other assignment leaving a Big-M relaxation. `STRICT_BOUNDARY` is
/// also the right-hand side of both relaxations (`s_i >= sb - M` and `s_i <= M - sb`).
pub fn if_in_value_core_relations() -> IfInValueCoreRelations {
    IfInValueCoreRelations {
        when_indicator_true: vec![
            (ConstraintRelation::LessEqual, STEP_EPSILON),
            (ConstraintRelation::GreaterEqual, -STEP_EPSILON),
        ],
        when_side_true: (ConstraintRelation::GreaterEqual, STRICT_BOUNDARY),
        when_side_false: (ConstraintRelation::LessEqual, -STRICT_BOUNDARY),
        relaxed_lower_rhs: STRICT_BOUNDARY,
        relaxed_upper_rhs: STRICT_BOUNDARY,
        band_tolerance: STEP_EPSILON,
        strict_boundary: STRICT_BOUNDARY,
    }
}

fn next_up_finite(value: f64) -> Option<f64> {
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    let next = if value == 0.0 {
        f64::from_bits(1)
    } else {
        f64::from_bits(value.to_bits() + 1)
    };
    next.is_finite().then_some(next)
}

fn expand_big_m_with_strict_boundary(max_difference: f64) -> Result<f64> {
    if !max_difference.is_finite() || max_difference < 0.0 {
        return Err(ModelError::InvalidConstraint(
            "if_in inferred maximum difference is not finite and non-negative".to_string(),
        )
        .into());
    }

    // 先尝试直接加严格余量；若被舍入吞掉，则按 ULP 向上扩张并重新验证实际差值。
    // Try the direct margin first; if rounding removes it, expand by ULPs and verify the actual gap.
    let mut expanded = max_difference + STRICT_BOUNDARY;
    if !expanded.is_finite() || expanded - max_difference < STRICT_BOUNDARY {
        expanded = next_up_finite(max_difference).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "if_in inferred Big-M cannot exceed maximum difference {max_difference} while preserving strict boundary"
            ))
        })?;
        while expanded - max_difference < STRICT_BOUNDARY {
            expanded = next_up_finite(expanded).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "if_in inferred Big-M cannot represent strict boundary above maximum difference {max_difference}"
                ))
            })?;
        }
    }

    let expanded = expanded.max(MIN_BIG_M);
    if expanded - max_difference < STRICT_BOUNDARY {
        return Err(ModelError::InvalidConstraint(format!(
            "if_in inferred Big-M {expanded} does not preserve strict boundary {STRICT_BOUNDARY} above maximum difference {max_difference}"
        ))
        .into());
    }
    Ok(expanded)
}

impl<V> IfInRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    /// 校验闭区间两侧关系和有限边界 / Validate both relations and finite bounds of a closed interval
    ///
    /// `IfInRangeFunction` 只接受 `x >= lower` 与 `upper >= x` 两个闭区间关系。
    /// `IfInRangeFunction` accepts only `x >= lower` and `upper >= x` for a closed interval.
    pub fn validate(&self) -> Result<()> {
        super::conditional::validate_if_in_range(&self.lower, &self.upper)
    }

    /// 使用校验后的描述器创建闭区间条件 / Create a closed-interval condition after validation
    pub fn try_new(
        lower: super::conditional::ConditionalIfFunction<V>,
        upper: super::conditional::ConditionalIfFunction<V>,
    ) -> Result<Self> {
        Self::new(lower, upper)
    }

    /// 创建可注册的闭区间函数 / Create a registerable closed-interval function
    ///
    /// 纯描述器保留在 `IfInRangeFunction` 中；需要模型变量、辅助令牌和约束时，
    /// 使用该入口生成 `RegisterableIfInRangeFunction`。
    /// The descriptor remains pure; use this entry point when model variables,
    /// helper tokens, and constraints are required.
    pub fn registerable(
        self,
        id: u64,
        name: impl AsRef<str>,
    ) -> Result<RegisterableIfInRangeFunction<V>> {
        RegisterableIfInRangeFunction::new(id, name, self)
    }

    /// 使用自动 ID 和调用方名称创建可注册函数 / Create a registerable function with an auto ID
    pub fn named_registerable(
        self,
        name: impl AsRef<str>,
    ) -> Result<RegisterableIfInRangeFunction<V>> {
        RegisterableIfInRangeFunction::named(name, self)
    }

    /// 使用自动 ID 和自动名称创建可注册函数 / Create a registerable function with an auto name
    pub fn auto_registerable(self) -> Result<RegisterableIfInRangeFunction<V>> {
        RegisterableIfInRangeFunction::auto(self)
    }

    /// 将两侧三值结果合并为闭区间结果 / Combine both three-valued sides into the interval result
    pub fn classify_checked(
        &self,
        lower_difference: &V,
        upper_difference: &V,
    ) -> Result<TruthValue> {
        self.validate()?;
        self.classify(lower_difference, upper_difference)
    }

    /// 求值闭区间条件，Undefined 映射为 None / Evaluate the interval, mapping Undefined to None
    pub fn evaluate(&self, lower_difference: &V, upper_difference: &V) -> Result<Option<V>> {
        match self.classify_checked(lower_difference, upper_difference)? {
            TruthValue::True => V::from_f64(1.0)
                .ok_or_else(|| {
                    ModelError::InvalidConstraint(
                        "failed to convert if_in_range true value".to_string(),
                    )
                    .into()
                })
                .map(Some),
            TruthValue::False => V::from_f64(0.0)
                .ok_or_else(|| {
                    ModelError::InvalidConstraint(
                        "failed to convert if_in_range false value".to_string(),
                    )
                    .into()
                })
                .map(Some),
            TruthValue::Undefined => Ok(None),
        }
    }
}

/// 可注册的闭区间条件函数 / Registerable closed-interval condition function
///
/// 该包装器为闭区间的两侧关系分别创建范围驱动指示器，再以三个线性 AND
/// 约束生成最终结果。`IfInRangeFunction` 本身仍是纯语义描述器，以保持旧调用方兼容。
/// This wrapper creates a range-driven indicator for each side of the interval and
/// combines them with three linear AND constraints. The descriptor itself remains
/// pure to preserve compatibility for existing callers.
#[derive(Debug, Clone)]
pub struct RegisterableIfInRangeFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 中间符号标识符 / Intermediate symbol identifier
    id: IntermediateSymbolId,
    /// 闭区间描述器 / Closed-interval descriptor
    range: IfInRangeFunction<V>,
    /// 下侧关系指示器 / Lower-side relation indicator
    lower_indicator: ConditionalIndicatorFunction<V>,
    /// 上侧关系指示器 / Upper-side relation indicator
    upper_indicator: ConditionalIndicatorFunction<V>,
    /// AND 结果变量 / AND result variable
    result_var: BinaryVariableItem,
    /// 声明的依赖 ID 列表 / Declared dependency IDs
    declared_dependency_ids: Vec<u64>,
}

impl<V> RegisterableIfInRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    /// 创建可注册闭区间函数 / Create a registerable closed-interval function
    pub fn new(id: u64, name: impl AsRef<str>, range: IfInRangeFunction<V>) -> Result<Self> {
        range.validate()?;
        let name = name.as_ref();
        let lower_indicator = ConditionalIndicatorFunction::new(
            auxiliary_symbol_id(id, 1),
            &format!("{name}_lower"),
            range.lower.condition.clone(),
            range.lower.relation,
            range.lower.strict_boundary.clone(),
            range.lower.bounds.clone(),
        )?;
        let upper_indicator = ConditionalIndicatorFunction::new(
            auxiliary_symbol_id(id, 2),
            &format!("{name}_upper"),
            range.upper.condition.clone(),
            range.upper.relation,
            range.upper.strict_boundary.clone(),
            range.upper.bounds.clone(),
        )?;
        let result_var = BinaryVariableItem::create(
            VariableId::new(new_group_id(), 0),
            &format!("{name}_if_in_range"),
        );
        Ok(Self {
            id: IntermediateSymbolId::new(id, name),
            range,
            lower_indicator,
            upper_indicator,
            result_var,
            declared_dependency_ids: Vec::new(),
        })
    }

    /// 使用自动 ID 和调用方名称创建函数 / Create with an auto ID and caller name
    pub fn named(name: impl AsRef<str>, range: IfInRangeFunction<V>) -> Result<Self> {
        Self::new(next_auto_intermediate_symbol_id(), name, range)
    }

    /// 使用自动 ID 和自动名称创建函数 / Create with an auto ID and generated name
    pub fn auto(range: IfInRangeFunction<V>) -> Result<Self> {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("if_in_range", id);
        Self::new(id, name, range)
    }

    /// 从两侧条件创建函数 / Create from the two side conditions
    pub fn from_parts(
        id: u64,
        name: impl AsRef<str>,
        lower: super::conditional::ConditionalIfFunction<V>,
        upper: super::conditional::ConditionalIfFunction<V>,
    ) -> Result<Self> {
        Self::new(id, name, IfInRangeFunction::try_new(lower, upper)?)
    }

    /// 设置声明的依赖 ID 列表 / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取闭区间描述器 / Get the interval descriptor
    pub fn condition_descriptor(&self) -> &IfInRangeFunction<V> {
        &self.range
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &BinaryVariableItem {
        &self.result_var
    }

    /// 获取下侧指示器 / Get the lower-side indicator
    pub fn lower_indicator(&self) -> &ConditionalIndicatorFunction<V> {
        &self.lower_indicator
    }

    /// 获取上侧指示器 / Get the upper-side indicator
    pub fn upper_indicator(&self) -> &ConditionalIndicatorFunction<V> {
        &self.upper_indicator
    }

    /// 获取两个侧面和结果的辅助变量 / Get side and result helper variables
    pub fn helper_variables(&self) -> Vec<&BinaryVariableItem> {
        vec![
            self.lower_indicator.result_variable(),
            self.upper_indicator.result_variable(),
            &self.result_var,
        ]
    }

    /// 校验闭区间函数 / Validate the closed-interval function
    pub fn validate(&self) -> Result<()> {
        self.range.validate()
    }

    /// 对两侧条件进行三值分类 / Classify both side conditions
    pub fn classify(&self, lower_difference: &V, upper_difference: &V) -> Result<TruthValue> {
        self.validate()?;
        self.range.classify(lower_difference, upper_difference)
    }

    /// 求值闭区间函数 / Evaluate the closed-interval function
    pub fn evaluate(&self, lower_difference: &V, upper_difference: &V) -> Result<Option<V>> {
        self.classify(lower_difference, upper_difference)?;
        self.range.evaluate(lower_difference, upper_difference)
    }

    /// 从令牌求值两侧条件 / Evaluate both side conditions from tokens
    pub fn evaluate_with_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Result<Option<V>>
    where
        V: Add<Output = V> + Mul<Output = V> + Zero,
    {
        let lower = match evaluate_linear(&self.range.lower.condition, token_table, zero_if_none) {
            Some(value) => value,
            None => return Ok(None),
        };
        let upper = match evaluate_linear(&self.range.upper.condition, token_table, zero_if_none) {
            Some(value) => value,
            None => return Ok(None),
        };
        self.evaluate(&lower, &upper)
    }

    fn variable_index(
        symbol_to_index: &HashMap<usize, usize>,
        variable: &BinaryVariableItem,
        role: &str,
    ) -> Result<usize> {
        symbol_to_index
            .get(&(variable.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "if_in_range {role} variable id {}",
                    variable.id().unique_id()
                ))
                .into()
            })
    }

    fn append_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()>
    where
        V: Add<Output = V> + Mul<Output = V> + Zero,
        f64: IntoValue<V>,
    {
        let mut staged = Vec::new();
        self.lower_indicator.register_tokens(&mut staged)?;
        self.upper_indicator.register_tokens(&mut staged)?;
        staged.push(Token::from_generic(self.result_var.clone(), usize::MAX));
        for token in &mut staged {
            token.solver_index = usize::MAX;
        }

        let mut ids = HashSet::with_capacity(staged.len());
        for token in &staged {
            if !ids.insert(token.id()) || tokens.iter().any(|existing| existing.id() == token.id())
            {
                return Err(ModelError::ConstraintConflict(format!(
                    "if_in_range `{}` helper token already exists",
                    self.id.name
                ))
                .into());
            }
        }
        tokens.extend(staged);
        Ok(())
    }

    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>>
    where
        V: Add<Output = V> + Mul<Output = V> + Zero,
        f64: IntoValue<V>,
    {
        self.validate()?;
        let mut constraints = self
            .lower_indicator
            .mechanism_constraints(symbol_to_index)?;
        constraints.extend(
            self.upper_indicator
                .mechanism_constraints(symbol_to_index)?,
        );

        let lower_index = Self::variable_index(
            symbol_to_index,
            self.lower_indicator.result_variable(),
            "lower indicator",
        )?;
        let upper_index = Self::variable_index(
            symbol_to_index,
            self.upper_indicator.result_variable(),
            "upper indicator",
        )?;
        let result_index = Self::variable_index(symbol_to_index, &self.result_var, "result")?;
        let source: Arc<dyn IntermediateSymbol<V>> = Arc::new(self.clone());
        let one = convert_f64_to_v::<V>(1.0, "if_in_range coefficient")?;
        let neg_one = convert_f64_to_v::<V>(-1.0, "if_in_range coefficient")?;
        let zero = convert_f64_to_v::<V>(0.0, "if_in_range rhs")?;
        let neg_one_rhs = convert_f64_to_v::<V>(-1.0, "if_in_range rhs")?;

        let link = |name: String,
                    monomials: Vec<LinearMonomial<V>>,
                    relation: ConstraintRelation,
                    rhs: V| {
            LinearConstraint::from_symbol(
                LinearInequality::new(Linear::new(monomials, zero.clone()), relation, rhs),
                &name,
                source.clone(),
            )
        };
        constraints.push(link(
            format!("{}_and_lower_ub", self.id.name),
            vec![
                LinearMonomial::new(one.clone(), result_index),
                LinearMonomial::new(neg_one.clone(), lower_index),
            ],
            ConstraintRelation::LessEqual,
            zero.clone(),
        ));
        constraints.push(link(
            format!("{}_and_upper_ub", self.id.name),
            vec![
                LinearMonomial::new(one.clone(), result_index),
                LinearMonomial::new(neg_one.clone(), upper_index),
            ],
            ConstraintRelation::LessEqual,
            zero.clone(),
        ));
        constraints.push(link(
            format!("{}_and_lb", self.id.name),
            vec![
                LinearMonomial::new(one, result_index),
                LinearMonomial::new(neg_one.clone(), lower_index),
                LinearMonomial::new(neg_one, upper_index),
            ],
            ConstraintRelation::GreaterEqual,
            neg_one_rhs,
        ));
        Ok(constraints)
    }
}

fn auxiliary_symbol_id(base: u64, salt: u64) -> u64 {
    base.wrapping_mul(0x9e37_79b9_7f4a_7c15)
        .wrapping_add(salt.wrapping_mul(0x517c_c1b7_2722_0a95))
}

impl<V> Display for RegisterableIfInRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "if_in_range({})", self.id.name)
    }
}

impl<V> DynSymbol for RegisterableIfInRangeFunction<V>
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

impl<V> Symbol for RegisterableIfInRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for RegisterableIfInRangeFunction<V>
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
        self.append_tokens(tokens)
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.build_mechanism_constraints(symbol_to_index)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        _tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.build_mechanism_constraints(symbol_to_index)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_var.index()).cloned()
    }

    fn range(&self) -> Option<VariableRange<V>> {
        Some(VariableRange::bounded(
            convert_f64_to_v::<V>(0.0, "if_in_range lower bound")
                .expect("zero is representable for a registered value type"),
            convert_f64_to_v::<V>(1.0, "if_in_range upper bound")
                .expect("one is representable for a registered value type"),
        ))
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("if_in_range({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for RegisterableIfInRangeFunction<V>
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
        self.append_tokens(tokens)
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        self.evaluate_with_tokens(token_table, zero_if_none)
            .ok()
            .flatten()
    }
}

impl<V> LinearIntermediateSymbol<V> for RegisterableIfInRangeFunction<V>
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

/// 检查输入值是否属于离散值集合。
/// Checks whether an input value belongs to a discrete set of values.
///
/// 数学形式 / Mathematical Form:
/// - `result = 1` if input in `{values[0], values[1], ...}`, otherwise `0`
#[derive(Debug, Clone)]
pub struct IfInFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 中间符号标识符 / Intermediate symbol identifier
    id: IntermediateSymbolId,
    /// 输入线性多项式 / Input linear polynomial
    input: Linear<V>,
    /// 结果二值变量 / Result binary variable
    result_var: BinaryVariableItem,
    /// 离散值集合 / Set of discrete values
    values: Vec<V>,
    /// 大 M 参数，用于机制约束松弛 / Big-M parameter for mechanism constraint relaxation
    big_m: V,
    /// 辅助变量组标识符 / Auxiliary variable group identifier
    aux_group_id: usize,
    /// 声明的依赖符号 ID 列表 / Declared dependency symbol IDs
    declared_dependency_ids: Vec<u64>,
}

impl<V> IfInFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建新的 if-in 函数 / Create new if-in function
    pub fn new(id: u64, name: &str, input: Linear<V>, values: Vec<V>, big_m: V) -> Self {
        let aux_group_id = new_group_id();
        let result_var = BinaryVariableItem::create(VariableId::new(aux_group_id, 0), name);

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            result_var,
            values,
            big_m,
            aux_group_id,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 使用自动 ID 与调用方提供的名称创建 if-in 函数。
    /// Create an if-in function with an auto id and caller-provided name.
    pub fn named(name: impl AsRef<str>, input: Linear<V>, values: Vec<V>, big_m: V) -> Self {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            input,
            values,
            big_m,
        )
    }

    /// 使用自动 ID 与自动名称创建 if-in 函数。
    /// Create an if-in function with an auto id and auto-generated name.
    pub fn auto(input: Linear<V>, values: Vec<V>, big_m: V) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("if_in", id);
        Self::new(id, &name, input, values, big_m)
    }

    /// 设置声明的依赖符号 ID 列表，返回修改后的自身。
    /// Set the declared dependency symbol IDs, returning the modified self.
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取结果二值变量的引用。
    /// Get a reference to the result binary variable.
    pub fn result_variable(&self) -> &BinaryVariableItem {
        &self.result_var
    }

    /// 获取输入线性多项式的引用。
    /// Get a reference to the input linear polynomial.
    pub fn input_polynomial(&self) -> &Linear<V> {
        &self.input
    }

    /// 获取离散值集合的切片。
    /// Get a slice of the discrete value set.
    pub fn values(&self) -> &[V] {
        &self.values
    }

    /// 获取大 M 参数的引用。
    /// Get a reference to the big-M parameter.
    pub fn big_m(&self) -> &V {
        &self.big_m
    }

    fn value_indicator_variable(&self, index: usize) -> BinaryVariableItem {
        BinaryVariableItem::create(
            VariableId::new(self.aux_group_id, index + 1),
            &format!("{}_ifin_val{}", self.id.name, index),
        )
    }

    fn value_side_variable(&self, count: usize, index: usize) -> BinaryVariableItem {
        BinaryVariableItem::create(
            VariableId::new(self.aux_group_id, count + index + 1),
            &format!("{}_ifin_side{}", self.id.name, index),
        )
    }
}

impl<V> IfInFunction<V>
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
    fn validate_set_values(&self) -> Result<()> {
        for (index, value) in self.values.iter().enumerate() {
            let value_f = to_f64(value).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "if_in `{}` value at index {} cannot be converted to f64",
                    self.id.name, index
                ))
            })?;
            if !value_f.is_finite() {
                return Err(ModelError::InvalidConstraint(format!(
                    "if_in `{}` contains a non-finite set value at index {}",
                    self.id.name, index
                ))
                .into());
            }
        }
        Ok(())
    }

    fn configured_big_m(&self) -> Result<f64> {
        let big_m = to_f64(&self.big_m).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "if_in `{}` big-M cannot be converted to f64",
                self.id.name
            ))
        })?;
        if !big_m.is_finite() || big_m <= 0.0 {
            return Err(ModelError::InvalidConstraint(format!(
                "if_in `{}` requires positive finite big-M for mechanism constraint injection",
                self.id.name
            ))
            .into());
        }
        Ok(big_m)
    }

    fn infer_big_m_from_tokens(&self, tokens: &[Token<V>]) -> Result<Option<f64>> {
        self.validate_set_values()?;
        let (input_lower, input_upper) = match infer_linear_bounds_from_tokens(&self.input, tokens)
        {
            Some(bounds) => bounds,
            None => return Ok(None),
        };
        let mut max_difference: f64 = 0.0;
        for value in &self.values {
            let value_f = match to_f64(value) {
                Some(value) => value,
                None => return Ok(None),
            };
            if !value_f.is_finite() {
                return Err(ModelError::InvalidConstraint(format!(
                    "if_in `{}` contains a non-finite set value",
                    self.id.name
                ))
                .into());
            }
            let lower_diff = (input_lower - value_f).abs();
            let upper_diff = (input_upper - value_f).abs();
            if !lower_diff.is_finite() || !upper_diff.is_finite() {
                return Err(ModelError::InvalidConstraint(format!(
                    "if_in `{}` inferred difference is not finite",
                    self.id.name
                ))
                .into());
            }
            max_difference = max_difference.max(lower_diff.max(upper_diff));
        }
        Ok(Some(expand_big_m_with_strict_boundary(max_difference)?))
    }

    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.validate_set_values()?;
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "if_in result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        if self.values.is_empty() {
            return Ok(vec![LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "if_in result coefficient")?,
                            result_index,
                        )],
                        convert_f64_to_v::<V>(0.0, "if_in constant")?,
                    ),
                    ConstraintRelation::Equal,
                    convert_f64_to_v::<V>(0.0, "if_in rhs")?,
                ),
                &format!("{}_empty", self.id.name),
                Arc::new(self.clone()),
            )]);
        }

        if !big_m.is_finite() || big_m < 0.0 {
            return Err(ModelError::InvalidConstraint(format!(
                "if_in `{}` requires finite non-negative big-M",
                self.id.name
            ))
            .into());
        }
        convert_f64_to_v::<V>(big_m, "if_in Big-M")?;

        let tolerance = STEP_EPSILON;
        let strict_boundary = STRICT_BOUNDARY;
        let source = Arc::new(self.clone());

        let mut base_monomials = Vec::with_capacity(self.input.monomials().len());
        for monomial in self.input.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "if_in `{}` input coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            base_monomials.push((coefficient, monomial.var_index()));
        }
        let input_constant = to_f64(self.input.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "if_in `{}` input constant cannot be converted to f64",
                self.id.name
            ))
        })?;

        let value_count = self.values.len();
        let mut value_indices = Vec::with_capacity(value_count);
        let mut side_indices = Vec::with_capacity(value_count);

        for i in 0..value_count {
            let indicator_var = self.value_indicator_variable(i);
            let side_var = self.value_side_variable(value_count, i);

            let indicator_index = symbol_to_index
                .get(&(indicator_var.id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "if_in value indicator variable id {}",
                        indicator_var.id().unique_id()
                    ))
                })?;
            let side_index = symbol_to_index
                .get(&(side_var.id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "if_in side variable id {}",
                        side_var.id().unique_id()
                    ))
                })?;
            value_indices.push(indicator_index);
            side_indices.push(side_index);
        }

        let mut constraints = Vec::with_capacity(value_count * 6 + 1);

        for (i, value) in self.values.iter().enumerate() {
            let indicator_index = value_indices[i];
            let side_index = side_indices[i];
            let value_f = to_f64(value).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "if_in `{}` value at index {} cannot be converted to f64",
                    self.id.name, i
                ))
            })?;
            let shifted_constant = input_constant - value_f;

            let build_value_constraint = |name_suffix: &str,
                                          relation: ConstraintRelation,
                                          rhs: f64,
                                          indicator_coeff: f64,
                                          side_coeff: f64|
             -> Result<LinearConstraint<V>> {
                let mut monomials = Vec::with_capacity(base_monomials.len() + 2);
                for (coefficient, var_index) in &base_monomials {
                    monomials.push(LinearMonomial::new(
                        convert_f64_to_v::<V>(*coefficient, "if_in input coefficient")?,
                        *var_index,
                    ));
                }
                monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(indicator_coeff, "if_in indicator coefficient")?,
                    indicator_index,
                ));
                monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(side_coeff, "if_in side coefficient")?,
                    side_index,
                ));

                Ok(LinearConstraint::from_symbol(
                    LinearInequality::new(
                        Linear::new(
                            monomials,
                            convert_f64_to_v::<V>(shifted_constant, "if_in constant")?,
                        ),
                        relation,
                        convert_f64_to_v::<V>(rhs, "if_in rhs")?,
                    ),
                    &format!("{}_pt{}_{}", self.id.name, i, name_suffix),
                    source.clone(),
                ))
            };

            constraints.push(build_value_constraint(
                "band_ub",
                ConstraintRelation::LessEqual,
                tolerance + big_m,
                big_m,
                0.0,
            )?);
            constraints.push(build_value_constraint(
                "band_lb",
                ConstraintRelation::GreaterEqual,
                -tolerance - big_m,
                -big_m,
                0.0,
            )?);
            constraints.push(build_value_constraint(
                "out_lb",
                ConstraintRelation::GreaterEqual,
                strict_boundary - big_m,
                big_m,
                -big_m,
            )?);
            constraints.push(build_value_constraint(
                "out_ub",
                ConstraintRelation::LessEqual,
                -strict_boundary,
                -big_m,
                -big_m,
            )?);

            // OR link lower bound: result >= b_i
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(1.0, "if_in result coefficient")?,
                                result_index,
                            ),
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(-1.0, "if_in indicator coefficient")?,
                                indicator_index,
                            ),
                        ],
                        convert_f64_to_v::<V>(0.0, "if_in link constant")?,
                    ),
                    ConstraintRelation::GreaterEqual,
                    convert_f64_to_v::<V>(0.0, "if_in link rhs")?,
                ),
                &format!("{}_or_lb_{}", self.id.name, i),
                source.clone(),
            ));
        }

        // OR link upper bound: result <= sum(b_i)
        let mut sum_monomials = Vec::with_capacity(value_indices.len() + 1);
        sum_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "if_in result coefficient")?,
            result_index,
        ));
        for indicator_index in value_indices {
            sum_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-1.0, "if_in indicator coefficient")?,
                indicator_index,
            ));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    sum_monomials,
                    convert_f64_to_v::<V>(0.0, "if_in sum constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(0.0, "if_in sum rhs")?,
            ),
            &format!("{}_or_ub", self.id.name),
            source,
        ));

        Ok(constraints)
    }
}

impl<V> Display for IfInFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "if_in({})", self.id.name)
    }
}

impl<V> DynSymbol for IfInFunction<V>
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

impl<V> Symbol for IfInFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for IfInFunction<V>
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
        let big_m = match self.infer_big_m_from_tokens(tokens)? {
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
        format!("if_in({})", self.id.name)
    }

    fn deferred_structure_with_tokens(
        &self,
        tokens: &[Token<V>],
    ) -> Option<Arc<dyn crate::model::intermediate::DeferredFunctionStructure<V>>> {
        // Big-M 必须在结构创建时固定，取法与即时路径的 `mechanism_constraints_with_tokens` 完全一致：
        // 先按令牌边界推断，取不到时回退到构造配置值；推断本身报错（集合值不可用）或两者都不可用
        // 时不提供结构，让即时展开路径给出同一个错误，而不是把错误推迟到物化阶段。
        // The Big-M must be fixed when the structure is created, resolved exactly like the eager
        // `mechanism_constraints_with_tokens`: infer from token bounds first and fall back to the
        // configured value. When inference itself fails (unusable set values) or neither source is
        // available, no structure is offered so eager expansion reports the same error instead of
        // deferring it to materialization.
        let big_m = match self.infer_big_m_from_tokens(tokens) {
            Ok(Some(inferred)) => inferred,
            Ok(None) => self.configured_big_m().ok()?,
            Err(_) => return None,
        };
        Some(Arc::new(IfInStructure::new(
            self.id.name.clone(),
            Arc::new(self.clone()),
            big_m,
        )))
    }
}

/// IF-IN 离散集合判定的求解器无关结构描述
/// Solver-neutral structure description of the IF-IN discrete-set test
///
/// 与 IF/极值采用同一模式：持有产生它的符号（`Arc`）与创建时固定的 Big-M，物化时回调手写路径
/// 的同一个公式生成器并传入同一个 M，因此延迟物化与 EAGER 展开逐行一致（含 M 取值）。结果列是
/// 集合判定的二值列；每个候选值的指示列与 side 列都是本结构的辅助列，全部上报以免原生路径误判
/// 可以省略。
///
/// Follows the same pattern as IF and the extrema: the structure holds the symbol that produced it
/// (an `Arc`) together with the Big-M fixed at creation time and materializes through the very same
/// formula generator as the handwritten eager path with that same M, so deferred materialization
/// matches eager expansion row by row, including the M value. The result column is the binary
/// set-membership column, while the indicator and side columns of every candidate value are helpers
/// and are all reported so a native path cannot wrongly omit them.
#[derive(Debug)]
pub struct IfInStructure<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 函数名称 / Function name
    name: String,
    /// 产生本结构的符号 / Symbol that produced this structure
    symbol: Arc<IfInFunction<V>>,
    /// 结果列 / Result column
    result: crate::variable::VariableId,
    /// 候选值的指示列与 side 列 / Indicator and side columns of the candidate values
    helpers: Vec<crate::variable::VariableId>,
    /// 创建时固定的 Big-M / Big-M fixed at creation time
    big_m: f64,
}

impl<V> IfInStructure<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建结构描述 / Create a structure description.
    pub fn new(name: impl Into<String>, symbol: Arc<IfInFunction<V>>, big_m: f64) -> Self {
        let result = symbol.result_variable().id();
        let count = symbol.values.len();
        let mut helpers = Vec::with_capacity(count * 2);
        for index in 0..count {
            helpers.push(symbol.value_indicator_variable(index).id());
        }
        for index in 0..count {
            helpers.push(symbol.value_side_variable(count, index).id());
        }
        Self {
            name: name.into(),
            symbol,
            result,
            helpers,
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

    /// 获取辅助列 / Get the helper columns.
    pub fn helpers(&self) -> &[crate::variable::VariableId] {
        &self.helpers
    }

    /// 获取固定的 Big-M / Get the fixed Big-M.
    pub fn big_m(&self) -> f64 {
        self.big_m
    }

    /// 获取即时展开使用的输入多项式（只读；单项式用列下标表示）
    /// Read-only access to the input polynomial used by the eager expansion (monomials use column
    /// indices).
    ///
    /// 用途：原生 writer 必须把**同一份**输入关系写进 SDK 的一般约束，而不是在别处重新推导；
    /// 本访问器只转发符号上的只读数据，不复制公式，也不暴露可变状态。
    ///
    /// Purpose: a native writer must write this **very** input relation into the SDK's general
    /// constraints instead of re-deriving it; this only forwards read-only data from the symbol, copies
    /// no formula and exposes no mutable state.
    pub fn input_polynomial(&self) -> &Linear<V> {
        self.symbol.input_polynomial()
    }

    /// 获取离散值集合（只读）/ Read-only access to the discrete value set.
    pub fn values(&self) -> &[V] {
        self.symbol.values()
    }

    /// 获取每个候选值的指示列，顺序与 [`Self::values`] 一致
    /// Read-only access to the per-candidate indicator columns, in [`Self::values`] order.
    ///
    /// 用途：原生 writer 要把每个候选值写成「指示列 = 1 ⇒ band 成立」的指示约束。
    /// Purpose: a native writer turns every candidate into an "indicator = 1 ⇒ band holds" indicator
    /// constraint.
    pub fn indicator_columns(&self) -> Vec<crate::variable::VariableId> {
        (0..self.symbol.values.len())
            .map(|index| self.symbol.value_indicator_variable(index).id())
            .collect()
    }

    /// 获取每个候选值的 side 列，顺序与 [`Self::values`] 一致
    /// Read-only access to the per-candidate side columns, in [`Self::values`] order.
    ///
    /// 用途：原生 writer 用 side 列的取值分裂「在集合外」的两种情形（低于 / 高于候选值）。
    /// Purpose: a native writer splits the two "outside the set" cases (below / above the candidate)
    /// by the value of the side column.
    pub fn side_columns(&self) -> Vec<crate::variable::VariableId> {
        let count = self.symbol.values.len();
        (0..count)
            .map(|index| self.symbol.value_side_variable(count, index).id())
            .collect()
    }
}

impl<V> crate::model::intermediate::DeferredFunctionStructure<V> for IfInStructure<V>
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
        // 候选值的指示列与 side 列都是本结构的辅助列，参与「是否被外部引用 / 是否可省略」的判定。
        // The indicator and side columns of the candidate values are helpers of this structure and
        // take part in the externally-referenced and omittable analysis.
        Some(crate::model::intermediate::StructureUsageBinding::new(
            self.symbol.id.id,
            self.result.clone(),
            self.helpers.clone(),
        ))
    }

    fn fingerprint(&self) -> Option<String> {
        // 全部辅助列与 Big-M 都进入指纹：任一语义字段变化都必须让旧记录失效。
        // Every helper column and the Big-M are part of the fingerprint: any semantic change must
        // invalidate old records.
        let helpers = self
            .helpers
            .iter()
            .map(|helper| helper.unique_id().to_string())
            .collect::<Vec<_>>()
            .join(",");
        Some(format!(
            "if_in|{}|{}|{}|{}|{}",
            self.name,
            self.symbol.id.id,
            self.result.unique_id(),
            helpers,
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

impl<V> FunctionSymbol<V> for IfInFunction<V>
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
        self.validate_set_values()?;
        tokens.push(Token::from_generic(self.result_var.clone(), usize::MAX));
        let count = self.values.len();
        for i in 0..count {
            let indicator_var = self.value_indicator_variable(i);
            tokens.push(Token::from_generic(indicator_var.clone(), usize::MAX));
        }
        for i in 0..count {
            let side_var = self.value_side_variable(count, i);
            tokens.push(Token::from_generic(side_var.clone(), usize::MAX));
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        self.validate_set_values().ok()?;
        let value = to_f64(&evaluate_linear(&self.input, token_table, zero_if_none)?)?;
        let eps = STEP_EPSILON;

        for v in &self.values {
            let v_f = to_f64(v)?;
            if (value - v_f).abs() <= eps {
                return from_f64(1.0);
            }
        }
        from_f64(0.0)
    }
}

impl<V> LinearIntermediateSymbol<V> for IfInFunction<V>
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
    use super::*;
    use crate::model::{ConstraintRelation, FunctionExpansionPolicy, LinearConstraint, MetaModel};
    use crate::symbol::functions::conditional::{
        ConditionBounds, ConditionRelation, ConditionalIfFunction,
    };
    use crate::token::{MutableTokenList, Token, TokenList, VecTokenList};
    use crate::variable::{ContinuousVariableItem, VariableRange};
    use std::collections::HashMap;

    fn token_index_map<V>(tokens: &[Token<V>]) -> HashMap<usize, usize>
    where
        V: Clone + Debug + Send + Sync + 'static,
    {
        tokens
            .iter()
            .enumerate()
            .map(|(index, token)| (token.id().unique_id() as usize, index + 1))
            .collect()
    }

    /// 逐行比较延迟物化与即时展开 / Compare deferred materialization with eager expansion row by row
    fn assert_rows_match(eager: &[LinearConstraint<f64>], deferred: &[LinearConstraint<f64>]) {
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
            ConstraintRelation::LessEqual => lhs <= constraint.inequality.rhs + 1e-6,
            ConstraintRelation::Equal => (lhs - constraint.inequality.rhs).abs() <= 1e-6,
            ConstraintRelation::GreaterEqual => lhs + 1e-6 >= constraint.inequality.rhs,
        }
    }

    fn satisfies_at_precision(
        constraint: &LinearConstraint<f64>,
        values: &HashMap<usize, f64>,
    ) -> bool {
        let lhs = constraint_lhs(constraint, values);
        match constraint.inequality.relation {
            ConstraintRelation::LessEqual => lhs <= constraint.inequality.rhs + 1e-12,
            ConstraintRelation::Equal => (lhs - constraint.inequality.rhs).abs() <= 1e-12,
            ConstraintRelation::GreaterEqual => lhs + 1e-12 >= constraint.inequality.rhs,
        }
    }

    /// 即时展开的一行在 `(b_i, side_i)` 取给定值时对 `s_i` 的投影：`(关系, 右端项)`
    /// Project one eager row onto `s_i` for a given `(b_i, side_i)`: `(relation, right-hand side)`
    ///
    /// 行本身是 `Σ c_k x_k + shifted_constant + ind·b_i + side·side_i REL rhs`，而
    /// `s_i = Σ c_k x_k + shifted_constant`，因此固定两个指示列后该行等价于
    /// `s_i REL rhs - ind·b_i - side·side_i`。列号用**列下标**口径（与 `symbol_to_index` 一致）。
    ///
    /// The row is `Σ c_k x_k + shifted_constant + ind·b_i + side·side_i REL rhs` while
    /// `s_i = Σ c_k x_k + shifted_constant`, so with both indicator columns fixed it is equivalent to
    /// `s_i REL rhs - ind·b_i - side·side_i`. Column numbers use the **column index** convention (the
    /// one `symbol_to_index` uses).
    fn eager_value_projection(
        constraint: &LinearConstraint<f64>,
        indicator_column: usize,
        side_column: usize,
        indicator_value: f64,
        side_value: f64,
    ) -> (ConstraintRelation, f64) {
        let coefficient_of = |column: usize| {
            constraint
                .inequality
                .polynomial
                .monomials()
                .iter()
                .find(|monomial| monomial.var_index() == column)
                .map(|monomial| *monomial.coefficient())
                .unwrap_or(0.0)
        };
        (
            constraint.inequality.relation,
            constraint.inequality.rhs
                - coefficient_of(indicator_column) * indicator_value
                - coefficient_of(side_column) * side_value,
        )
    }

    /// IF-IN 的核心关系表必须与即时展开里「不含 Big-M 的那些行」逐位一致
    /// The IF-IN core-relation table must agree bit for bit with the eager rows carrying no Big-M.
    ///
    /// 验证手法（与关系指示同一套）：用两个不同的 Big-M 生成同一候选值的四行即时展开，对每个
    /// `(b_i, side_i)` 取值组合做投影。投影不随 M 变化的行就是核心行，必须与表逐位相等（含 band
    /// 容差与严格边界）；投影随 M 线性变化的行必须给出表里登记的松弛右端，即
    /// `s_i >= relaxed_lower_rhs - M` 与 `s_i <= M - relaxed_upper_rhs`。
    ///
    /// How it is verified (the same scheme as the relation indicator): two different Big-M values
    /// generate the four eager rows of one candidate and each `(b_i, side_i)` assignment is projected.
    /// A row whose projection does not move with M is a core row and must equal the table bit for bit
    /// (including the band tolerance and the strict boundary), while a row whose projection moves
    /// linearly with M must yield the tabulated relaxation right-hand sides, namely
    /// `s_i >= relaxed_lower_rhs - M` and `s_i <= M - relaxed_upper_rhs`.
    #[test]
    fn if_in_core_relations_match_the_big_m_free_eager_rows() {
        // 列号口径：结果 1，指示列 2 / 3，side 列 4 / 5（两个候选值）。
        // Column numbering: result 1, indicator columns 2 / 3, side columns 4 / 5 (two candidates).
        const RESULT_COLUMN: usize = 1;
        const INDICATOR_COLUMNS: [usize; 2] = [2, 3];
        const SIDE_COLUMNS: [usize; 2] = [4, 5];
        let small_m = 8.0f64;
        let large_m = 16.0f64;
        let projection_tolerance = 1e-12;

        let f: IfInFunction<f64> = IfInFunction::new(
            9100,
            "ifin_core_table",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
            vec![3.0, 5.0],
            10.0,
        );
        let mut symbol_to_index = HashMap::new();
        symbol_to_index.insert(f.result_variable().id().unique_id() as usize, RESULT_COLUMN);
        for index in 0..2 {
            symbol_to_index.insert(
                f.value_indicator_variable(index).id().unique_id() as usize,
                INDICATOR_COLUMNS[index],
            );
            symbol_to_index.insert(
                f.value_side_variable(2, index).id().unique_id() as usize,
                SIDE_COLUMNS[index],
            );
        }

        let small_rows = f
            .build_mechanism_constraints(&symbol_to_index, small_m)
            .expect("eager rows with the smaller big-M");
        let large_rows = f
            .build_mechanism_constraints(&symbol_to_index, large_m)
            .expect("eager rows with the larger big-M");
        assert_eq!(small_rows.len(), large_rows.len());

        let expected = if_in_value_core_relations();
        assert_eq!(expected.band_tolerance, STEP_EPSILON);
        assert_eq!(expected.relaxed_lower_rhs, STRICT_BOUNDARY);
        assert_eq!(expected.relaxed_upper_rhs, STRICT_BOUNDARY);

        // 只分析第一个候选值的四行（`{name}_pt0_*`），其余候选值形态完全相同。
        // Only the first candidate's four rows (`{name}_pt0_*`) are analysed; every other candidate has
        // the same shape.
        let value_rows: Vec<usize> = small_rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.name.starts_with("ifin_core_table_pt0_"))
            .map(|(index, _)| index)
            .collect();
        assert_eq!(value_rows.len(), 4, "one candidate emits exactly four value rows");

        let mut lower_rhs: Option<f64> = None;
        let mut upper_rhs: Option<f64> = None;
        for (indicator_value, side_value) in [
            (1.0f64, 0.0f64),
            (1.0f64, 1.0f64),
            (0.0f64, 0.0f64),
            (0.0f64, 1.0f64),
        ] {
            let mut core = Vec::new();
            for index in &value_rows {
                let small = eager_value_projection(
                    &small_rows[*index],
                    INDICATOR_COLUMNS[0],
                    SIDE_COLUMNS[0],
                    indicator_value,
                    side_value,
                );
                let large = eager_value_projection(
                    &large_rows[*index],
                    INDICATOR_COLUMNS[0],
                    SIDE_COLUMNS[0],
                    indicator_value,
                    side_value,
                );
                assert_eq!(small.0, large.0, "a row's relation must not depend on the big-M");
                if (small.1 - large.1).abs() <= projection_tolerance {
                    core.push(small);
                } else {
                    // 投影随 M 线性变化：`k` 是斜率大小（以 M 为单位），`c` 是松弛右端。上界行的
                    // 斜率必须为正（`s_i <= k·M - c`），下界行的斜率必须为负（`s_i >= c - k·M`），
                    // 也就是 Big-M 只会**放松**该行。
                    // The projection moves linearly with M: `k` is the slope's magnitude in units of M
                    // and `c` is the relaxation's right-hand side. An upper row's slope must be
                    // positive (`s_i <= k·M - c`) and a lower row's negative (`s_i >= c - k·M`), that
                    // is the Big-M can only **relax** the row.
                    let slope = (large.1 - small.1) / (large_m - small_m);
                    let k = slope.abs().round();
                    assert!(
                        (slope.abs() - k).abs() <= 1e-9 && (k == 1.0 || k == 2.0),
                        "a relaxed row must be linear in M with slope magnitude 1 or 2, got {slope}"
                    );
                    let c = match small.0 {
                        // `s_i <= k·M - c`
                        ConstraintRelation::LessEqual => {
                            assert!(slope > 0.0, "an upper relaxation must grow with the big-M");
                            k * small_m - small.1
                        }
                        // `s_i >= c - k·M`
                        ConstraintRelation::GreaterEqual => {
                            assert!(slope < 0.0, "a lower relaxation must fall with the big-M");
                            k * small_m + small.1
                        }
                        ConstraintRelation::Equal => panic!("if-in value rows are never equalities"),
                    };
                    match small.0 {
                        ConstraintRelation::LessEqual => {
                            upper_rhs = Some(upper_rhs.map_or(c, |current: f64| current.max(c)));
                        }
                        ConstraintRelation::GreaterEqual => {
                            lower_rhs = Some(lower_rhs.map_or(c, |current: f64| current.max(c)));
                        }
                        ConstraintRelation::Equal => unreachable!(),
                    }
                }
            }

            let expected_core: Vec<(ConstraintRelation, f64)> = match (indicator_value, side_value) {
                (1.0, _) => expected.when_indicator_true.clone(),
                (0.0, 0.0) => vec![expected.when_side_false],
                (0.0, _) => vec![expected.when_side_true],
                _ => unreachable!(),
            };
            assert_eq!(
                core.len(),
                expected_core.len(),
                "unexpected number of big-M-free rows at (b, side) = ({indicator_value}, {side_value})"
            );
            for (relation, rhs) in &expected_core {
                assert!(
                    core.iter().any(|(core_relation, core_rhs)| {
                        core_relation == relation && (core_rhs - rhs).abs() <= projection_tolerance
                    }),
                    "core relation ({relation:?}, {rhs}) is missing from the eager rows at (b, side) = ({indicator_value}, {side_value}); eager cores are {core:?}"
                );
            }
        }

        // 全部斜率-1 松弛行给出的最大右端就是表里登记的证明目标。
        // The largest right-hand side over all slope-1 relaxed rows is exactly the tabulated proof
        // target.
        assert!(
            (lower_rhs.expect("a lower relaxation must exist") - expected.relaxed_lower_rhs).abs()
                <= projection_tolerance,
            "lower relaxation mismatch: eager {lower_rhs:?} vs table {}",
            expected.relaxed_lower_rhs
        );
        assert!(
            (upper_rhs.expect("an upper relaxation must exist") - expected.relaxed_upper_rhs).abs()
                <= projection_tolerance,
            "upper relaxation mismatch: eager {upper_rhs:?} vs table {}",
            expected.relaxed_upper_rhs
        );
    }

    #[test]
    fn if_in_function_calculate_value_in_set() {
        let x = ContinuousVariableItem::create(VariableId::standalone(90_000), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(3.0);
        tokens.add_token(tx);

        let f: IfInFunction<f64> = IfInFunction::new(
            9000,
            "ifin_test",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            vec![1.0, 3.0, 5.0],
            100.0,
        );
        let value = <IfInFunction as FunctionSymbol>::calculate_value(&f, &tokens, false);
        assert_eq!(value, Some(1.0));
    }

    #[test]
    fn if_in_function_calculate_value_not_in_set() {
        let x = ContinuousVariableItem::create(VariableId::standalone(90_010), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(2.0);
        tokens.add_token(tx);

        let f: IfInFunction<f64> = IfInFunction::new(
            9001,
            "ifin_test2",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            vec![1.0, 3.0, 5.0],
            100.0,
        );
        let value = <IfInFunction as FunctionSymbol>::calculate_value(&f, &tokens, false);
        assert_eq!(value, Some(0.0));
    }

    #[test]
    fn if_in_function_calculate_value_rejects_non_finite_set_values() {
        let x = ContinuousVariableItem::create(VariableId::standalone(90_015), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(1.0);
        tokens.add_token(tx);

        for (index, invalid_value) in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY]
            .into_iter()
            .enumerate()
        {
            let f: IfInFunction<f64> = IfInFunction::new(
                90015 + index as u64,
                "ifin_non_finite_calculate",
                Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
                vec![invalid_value],
                100.0,
            );
            let value = <IfInFunction as FunctionSymbol>::calculate_value(&f, &tokens, false);

            assert_eq!(value, None);
            assert_ne!(value, Some(0.0));
        }
    }

    #[test]
    fn if_in_function_calculate_value_with_polynomial_input() {
        let x = ContinuousVariableItem::create(VariableId::standalone(90_020), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(1.0);
        tokens.add_token(tx);

        // input = 2x + 1, when x=1 => input=3
        let f: IfInFunction<f64> = IfInFunction::new(
            9002,
            "ifin_poly",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
            vec![1.0, 3.0, 5.0],
            100.0,
        );
        let value = <IfInFunction as FunctionSymbol>::calculate_value(&f, &tokens, false);
        assert_eq!(value, Some(1.0));
    }

    #[test]
    fn if_in_function_zero_if_none() {
        let x = ContinuousVariableItem::create(VariableId::standalone(90_030), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        // no result set, so evaluate_linear returns None normally
        tokens.add_token(tx);

        let f: IfInFunction<f64> = IfInFunction::new(
            9003,
            "ifin_none",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            vec![0.0],
            100.0,
        );
        let value = <IfInFunction as FunctionSymbol>::calculate_value(&f, &tokens, true);
        assert_eq!(value, Some(1.0));
    }

    #[test]
    fn if_in_function_empty_values_forces_zero() {
        let f: IfInFunction<f64> = IfInFunction::new(
            9004,
            "ifin_empty",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            vec![],
            100.0,
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize)]);
        let constraints = f
            .mechanism_constraints(&symbol_to_index)
            .expect("empty if_in constraints should be generated");

        assert_eq!(constraints.len(), 1);
        assert_eq!(constraints[0].name, "ifin_empty_empty");
        assert_eq!(
            constraints[0].inequality.relation,
            ConstraintRelation::Equal
        );
        assert_eq!(constraints[0].inequality.rhs, 0.0);
    }

    #[test]
    fn if_in_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(90_040),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        // input = 2x + 1 with x in [-2, 3] => range [-3, 7]
        // values = [1.0, 3.0, 5.0]
        // |lower - 1| = 4, |upper - 1| = 6 => max 6
        // |lower - 3| = 6, |upper - 3| = 4 => max 6
        // |lower - 5| = 8, |upper - 5| = 2 => max 8
        // 最大偏差为 8，推断 M = 8 + 严格边界。
        // Maximum difference is 8, so inferred M = 8 + strict boundary.
        let f: IfInFunction<f64> = IfInFunction::new(
            9005,
            "ifin_bound",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
            vec![1.0, 3.0, 5.0],
            100.0,
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let mut symbol_to_index = HashMap::new();
        symbol_to_index.insert(result_id, 1usize);
        for i in 0..3 {
            let indicator = f.value_indicator_variable(i);
            let side = f.value_side_variable(3, i);
            symbol_to_index.insert(indicator.id().unique_id() as usize, 2 + i);
            symbol_to_index.insert(side.id().unique_id() as usize, 5 + i);
        }

        let tokens = vec![Token::from_generic(x, 0)];
        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("if_in constraints should be generated");
        let band_ub = constraints
            .iter()
            .find(|constraint| constraint.name == "ifin_bound_pt2_band_ub")
            .expect("pt2 upper-band constraint should exist");
        let indicator_term = band_ub
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 4)
            .expect("indicator term should exist");

        // value=5.0（第三个值 pt2）时，推断 M = 8 + 严格边界。
        // For value=5.0 (the third value, pt2), inferred M = 8 + strict boundary.
        let expected_m = 8.0 + STRICT_BOUNDARY;
        let expected_rhs = expected_m + STEP_EPSILON;
        assert!((band_ub.inequality.rhs - expected_rhs).abs() <= 1e-9);
        assert!((*indicator_term.coefficient() - expected_m).abs() <= 1e-9);
    }

    #[test]
    fn if_in_function_inferred_big_m_keeps_upper_endpoint_non_member_feasible() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(90_045),
            "x",
            VariableRange::bounded(0.0, 1.0),
        );
        let f: IfInFunction<f64> = IfInFunction::new(
            9011,
            "ifin_endpoint",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            vec![0.0],
            100.0,
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let indicator = f.value_indicator_variable(0);
        let side = f.value_side_variable(1, 0);
        let symbol_to_index = HashMap::from([
            (result_id, 1usize),
            (indicator.id().unique_id() as usize, 2usize),
            (side.id().unique_id() as usize, 3usize),
        ]);
        let tokens = vec![Token::from_generic(x, 0)];
        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("if_in endpoint constraints should be generated");

        let band_ub = constraints
            .iter()
            .find(|constraint| constraint.name == "ifin_endpoint_pt0_band_ub")
            .expect("endpoint upper-band constraint should exist");
        let indicator_term = band_ub
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 2)
            .expect("endpoint indicator term should exist");
        let expected_m = 1.0 + STRICT_BOUNDARY;
        assert!((*indicator_term.coefficient() - expected_m).abs() <= 1e-12);

        // x=0 命中集合下端点，x=1 是合法非成员上端点。
        // x=0 hits the set endpoint, while x=1 is a valid non-member endpoint.
        let member_at_lower_endpoint = HashMap::from([
            (0usize, 0.0_f64),
            (1usize, 1.0),
            (2usize, 1.0),
            (3usize, 0.0),
        ]);
        assert!(
            constraints
                .iter()
                .all(|constraint| satisfies_at_precision(constraint, &member_at_lower_endpoint))
        );

        let non_member_at_upper_endpoint = HashMap::from([
            (0usize, 1.0_f64),
            (1usize, 0.0),
            (2usize, 0.0),
            (3usize, 1.0),
        ]);
        assert!(
            constraints.iter().all(|constraint| satisfies_at_precision(
                constraint,
                &non_member_at_upper_endpoint
            ))
        );
    }

    #[test]
    fn if_in_function_inferred_big_m_uses_ulp_for_large_endpoint_gaps() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(90_046),
            "large_x",
            VariableRange::bounded(0.0, 1e16),
        );
        let f: IfInFunction<f64> = IfInFunction::new(
            9012,
            "ifin_large_endpoint",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            vec![0.0],
            100.0,
        );
        let result_id = f.result_variable().id().unique_id() as usize;
        let indicator = f.value_indicator_variable(0);
        let side = f.value_side_variable(1, 0);
        let symbol_to_index = HashMap::from([
            (result_id, 1usize),
            (indicator.id().unique_id() as usize, 2usize),
            (side.id().unique_id() as usize, 3usize),
        ]);
        let tokens = vec![Token::from_generic(x, 0)];
        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("large-scale if_in constraints should be generated");
        let band_ub = constraints
            .iter()
            .find(|constraint| constraint.name == "ifin_large_endpoint_pt0_band_ub")
            .expect("large-scale upper-band constraint should exist");
        let inferred_m = *band_ub
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 2)
            .expect("large-scale indicator term should exist")
            .coefficient();
        assert!(inferred_m > 1e16);
        assert!(inferred_m - 1e16 >= STRICT_BOUNDARY);

        // x=1e16 是集合外上端点，正差值侧变量取 1。
        // x=1e16 is the out-of-set upper endpoint, so the positive side variable is 1.
        let non_member_at_upper_endpoint = HashMap::from([
            (0usize, 1e16_f64),
            (1usize, 0.0),
            (2usize, 0.0),
            (3usize, 1.0),
        ]);
        assert!(
            constraints.iter().all(|constraint| satisfies_at_precision(
                constraint,
                &non_member_at_upper_endpoint
            ))
        );

        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(90_047),
            "large_symmetric_x",
            VariableRange::bounded(0.0, 1e16),
        );
        let f: IfInFunction<f64> = IfInFunction::new(
            9013,
            "ifin_large_symmetric_endpoint",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            vec![1e16],
            100.0,
        );
        let result_id = f.result_variable().id().unique_id() as usize;
        let indicator = f.value_indicator_variable(0);
        let side = f.value_side_variable(1, 0);
        let symbol_to_index = HashMap::from([
            (result_id, 1usize),
            (indicator.id().unique_id() as usize, 2usize),
            (side.id().unique_id() as usize, 3usize),
        ]);
        let tokens = vec![Token::from_generic(x, 0)];
        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("symmetric large-scale if_in constraints should be generated");
        let band_ub = constraints
            .iter()
            .find(|constraint| constraint.name == "ifin_large_symmetric_endpoint_pt0_band_ub")
            .expect("symmetric upper-band constraint should exist");
        let inferred_m = *band_ub
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 2)
            .expect("symmetric indicator term should exist")
            .coefficient();
        assert!(inferred_m > 1e16);
        assert!(inferred_m - 1e16 >= STRICT_BOUNDARY);

        // x=0 是集合外下端点，负差值侧变量取 0。
        // x=0 is the out-of-set lower endpoint, so the negative side variable is 0.
        let non_member_at_lower_endpoint = HashMap::from([
            (0usize, 0.0_f64),
            (1usize, 0.0),
            (2usize, 0.0),
            (3usize, 0.0),
        ]);
        assert!(
            constraints.iter().all(|constraint| satisfies_at_precision(
                constraint,
                &non_member_at_lower_endpoint
            ))
        );
    }

    #[test]
    fn if_in_function_rejects_inferred_big_m_without_a_finite_next_up() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(90_048),
            "max_x",
            VariableRange::bounded(0.0, f64::MAX),
        );
        let f: IfInFunction<f64> = IfInFunction::new(
            9014,
            "ifin_max_endpoint",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            vec![0.0],
            100.0,
        );
        let tokens = vec![Token::from_generic(x, 0)];
        let error = f
            .infer_big_m_from_tokens(&tokens)
            .expect_err("an unsafe inferred Big-M should be rejected");
        assert!(error.to_string().contains("preserving strict boundary"));
    }

    #[test]
    fn if_in_function_falls_back_to_configured_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(90_050), "x");
        let f: IfInFunction<f64> = IfInFunction::new(
            9006,
            "ifin_default",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            vec![1.0, 2.0],
            13.0,
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let mut symbol_to_index = HashMap::new();
        symbol_to_index.insert(result_id, 1usize);
        for i in 0..2 {
            let indicator = f.value_indicator_variable(i);
            let side = f.value_side_variable(2, i);
            symbol_to_index.insert(indicator.id().unique_id() as usize, 2 + i);
            symbol_to_index.insert(side.id().unique_id() as usize, 4 + i);
        }

        let tokens = vec![Token::from_generic(x, 0)];
        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("if_in constraints should be generated");
        let band_ub = constraints
            .iter()
            .find(|constraint| constraint.name == "ifin_default_pt0_band_ub")
            .expect("pt0 upper-band constraint should exist");
        let indicator_term = band_ub
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 2)
            .expect("indicator term should exist");

        assert!((band_ub.inequality.rhs - (13.0 + STEP_EPSILON)).abs() <= 1e-9);
        assert!((*indicator_term.coefficient() - 13.0).abs() <= 1e-9);
    }

    #[test]
    fn if_in_function_rejects_non_finite_set_value_without_partial_registration() {
        let x = ContinuousVariableItem::create(VariableId::standalone(90_051), "x");
        let f: IfInFunction<f64> = IfInFunction::new(
            9015,
            "ifin_non_finite_value",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            vec![f64::NAN],
            13.0,
        );

        let mut registered_tokens = Vec::new();
        let registration_error = f
            .register_tokens(&mut registered_tokens)
            .expect_err("non-finite set values must be rejected before registration");
        assert!(matches!(
            registration_error,
            crate::error::CoreError::Model(crate::error::ModelError::InvalidConstraint(message))
                if message.contains("non-finite set value")
        ));
        assert!(registered_tokens.is_empty());

        let result_id = f.result_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize)]);
        let input_tokens = vec![Token::from_generic(x, 0)];
        let constraint_error = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &input_tokens)
            .expect_err("non-finite set values must not use configured Big-M fallback");
        assert!(matches!(
            constraint_error,
            crate::error::CoreError::Model(crate::error::ModelError::InvalidConstraint(message))
                if message.contains("non-finite set value")
        ));
    }

    #[test]
    fn if_in_function_rejects_f32_big_m_overflow_before_constraints() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(90_052),
            "f32_x",
            VariableRange::bounded(1.0e20_f64, 2.0e20_f64),
        );
        let f: IfInFunction<f32> = IfInFunction::new(
            9016,
            "ifin_f32_big_m",
            Linear::new(vec![LinearMonomial::new(1.0e20_f32, 0)], 0.0_f32),
            vec![0.0_f32],
            100.0_f32,
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let indicator = f.value_indicator_variable(0);
        let side = f.value_side_variable(1, 0);
        let symbol_to_index = HashMap::from([
            (result_id, 1usize),
            (indicator.id().unique_id() as usize, 2usize),
            (side.id().unique_id() as usize, 3usize),
        ]);
        let input_tokens = vec![Token::from_generic(x, 0)];
        let error = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &input_tokens)
            .expect_err("an inferred Big-M outside f32 must be rejected");

        assert!(matches!(
            error,
            crate::error::CoreError::Model(crate::error::ModelError::InvalidConstraint(message))
                if message.contains("if_in Big-M") && message.contains("finite")
        ));
    }

    #[test]
    fn if_in_function_mechanism_constraints_or_link() {
        let f: IfInFunction<f64> = IfInFunction::new(
            9007,
            "ifin_or",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            vec![1.0, 3.0],
            10.0,
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let mut symbol_to_index = HashMap::new();
        symbol_to_index.insert(result_id, 1usize);
        for i in 0..2 {
            let indicator = f.value_indicator_variable(i);
            let side = f.value_side_variable(2, i);
            symbol_to_index.insert(indicator.id().unique_id() as usize, 2 + i);
            symbol_to_index.insert(side.id().unique_id() as usize, 4 + i);
        }

        let constraints = f
            .mechanism_constraints(&symbol_to_index)
            .expect("if_in constraints should be generated");

        // 2 values * 5 constraints each (4 point + 1 or_lb) + 1 or_ub = 11
        assert_eq!(constraints.len(), 11);

        // Verify OR link lower bounds exist
        let or_lb_0 = constraints
            .iter()
            .find(|c| c.name == "ifin_or_or_lb_0")
            .expect("or_lb_0 should exist");
        assert_eq!(
            or_lb_0.inequality.relation,
            ConstraintRelation::GreaterEqual
        );

        let or_lb_1 = constraints
            .iter()
            .find(|c| c.name == "ifin_or_or_lb_1")
            .expect("or_lb_1 should exist");
        assert_eq!(
            or_lb_1.inequality.relation,
            ConstraintRelation::GreaterEqual
        );

        // Verify OR link upper bound exists
        let or_ub = constraints
            .iter()
            .find(|c| c.name == "ifin_or_or_ub")
            .expect("or_ub should exist");
        assert_eq!(or_ub.inequality.relation, ConstraintRelation::LessEqual);

        // Verify point band constraints
        let band_ub_0 = constraints
            .iter()
            .find(|c| c.name == "ifin_or_pt0_band_ub")
            .expect("pt0 band_ub should exist");
        assert_eq!(band_ub_0.inequality.relation, ConstraintRelation::LessEqual);

        let band_lb_0 = constraints
            .iter()
            .find(|c| c.name == "ifin_or_pt0_band_lb")
            .expect("pt0 band_lb should exist");
        assert_eq!(
            band_lb_0.inequality.relation,
            ConstraintRelation::GreaterEqual
        );
    }

    #[test]
    fn if_in_function_satisfies_constraints_when_result_is_one() {
        let _x = ContinuousVariableItem::create(VariableId::standalone(90_060), "x");
        let f: IfInFunction<f64> = IfInFunction::new(
            9008,
            "ifin_sat",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            vec![1.0, 3.0],
            10.0,
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let indicator_0 = f.value_indicator_variable(0);
        let indicator_1 = f.value_indicator_variable(1);
        let side_0 = f.value_side_variable(2, 0);
        let side_1 = f.value_side_variable(2, 1);

        let mut symbol_to_index = HashMap::new();
        symbol_to_index.insert(result_id, 1usize);
        symbol_to_index.insert(indicator_0.id().unique_id() as usize, 2usize);
        symbol_to_index.insert(indicator_1.id().unique_id() as usize, 3usize);
        symbol_to_index.insert(side_0.id().unique_id() as usize, 4usize);
        symbol_to_index.insert(side_1.id().unique_id() as usize, 5usize);

        let constraints = f
            .mechanism_constraints(&symbol_to_index)
            .expect("if_in constraints should be generated");

        // Scenario: x=3.0, result=1, indicator_1=1, indicator_0=0
        // pt0 (value=1.0): x=3 > value, so side_0=1 (upper side)
        // pt1 (value=3.0): x=3 ≈ value, indicator_1=1, side_1 doesn't matter
        let assignment = HashMap::from([
            (0usize, 3.0_f64), // x
            (1usize, 1.0),     // result
            (2usize, 0.0),     // indicator_0
            (3usize, 1.0),     // indicator_1
            (4usize, 1.0),     // side_0 (upper: x > value[0])
            (5usize, 0.0),     // side_1
        ]);

        assert!(
            constraints
                .iter()
                .all(|constraint| satisfies(constraint, &assignment)),
            "all constraints should be satisfied when x=3, result=1, indicator_1=1"
        );
    }

    #[test]
    fn if_in_function_satisfies_constraints_when_result_is_zero() {
        let _x = ContinuousVariableItem::create(VariableId::standalone(90_070), "x");
        let f: IfInFunction<f64> = IfInFunction::new(
            9009,
            "ifin_zero",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            vec![1.0, 3.0],
            10.0,
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let indicator_0 = f.value_indicator_variable(0);
        let indicator_1 = f.value_indicator_variable(1);
        let side_0 = f.value_side_variable(2, 0);
        let side_1 = f.value_side_variable(2, 1);

        let mut symbol_to_index = HashMap::new();
        symbol_to_index.insert(result_id, 1usize);
        symbol_to_index.insert(indicator_0.id().unique_id() as usize, 2usize);
        symbol_to_index.insert(indicator_1.id().unique_id() as usize, 3usize);
        symbol_to_index.insert(side_0.id().unique_id() as usize, 4usize);
        symbol_to_index.insert(side_1.id().unique_id() as usize, 5usize);

        let constraints = f
            .mechanism_constraints(&symbol_to_index)
            .expect("if_in constraints should be generated");

        // Scenario: x=2.0, result=0, all indicators=0
        // pt0 (value=1.0): x=2 > value, so side_0=1 (upper side)
        // pt1 (value=3.0): x=2 < value, so side_1=0 (lower side)
        let assignment = HashMap::from([
            (0usize, 2.0_f64), // x
            (1usize, 0.0),     // result
            (2usize, 0.0),     // indicator_0
            (3usize, 0.0),     // indicator_1
            (4usize, 1.0),     // side_0 (upper: x > value[0])
            (5usize, 0.0),     // side_1 (lower: x < value[1])
        ]);

        assert!(
            constraints
                .iter()
                .all(|constraint| satisfies(constraint, &assignment)),
            "all constraints should be satisfied when x=2, result=0, all indicators=0"
        );
    }

    #[test]
    fn if_in_function_violates_when_result_wrong() {
        let _x = ContinuousVariableItem::create(VariableId::standalone(90_080), "x");
        let f: IfInFunction<f64> = IfInFunction::new(
            9010,
            "ifin_viol",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            vec![1.0, 3.0],
            10.0,
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let indicator_0 = f.value_indicator_variable(0);
        let indicator_1 = f.value_indicator_variable(1);
        let side_0 = f.value_side_variable(2, 0);
        let side_1 = f.value_side_variable(2, 1);

        let mut symbol_to_index = HashMap::new();
        symbol_to_index.insert(result_id, 1usize);
        symbol_to_index.insert(indicator_0.id().unique_id() as usize, 2usize);
        symbol_to_index.insert(indicator_1.id().unique_id() as usize, 3usize);
        symbol_to_index.insert(side_0.id().unique_id() as usize, 4usize);
        symbol_to_index.insert(side_1.id().unique_id() as usize, 5usize);

        let constraints = f
            .mechanism_constraints(&symbol_to_index)
            .expect("if_in constraints should be generated");

        // Scenario: x=2.0 (not in set), but result=1 (wrong!)
        // All indicators must be 0 since x doesn't match any value,
        // but result=1 violates or_ub: result <= sum(indicators) = 0
        let assignment = HashMap::from([
            (0usize, 2.0_f64), // x
            (1usize, 1.0),     // result (wrong!)
            (2usize, 0.0),     // indicator_0
            (3usize, 0.0),     // indicator_1
            (4usize, 1.0),     // side_0
            (5usize, 1.0),     // side_1
        ]);

        assert!(
            !constraints
                .iter()
                .all(|constraint| satisfies(constraint, &assignment)),
            "constraints should be violated when x=2, result=1, all indicators=0"
        );
    }

    #[test]
    fn if_in_range_keeps_closed_interval_three_valued_semantics() {
        let bounds = ConditionBounds {
            lower: -2.0,
            upper: 2.0,
        };
        let lower = ConditionalIfFunction::new(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 1.0),
            ConditionRelation::GreaterEqual,
            0.1,
            bounds.clone(),
        )
        .unwrap();
        let upper = ConditionalIfFunction::new(
            Linear::new(vec![LinearMonomial::new(-1.0, 0)], 1.0),
            ConditionRelation::GreaterEqual,
            0.1,
            bounds,
        )
        .unwrap();
        let range = IfInRangeFunction::try_new(lower, upper).unwrap();

        assert_eq!(range.classify(&0.0, &0.0).unwrap(), TruthValue::True);
        assert_eq!(range.classify(&-0.05, &0.0).unwrap(), TruthValue::Undefined);
        assert_eq!(range.classify(&-0.2, &0.0).unwrap(), TruthValue::False);
        assert_eq!(range.evaluate(&0.0, &0.0).unwrap(), Some(1.0));
        assert_eq!(range.evaluate(&-0.05, &0.0).unwrap(), None);
    }

    #[test]
    fn if_in_range_rejects_non_closed_side_relations() {
        let bounds = ConditionBounds {
            lower: -1.0,
            upper: 1.0,
        };
        let lower = ConditionalIfFunction::new(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 1.0),
            ConditionRelation::Greater,
            0.1,
            bounds.clone(),
        )
        .unwrap();
        let upper = ConditionalIfFunction::new(
            Linear::new(vec![LinearMonomial::new(-1.0, 0)], 1.0),
            ConditionRelation::GreaterEqual,
            0.1,
            bounds,
        )
        .unwrap();
        assert!(IfInRangeFunction::try_new(lower, upper).is_err());
    }

    #[test]
    fn if_in_range_constructor_validates_relations_and_endpoint_order() {
        let bounds = ConditionBounds {
            lower: -2.0,
            upper: 2.0,
        };
        let strict_lower = ConditionalIfFunction::new(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            ConditionRelation::Greater,
            0.1,
            bounds.clone(),
        )
        .unwrap();
        let upper = ConditionalIfFunction::new(
            Linear::new(vec![LinearMonomial::new(-1.0, 0)], 1.0),
            ConditionRelation::GreaterEqual,
            0.1,
            bounds.clone(),
        )
        .unwrap();
        assert!(IfInRangeFunction::new(strict_lower, upper.clone()).is_err());

        let lower = ConditionalIfFunction::new(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], -2.0),
            ConditionRelation::GreaterEqual,
            0.1,
            bounds,
        )
        .unwrap();
        assert!(IfInRangeFunction::new(lower, upper).is_err());
    }

    #[test]
    fn if_in_range_constructor_rejects_unrepresentable_side_conditions() {
        let bounds = ConditionBounds {
            lower: -2.0,
            upper: 2.0,
        };
        let upper = ConditionalIfFunction::new(
            Linear::new(vec![LinearMonomial::new(-1.0, 0)], 1.0),
            ConditionRelation::GreaterEqual,
            0.1,
            bounds.clone(),
        )
        .unwrap();

        let polynomial_lower = ConditionalIfFunction::new(
            Linear::new(
                vec![LinearMonomial::new(1.0, 0), LinearMonomial::new(1.0, 1)],
                1.0,
            ),
            ConditionRelation::GreaterEqual,
            0.1,
            bounds.clone(),
        )
        .unwrap();
        assert!(IfInRangeFunction::new(polynomial_lower, upper.clone()).is_err());

        let different_variable_upper = ConditionalIfFunction::new(
            Linear::new(vec![LinearMonomial::new(-1.0, 1)], 1.0),
            ConditionRelation::GreaterEqual,
            0.1,
            bounds.clone(),
        )
        .unwrap();
        let lower = ConditionalIfFunction::new(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 1.0),
            ConditionRelation::GreaterEqual,
            0.1,
            bounds.clone(),
        )
        .unwrap();
        assert!(IfInRangeFunction::new(lower.clone(), different_variable_upper).is_err());

        let wrong_sign_lower = ConditionalIfFunction::new(
            Linear::new(vec![LinearMonomial::new(-1.0, 0)], -1.0),
            ConditionRelation::GreaterEqual,
            0.1,
            bounds.clone(),
        )
        .unwrap();
        assert!(IfInRangeFunction::new(wrong_sign_lower, upper.clone()).is_err());

        let wrong_sign_upper = ConditionalIfFunction::new(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], -1.0),
            ConditionRelation::GreaterEqual,
            0.1,
            bounds.clone(),
        )
        .unwrap();
        assert!(IfInRangeFunction::new(lower.clone(), wrong_sign_upper).is_err());

        let manually_constructed = IfInRangeFunction {
            lower,
            upper: ConditionalIfFunction::new(
                Linear::new(vec![LinearMonomial::new(1.0, 0)], -1.0),
                ConditionRelation::GreaterEqual,
                0.1,
                bounds,
            )
            .unwrap(),
        };
        assert!(manually_constructed.validate().is_err());
    }

    #[test]
    fn registerable_if_in_range_builds_two_indicators_and_an_and_result() {
        let lower = ConditionalIfFunction::new(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 1.0),
            ConditionRelation::GreaterEqual,
            0.1,
            ConditionBounds {
                lower: -1.0,
                upper: 1.0,
            },
        )
        .unwrap();
        let upper = ConditionalIfFunction::new(
            Linear::new(vec![LinearMonomial::new(-1.0, 0)], 1.0),
            ConditionRelation::GreaterEqual,
            0.1,
            ConditionBounds {
                lower: -1.0,
                upper: 1.0,
            },
        )
        .unwrap();
        let range = IfInRangeFunction::try_new(lower, upper)
            .unwrap()
            .registerable(93_001, "range")
            .unwrap();

        let mut tokens = Vec::new();
        range.register_tokens(&mut tokens).unwrap();
        assert_eq!(tokens.len(), 5);
        let symbol_to_index = tokens
            .iter()
            .enumerate()
            .map(|(index, token)| (token.id().unique_id() as usize, index + 1))
            .collect::<HashMap<_, _>>();
        let constraints = range.mechanism_constraints(&symbol_to_index).unwrap();
        assert_eq!(constraints.len(), 9);
        assert!(
            constraints
                .iter()
                .any(|constraint| constraint.name.ends_with("_and_lower_ub"))
        );
        assert!(
            constraints
                .iter()
                .any(|constraint| constraint.name.ends_with("_and_upper_ub"))
        );
        assert!(
            constraints
                .iter()
                .any(|constraint| constraint.name.ends_with("_and_lb"))
        );
        assert_eq!(range.evaluate(&0.0, &0.0).unwrap(), Some(1.0));
    }

    #[test]
    fn registerable_if_in_range_token_registration_is_atomic() {
        let lower = || {
            ConditionalIfFunction::new(
                Linear::new(vec![LinearMonomial::new(1.0, 0)], 1.0),
                ConditionRelation::GreaterEqual,
                0.1,
                ConditionBounds {
                    lower: -1.0,
                    upper: 1.0,
                },
            )
            .unwrap()
        };
        let upper = || {
            ConditionalIfFunction::new(
                Linear::new(vec![LinearMonomial::new(-1.0, 0)], 1.0),
                ConditionRelation::GreaterEqual,
                0.1,
                ConditionBounds {
                    lower: -1.0,
                    upper: 1.0,
                },
            )
            .unwrap()
        };
        let range = IfInRangeFunction::try_new(lower(), upper())
            .unwrap()
            .registerable(93_002, "range_atomic")
            .unwrap();
        let mut tokens = vec![Token::from_generic(
            range.lower_indicator().result_variable().clone(),
            range.lower_indicator().result_variable().index(),
        )];
        assert!(range.register_tokens(&mut tokens).is_err());
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn registerable_if_in_range_tokens_are_unassigned_until_added_to_a_list() {
        let lower = ConditionalIfFunction::new(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 1.0),
            ConditionRelation::GreaterEqual,
            0.1,
            ConditionBounds {
                lower: -1.0,
                upper: 1.0,
            },
        )
        .unwrap();
        let upper = ConditionalIfFunction::new(
            Linear::new(vec![LinearMonomial::new(-1.0, 0)], 1.0),
            ConditionRelation::GreaterEqual,
            0.1,
            ConditionBounds {
                lower: -1.0,
                upper: 1.0,
            },
        )
        .unwrap();
        let range = IfInRangeFunction::try_new(lower, upper)
            .unwrap()
            .registerable(93_003, "range_unassigned")
            .unwrap();

        let mut staged = Vec::new();
        range.register_tokens(&mut staged).unwrap();
        let ids = staged.iter().map(|token| token.id()).collect::<Vec<_>>();
        assert_eq!(staged.len(), 5);
        assert!(staged.iter().all(|token| token.solver_index == usize::MAX));

        let mut token_list = VecTokenList::<f64>::new();
        token_list.try_add_tokens(staged.clone()).unwrap();
        assert_eq!(token_list.len(), 5);
        assert_eq!(
            token_list
                .tokens()
                .iter()
                .map(|token| token.id())
                .collect::<Vec<_>>(),
            ids
        );
        assert!(
            token_list
                .tokens()
                .iter()
                .all(|token| token.solver_index == usize::MAX)
        );
    }

    #[test]
    fn if_in_structure_materializes_the_same_rows_as_eager_expansion() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(96_600),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        // input = 2x + 1：x ∈ [-2, 3] 上取值范围 [-3, 7]；候选 1 与 3 的最大偏差都是 6，
        // 因此推断 M = 6 + 严格边界，而不是构造时配置的 100。
        // input = 2x + 1 spans [-3, 7] over x ∈ [-2, 3]; both candidates 1 and 3 deviate by at most
        // 6, so the inferred M is 6 + strict boundary instead of the configured 100.
        let function: IfInFunction<f64> = IfInFunction::new(
            96_601,
            "if_in_deferred",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
            vec![1.0, 3.0],
            100.0,
        );

        let mut auxiliary_tokens = Vec::new();
        function
            .register_tokens(&mut auxiliary_tokens)
            .expect("if_in tokens should be registered");
        let symbol_to_index = token_index_map(&auxiliary_tokens);
        let mut tokens = vec![Token::from_generic(x, 0)];
        tokens.extend(auxiliary_tokens);

        let structure = function
            .deferred_structure_with_tokens(&tokens)
            .expect("if_in should always expose a deferred structure");
        assert_eq!(structure.function_name(), "if_in_deferred");
        let binding = structure
            .usage_binding()
            .expect("if_in structure should expose a usage binding");
        assert_eq!(binding.result, function.result_variable().id());
        let mut expected_helpers = Vec::new();
        for index in 0..2 {
            expected_helpers.push(function.value_indicator_variable(index).id());
        }
        for index in 0..2 {
            expected_helpers.push(function.value_side_variable(2, index).id());
        }
        assert_eq!(binding.helpers, expected_helpers);
        assert!(structure.fingerprint().is_some());

        let eager = function
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("eager if_in constraints should be generated");
        let deferred = structure
            .materialize(&symbol_to_index)
            .expect("if_in structure should materialize");
        assert!(!eager.is_empty());
        assert_rows_match(&eager, &deferred);

        // 结构必须冻结即时路径推断出的 M：所有候选的最大偏差都是 6。
        // The structure must freeze the M inferred by the eager path: every candidate deviates by 6.
        let band_upper = deferred
            .iter()
            .find(|constraint| constraint.name == "if_in_deferred_pt0_band_ub")
            .expect("pt0 upper-band row should exist");
        let expected_m = 6.0 + STRICT_BOUNDARY;
        assert!((band_upper.inequality.rhs - (expected_m + STEP_EPSILON)).abs() <= 1e-9);
        let concrete = structure
            .as_any()
            .downcast_ref::<IfInStructure<f64>>()
            .expect("if_in structure should downcast to the concrete structure");
        assert!((concrete.big_m() - expected_m).abs() <= 1e-9);

        // 没有令牌边界时回退到构造配置的 M，两条路径仍然逐行一致。
        // Without token bounds the configured M is used and both paths still agree row by row.
        let configured_structure = function
            .deferred_structure_with_tokens(&[])
            .expect("if_in should fall back to the configured big-M");
        let eager_default = function
            .mechanism_constraints_with_tokens(&symbol_to_index, &[])
            .expect("eager if_in constraints should be generated");
        let deferred_default = configured_structure
            .materialize(&symbol_to_index)
            .expect("if_in structure should materialize");
        assert_rows_match(&eager_default, &deferred_default);
        let configured_concrete = configured_structure
            .as_any()
            .downcast_ref::<IfInStructure<f64>>()
            .expect("if_in structure should downcast to the concrete structure");
        assert!((configured_concrete.big_m() - 100.0).abs() <= 1e-9);
    }

    #[test]
    fn if_in_structure_is_withheld_when_big_m_inference_fails() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(96_602),
            "x",
            VariableRange::bounded(0.0, f64::MAX),
        );
        let function: IfInFunction<f64> = IfInFunction::new(
            96_603,
            "if_in_without_big_m",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            vec![0.0],
            100.0,
        );

        // 令牌边界推出的 M 无法保持严格边界：必须留给 EAGER 路径报错。
        // The M inferred from token bounds cannot preserve the strict boundary, so the error must be
        // left to the eager path.
        let tokens = vec![Token::from_generic(x, 0)];
        assert!(function.deferred_structure_with_tokens(&tokens).is_none());
        assert!(
            function
                .mechanism_constraints_with_tokens(&HashMap::new(), &tokens)
                .is_err()
        );
    }

    #[test]
    fn if_in_defers_through_the_model_pipeline() {
        fn rows(policy: FunctionExpansionPolicy) -> Vec<String> {
            let mut model = MetaModel::<f64>::new("if_in_deferred_pipeline");
            model.set_function_expansion_policy(policy);
            let x = ContinuousVariableItem::with_range(
                VariableId::standalone(96_700),
                "x",
                VariableRange::bounded(-2.0, 3.0),
            );
            let x_index = model.register_variable(x).expect("x should register");
            let function: IfInFunction<f64> = IfInFunction::new(
                96_701,
                "if_in_pipeline",
                Linear::new(vec![LinearMonomial::new(2.0, x_index)], 1.0),
                vec![1.0, 3.0],
                100.0,
            );
            model
                .add_symbol(Arc::new(function))
                .expect("if_in symbol should register");

            let mechanism = model
                .try_into_mechanism_model()
                .expect("mechanism model should build");
            if policy.is_deferred() {
                // 延迟策略下不写即时行，但保留结构描述。
                // A deferred policy writes no eager row while keeping the structure description.
                assert!(mechanism.as_basic().constraints().is_empty());
                assert_eq!(mechanism.as_basic().deferred_functions().len(), 1);
                assert_eq!(
                    mechanism.as_basic().deferred_functions()[0].function_name(),
                    "if_in_pipeline"
                );
            }

            let linear = mechanism.into_linear_triad_model();
            let mut names = linear.basic.constraint_names.clone();
            names.sort();
            names
        }

        let eager = rows(FunctionExpansionPolicy::Eager);
        assert!(!eager.is_empty(), "eager rows: {eager:?}");
        // 延迟路径经物化后必须与 EAGER 得到同一批行，并使用同一个推断 Big-M。
        // The deferred path must produce the same rows as eager expansion once materialized, using
        // the same inferred Big-M.
        assert_eq!(eager, rows(FunctionExpansionPolicy::DeferredNativeFirst));
    }
}
