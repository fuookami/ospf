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
