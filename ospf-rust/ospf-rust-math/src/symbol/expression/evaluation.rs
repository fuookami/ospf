//! 表达式求值
//! Expression evaluation

use super::boolean::{BooleanExpression, ParsedBooleanExpression};
use super::math_functions::{DefaultScalarFunctionEvaluator, ScalarFunctionEvaluator};
use super::operators::*;
use super::property_path::PropertyPath;
use super::property_path_from_owned_symbol;
use super::scalar::{ParsedScalarExpression, ScalarExpression};
use super::value::ExpressionValue;
use crate::Trivalent;
use std::collections::HashMap;

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
            let Some(pattern) = evaluate_scalar_expression(pattern, context, function_evaluator)
            else {
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
            Trivalent::False => {
                evaluate_scalar_expression(else_branch, context, function_evaluator)
            }
            Trivalent::Unknown => None,
        },
        ScalarExpression::Boolean(expr) => {
            match evaluate_boolean_with_evaluator(expr, context, function_evaluator) {
                Trivalent::True => Some(ExpressionValue::Boolean(true)),
                Trivalent::False => Some(ExpressionValue::Boolean(false)),
                Trivalent::Unknown => None,
            }
        }
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

// ============================================================================
// 表达式求值测试 / Expression evaluation tests
// ============================================================================

#[cfg(test)]
mod tests {
    use crate::symbol::OwnedSymbol;
    use crate::symbol::expression::{MathFunctionEvaluator, path_owned_symbol};

    use super::*;

    /// 构造统一的运行时求值上下文。
    /// Build the shared runtime evaluation context.
    fn context() -> MapEvaluationContext {
        MapEvaluationContext::from_string_map([
            ("age", ExpressionValue::from(20)),
            ("status", ExpressionValue::from("active")),
            ("name", ExpressionValue::from("Alice")),
            ("delta", ExpressionValue::from(-3)),
        ])
    }

    /// 构造 `path <op> value` 运行时常量比较。
    /// Build the `path <op> value` runtime constant comparison.
    fn comparison(
        path: &str,
        operator: ComparisonOperator,
        value: ExpressionValue,
    ) -> ParsedBooleanExpression {
        BooleanExpression::comparison(
            operator,
            ScalarExpression::reference(path),
            ScalarExpression::constant(value),
        )
    }

    fn number(value: f64) -> ParsedScalarExpression {
        ScalarExpression::constant(ExpressionValue::Number(value))
    }

    fn reference(path: &str) -> ParsedScalarExpression {
        ScalarExpression::reference(path)
    }

    /// 使用默认函数求值器求值标量表达式。
    /// Evaluate a scalar expression with the default function evaluator.
    fn evaluate(
        expression: &ParsedScalarExpression,
        context: &MapEvaluationContext,
    ) -> Option<ExpressionValue> {
        evaluate_scalar_expression(expression, context, &DefaultScalarFunctionEvaluator)
    }

    // ========================================================================
    // 求值上下文 / Evaluation context
    // ========================================================================

    #[test]
    fn map_context_from_string_map_parses_and_trims_paths() {
        let context = MapEvaluationContext::from_string_map([
            ("  age  ", ExpressionValue::from(20)),
            ("user.name", ExpressionValue::from("Alice")),
        ]);

        assert_eq!(
            context.get(&PropertyPath::parse("age")),
            Some(&ExpressionValue::Number(20.0))
        );
        assert_eq!(
            context.get(&PropertyPath::parse("user.name")),
            Some(&ExpressionValue::String("Alice".to_string()))
        );
        assert!(context.contains(&PropertyPath::parse("age")));
        assert!(!context.contains(&PropertyPath::parse("missing")));
    }

    #[test]
    fn map_context_from_path_map_and_insert_agree() {
        let mut context = MapEvaluationContext::from_path_map(HashMap::from([(
            PropertyPath::parse("age"),
            ExpressionValue::Number(20.0),
        )]));
        context.insert("status", "active");
        context.insert(PropertyPath::parse("age"), ExpressionValue::Number(21.0));

        assert_eq!(context.values().len(), 2);
        assert_eq!(
            context.get(&PropertyPath::parse("age")),
            Some(&ExpressionValue::Number(21.0)),
            "重复插入覆盖旧值 / Re-inserting overwrites the previous value"
        );
        assert_eq!(
            context.get(&PropertyPath::parse("status")),
            Some(&ExpressionValue::String("active".to_string()))
        );
    }

    #[test]
    fn map_context_default_is_empty() {
        let context = MapEvaluationContext::default();

        assert!(context.values().is_empty());
        assert!(!context.contains(&PropertyPath::parse("age")));
        assert_eq!(context.get(&PropertyPath::parse("age")), None);
    }

    #[test]
    fn empty_context_returns_none_for_every_path() {
        let context = EmptyEvaluationContext;

        assert_eq!(context.get(&PropertyPath::parse("age")), None);
        assert!(!context.contains(&PropertyPath::parse("age")));
        assert_eq!(
            evaluate_boolean(&comparison("age", ComparisonOperator::Eq, 20.into()), &context),
            Trivalent::Unknown
        );
    }

