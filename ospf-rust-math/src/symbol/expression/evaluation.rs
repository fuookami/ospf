//! 表达式求值
//! Expression evaluation

use std::collections::HashMap;
use crate::Trivalent;
use super::property_path::PropertyPath;
use super::operators::*;
use super::value::ExpressionValue;
use super::scalar::{ScalarExpression, ParsedScalarExpression};
use super::boolean::{BooleanExpression, ParsedBooleanExpression};
use super::math_functions::{ScalarFunctionEvaluator, DefaultScalarFunctionEvaluator};
use super::property_path_from_owned_symbol;

pub trait EvaluationContext {
    /// 获取指定属性路径的值。
    /// Get value at the specified property path.
    fn get(&self, path: &PropertyPath) -> Option<&ExpressionValue>;

    /// 检查指定属性路径是否存在。
    /// Check whether the specified property path exists.
    fn contains(&self, path: &PropertyPath) -> bool {
        self.get(path).is_some()
    }
}

/// 基于哈希表的运行时表达式求值上下文。
/// HashMap-based runtime expression evaluation context.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MapEvaluationContext {
    values: HashMap<PropertyPath, ExpressionValue>,
}

impl MapEvaluationContext {
    /// 从属性路径映射创建求值上下文。
    /// Create an evaluation context from a property-path map.
    pub fn from_path_map(values: HashMap<PropertyPath, ExpressionValue>) -> Self {
        Self { values }
    }

    /// 从字符串路径映射创建求值上下文。
    /// Create an evaluation context from a string-path map.
    pub fn from_string_map<I, K>(values: I) -> Self
    where
        I: IntoIterator<Item = (K, ExpressionValue)>,
        K: AsRef<str>,
    {
        Self {
            values: values
                .into_iter()
                .map(|(key, value)| (PropertyPath::parse(key.as_ref()), value))
                .collect(),
        }
    }

    /// 插入或替换一个路径值。
    /// Insert or replace a path value.
    pub fn insert(&mut self, path: impl Into<PropertyPath>, value: impl Into<ExpressionValue>) {
        self.values.insert(path.into(), value.into());
    }

    /// 获取底层值映射。
    /// Get underlying value map.
    pub fn values(&self) -> &HashMap<PropertyPath, ExpressionValue> {
        &self.values
    }
}

impl EvaluationContext for MapEvaluationContext {
    fn get(&self, path: &PropertyPath) -> Option<&ExpressionValue> {
        self.values.get(path)
    }
}

impl EvaluationContext for HashMap<PropertyPath, ExpressionValue> {
    fn get(&self, path: &PropertyPath) -> Option<&ExpressionValue> {
        self.get(path)
    }
}

/// 空运行时表达式求值上下文。
/// Empty runtime expression evaluation context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EmptyEvaluationContext;

impl EvaluationContext for EmptyEvaluationContext {
    fn get(&self, _path: &PropertyPath) -> Option<&ExpressionValue> {
        None
    }
}

/// 运行时表达式求值结果。
/// Runtime expression evaluation result.
pub type EvaluationResult = Trivalent;

