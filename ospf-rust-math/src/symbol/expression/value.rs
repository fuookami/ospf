//! 表达式运行时值
//! Expression runtime value

use std::fmt::{Display, Formatter};

/// 表达式解析器使用的标量字面量。
/// Scalar literal used by the expression parser.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExpressionValue {
    /// 空值 / Null value
    Null,
    /// 布尔值 / Boolean value
    Boolean(bool),
    /// 数字值 / Number value
    Number(f64),
    /// 字符串值 / String value
    String(String),
}

impl Display for ExpressionValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Boolean(value) => write!(f, "{}", value),
            Self::Number(value) => write!(f, "{}", value),
            Self::String(value) => write!(f, "{}", value),
        }
    }
}

impl From<bool> for ExpressionValue {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<f64> for ExpressionValue {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl From<f32> for ExpressionValue {
    fn from(value: f32) -> Self {
        Self::Number(f64::from(value))
    }
}

macro_rules! impl_expression_value_from_integer {
    ($($type:ty),* $(,)?) => {
        $(
            impl From<$type> for ExpressionValue {
                fn from(value: $type) -> Self {
                    Self::Number(value as f64)
                }
            }
        )*
    };
}

impl_expression_value_from_integer!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

impl From<&str> for ExpressionValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<String> for ExpressionValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

// ============================================================================
// 运行时值测试 / Runtime value tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_renders_each_variant_as_plain_text() {
        assert_eq!(ExpressionValue::Null.to_string(), "null");
        assert_eq!(ExpressionValue::Boolean(true).to_string(), "true");
        assert_eq!(ExpressionValue::Boolean(false).to_string(), "false");
        assert_eq!(ExpressionValue::Number(18.0).to_string(), "18");
        assert_eq!(ExpressionValue::Number(-0.5).to_string(), "-0.5");
        assert_eq!(
            ExpressionValue::String("active".to_string()).to_string(),
            "active"
        );
    }

    #[test]
    fn boolean_conversion_wraps_boolean_variant() {
        assert_eq!(ExpressionValue::from(true), ExpressionValue::Boolean(true));
        assert_eq!(
            ExpressionValue::from(false),
            ExpressionValue::Boolean(false)
        );
    }

    #[test]
    fn float_conversions_widen_to_f64() {
        assert_eq!(ExpressionValue::from(1.5_f64), ExpressionValue::Number(1.5));
        assert_eq!(
            ExpressionValue::from(1.5_f32),
            ExpressionValue::Number(f64::from(1.5_f32))
        );
        assert_eq!(
            ExpressionValue::from(0.1_f32),
            ExpressionValue::Number(f64::from(0.1_f32))
        );
    }

    #[test]
    fn signed_integer_conversions_preserve_sign() {
        assert_eq!(ExpressionValue::from(-8_i8), ExpressionValue::Number(-8.0));
        assert_eq!(
            ExpressionValue::from(-16_i16),
            ExpressionValue::Number(-16.0)
        );
        assert_eq!(
            ExpressionValue::from(-32_i32),
            ExpressionValue::Number(-32.0)
        );
        assert_eq!(
            ExpressionValue::from(-64_i64),
            ExpressionValue::Number(-64.0)
        );
        assert_eq!(
            ExpressionValue::from(-8_isize),
            ExpressionValue::Number(-8.0)
        );
    }

    #[test]
    fn unsigned_integer_conversions_produce_non_negative_numbers() {
        assert_eq!(ExpressionValue::from(8_u8), ExpressionValue::Number(8.0));
        assert_eq!(ExpressionValue::from(16_u16), ExpressionValue::Number(16.0));
        assert_eq!(ExpressionValue::from(32_u32), ExpressionValue::Number(32.0));
        assert_eq!(ExpressionValue::from(64_u64), ExpressionValue::Number(64.0));
        assert_eq!(
            ExpressionValue::from(8_usize),
            ExpressionValue::Number(8.0)
        );
    }

    #[test]
    fn wide_unsigned_values_saturate_to_f64_representation() {
        // 超出 f64 精确表示范围的整数会按 f64 取整，偏差只体现在低位。
        // Integers beyond exact f64 representation are rounded to f64, deviating only in low bits.
        let value = ExpressionValue::from(u64::MAX);
        let ExpressionValue::Number(number) = value else {
            panic!("expected number value");
        };
        assert!(number.is_finite());
        assert!(number > 1.8e19);
    }

    #[test]
    fn string_conversions_accept_borrowed_and_owned_text() {
        assert_eq!(
            ExpressionValue::from("active"),
            ExpressionValue::String("active".to_string())
        );
        assert_eq!(
            ExpressionValue::from("active".to_string()),
            ExpressionValue::String("active".to_string())
        );
        assert_eq!(
            ExpressionValue::from(""),
            ExpressionValue::String(String::new())
        );
    }

    #[test]
    fn equality_is_variant_sensitive() {
        // 不同变体即使文本相同也不相等，数值绝不与字符串隐式互相转换。
        // Different variants are never equal even with identical text, and numbers never coerce to strings.
        assert_ne!(
            ExpressionValue::Number(1.0),
            ExpressionValue::String("1".to_string())
        );
        assert_ne!(ExpressionValue::Number(0.0), ExpressionValue::Boolean(false));
        assert_ne!(ExpressionValue::Null, ExpressionValue::Boolean(false));
        assert_eq!(ExpressionValue::Number(1.0), ExpressionValue::Number(1.0));
    }

    #[test]
    fn non_finite_numbers_never_equal_themselves_under_partial_eq() {
        assert_ne!(
            ExpressionValue::Number(f64::NAN),
            ExpressionValue::Number(f64::NAN)
        );
        assert_eq!(
            ExpressionValue::Number(f64::INFINITY),
            ExpressionValue::Number(f64::INFINITY)
        );
    }

    #[test]
    fn values_are_clonable_and_debuggable() {
        let value = ExpressionValue::String("active".to_string());
        let cloned = value.clone();
        assert_eq!(cloned, value);
        assert_eq!(format!("{value:?}"), "String(\"active\")");
        assert_eq!(format!("{:?}", ExpressionValue::Null), "Null");
    }
}