    #[test]
    fn hash_map_implements_evaluation_context() {
        let mut raw: HashMap<PropertyPath, ExpressionValue> = HashMap::new();
        raw.insert(PropertyPath::parse("age"), ExpressionValue::from(20));

        assert_eq!(
            evaluate_boolean(&comparison("age", ComparisonOperator::Gt, 18.into()), &raw),
            Trivalent::True
        );
    }

    // ========================================================================
    // 比较求值 / Comparison evaluation
    // ========================================================================

    #[test]
    fn evaluate_comparison_equality_and_inequality() {
        let context = context();

        assert_eq!(
            evaluate_boolean(
                &comparison("status", ComparisonOperator::Eq, "active".into()),
                &context
            ),
            Trivalent::True
        );
        assert_eq!(
            evaluate_boolean(
                &comparison("status", ComparisonOperator::Eq, "inactive".into()),
                &context
            ),
            Trivalent::False
        );
        assert_eq!(
            evaluate_boolean(
                &comparison("status", ComparisonOperator::Ne, "inactive".into()),
                &context
            ),
            Trivalent::True
        );
        assert_eq!(
            evaluate_boolean(
                &comparison("status", ComparisonOperator::Ne, "active".into()),
                &context
            ),
            Trivalent::False
        );
    }

    #[test]
    fn evaluate_comparison_orders_numbers() {
        let context = context();

        assert_eq!(
            evaluate_boolean(
                &comparison("age", ComparisonOperator::Gt, 18.into()),
                &context
            ),
            Trivalent::True
        );
        assert_eq!(
            evaluate_boolean(
                &comparison("age", ComparisonOperator::Ge, 20.into()),
                &context
            ),
            Trivalent::True
        );
        assert_eq!(
            evaluate_boolean(
                &comparison("age", ComparisonOperator::Lt, 18.into()),
                &context
            ),
            Trivalent::False
        );
        assert_eq!(
            evaluate_boolean(
                &comparison("age", ComparisonOperator::Le, 19.into()),
                &context
            ),
            Trivalent::False
        );
    }

    #[test]
    fn evaluate_comparison_orders_strings_and_booleans() {
        let flag_context =
            MapEvaluationContext::from_string_map([("flag", ExpressionValue::from(true))]);
        let text_context = context();

        assert_eq!(
            evaluate_boolean(
                &comparison("name", ComparisonOperator::Lt, "Bob".into()),
                &text_context
            ),
            Trivalent::True,
            "字符串按字典序比较 / Strings compare lexicographically"
        );
        assert_eq!(
            evaluate_boolean(
                &comparison("flag", ComparisonOperator::Gt, false.into()),
                &flag_context
            ),
            Trivalent::True,
            "false 小于 true / false sorts before true"
        );
        assert_eq!(
            evaluate_boolean(
                &comparison("flag", ComparisonOperator::Eq, true.into()),
                &flag_context
            ),
            Trivalent::True
        );
    }

    #[test]
    fn evaluate_comparison_returns_unknown_for_missing_references() {
        let context = context();

        assert_eq!(
            evaluate_boolean(
                &comparison("missing", ComparisonOperator::Eq, 1.into()),
                &context
            ),
            Trivalent::Unknown
        );
        assert_eq!(
            evaluate_boolean(
                &comparison("missing", ComparisonOperator::Gt, 1.into()),
                &context
            ),
            Trivalent::Unknown
        );
    }

    #[test]
    fn evaluate_comparison_returns_unknown_for_incomparable_types() {
        let context =
            MapEvaluationContext::from_string_map([("payload", ExpressionValue::from("text"))]);

        assert_eq!(
            evaluate_boolean(
                &comparison("payload", ComparisonOperator::Le, 1.into()),
                &context
            ),
            Trivalent::Unknown,
            "排序比较在类型不匹配时返回 Unknown / Ordering comparisons return Unknown on type mismatch"
        );
        assert_eq!(
            evaluate_boolean(
                &comparison("payload", ComparisonOperator::Ge, true.into()),
                &context
            ),
            Trivalent::Unknown
        );
    }

    #[test]
    fn evaluate_equality_between_different_types_is_false_not_unknown() {
        let context =
            MapEvaluationContext::from_string_map([("payload", ExpressionValue::from("1"))]);

        // 相等性比较不做隐式类型转换，直接判定为不相等。
        // Equality does not coerce types and simply reports inequality.
        assert_eq!(
            evaluate_boolean(
                &comparison("payload", ComparisonOperator::Eq, 1.into()),
                &context
            ),
            Trivalent::False
        );
        assert_eq!(
            evaluate_boolean(
                &comparison("payload", ComparisonOperator::Ne, 1.into()),
                &context
            ),
            Trivalent::True
        );
    }

    #[test]
    fn evaluate_comparison_reads_nested_paths() {
        let context =
            MapEvaluationContext::from_string_map([("user.address.city", "Beijing".into())]);

        assert_eq!(
            evaluate_boolean(
                &comparison("user.address.city", ComparisonOperator::Eq, "Beijing".into()),
                &context
            ),
            Trivalent::True
        );
    }