/// 求值运行时布尔表达式。
/// Evaluate a runtime boolean expression.
pub fn evaluate_boolean(
    expression: &ParsedBooleanExpression,
    context: &impl EvaluationContext,
) -> EvaluationResult {
    match expression {
        BooleanExpression::Constant(value) => *value,
        BooleanExpression::Comparison {
            operator,
            left,
            right,
        } => {
            let Some(left) = evaluate_scalar(left, context) else {
                return Trivalent::Unknown;
            };
            let Some(right) = evaluate_scalar(right, context) else {
                return Trivalent::Unknown;
            };
            Trivalent::from(compare_expression_values(&left, &right, *operator))
        }
        BooleanExpression::In {
            value,
            candidates,
            negated,
        } => {
            let Some(value) = evaluate_scalar(value, context) else {
                return Trivalent::Unknown;
            };
            if value == ExpressionValue::Null {
                return Trivalent::Unknown;
            }

            let contains = candidates.iter().any(|candidate| {
                evaluate_scalar(candidate, context)
                    .filter(|candidate| *candidate != ExpressionValue::Null)
                    .is_some_and(|candidate| expression_values_equal(&value, &candidate))
            });
            Trivalent::from(if *negated { !contains } else { contains })
        }
        BooleanExpression::PatternMatch {
            value,
            pattern,
            mode,
            negated,
        } => {
            let Some(value) = evaluate_scalar(value, context) else {
                return Trivalent::Unknown;
            };
            let Some(pattern) = evaluate_scalar(pattern, context) else {
                return Trivalent::Unknown;
            };
            let Some(matches) = evaluate_pattern_match(&value, &pattern, *mode) else {
                return Trivalent::Unknown;
            };
            Trivalent::from(if *negated { !matches } else { matches })
        }
        BooleanExpression::NullCheck {
            path,
            null_check_type,
        } => {
            if !context.contains(path) {
                return Trivalent::Unknown;
            }
            let is_null = context
                .get(path)
                .is_none_or(|value| *value == ExpressionValue::Null);
            Trivalent::from(match null_check_type {
                NullCheckType::IsNull => is_null,
                NullCheckType::IsNotNull => !is_null,
            })
        }
        BooleanExpression::And(operands) => {
            let mut has_unknown = false;
            for operand in operands {
                match evaluate_boolean(operand, context) {
                    Trivalent::False => return Trivalent::False,
                    Trivalent::Unknown => has_unknown = true,
                    Trivalent::True => {}
                }
            }
            if has_unknown {
                Trivalent::Unknown
            } else {
                Trivalent::True
            }
        }
        BooleanExpression::Or(operands) => {
            let mut has_unknown = false;
            for operand in operands {
                match evaluate_boolean(operand, context) {
                    Trivalent::True => return Trivalent::True,
                    Trivalent::Unknown => has_unknown = true,
                    Trivalent::False => {}
                }
            }
            if has_unknown {
                Trivalent::Unknown
            } else {
                Trivalent::False
            }
        }
        BooleanExpression::Not(operand) => match evaluate_boolean(operand, context) {
            Trivalent::True => Trivalent::False,
            Trivalent::False => Trivalent::True,
            Trivalent::Unknown => Trivalent::Unknown,
        },
        BooleanExpression::Custom { .. } => Trivalent::Unknown,
    }
}

