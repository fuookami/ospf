//! 数学函数求值器
//! Math function evaluator
//!
//! 提供公式域白名单中的 math.* 函数求值能力。
//! Provides evaluation for math.* functions from the formula domain whitelist.

use super::ExpressionValue;

/// 标量函数求值器 trait。
/// Scalar function evaluator trait.
///
/// 允许注入自定义函数求值逻辑，默认实现覆盖 abs/lower/upper/trim/length/coalesce。
/// Allows injecting custom function evaluation logic,
/// with a default implementation covering abs/lower/upper/trim/length/coalesce.
pub trait ScalarFunctionEvaluator {
    /// 求值指定名称的函数。
    /// Evaluate the function with the given name.
    ///
    /// # 参数 / Arguments
    /// - `name`: 函数名 / Function name
    /// - `arguments`: 参数值列表（已求值，None 表示求值失败）/ Argument values (evaluated, None means evaluation failed)
    ///
    /// # 返回 / Returns
    /// 求值结果，未识别的函数返回 None / Evaluation result, None for unrecognized functions
    fn evaluate(
        &self,
        name: &str,
        arguments: &[Option<ExpressionValue>],
    ) -> Option<ExpressionValue>;
}

/// 默认标量函数求值器。
/// Default scalar function evaluator.
///
/// 覆盖 abs/lower/upper/trim/length/coalesce 函数。
/// Covers abs/lower/upper/trim/length/coalesce functions.
pub struct DefaultScalarFunctionEvaluator;

impl ScalarFunctionEvaluator for DefaultScalarFunctionEvaluator {
    fn evaluate(
        &self,
        name: &str,
        arguments: &[Option<ExpressionValue>],
    ) -> Option<ExpressionValue> {
        match name.to_ascii_lowercase().as_str() {
            super::ScalarFunctionNames::ABS => {
                let value = arguments.first()?.as_ref()?;
                Some(ExpressionValue::Number(expression_number(value)?.abs()))
            }
            super::ScalarFunctionNames::LOWER => evaluate_string_unary(arguments, |value| {
                ExpressionValue::String(value.to_ascii_lowercase())
            }),
            super::ScalarFunctionNames::UPPER => evaluate_string_unary(arguments, |value| {
                ExpressionValue::String(value.to_ascii_uppercase())
            }),
            super::ScalarFunctionNames::TRIM => evaluate_string_unary(arguments, |value| {
                ExpressionValue::String(value.trim().to_string())
            }),
            super::ScalarFunctionNames::LENGTH => evaluate_string_unary(arguments, |value| {
                ExpressionValue::Number(value.chars().count() as f64)
            }),
            super::ScalarFunctionNames::COALESCE => arguments
                .iter()
                .flatten()
                .find(|value| **value != ExpressionValue::Null)
                .cloned(),
            _ => None,
        }
    }
}

/// 数学函数求值器。
/// Math function evaluator.
///
/// 实现 `ScalarFunctionEvaluator`，覆盖 17 个 math.* 函数。
/// 未识别的函数委托给 `DefaultScalarFunctionEvaluator`。
/// Implements `ScalarFunctionEvaluator`, covering 17 math.* functions.
/// Unrecognized functions fall through to `DefaultScalarFunctionEvaluator`.
///
/// 注：公式域的 validate() 必须在 AST 层面检查 ScalarFunction.name 是否在白名单内，
/// 不能依赖求值器拒绝未知函数——因为组合链包含 DefaultScalarFunctionEvaluator，
/// 后者暴露 lower/upper/trim/length/coalesce 等字符串函数。
/// Note: The formula domain's validate() must check ScalarFunction.name against the whitelist
/// at the AST level, not rely on evaluator rejection — because the composition chain includes
/// DefaultScalarFunctionEvaluator, which exposes string functions like lower/upper/trim/length/coalesce.
pub struct MathFunctionEvaluator;