    #[test]
    fn evaluate_comparison_resolves_path_symbol_references() {
        let context = context();
        let expression = BooleanExpression::eq(
            ScalarExpression::symbol_reference(path_owned_symbol("age")),
            ScalarExpression::constant(ExpressionValue::from(20)),
        );
        let foreign = BooleanExpression::eq(
            ScalarExpression::symbol_reference(OwnedSymbol::new(
                crate::symbol::test_utils::SimpleSymbol::new("age"),
            )),
            ScalarExpression::constant(ExpressionValue::from(20)),
        );

        assert_eq!(evaluate_boolean(&expression, &context), Trivalent::True);
        assert_eq!(
            evaluate_boolean(&foreign, &context),
            Trivalent::Unknown,
            "无路径的符号引用无法解析 / A symbol without a path cannot be resolved"
        );
    }

    #[test]
    fn evaluate_comparison_with_non_finite_values_returns_unknown() {
        let context = MapEvaluationContext::from_string_map([(
            "value",
            ExpressionValue::Number(f64::INFINITY),
        )]);

        assert_eq!(
            evaluate_boolean(
                &comparison("value", ComparisonOperator::Lt, 1.into()),
                &context
            ),
            Trivalent::Unknown
        );
        assert_eq!(
            evaluate_boolean(
                &comparison("value", ComparisonOperator::Eq, f64::INFINITY.into()),
                &context
            ),
            Trivalent::True,
            "相等性比较不要求有限值 / Equality does not require finite values"
        );
    }

    // ========================================================================
    // 集合成员求值 / Set membership evaluation
    // ========================================================================

    #[test]
    fn evaluate_in_matches_any_candidate() {
        let context = context();
        let expression = BooleanExpression::in_expr(
            ScalarExpression::reference("status"),
            vec![
                ScalarExpression::constant(ExpressionValue::from("pending")),
                ScalarExpression::constant(ExpressionValue::from("active")),
            ],
            false,
        );

        assert_eq!(evaluate_boolean(&expression, &context), Trivalent::True);

        let no_match = BooleanExpression::in_expr(
            ScalarExpression::reference("status"),
            vec![ScalarExpression::constant(ExpressionValue::from("deleted"))],
            false,
        );
        assert_eq!(evaluate_boolean(&no_match, &context), Trivalent::False);
    }

    #[test]
    fn evaluate_not_in_negates_the_match() {
        let context = context();
        let expression = BooleanExpression::in_expr(
            ScalarExpression::reference("status"),
            vec![ScalarExpression::constant(ExpressionValue::from("archived"))],
            true,
        );

        assert_eq!(evaluate_boolean(&expression, &context), Trivalent::True);

        let negated_match = BooleanExpression::in_expr(
            ScalarExpression::reference("status"),
            vec![ScalarExpression::constant(ExpressionValue::from("active"))],
            true,
        );
        assert_eq!(evaluate_boolean(&negated_match, &context), Trivalent::False);
    }

    #[test]
    fn evaluate_in_with_null_value_returns_unknown() {
        let context =
            MapEvaluationContext::from_string_map([("status", ExpressionValue::Null)]);
        let expression = BooleanExpression::in_expr(
            ScalarExpression::reference("status"),
            vec![ScalarExpression::constant(ExpressionValue::Null)],
            false,
        );

        assert_eq!(evaluate_boolean(&expression, &context), Trivalent::Unknown);
    }

    #[test]
    fn evaluate_in_skips_null_and_unresolved_candidates() {
        let context = context();
        let expression = BooleanExpression::in_expr(
            ScalarExpression::reference("status"),
            vec![
                ScalarExpression::constant(ExpressionValue::Null),
                ScalarExpression::reference("missing"),
                ScalarExpression::constant(ExpressionValue::from("active")),
            ],
            false,
        );

        assert_eq!(evaluate_boolean(&expression, &context), Trivalent::True);
    }

    #[test]
    fn evaluate_in_without_resolvable_candidates_is_false() {
        let context = context();
        let expression = BooleanExpression::in_expr(
            ScalarExpression::reference("status"),
            vec![
                ScalarExpression::constant(ExpressionValue::Null),
                ScalarExpression::reference("missing"),
            ],
            false,
        );

        assert_eq!(evaluate_boolean(&expression, &context), Trivalent::False);
        let negated = BooleanExpression::in_expr(
            ScalarExpression::reference("status"),
            vec![ScalarExpression::constant(ExpressionValue::Null)],
            true,
        );
        assert_eq!(evaluate_boolean(&negated, &context), Trivalent::True);
    }

    #[test]
    fn evaluate_in_does_not_coerce_numbers_to_strings() {
        let context =
            MapEvaluationContext::from_string_map([("code", ExpressionValue::from(1))]);
        let expression = BooleanExpression::in_expr(
            ScalarExpression::reference("code"),
            vec![ScalarExpression::constant(ExpressionValue::from("1"))],
            false,
        );

        assert_eq!(evaluate_boolean(&expression, &context), Trivalent::False);
    }

    // ========================================================================
    // 模式匹配求值 / Pattern match evaluation
    // ========================================================================