/// 求值运行时布尔表达式（带可注入函数求值器）。
/// Evaluate a runtime boolean expression with an injectable function evaluator.
///
/// 与 `evaluate_boolean` 不同，此函数内部标量求值使用注入的函数求值器，
/// 使 `math.sqrt(x) > 2` 等公式能正确求值。
/// Unlike `evaluate_boolean`, this function uses the injected function evaluator
/// for internal scalar evaluation, enabling formulas like `math.sqrt(x) > 2`
/// to evaluate correctly.
pub fn evaluate_boolean_with_evaluator(
    expression: &ParsedBooleanExpression,
    context: &impl EvaluationContext,
    function_evaluator: &dyn ScalarFunctionEvaluator,
) -> EvaluationResult {
    match expression {
        BooleanExpression::Constant(value) => *value,
        BooleanExpression::Comparison {
            operator,
            left,
            right,
        } => {
            let Some(left) = evaluate_scalar_expression(left, context, function_evaluator) else {
                return Trivalent::Unknown;
            };
            let Some(right) = evaluate_scalar_expression(right, context, function_evaluator) else {
                return Trivalent::Unknown;
            };
            Trivalent::from(compare_expression_values(&left, &right, *operator))
        }
        BooleanExpression::In {
            value,
            candidates,
            negated,
        } => {
            let Some(value) = evaluate_scalar_expression(value, context, function_evaluator) else {
                return Trivalent::Unknown;
            };
            if value == ExpressionValue::Null {
                return Trivalent::Unknown;
            }

            let contains = candidates.iter().any(|candidate| {
                evaluate_scalar_expression(candidate, context, function_evaluator)
                    .filter(|candidate| *candidate != ExpressionValue::Null)
                    .is_some_and(|candidate| expression_values_equal(&value, &candidate))
            });
            Trivalent::from(if *negated { !contains } else { contains })
        }
        BooleanExpression::PatternMatch {
            value,
            pattern,
            mode,
            negated,
        } => {
            let Some(value) = evaluate_scalar_expression(value, context, function_evaluator) else {
                return Trivalent::Unknown;
            };
            let Some(pattern) = evaluate_scalar_expression(pattern, context, function_evaluator) else {
                return Trivalent::Unknown;
            };
            let Some(matches) = evaluate_pattern_match(&value, &pattern, *mode) else {
                return Trivalent::Unknown;
            };
            Trivalent::from(if *negated { !matches } else { matches })
        }
        BooleanExpression::NullCheck {
            path,
            null_check_type,
        } => {
            if !context.contains(path) {
                return Trivalent::Unknown;
            }
            let is_null = context
                .get(path)
                .is_none_or(|value| *value == ExpressionValue::Null);
            Trivalent::from(match null_check_type {
                NullCheckType::IsNull => is_null,
                NullCheckType::IsNotNull => !is_null,
            })
        }
        BooleanExpression::And(operands) => {
            let mut has_unknown = false;
            for operand in operands {
                match evaluate_boolean_with_evaluator(operand, context, function_evaluator) {
                    Trivalent::False => return Trivalent::False,
                    Trivalent::Unknown => has_unknown = true,
                    Trivalent::True => {}
                }
            }
            if has_unknown {
                Trivalent::Unknown
            } else {
                Trivalent::True
            }
        }
        BooleanExpression::Or(operands) => {
            let mut has_unknown = false;
            for operand in operands {
                match evaluate_boolean_with_evaluator(operand, context, function_evaluator) {
                    Trivalent::True => return Trivalent::True,
                    Trivalent::Unknown => has_unknown = true,
                    Trivalent::False => {}
                }
            }
            if has_unknown {
                Trivalent::Unknown
            } else {
                Trivalent::False
            }
        }
        BooleanExpression::Not(operand) => {
            match evaluate_boolean_with_evaluator(operand, context, function_evaluator) {
                Trivalent::True => Trivalent::False,
                Trivalent::False => Trivalent::True,
                Trivalent::Unknown => Trivalent::Unknown,
            }
        }
        BooleanExpression::Custom { .. } => Trivalent::Unknown,
    }
}

/// 求值运行时布尔表达式，并以可空布尔值返回结果。
/// Evaluate a runtime boolean expression and return a nullable boolean result.
pub fn evaluate_boolean_or_none(
    expression: &ParsedBooleanExpression,
    context: &impl EvaluationContext,
) -> Option<bool> {
    evaluate_boolean(expression, context).into()
}

/// 运行时布尔表达式求值扩展。
/// Runtime boolean expression evaluation extension.
pub trait EvaluateBoolean {
    /// 求值表达式。
    /// Evaluate expression.
    fn evaluate_with(&self, context: &impl EvaluationContext) -> Trivalent;

    /// 求值表达式，并以可空布尔值返回结果。
    /// Evaluate expression and return a nullable boolean result.
    fn evaluate_with_or_none(&self, context: &impl EvaluationContext) -> Option<bool>;
}

impl EvaluateBoolean for ParsedBooleanExpression {
    fn evaluate_with(&self, context: &impl EvaluationContext) -> Trivalent {
        evaluate_boolean(self, context)
    }

    fn evaluate_with_or_none(&self, context: &impl EvaluationContext) -> Option<bool> {
        evaluate_boolean_or_none(self, context)
    }
}

