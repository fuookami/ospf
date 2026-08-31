//! 语义化函数符号便利入口 / Semantic function-symbol convenience entry points

use super::conditional::{ConditionBounds, ConditionRelation};
use super::{
    BinaryzationFunction, ConditionalIfFunction, ConditionalImplyFunction,
    ConditionalIndicatorFunction, ConditionalThenFunction, IfElseFunction, IfInRangeFunction,
    ImplyFunction,
};
use crate::error::Result;
use crate::model::LinearInequality;
use crate::symbol::flatten::Linear;
use crate::variable::BinaryVariableItem;
use num_traits::FromPrimitive;
use std::fmt::Debug;

/// 显式关系条件便利入口 / Explicit relation-condition convenience entry.
///
/// 新入口要求调用方提供关系、严格边界和覆盖条件多项式的有限范围；未定义区间
/// 不会被静默归类为假，也不会回退到默认 Big-M。
/// The new entry requires the relation, strict boundary, and finite bounds
/// covering the condition polynomial. Undefined values are not silently treated
/// as false, and no default Big-M is used.
pub fn if_<V>(
    condition: Linear<V>,
    relation: ConditionRelation,
    strict_boundary: V,
    bounds: ConditionBounds<V>,
) -> Result<ConditionalIndicatorFunction<V>>
where
    V: Clone + Debug + Send + Sync + 'static + num_traits::ToPrimitive + FromPrimitive,
{
    ConditionalIndicatorFunction::auto(condition, relation, strict_boundary, bounds)
}

/// 带名称的显式关系条件便利入口 / Named explicit relation-condition entry.
pub fn if_named<V>(
    name: impl AsRef<str>,
    condition: Linear<V>,
    relation: ConditionRelation,
    strict_boundary: V,
    bounds: ConditionBounds<V>,
) -> Result<ConditionalIndicatorFunction<V>>
where
    V: Clone + Debug + Send + Sync + 'static + num_traits::ToPrimitive + FromPrimitive,
{
    ConditionalIndicatorFunction::named(name, condition, relation, strict_boundary, bounds)
}

/// 旧默认阈值二值化入口 / Legacy default-threshold binaryization entry.
///
/// 该入口保留历史 `condition >= 0` 和默认 Big-M 行为；新代码应使用 [`if_`]。
/// This entry preserves the historical `condition >= 0` and default Big-M
/// behavior; new code should use [`if_`].
pub fn if_legacy<V>(condition: Linear<V>) -> BinaryzationFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    BinaryzationFunction::auto_threshold(condition, V::from_f64(0.0).expect("convert 0.0"))
}

/// 带名称的旧阈值二值化入口 / Named legacy-threshold binaryization entry.
pub fn if_named_legacy<V>(name: impl AsRef<str>, condition: Linear<V>) -> BinaryzationFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    BinaryzationFunction::named_threshold(name, condition, V::from_f64(0.0).expect("convert 0.0"))
}

/// 旧入口的自然命名别名 / Natural-name alias for the legacy entry.
pub fn legacy_if<V>(condition: Linear<V>) -> BinaryzationFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    if_legacy(condition)
}

/// 带名称的旧入口自然命名别名 / Named natural-name alias for the legacy entry.
pub fn legacy_if_named<V>(name: impl AsRef<str>, condition: Linear<V>) -> BinaryzationFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    if_named_legacy(name, condition)
}

/// 显式关系条件便利入口 / Explicit relation-condition convenience entry
pub fn conditional_if<V>(
    condition: Linear<V>,
    relation: ConditionRelation,
    strict_boundary: V,
    bounds: ConditionBounds<V>,
) -> Result<ConditionalIndicatorFunction<V>>
where
    V: Clone + Debug + Send + Sync + 'static + num_traits::ToPrimitive + num_traits::FromPrimitive,
{
    ConditionalIndicatorFunction::auto(condition, relation, strict_boundary, bounds)
}

/// 纯关系描述器便利入口 / Pure relation-descriptor convenience entry
pub fn conditional_descriptor<V>(
    condition: Linear<V>,
    relation: ConditionRelation,
    strict_boundary: V,
    bounds: ConditionBounds<V>,
) -> Result<ConditionalIfFunction<V>>
where
    V: Clone + Debug + Send + Sync + 'static + num_traits::ToPrimitive + num_traits::FromPrimitive,
{
    ConditionalIfFunction::new(condition, relation, strict_boundary, bounds)
}