    #[test]
    fn evaluate_pattern_match_modes() {
        let context = context();
        let modes = [
            (PatternMatchMode::Exact, "Alice", true),
            (PatternMatchMode::Exact, "Alice2", false),
            (PatternMatchMode::Prefix, "Al", true),
            (PatternMatchMode::Prefix, "ce", false),
            (PatternMatchMode::Suffix, "ce", true),
            (PatternMatchMode::Suffix, "Al", false),
            (PatternMatchMode::Contains, "lic", true),
            (PatternMatchMode::Contains, "xyz", false),
        ];

        for (mode, pattern, expected) in modes {
            let expression = BooleanExpression::pattern_match(
                ScalarExpression::reference("name"),
                ScalarExpression::constant(ExpressionValue::from(pattern)),
                mode,
                false,
            );
            assert_eq!(
                evaluate_boolean(&expression, &context),
                Trivalent::from(expected),
                "unexpected result for {mode:?} with pattern {pattern}"
            );
        }
    }

    #[test]
    fn evaluate_pattern_match_negation_inverts_the_result() {
        let context = context();
        let expression = BooleanExpression::pattern_match(
            ScalarExpression::reference("name"),
            ScalarExpression::constant(ExpressionValue::from("Alice")),
            PatternMatchMode::Exact,
            true,
        );

        assert_eq!(evaluate_boolean(&expression, &context), Trivalent::False);
    }

    #[test]
    fn evaluate_like_pattern_wildcards() {
        let context = context();
        let patterns = [("A%e", true), ("_lice", true), ("B%", false)];

        for (pattern, expected) in patterns {
            let expression = BooleanExpression::pattern_match(
                ScalarExpression::reference("name"),
                ScalarExpression::constant(ExpressionValue::from(pattern)),
                PatternMatchMode::Like,
                false,
            );
            assert_eq!(
                evaluate_boolean(&expression, &context),
                Trivalent::from(expected),
                "unexpected LIKE result for {pattern}"
            );
        }
    }

    #[test]
    fn evaluate_like_escapes_regex_metacharacters() {
        let context =
            MapEvaluationContext::from_string_map([("name", ExpressionValue::from("AxB"))]);
        let expression = BooleanExpression::pattern_match(
            ScalarExpression::reference("name"),
            ScalarExpression::constant(ExpressionValue::from("A.B")),
            PatternMatchMode::Like,
            false,
        );

        assert_eq!(
            evaluate_boolean(&expression, &context),
            Trivalent::False,
            "LIKE 中的点号应按字面匹配 / A dot in LIKE matches literally"
        );
    }

    #[test]
    fn evaluate_regex_pattern_match() {
        let context =
            MapEvaluationContext::from_string_map([("code", ExpressionValue::from("A12"))]);
        let expression = BooleanExpression::pattern_match(
            ScalarExpression::reference("code"),
            ScalarExpression::constant(ExpressionValue::from("^A[0-9]+$")),
            PatternMatchMode::Regex,
            false,
        );

        assert_eq!(evaluate_boolean(&expression, &context), Trivalent::True);
    }

    #[test]
    fn evaluate_pattern_match_with_invalid_regex_returns_unknown() {
        let context = context();
        let expression = BooleanExpression::pattern_match(
            ScalarExpression::reference("name"),
            ScalarExpression::constant(ExpressionValue::from("[")),
            PatternMatchMode::Regex,
            false,
        );

        assert_eq!(evaluate_boolean(&expression, &context), Trivalent::Unknown);
    }

    #[test]
    fn evaluate_pattern_match_with_null_operand_returns_unknown() {
        let context = MapEvaluationContext::from_string_map([("name", ExpressionValue::Null)]);

        let null_value = BooleanExpression::pattern_match(
            ScalarExpression::reference("name"),
            ScalarExpression::constant(ExpressionValue::from("Alice")),
            PatternMatchMode::Exact,
            false,
        );
        let null_pattern = BooleanExpression::pattern_match(
            ScalarExpression::reference("name"),
            ScalarExpression::constant(ExpressionValue::Null),
            PatternMatchMode::Exact,
            false,
        );

        assert_eq!(evaluate_boolean(&null_value, &context), Trivalent::Unknown);
        assert_eq!(evaluate_boolean(&null_pattern, &context), Trivalent::Unknown);
    }

    #[test]
    fn evaluate_pattern_match_converts_numbers_to_text() {
        let context =
            MapEvaluationContext::from_string_map([("code", ExpressionValue::from(18))]);
        let expression = BooleanExpression::pattern_match(
            ScalarExpression::reference("code"),
            ScalarExpression::constant(ExpressionValue::from("18")),
            PatternMatchMode::Exact,
            false,
        );

        assert_eq!(evaluate_boolean(&expression, &context), Trivalent::True);
    }

    // ========================================================================
    // 空值检查求值 / Null check evaluation
    // ========================================================================

    #[test]
    fn evaluate_null_check_expressions() {
        let context = MapEvaluationContext::from_string_map([
            ("email", ExpressionValue::Null),
            ("name", ExpressionValue::from("Alice")),
        ]);

        assert_eq!(
            evaluate_boolean(&BooleanExpression::is_null("email"), &context),
            Trivalent::True
        );
        assert_eq!(
            evaluate_boolean(&BooleanExpression::is_not_null("email"), &context),
            Trivalent::False
        );
        assert_eq!(
            evaluate_boolean(&BooleanExpression::is_null("name"), &context),
            Trivalent::False
        );
        assert_eq!(
            evaluate_boolean(&BooleanExpression::is_not_null("name"), &context),
            Trivalent::True
        );
    }