fn evaluate_scalar(
    expression: &ParsedScalarExpression,
    context: &impl EvaluationContext,
) -> Option<ExpressionValue> {
    match expression {
        ScalarExpression::Constant(value) => Some(value.clone()),
        ScalarExpression::Reference(path) => context.get(path).cloned(),
        ScalarExpression::SymbolReference(symbol) => {
            property_path_from_owned_symbol(symbol).and_then(|path| context.get(path).cloned())
        }
        ScalarExpression::Unary { operator, operand } => {
            let operand = evaluate_scalar(operand, context)?;
            evaluate_unary_expression_value(*operator, operand)
        }
        ScalarExpression::Binary {
            operator,
            left,
            right,
        } => {
            let left = evaluate_scalar(left, context)?;
            let right = evaluate_scalar(right, context)?;
            evaluate_binary_expression_value(*operator, left, right)
        }
        ScalarExpression::Function { name, arguments } => {
            let arguments = arguments
                .iter()
                .map(|argument| evaluate_scalar(argument, context))
                .collect::<Vec<_>>();
            DefaultScalarFunctionEvaluator.evaluate(name, &arguments)
        }
        ScalarExpression::Conditional {
            condition,
            then_branch,
            else_branch,
        } => match evaluate_boolean(condition, context) {
            Trivalent::True => evaluate_scalar(then_branch, context),
            Trivalent::False => evaluate_scalar(else_branch, context),
            Trivalent::Unknown => None,
        },
        ScalarExpression::Boolean(expr) => match evaluate_boolean(expr, context) {
            Trivalent::True => Some(ExpressionValue::Boolean(true)),
            Trivalent::False => Some(ExpressionValue::Boolean(false)),
            Trivalent::Unknown => None,
        },
        ScalarExpression::Custom { .. } => None,
    }
}

fn evaluate_unary_expression_value(
    operator: UnaryOperator,
    value: ExpressionValue,
) -> Option<ExpressionValue> {
    match operator {
        UnaryOperator::Positive => Some(value),
        UnaryOperator::Negate => Some(ExpressionValue::Number(-expression_number(&value)?)),
        UnaryOperator::Abs => Some(ExpressionValue::Number(expression_number(&value)?.abs())),
    }
}

fn evaluate_binary_expression_value(
    operator: BinaryOperator,
    left: ExpressionValue,
    right: ExpressionValue,
) -> Option<ExpressionValue> {
    let left = expression_number(&left)?;
    let right = expression_number(&right)?;
    let value = match operator {
        BinaryOperator::Add => left + right,
        BinaryOperator::Subtract => left - right,
        BinaryOperator::Multiply => left * right,
        BinaryOperator::Divide => {
            if right == 0.0 {
                return None;
            }
            left / right
        }
        BinaryOperator::Modulo => {
            if right == 0.0 {
                return None;
            }
            left % right
        }
        BinaryOperator::Power => left.powf(right),
    };
    value.is_finite().then_some(ExpressionValue::Number(value))
}

/// 求值运行时标量表达式（公开接口）。
/// Evaluate a runtime scalar expression (public interface).
///
/// 与私有 `evaluate_scalar` 不同，此函数接受可注入的函数求值器，
/// 支持自定义函数求值逻辑（如 `MathFunctionEvaluator`）。
/// Unlike the private `evaluate_scalar`, this function accepts an injectable function evaluator,
/// supporting custom function evaluation logic (e.g., `MathFunctionEvaluator`).
pub fn evaluate_scalar_expression(
    expression: &ParsedScalarExpression,
    context: &impl EvaluationContext,
    function_evaluator: &dyn ScalarFunctionEvaluator,
) -> Option<ExpressionValue> {
    match expression {
        ScalarExpression::Constant(value) => Some(value.clone()),
        ScalarExpression::Reference(path) => context.get(path).cloned(),
        ScalarExpression::SymbolReference(symbol) => {
            property_path_from_owned_symbol(symbol).and_then(|path| context.get(path).cloned())
        }
        ScalarExpression::Unary { operator, operand } => {
            let operand = evaluate_scalar_expression(operand, context, function_evaluator)?;
            evaluate_unary_expression_value(*operator, operand)
        }
        ScalarExpression::Binary {
            operator,
            left,
            right,
        } => {
            let left = evaluate_scalar_expression(left, context, function_evaluator)?;
            let right = evaluate_scalar_expression(right, context, function_evaluator)?;
            evaluate_binary_expression_value(*operator, left, right)
        }
        ScalarExpression::Function { name, arguments } => {
            let evaluated_args = arguments
                .iter()
                .map(|argument| evaluate_scalar_expression(argument, context, function_evaluator))
                .collect::<Vec<_>>();
            function_evaluator.evaluate(name, &evaluated_args)
        }
        ScalarExpression::Conditional {
            condition,
            then_branch,
            else_branch,
        } => match evaluate_boolean_with_evaluator(condition, context, function_evaluator) {
            Trivalent::True => evaluate_scalar_expression(then_branch, context, function_evaluator),
            Trivalent::False => evaluate_scalar_expression(else_branch, context, function_evaluator),
            Trivalent::Unknown => None,
        },
        ScalarExpression::Boolean(expr) => match evaluate_boolean_with_evaluator(expr, context, function_evaluator) {
            Trivalent::True => Some(ExpressionValue::Boolean(true)),
            Trivalent::False => Some(ExpressionValue::Boolean(false)),
            Trivalent::Unknown => None,
        },
        ScalarExpression::Custom { .. } => None,
    }
}