/// 显式闭区间条件便利入口 / Explicit closed-interval condition convenience entry
pub fn if_in_range<V>(
    lower: ConditionalIfFunction<V>,
    upper: ConditionalIfFunction<V>,
) -> Result<IfInRangeFunction<V>>
where
    V: Clone + Debug + Send + Sync + 'static + num_traits::ToPrimitive + num_traits::FromPrimitive,
{
    IfInRangeFunction::try_new(lower, upper)
}

/// 显式范围关系蕴含函数：`premise => consequence`。
/// Explicit range-relation implication function: `premise => consequence`.
///
/// 两个描述器分别携带 `ConditionRelation`、`strict_boundary` 和有限范围；前件为假
/// 时后件关系约束被门控，因而后件 Undefined 不会使模型不可行。
/// Each descriptor carries its `ConditionRelation`, `strict_boundary`, and finite
/// bounds. Consequent rows are gated by a false premise, so an undefined
/// consequent cannot make the model infeasible.
pub fn imply<V>(
    premise: ConditionalIfFunction<V>,
    consequence: ConditionalIfFunction<V>,
) -> Result<ConditionalImplyFunction<V>>
where
    V: Clone + Debug + Send + Sync + 'static + num_traits::ToPrimitive + FromPrimitive,
{
    ConditionalImplyFunction::auto(premise, consequence)
}

/// 使用显式线性部分创建自动命名安全蕴含 / Create an auto-named safe implication from explicit linear parts.
pub fn imply_from_parts<V>(
    premise: Linear<V>,
    premise_relation: ConditionRelation,
    premise_strict_boundary: V,
    premise_bounds: ConditionBounds<V>,
    consequence: Linear<V>,
    consequence_relation: ConditionRelation,
    consequence_strict_boundary: V,
    consequence_bounds: ConditionBounds<V>,
) -> Result<ConditionalImplyFunction<V>>
where
    V: Clone + Debug + Send + Sync + 'static + num_traits::ToPrimitive + FromPrimitive,
{
    ConditionalImplyFunction::auto(
        ConditionalIfFunction::new(
            premise,
            premise_relation,
            premise_strict_boundary,
            premise_bounds,
        )?,
        ConditionalIfFunction::new(
            consequence,
            consequence_relation,
            consequence_strict_boundary,
            consequence_bounds,
        )?,
    )
}

/// 带名称的显式范围关系蕴含入口 / Named explicit range-relation implication entry.
pub fn imply_named<V>(
    name: impl AsRef<str>,
    premise: ConditionalIfFunction<V>,
    consequence: ConditionalIfFunction<V>,
) -> Result<ConditionalImplyFunction<V>>
where
    V: Clone + Debug + Send + Sync + 'static + num_traits::ToPrimitive + FromPrimitive,
{
    ConditionalImplyFunction::named(name, premise, consequence)
}

/// 显式使用旧 Big-M 蕴含约束的便利入口 / Explicit convenience entry for the legacy Big-M implication constraint
///
/// 该入口保留旧 `ImplyFunction` 的注册形状，`big_m` 必须由调用方明确提供；它不承诺
/// 为不可判定的后件提供门控语义。/ This entry retains the legacy registration shape;
/// the caller must provide `big_m`, and it does not claim to gate an undefined consequence.
pub fn imply_constraint<V>(
    premise: LinearInequality<V>,
    consequence: LinearInequality<V>,
    big_m: V,
) -> ImplyFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    ImplyFunction::auto(premise, consequence, big_m)
}

/// 显式条件值函数便利入口 / Explicit conditional-value function convenience entry
pub fn conditional_then<V>(
    condition: Linear<V>,
    relation: ConditionRelation,
    strict_boundary: V,
    bounds: ConditionBounds<V>,
    then_poly: Linear<V>,
) -> Result<ConditionalThenFunction<V>>
where
    V: Clone + Debug + Send + Sync + 'static + num_traits::ToPrimitive + num_traits::FromPrimitive,
{
    ConditionalThenFunction::from_parts(condition, relation, strict_boundary, bounds, then_poly)
}

/// 条件表达式函数：`condition ? then_expr : else_expr`。
/// Conditional-expression function: `condition ? then_expr : else_expr`.
pub fn if_else<V>(
    condition: BinaryVariableItem,
    then_expr: Linear<V>,
    else_expr: Linear<V>,
) -> IfElseFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    IfElseFunction::auto(condition, then_expr, else_expr)
}

/// 带名称的条件表达式函数便利入口。
/// Named convenience entry for conditional-expression functions.
pub fn if_else_named<V>(
    name: impl AsRef<str>,
    condition: BinaryVariableItem,
    then_expr: Linear<V>,
    else_expr: Linear<V>,
) -> IfElseFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    IfElseFunction::named(name, condition, then_expr, else_expr)
}