    #[test]
    fn evaluate_null_check_on_missing_path_returns_unknown() {
        let context = MapEvaluationContext::from_string_map([("name", ExpressionValue::from("Alice"))]);

        assert_eq!(
            evaluate_boolean(&BooleanExpression::is_null("email"), &context),
            Trivalent::Unknown,
            "缺失路径与显式 null 必须区分 / A missing path differs from an explicit null"
        );
        assert_eq!(
            evaluate_boolean(&BooleanExpression::is_not_null("email"), &context),
            Trivalent::Unknown
        );
    }

    // ========================================================================
    // 逻辑求值 / Logical evaluation
    // ========================================================================

    #[test]
    fn evaluate_and_returns_true_only_when_all_operands_are_true() {
        let context = context();
        let all_true = BooleanExpression::and(vec![
            comparison("age", ComparisonOperator::Gt, 18.into()),
            comparison("status", ComparisonOperator::Eq, "active".into()),
        ]);
        let one_false = BooleanExpression::and(vec![
            comparison("age", ComparisonOperator::Gt, 18.into()),
            comparison("status", ComparisonOperator::Eq, "inactive".into()),
        ]);

        assert_eq!(evaluate_boolean(&all_true, &context), Trivalent::True);
        assert_eq!(evaluate_boolean(&one_false, &context), Trivalent::False);
    }

    #[test]
    fn evaluate_or_returns_true_when_any_operand_is_true() {
        let context = context();
        let one_true = BooleanExpression::or(vec![
            comparison("age", ComparisonOperator::Lt, 18.into()),
            comparison("status", ComparisonOperator::Eq, "active".into()),
        ]);
        let all_false = BooleanExpression::or(vec![
            comparison("age", ComparisonOperator::Lt, 18.into()),
            comparison("age", ComparisonOperator::Gt, 65.into()),
        ]);

        assert_eq!(evaluate_boolean(&one_true, &context), Trivalent::True);
        assert_eq!(evaluate_boolean(&all_false, &context), Trivalent::False);
    }

    #[test]
    fn evaluate_and_lets_false_dominate_unknown() {
        let context = context();
        let unknown_first = BooleanExpression::and(vec![
            BooleanExpression::is_null("missing"),
            comparison("status", ComparisonOperator::Eq, "inactive".into()),
        ]);
        let unknown_last = BooleanExpression::and(vec![
            comparison("status", ComparisonOperator::Eq, "inactive".into()),
            BooleanExpression::is_null("missing"),
        ]);
        let only_unknown = BooleanExpression::and(vec![
            comparison("status", ComparisonOperator::Eq, "active".into()),
            BooleanExpression::is_null("missing"),
        ]);

        assert_eq!(evaluate_boolean(&unknown_first, &context), Trivalent::False);
        assert_eq!(evaluate_boolean(&unknown_last, &context), Trivalent::False);
        assert_eq!(evaluate_boolean(&only_unknown, &context), Trivalent::Unknown);
    }

    #[test]
    fn evaluate_or_lets_true_dominate_unknown() {
        let context = context();
        let unknown_first = BooleanExpression::or(vec![
            BooleanExpression::is_null("missing"),
            comparison("status", ComparisonOperator::Eq, "active".into()),
        ]);
        let only_unknown = BooleanExpression::or(vec![
            comparison("status", ComparisonOperator::Eq, "inactive".into()),
            BooleanExpression::is_null("missing"),
        ]);

        assert_eq!(evaluate_boolean(&unknown_first, &context), Trivalent::True);
        assert_eq!(evaluate_boolean(&only_unknown, &context), Trivalent::Unknown);
    }

    #[test]
    fn evaluate_not_negates_trivalent_values() {
        let context = context();

        assert_eq!(
            evaluate_boolean(
                &BooleanExpression::not_expr(comparison(
                    "status",
                    ComparisonOperator::Eq,
                    "active".into()
                )),
                &context
            ),
            Trivalent::False
        );
        assert_eq!(
            evaluate_boolean(
                &BooleanExpression::not_expr(comparison(
                    "status",
                    ComparisonOperator::Eq,
                    "inactive".into()
                )),
                &context
            ),
            Trivalent::True
        );
        assert_eq!(
            evaluate_boolean(
                &BooleanExpression::not_expr(BooleanExpression::is_null("missing")),
                &context
            ),
            Trivalent::Unknown
        );
    }

    #[test]
    fn evaluate_boolean_constant_and_custom_expressions() {
        let context = EmptyEvaluationContext;

        assert_eq!(
            evaluate_boolean(&BooleanExpression::true_constant(), &context),
            Trivalent::True
        );
        assert_eq!(
            evaluate_boolean(&BooleanExpression::false_constant(), &context),
            Trivalent::False
        );
        assert_eq!(
            evaluate_boolean(&BooleanExpression::<ExpressionValue>::unknown_constant(), &context),
            Trivalent::Unknown
        );
        assert_eq!(
            evaluate_boolean(
                &BooleanExpression::<ExpressionValue>::custom("raw", None),
                &context
            ),
            Trivalent::Unknown,
            "自定义表达式在本地求值中恒为 Unknown / Custom expressions always evaluate to Unknown locally"
        );
    }

