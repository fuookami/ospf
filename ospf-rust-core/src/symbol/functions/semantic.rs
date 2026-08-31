//! 语义化函数符号便利入口。
//! Semantic function-symbol convenience entry points.

use std::fmt::Debug;
use num_traits::FromPrimitive;
use crate::model::LinearInequality;
use crate::symbol::flatten::Linear;
use crate::variable::BinaryVariableItem;
use super::{BinaryzationFunction, IfElseFunction, IfThenFunction};

/// Kotlin 概念对齐的条件函数：当 `condition >= 0` 时结果为 1，否则为 0。
/// Kotlin-concept-aligned condition function: returns 1 when `condition >= 0`, otherwise 0.
pub fn if_<V>(condition: Linear<V>) -> BinaryzationFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    BinaryzationFunction::auto_threshold(condition, V::from_f64(0.0).expect("convert 0.0"))
}

/// 带名称的条件函数便利入口。
/// Named convenience entry for condition functions.
pub fn if_named<V>(name: impl AsRef<str>, condition: Linear<V>) -> BinaryzationFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    BinaryzationFunction::named_threshold(name, condition, V::from_f64(0.0).expect("convert 0.0"))
}

/// Kotlin 概念对齐的蕴含函数：`premise => consequence`。
/// Kotlin-concept-aligned implication function: `premise => consequence`.
pub fn imply<V>(
    premise: LinearInequality<V>,
    consequence: LinearInequality<V>,
    big_m: V,
) -> IfThenFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    IfThenFunction::auto(premise, consequence, big_m)
}

/// 带名称的蕴含函数便利入口。
/// Named convenience entry for implication functions.
pub fn imply_named<V>(
    name: impl AsRef<str>,
    premise: LinearInequality<V>,
    consequence: LinearInequality<V>,
    big_m: V,
) -> IfThenFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    IfThenFunction::named(name, premise, consequence, big_m)
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
