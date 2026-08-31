//! 布尔表达式规范化与结构键
//! Boolean expression normalization and structural keys

use std::collections::HashSet;
use std::fmt::Display;
use crate::Trivalent;
use super::property_path::PropertyPath;
use super::operators::*;
use super::value::ExpressionValue;
use super::scalar::{ScalarExpression, ParsedScalarExpression};
use super::boolean::{BooleanExpression, ParsedBooleanExpression};

#[derive(Debug, Clone, Copy)]
pub struct NormalizeConfig {
    /// 是否扁平化 And/Or。
    /// Whether to flatten And/Or.
    pub flatten: bool,
    /// 是否进行常量折叠。
    /// Whether to perform constant folding.
    pub constant_folding: bool,
    /// 是否去重。
    /// Whether to deduplicate.
    pub deduplicate: bool,
    /// 是否消除双重否定。
    /// Whether to eliminate double negation.
    pub eliminate_double_negation: bool,
    /// 是否应用德摩根定律。
    /// Whether to apply De Morgan's laws.
    pub apply_de_morgan: bool,
    /// 是否按结构键排序操作数。
    /// Whether to sort operands by structural key.
    pub sort_operands: bool,
}

impl Default for NormalizeConfig {
    fn default() -> Self {
        Self {
            flatten: true,
            constant_folding: true,
            deduplicate: true,
            eliminate_double_negation: true,
            apply_de_morgan: false,
            sort_operands: false,
        }
    }
}

/// 规范化布尔表达式。
/// Normalize boolean expression.
pub fn normalize_boolean_expression<T>(
    expression: &BooleanExpression<T>,
    config: NormalizeConfig,
) -> BooleanExpression<T>
where
    T: Clone + Display,
{
    let mut result = expression.clone();
    if config.flatten {
        result = flatten_boolean_expression(&result);
    }
    if config.eliminate_double_negation {
        result = eliminate_double_negation(&result);
    }
    if config.apply_de_morgan {
        result = apply_de_morgan(&result);
    }
    if config.constant_folding {
        result = constant_fold_boolean_expression(&result);
    }
    if config.deduplicate {
        result = deduplicate_boolean_expression(&result);
    }
    if config.sort_operands {
        result = sort_boolean_operands(&result);
    }
    result = normalize_boolean_children(&result, config);
    simplify_single_boolean_operand(&result)
}

/// 扁平化布尔表达式中的 And/Or。
/// Flatten And/Or in a boolean expression.
pub fn flatten_boolean_expression<T>(expression: &BooleanExpression<T>) -> BooleanExpression<T>
where
    T: Clone,
{
    match expression {
        BooleanExpression::And(operands) => {
            let mut flattened = Vec::new();
            for operand in operands {
                match flatten_boolean_expression(operand) {
                    BooleanExpression::And(items) => flattened.extend(items),
                    item => flattened.push(item),
                }
            }
            BooleanExpression::And(flattened)
        }
        BooleanExpression::Or(operands) => {
            let mut flattened = Vec::new();
            for operand in operands {
                match flatten_boolean_expression(operand) {
                    BooleanExpression::Or(items) => flattened.extend(items),
                    item => flattened.push(item),
                }
            }
            BooleanExpression::Or(flattened)
        }
        BooleanExpression::Not(operand) => {
            BooleanExpression::Not(Box::new(flatten_boolean_expression(operand)))
        }
        _ => expression.clone(),
    }
}

/// 常量折叠布尔表达式。
/// Constant-fold boolean expression.
pub fn constant_fold_boolean_expression<T>(
    expression: &BooleanExpression<T>,
) -> BooleanExpression<T>
where
    T: Clone,
{
    match expression {
        BooleanExpression::Constant(_) => expression.clone(),
        BooleanExpression::And(operands) => {
            let operands = operands
                .iter()
                .map(constant_fold_boolean_expression)
                .collect::<Vec<_>>();
            if operands
                .iter()
                .any(|operand| matches!(operand, BooleanExpression::Constant(Trivalent::False)))
            {
                return BooleanExpression::Constant(Trivalent::False);
            }
            let filtered = operands
                .into_iter()
                .filter(|operand| !matches!(operand, BooleanExpression::Constant(Trivalent::True)))
                .collect::<Vec<_>>();
            match filtered.len() {
                0 => BooleanExpression::Constant(Trivalent::True),
                1 => filtered.into_iter().next().expect("filtered has exactly one element / filtered 恰好有一个元素"),
                _ => BooleanExpression::And(filtered),
            }
        }
        BooleanExpression::Or(operands) => {
            let operands = operands
                .iter()
                .map(constant_fold_boolean_expression)
                .collect::<Vec<_>>();
            if operands
                .iter()
                .any(|operand| matches!(operand, BooleanExpression::Constant(Trivalent::True)))
            {
                return BooleanExpression::Constant(Trivalent::True);
            }
            let filtered = operands
                .into_iter()
                .filter(|operand| !matches!(operand, BooleanExpression::Constant(Trivalent::False)))
                .collect::<Vec<_>>();
            match filtered.len() {
                0 => BooleanExpression::Constant(Trivalent::False),
                1 => filtered.into_iter().next().expect("filtered has exactly one element / filtered 恰好有一个元素"),
                _ => BooleanExpression::Or(filtered),
            }
        }
        BooleanExpression::Not(operand) => match constant_fold_boolean_expression(operand) {
            BooleanExpression::Constant(value) => BooleanExpression::Constant(match value {
                Trivalent::True => Trivalent::False,
                Trivalent::False => Trivalent::True,
                Trivalent::Unknown => Trivalent::Unknown,
            }),
            operand => BooleanExpression::Not(Box::new(operand)),
        },
        _ => expression.clone(),
    }
}