    // ========================================================================
    // 便捷接口 / Convenience interfaces
    // ========================================================================

    #[test]
    fn evaluate_boolean_or_none_maps_unknown_to_none() {
        let context = context();

        assert_eq!(
            evaluate_boolean_or_none(
                &comparison("status", ComparisonOperator::Eq, "active".into()),
                &context
            ),
            Some(true)
        );
        assert_eq!(
            evaluate_boolean_or_none(
                &comparison("status", ComparisonOperator::Eq, "inactive".into()),
                &context
            ),
            Some(false)
        );
        assert_eq!(
            evaluate_boolean_or_none(&BooleanExpression::is_null("missing"), &context),
            None
        );
    }

    #[test]
    fn evaluate_boolean_extension_trait_delegates_to_free_functions() {
        let context = context();
        let expression = comparison("age", ComparisonOperator::Ge, 18.into());

        assert_eq!(expression.evaluate_with(&context), Trivalent::True);
        assert_eq!(expression.evaluate_with_or_none(&context), Some(true));
        assert_eq!(
            BooleanExpression::is_null("missing").evaluate_with_or_none(&context),
            None
        );
    }

    // ========================================================================
    // 标量求值 / Scalar evaluation
    // ========================================================================

    #[test]
    fn evaluate_scalar_arithmetic_operators() {
        let context = context();
        let cases = [
            (
                ScalarExpression::binary(BinaryOperator::Add, number(2.0), number(3.0)),
                5.0,
            ),
            (
                ScalarExpression::binary(BinaryOperator::Subtract, number(7.0), number(2.0)),
                5.0,
            ),
            (
                ScalarExpression::binary(BinaryOperator::Multiply, number(2.0), number(3.0)),
                6.0,
            ),
            (
                ScalarExpression::binary(BinaryOperator::Divide, number(7.0), number(2.0)),
                3.5,
            ),
            (
                ScalarExpression::binary(BinaryOperator::Modulo, number(7.0), number(3.0)),
                1.0,
            ),
            (
                ScalarExpression::binary(BinaryOperator::Power, number(2.0), number(3.0)),
                8.0,
            ),
        ];

        for (expression, expected) in cases {
            assert_eq!(
                evaluate(&expression, &context),
                Some(ExpressionValue::Number(expected))
            );
        }
    }

    #[test]
    fn evaluate_scalar_reads_references_and_symbol_references() {
        let context = context();

        assert_eq!(
            evaluate(&reference("age"), &context),
            Some(ExpressionValue::Number(20.0))
        );
        assert_eq!(
            evaluate(
                &ScalarExpression::symbol_reference(path_owned_symbol("status")),
                &context
            ),
            Some(ExpressionValue::String("active".to_string()))
        );
        assert_eq!(evaluate(&reference("missing"), &context), None);
        assert_eq!(
            evaluate(
                &ScalarExpression::symbol_reference(OwnedSymbol::new(
                    crate::symbol::test_utils::SimpleSymbol::new("age"),
                )),
                &context
            ),
            None
        );
    }

    #[test]
    fn evaluate_scalar_division_and_modulo_by_zero_return_none() {
        let context = context();
        let divide = ScalarExpression::binary(BinaryOperator::Divide, number(1.0), number(0.0));
        let modulo = ScalarExpression::binary(BinaryOperator::Modulo, number(1.0), number(0.0));

        assert_eq!(evaluate(&divide, &context), None);
        assert_eq!(evaluate(&modulo, &context), None);
    }

    #[test]
    fn evaluate_scalar_rejects_non_finite_results() {
        let context = context();
        let overflow =
            ScalarExpression::binary(BinaryOperator::Multiply, number(f64::MAX), number(2.0));
        let power = ScalarExpression::binary(BinaryOperator::Power, number(2.0), number(10000.0));
        let non_finite_operand =
            ScalarExpression::binary(BinaryOperator::Add, number(f64::NAN), number(1.0));

        assert_eq!(evaluate(&overflow, &context), None);
        assert_eq!(evaluate(&power, &context), None);
        assert_eq!(evaluate(&non_finite_operand, &context), None);
    }

    #[test]
    fn evaluate_scalar_unary_operators() {
        let context = context();
        let negate = ScalarExpression::unary(UnaryOperator::Negate, number(3.0));
        let abs = ScalarExpression::unary(UnaryOperator::Abs, number(-3.0));
        let positive = ScalarExpression::unary(UnaryOperator::Positive, number(-3.0));

        assert_eq!(
            evaluate(&negate, &context),
            Some(ExpressionValue::Number(-3.0))
        );
        assert_eq!(evaluate(&abs, &context), Some(ExpressionValue::Number(3.0)));
        assert_eq!(
            evaluate(&positive, &context),
            Some(ExpressionValue::Number(-3.0))
        );
        assert_eq!(
            evaluate(
                &ScalarExpression::unary(UnaryOperator::Positive, reference("status")),
                &context
            ),
            Some(ExpressionValue::String("active".to_string())),
            "正号不校验类型 / The positive sign does not type-check"
        );
    }