impl MathFunctionEvaluator {
    /// 支持的数学函数名集合。
    /// Set of supported math function names.
    pub const SUPPORTED_FUNCTIONS: &'static [&'static str] = &[
        "sqrt", "pow", "log", "log10", "exp", "sin", "cos", "tan", "asin", "acos", "atan", "floor",
        "ceil", "round", "max", "min", "abs",
    ];
}

impl ScalarFunctionEvaluator for MathFunctionEvaluator {
    fn evaluate(
        &self,
        name: &str,
        arguments: &[Option<ExpressionValue>],
    ) -> Option<ExpressionValue> {
        match name.to_ascii_lowercase().as_str() {
            "sqrt" => evaluate_single_arg(arguments, f64::sqrt),
            "pow" => evaluate_two_arg(arguments, f64::powf),
            "log" => evaluate_single_arg(arguments, f64::ln),
            "log10" => evaluate_single_arg(arguments, f64::log10),
            "exp" => evaluate_single_arg(arguments, f64::exp),
            "sin" => evaluate_single_arg(arguments, f64::sin),
            "cos" => evaluate_single_arg(arguments, f64::cos),
            "tan" => evaluate_single_arg(arguments, f64::tan),
            "asin" => evaluate_single_arg(arguments, f64::asin),
            "acos" => evaluate_single_arg(arguments, f64::acos),
            "atan" => evaluate_single_arg(arguments, f64::atan),
            "floor" => evaluate_single_arg(arguments, |v| v.floor()),
            "ceil" => evaluate_single_arg(arguments, |v| v.ceil()),
            "round" => evaluate_single_arg(arguments, |v| v.round()),
            "max" => evaluate_two_arg(arguments, f64::max),
            "min" => evaluate_two_arg(arguments, f64::min),
            "abs" => DefaultScalarFunctionEvaluator.evaluate("abs", arguments),
            _ => DefaultScalarFunctionEvaluator.evaluate(name, arguments),
        }
    }
}

/// 组合函数求值器。
/// Composite function evaluator.
///
/// 先尝试第一个求值器，失败则委托第二个。
/// Tries the first evaluator, falls through to the second on failure.
pub struct CompositeFunctionEvaluator<'a, A: ScalarFunctionEvaluator, B: ScalarFunctionEvaluator> {
    /// 第一求值器 / Primary evaluator
    pub primary: &'a A,
    /// 第二求值器 / Fallback evaluator
    pub fallback: &'a B,
}

impl<'a, A: ScalarFunctionEvaluator, B: ScalarFunctionEvaluator> ScalarFunctionEvaluator
    for CompositeFunctionEvaluator<'a, A, B>
{
    fn evaluate(
        &self,
        name: &str,
        arguments: &[Option<ExpressionValue>],
    ) -> Option<ExpressionValue> {
        self.primary
            .evaluate(name, arguments)
            .or_else(|| self.fallback.evaluate(name, arguments))
    }
}

// ========== 辅助函数 / Helper Functions ==========

fn evaluate_single_arg(
    arguments: &[Option<ExpressionValue>],
    operation: impl FnOnce(f64) -> f64,
) -> Option<ExpressionValue> {
    if arguments.len() != 1 {
        return None;
    }
    let value = arguments.first()?.as_ref()?;
    let number = expression_number(value)?;
    let result = operation(number);
    result
        .is_finite()
        .then_some(ExpressionValue::Number(result))
}

fn evaluate_two_arg(
    arguments: &[Option<ExpressionValue>],
    operation: impl FnOnce(f64, f64) -> f64,
) -> Option<ExpressionValue> {
    if arguments.len() != 2 {
        return None;
    }
    let left = arguments.first()?.as_ref()?;
    let right = arguments.get(1)?.as_ref()?;
    let left = expression_number(left)?;
    let right = expression_number(right)?;
    let result = operation(left, right);
    result
        .is_finite()
        .then_some(ExpressionValue::Number(result))
}

fn expression_number(value: &ExpressionValue) -> Option<f64> {
    match value {
        ExpressionValue::Number(value) if value.is_finite() => Some(*value),
        _ => None,
    }
}