/// 对布尔表达式操作数去重。
/// Deduplicate boolean expression operands.
pub fn deduplicate_boolean_expression<T>(expression: &BooleanExpression<T>) -> BooleanExpression<T>
where
    T: Clone + Display,
{
    match expression {
        BooleanExpression::And(operands) => {
            BooleanExpression::And(deduplicate_boolean_operands(operands))
        }
        BooleanExpression::Or(operands) => {
            BooleanExpression::Or(deduplicate_boolean_operands(operands))
        }
        BooleanExpression::Not(operand) => {
            BooleanExpression::Not(Box::new(deduplicate_boolean_expression(operand)))
        }
        _ => expression.clone(),
    }
}

/// 消除布尔表达式中的双重否定。
/// Eliminate double negation in a boolean expression.
pub fn eliminate_double_negation<T>(expression: &BooleanExpression<T>) -> BooleanExpression<T>
where
    T: Clone,
{
    match expression {
        BooleanExpression::Not(operand) => match eliminate_double_negation(operand) {
            BooleanExpression::Not(inner) => *inner,
            operand => BooleanExpression::Not(Box::new(operand)),
        },
        BooleanExpression::And(operands) => {
            BooleanExpression::And(operands.iter().map(eliminate_double_negation).collect())
        }
        BooleanExpression::Or(operands) => {
            BooleanExpression::Or(operands.iter().map(eliminate_double_negation).collect())
        }
        _ => expression.clone(),
    }
}

/// 对布尔表达式应用德摩根定律。
/// Apply De Morgan's laws to a boolean expression.
pub fn apply_de_morgan<T>(expression: &BooleanExpression<T>) -> BooleanExpression<T>
where
    T: Clone,
{
    match expression {
        BooleanExpression::Not(operand) => match apply_de_morgan(operand) {
            BooleanExpression::And(operands) => BooleanExpression::Or(
                operands
                    .into_iter()
                    .map(|operand| BooleanExpression::Not(Box::new(operand)))
                    .collect(),
            ),
            BooleanExpression::Or(operands) => BooleanExpression::And(
                operands
                    .into_iter()
                    .map(|operand| BooleanExpression::Not(Box::new(operand)))
                    .collect(),
            ),
            operand => BooleanExpression::Not(Box::new(operand)),
        },
        BooleanExpression::And(operands) => {
            BooleanExpression::And(operands.iter().map(apply_de_morgan).collect())
        }
        BooleanExpression::Or(operands) => {
            BooleanExpression::Or(operands.iter().map(apply_de_morgan).collect())
        }
        _ => expression.clone(),
    }
}

/// 按结构键排序布尔表达式操作数。
/// Sort boolean expression operands by structural key.
pub fn sort_boolean_operands<T>(expression: &BooleanExpression<T>) -> BooleanExpression<T>
where
    T: Clone + Display,
{
    match expression {
        BooleanExpression::And(operands) => {
            let mut operands = operands
                .iter()
                .map(sort_boolean_operands)
                .collect::<Vec<_>>();
            operands.sort_by_key(boolean_structural_key);
            BooleanExpression::And(operands)
        }
        BooleanExpression::Or(operands) => {
            let mut operands = operands
                .iter()
                .map(sort_boolean_operands)
                .collect::<Vec<_>>();
            operands.sort_by_key(boolean_structural_key);
            BooleanExpression::Or(operands)
        }
        BooleanExpression::Not(operand) => {
            BooleanExpression::Not(Box::new(sort_boolean_operands(operand)))
        }
        _ => expression.clone(),
    }
}

fn deduplicate_boolean_operands<T>(operands: &[BooleanExpression<T>]) -> Vec<BooleanExpression<T>>
where
    T: Clone + Display,
{
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for operand in operands {
        let operand = deduplicate_boolean_expression(operand);
        if seen.insert(boolean_structural_key(&operand)) {
            result.push(operand);
        }
    }
    result
}