    #[test]
    fn evaluate_scalar_unary_operators_reject_non_numeric_operands() {
        let context = context();

        assert_eq!(
            evaluate(
                &ScalarExpression::unary(UnaryOperator::Negate, reference("status")),
                &context
            ),
            None
        );
        assert_eq!(
            evaluate(
                &ScalarExpression::unary(
                    UnaryOperator::Abs,
                    ScalarExpression::constant(ExpressionValue::Boolean(true))
                ),
                &context
            ),
            None
        );
    }

    #[test]
    fn evaluate_scalar_binary_operators_reject_non_numeric_operands() {
        let context = context();

        assert_eq!(
            evaluate(
                &ScalarExpression::binary(BinaryOperator::Add, reference("status"), number(1.0)),
                &context
            ),
            None
        );
        assert_eq!(
            evaluate(
                &ScalarExpression::binary(
                    BinaryOperator::Add,
                    number(1.0),
                    ScalarExpression::constant(ExpressionValue::Boolean(true))
                ),
                &context
            ),
            None,
            "布尔值不参与算术 / Booleans do not participate in arithmetic"
        );
    }

    #[test]
    fn evaluate_scalar_custom_expression_returns_none() {
        let context = context();
        let expression = ScalarExpression::<ExpressionValue>::custom("raw", Some("shown".into()));

        assert_eq!(evaluate(&expression, &context), None);
    }

    #[test]
    fn evaluate_scalar_default_function_evaluator_covers_builtin_functions() {
        let context = context();
        let cases = [
            (
                ScalarExpression::function(
                    ScalarFunctionNames::ABS,
                    vec![ScalarExpression::reference("delta")],
                ),
                ExpressionValue::Number(3.0),
            ),
            (
                ScalarExpression::function(
                    ScalarFunctionNames::LOWER,
                    vec![ScalarExpression::reference("name")],
                ),
                ExpressionValue::String("alice".to_string()),
            ),
            (
                ScalarExpression::function(
                    ScalarFunctionNames::UPPER,
                    vec![ScalarExpression::reference("name")],
                ),
                ExpressionValue::String("ALICE".to_string()),
            ),
            (
                ScalarExpression::function(
                    ScalarFunctionNames::LENGTH,
                    vec![ScalarExpression::reference("name")],
                ),
                ExpressionValue::Number(5.0),
            ),
            (
                ScalarExpression::function(
                    ScalarFunctionNames::TRIM,
                    vec![ScalarExpression::constant(ExpressionValue::from("  Alice  "))],
                ),
                ExpressionValue::String("Alice".to_string()),
            ),
            (
                ScalarExpression::function(
                    ScalarFunctionNames::COALESCE,
                    vec![
                        ScalarExpression::reference("missing"),
                        ScalarExpression::constant(ExpressionValue::Null),
                        ScalarExpression::reference("name"),
                    ],
                ),
                ExpressionValue::String("Alice".to_string()),
            ),
        ];

        for (expression, expected) in cases {
            assert_eq!(evaluate(&expression, &context), Some(expected));
        }
    }

    #[test]
    fn evaluate_scalar_function_errors_return_none() {
        let context = context();
        let cases = [
            ScalarExpression::function("unknownFunction", vec![number(1.0)]),
            ScalarExpression::function(ScalarFunctionNames::ABS, vec![reference("status")]),
            ScalarExpression::function(ScalarFunctionNames::COALESCE, Vec::new()),
            ScalarExpression::function(
                ScalarFunctionNames::LOWER,
                vec![ScalarExpression::reference("missing")],
            ),
        ];

        for expression in cases {
            assert_eq!(evaluate(&expression, &context), None);
        }
    }

    #[test]
    fn evaluate_scalar_conditional_picks_branch_by_condition() {
        let context = context();
        let build = |condition: ParsedBooleanExpression| {
            ScalarExpression::conditional(condition, number(1.0), number(2.0))
        };

        assert_eq!(
            evaluate(
                &build(comparison("age", ComparisonOperator::Gt, 18.into())),
                &context
            ),
            Some(ExpressionValue::Number(1.0))
        );
        assert_eq!(
            evaluate(
                &build(comparison("age", ComparisonOperator::Gt, 30.into())),
                &context
            ),
            Some(ExpressionValue::Number(2.0))
        );
        assert_eq!(
            evaluate(&build(BooleanExpression::is_null("missing")), &context),
            None,
            "条件未知时分支求值失败 / An unknown condition fails the whole conditional"
        );
    }

    #[test]
    fn evaluate_scalar_nested_conditional_uses_inner_result() {
        let context = context();
        let inner = ScalarExpression::conditional(
            comparison("status", ComparisonOperator::Eq, "active".into()),
            number(10.0),
            number(20.0),
        );
        let outer = ScalarExpression::conditional(
            comparison("age", ComparisonOperator::Gt, 18.into()),
            inner,
            number(30.0),
        );

        assert_eq!(
            evaluate(&outer, &context),
            Some(ExpressionValue::Number(10.0))
        );
    }