fn evaluate_string_unary(
    arguments: &[Option<ExpressionValue>],
    operation: impl FnOnce(&str) -> ExpressionValue,
) -> Option<ExpressionValue> {
    let value = arguments.first()?.as_ref()?;
    match value {
        ExpressionValue::String(value) => Some(operation(value)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn math_function_evaluator_supports_all_17_functions() {
        assert_eq!(MathFunctionEvaluator::SUPPORTED_FUNCTIONS.len(), 17);
        let expected = [
            "sqrt", "pow", "log", "log10", "exp", "sin", "cos", "tan", "asin", "acos", "atan",
            "floor", "ceil", "round", "max", "min", "abs",
        ];
        for name in &expected {
            assert!(
                MathFunctionEvaluator::SUPPORTED_FUNCTIONS.contains(name),
                "missing function: {name}"
            );
        }
    }

    #[test]
    fn math_sqrt() {
        let result = MathFunctionEvaluator.evaluate("sqrt", &[Some(ExpressionValue::Number(16.0))]);
        assert_eq!(result, Some(ExpressionValue::Number(4.0)));
    }

    #[test]
    fn math_pow() {
        let result = MathFunctionEvaluator.evaluate(
            "pow",
            &[
                Some(ExpressionValue::Number(2.0)),
                Some(ExpressionValue::Number(3.0)),
            ],
        );
        assert_eq!(result, Some(ExpressionValue::Number(8.0)));
    }

    #[test]
    fn math_floor_ceil_round() {
        let result = MathFunctionEvaluator.evaluate("floor", &[Some(ExpressionValue::Number(3.7))]);
        assert_eq!(result, Some(ExpressionValue::Number(3.0)));

        let result = MathFunctionEvaluator.evaluate("ceil", &[Some(ExpressionValue::Number(3.2))]);
        assert_eq!(result, Some(ExpressionValue::Number(4.0)));

        let result = MathFunctionEvaluator.evaluate("round", &[Some(ExpressionValue::Number(3.7))]);
        assert_eq!(result, Some(ExpressionValue::Number(4.0)));
    }

    #[test]
    fn math_max_min() {
        let result = MathFunctionEvaluator.evaluate(
            "max",
            &[
                Some(ExpressionValue::Number(3.0)),
                Some(ExpressionValue::Number(4.0)),
            ],
        );
        assert_eq!(result, Some(ExpressionValue::Number(4.0)));

        let result = MathFunctionEvaluator.evaluate(
            "min",
            &[
                Some(ExpressionValue::Number(3.0)),
                Some(ExpressionValue::Number(4.0)),
            ],
        );
        assert_eq!(result, Some(ExpressionValue::Number(3.0)));
    }

    #[test]
    fn math_exp() {
        let result = MathFunctionEvaluator.evaluate("exp", &[Some(ExpressionValue::Number(0.0))]);
        assert_eq!(result, Some(ExpressionValue::Number(1.0)));
    }

    #[test]
    fn math_abs() {
        let result = MathFunctionEvaluator.evaluate("abs", &[Some(ExpressionValue::Number(-5.0))]);
        assert_eq!(result, Some(ExpressionValue::Number(5.0)));
    }

    #[test]
    fn math_unknown_function_falls_through() {
        let result =
            MathFunctionEvaluator.evaluate("unknownFunc", &[Some(ExpressionValue::Number(1.0))]);
        assert_eq!(result, None);
    }

    #[test]
    fn math_wrong_arity_returns_none() {
        let result = MathFunctionEvaluator.evaluate(
            "sqrt",
            &[
                Some(ExpressionValue::Number(1.0)),
                Some(ExpressionValue::Number(2.0)),
            ],
        );
        assert_eq!(result, None);
    }

    #[test]
    fn math_string_arg_returns_none() {
        let result = MathFunctionEvaluator
            .evaluate("sqrt", &[Some(ExpressionValue::String("abc".to_string()))]);
        assert_eq!(result, None);
    }
}