fn expression_number(value: &ExpressionValue) -> Option<f64> {
    match value {
        ExpressionValue::Number(value) if value.is_finite() => Some(*value),
        _ => None,
    }
}

fn compare_expression_values(
    left: &ExpressionValue,
    right: &ExpressionValue,
    operator: ComparisonOperator,
) -> Option<bool> {
    match operator {
        ComparisonOperator::Eq => Some(expression_values_equal(left, right)),
        ComparisonOperator::Ne => Some(!expression_values_equal(left, right)),
        ComparisonOperator::Lt => compare_expression_order(left, right).map(|order| order < 0),
        ComparisonOperator::Le => compare_expression_order(left, right).map(|order| order <= 0),
        ComparisonOperator::Gt => compare_expression_order(left, right).map(|order| order > 0),
        ComparisonOperator::Ge => compare_expression_order(left, right).map(|order| order >= 0),
    }
}

fn compare_expression_order(left: &ExpressionValue, right: &ExpressionValue) -> Option<i8> {
    match (left, right) {
        (ExpressionValue::Number(left), ExpressionValue::Number(right))
            if left.is_finite() && right.is_finite() =>
        {
            left.partial_cmp(right)
                .map(|order| order as i8)
                .or_else(|| Some(0))
        }
        (ExpressionValue::String(left), ExpressionValue::String(right)) => {
            Some(left.cmp(right) as i8)
        }
        (ExpressionValue::Boolean(left), ExpressionValue::Boolean(right)) => {
            Some(left.cmp(right) as i8)
        }
        _ => None,
    }
}

fn expression_values_equal(left: &ExpressionValue, right: &ExpressionValue) -> bool {
    match (left, right) {
        (ExpressionValue::Number(left), ExpressionValue::Number(right)) => left == right,
        _ => left == right,
    }
}

fn evaluate_pattern_match(
    value: &ExpressionValue,
    pattern: &ExpressionValue,
    mode: PatternMatchMode,
) -> Option<bool> {
    if *value == ExpressionValue::Null || *pattern == ExpressionValue::Null {
        return None;
    }
    let value = expression_value_to_match_text(value);
    let pattern = expression_value_to_match_text(pattern);
    match mode {
        PatternMatchMode::Exact => Some(value == pattern),
        PatternMatchMode::Prefix => Some(value.starts_with(&pattern)),
        PatternMatchMode::Suffix => Some(value.ends_with(&pattern)),
        PatternMatchMode::Contains => Some(value.contains(&pattern)),
        PatternMatchMode::Like => Some(match_like(&value, &pattern)),
        PatternMatchMode::Regex => regex::Regex::new(&pattern)
            .ok()
            .map(|regex| regex.is_match(&value)),
    }
}

fn expression_value_to_match_text(value: &ExpressionValue) -> String {
    match value {
        ExpressionValue::Null => String::new(),
        ExpressionValue::Boolean(value) => value.to_string(),
        ExpressionValue::Number(value) => value.to_string(),
        ExpressionValue::String(value) => value.clone(),
    }
}

fn match_like(value: &str, pattern: &str) -> bool {
    let mut regex_pattern = String::from("^");
    for ch in pattern.chars() {
        match ch {
            '%' => regex_pattern.push_str(".*"),
            '_' => regex_pattern.push('.'),
            _ => regex_pattern.push_str(&regex::escape(&ch.to_string())),
        }
    }
    regex_pattern.push('$');
    regex::Regex::new(&regex_pattern)
        .map(|regex| regex.is_match(value))
        .unwrap_or(false)
}