    #[test]
    fn evaluate_scalar_boolean_wrapper_returns_boolean_or_none() {
        let context = context();
        let build = |condition: ParsedBooleanExpression| ScalarExpression::boolean_expr(condition);

        assert_eq!(
            evaluate(
                &build(comparison("age", ComparisonOperator::Gt, 18.into())),
                &context
            ),
            Some(ExpressionValue::Boolean(true))
        );
        assert_eq!(
            evaluate(
                &build(comparison("age", ComparisonOperator::Gt, 30.into())),
                &context
            ),
            Some(ExpressionValue::Boolean(false))
        );
        assert_eq!(
            evaluate(&build(BooleanExpression::is_null("missing")), &context),
            None
        );
    }

    #[test]
    fn evaluate_scalar_condition_uses_default_evaluator_for_functions() {
        let context = context();
        let expression = ScalarExpression::conditional(
            BooleanExpression::gt(
                ScalarExpression::function(
                    ScalarFunctionNames::ABS,
                    vec![ScalarExpression::reference("delta")],
                ),
                ScalarExpression::constant(ExpressionValue::Number(2.0)),
            ),
            number(1.0),
            number(0.0),
        );

        assert_eq!(
            evaluate(&expression, &context),
            Some(ExpressionValue::Number(1.0))
        );
    }

    // ========================================================================
    // 可注入函数求值器 / Injectable function evaluator
    // ========================================================================

    #[test]
    fn evaluate_scalar_expression_with_math_evaluator_resolves_math_functions() {
        let context = MapEvaluationContext::from_string_map([("x", ExpressionValue::from(16))]);
        let sqrt = ScalarExpression::function(
            "sqrt",
            vec![ScalarExpression::reference("x")],
        );
        let pow = ScalarExpression::function("pow", vec![number(2.0), number(3.0)]);
        let floor = ScalarExpression::function("floor", vec![number(3.7)]);

        assert_eq!(
            evaluate_scalar_expression(&sqrt, &context, &MathFunctionEvaluator),
            Some(ExpressionValue::Number(4.0))
        );
        assert_eq!(
            evaluate_scalar_expression(&pow, &context, &MathFunctionEvaluator),
            Some(ExpressionValue::Number(8.0))
        );
        assert_eq!(
            evaluate_scalar_expression(&floor, &context, &MathFunctionEvaluator),
            Some(ExpressionValue::Number(3.0))
        );
        assert_eq!(
            evaluate_scalar_expression(&sqrt, &context, &DefaultScalarFunctionEvaluator),
            None,
            "默认求值器不识别 sqrt / The default evaluator does not know sqrt"
        );
    }

    #[test]
    fn evaluate_boolean_with_evaluator_uses_the_injected_evaluator() {
        let context = MapEvaluationContext::from_string_map([("x", ExpressionValue::from(16))]);
        let expression = BooleanExpression::gt(
            ScalarExpression::function("sqrt", vec![ScalarExpression::reference("x")]),
            ScalarExpression::constant(ExpressionValue::Number(2.0)),
        );

        assert_eq!(
            evaluate_boolean_with_evaluator(&expression, &context, &MathFunctionEvaluator),
            Trivalent::True
        );
        assert_eq!(
            evaluate_boolean(&expression, &context),
            Trivalent::Unknown,
            "无注入求值器时未知函数导致 Unknown / Without an injected evaluator the unknown function yields Unknown"
        );
    }

    #[test]
    fn evaluate_boolean_with_evaluator_matches_default_for_builtins() {
        let context = context();
        let expression = BooleanExpression::and(vec![
            BooleanExpression::gt(
                ScalarExpression::function(
                    ScalarFunctionNames::ABS,
                    vec![ScalarExpression::reference("delta")],
                ),
                ScalarExpression::constant(ExpressionValue::Number(2.0)),
            ),
            comparison("status", ComparisonOperator::Eq, "active".into()),
            BooleanExpression::is_not_null("name"),
            BooleanExpression::not_expr(BooleanExpression::is_null("name")),
        ]);

        assert_eq!(
            evaluate_boolean_with_evaluator(&expression, &context, &DefaultScalarFunctionEvaluator),
            evaluate_boolean(&expression, &context)
        );
        assert_eq!(
            evaluate_boolean_with_evaluator(&expression, &context, &DefaultScalarFunctionEvaluator),
            Trivalent::True
        );
    }

    #[test]
    fn evaluate_boolean_with_evaluator_keeps_trivalent_semantics() {
        let context = context();
        let expression = BooleanExpression::or(vec![
            BooleanExpression::is_null("missing"),
            comparison("status", ComparisonOperator::Eq, "active".into()),
        ]);
        let unknown_only = BooleanExpression::and(vec![
            comparison("status", ComparisonOperator::Eq, "active".into()),
            BooleanExpression::is_null("missing"),
        ]);

        assert_eq!(
            evaluate_boolean_with_evaluator(&expression, &context, &DefaultScalarFunctionEvaluator),
            Trivalent::True
        );
        assert_eq!(
            evaluate_boolean_with_evaluator(
                &unknown_only,
                &context,
                &DefaultScalarFunctionEvaluator
            ),
            Trivalent::Unknown
        );
        assert_eq!(
            evaluate_boolean_with_evaluator(
                &BooleanExpression::<ExpressionValue>::custom("raw", None),
                &context,
                &DefaultScalarFunctionEvaluator
            ),
            Trivalent::Unknown
        );
    }
}