fn normalize_boolean_children<T>(
    expression: &BooleanExpression<T>,
    config: NormalizeConfig,
) -> BooleanExpression<T>
where
    T: Clone + Display,
{
    match expression {
        BooleanExpression::And(operands) => BooleanExpression::And(
            operands
                .iter()
                .map(|operand| normalize_boolean_expression(operand, config))
                .collect(),
        ),
        BooleanExpression::Or(operands) => BooleanExpression::Or(
            operands
                .iter()
                .map(|operand| normalize_boolean_expression(operand, config))
                .collect(),
        ),
        BooleanExpression::Not(operand) => {
            BooleanExpression::Not(Box::new(normalize_boolean_expression(operand, config)))
        }
        _ => expression.clone(),
    }
}

fn simplify_single_boolean_operand<T>(expression: &BooleanExpression<T>) -> BooleanExpression<T>
where
    T: Clone,
{
    match expression {
        BooleanExpression::And(operands) => match operands.as_slice() {
            [] => BooleanExpression::Constant(Trivalent::True),
            [operand] => simplify_single_boolean_operand(operand),
            _ => BooleanExpression::And(
                operands
                    .iter()
                    .map(simplify_single_boolean_operand)
                    .collect(),
            ),
        },
        BooleanExpression::Or(operands) => match operands.as_slice() {
            [] => BooleanExpression::Constant(Trivalent::False),
            [operand] => simplify_single_boolean_operand(operand),
            _ => BooleanExpression::Or(
                operands
                    .iter()
                    .map(simplify_single_boolean_operand)
                    .collect(),
            ),
        },
        BooleanExpression::Not(operand) => {
            BooleanExpression::Not(Box::new(simplify_single_boolean_operand(operand)))
        }
        _ => expression.clone(),
    }
}

pub(super) fn boolean_structural_key<T>(expression: &BooleanExpression<T>) -> String
where
    T: Display,
{
    match expression {
        BooleanExpression::Constant(value) => format!("Const:{value:?}"),
        BooleanExpression::Comparison {
            operator,
            left,
            right,
        } => format!(
            "Cmp:{operator:?}:{}:{}",
            scalar_structural_key(left),
            scalar_structural_key(right)
        ),
        BooleanExpression::In {
            value,
            candidates,
            negated,
        } => format!(
            "In:{negated}:{}:{}",
            scalar_structural_key(value),
            candidates
                .iter()
                .map(scalar_structural_key)
                .collect::<Vec<_>>()
                .join(",")
        ),
        BooleanExpression::PatternMatch {
            value,
            pattern,
            mode,
            negated,
        } => format!(
            "Match:{mode:?}:{negated}:{}:{}",
            scalar_structural_key(value),
            scalar_structural_key(pattern)
        ),
        BooleanExpression::NullCheck {
            path,
            null_check_type,
        } => format!("Null:{null_check_type:?}:{path}"),
        BooleanExpression::And(operands) => format!(
            "And:{}",
            operands
                .iter()
                .map(boolean_structural_key)
                .collect::<Vec<_>>()
                .join(",")
        ),
        BooleanExpression::Or(operands) => format!(
            "Or:{}",
            operands
                .iter()
                .map(boolean_structural_key)
                .collect::<Vec<_>>()
                .join(",")
        ),
        BooleanExpression::Not(operand) => format!("Not:{}", boolean_structural_key(operand)),
        BooleanExpression::Custom {
            payload,
            description,
        } => format!("Custom:{}", description.as_deref().unwrap_or(payload)),
    }
}

pub(super) fn scalar_structural_key<T>(expression: &ScalarExpression<T>) -> String
where
    T: Display,
{
    match expression {
        ScalarExpression::Constant(value) => format!("Const:{value}"),
        ScalarExpression::Reference(path) => format!("Ref:{path}"),
        ScalarExpression::SymbolReference(symbol) => format!("SymRef:{}", symbol.name()),
        ScalarExpression::Unary { operator, operand } => {
            format!("Unary:{operator:?}:{}", scalar_structural_key(operand))
        }
        ScalarExpression::Binary {
            operator,
            left,
            right,
        } => format!(
            "Bin:{operator:?}:{}:{}",
            scalar_structural_key(left),
            scalar_structural_key(right)
        ),
        ScalarExpression::Function { name, arguments } => format!(
            "Func:{name}:{}",
            arguments
                .iter()
                .map(scalar_structural_key)
                .collect::<Vec<_>>()
                .join(",")
        ),
        ScalarExpression::Custom {
            payload,
            description,
        } => format!("Custom:{}", description.as_deref().unwrap_or(payload)),
        ScalarExpression::Conditional {
            condition,
            then_branch,
            else_branch,
        } => format!(
            "Cond:{}:{}:{}",
            boolean_structural_key(condition),
            scalar_structural_key(then_branch),
            scalar_structural_key(else_branch)
        ),
        ScalarExpression::Boolean(expr) => format!("Bool:{}", boolean_structural_key(expr)),
    }
}
