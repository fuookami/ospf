//! 比较运算符
//! Comparison operators
//!
//! 定义不等式中使用的比较运算符。
//! Defines comparison operators used in inequalities.

use std::fmt::{Debug, Display};

// ============================================================================
// Comparison - 比较运算符
// ============================================================================

/// 比较运算符 / Comparison operator
///
/// 用于表示不等式中的比较关系。
/// Used to represent comparison relations in inequalities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Comparison {
    /// 小于 / Less than
    ///
    /// `a < b`
    Less,

    /// 小于等于 / Less than or equal
    ///
    /// `a ≤ b`
    LessEqual,

    /// 大于 / Greater than
    ///
    /// `a > b`
    Greater,

    /// 大于等于 / Greater than or equal
    ///
    /// `a ≥ b`
    GreaterEqual,

    /// 等于 / Equal
    ///
    /// `a = b`
    Equal,
}

impl Comparison {
    /// 获取比较运算符的符号表示
    /// Get the symbol representation of the comparison operator
    pub fn symbol(&self) -> &'static str {
        match self {
            Comparison::Less => "<",
            Comparison::LessEqual => "≤",
            Comparison::Greater => ">",
            Comparison::GreaterEqual => "≥",
            Comparison::Equal => "=",
        }
    }

    /// 是否为严格比较（不包含等于）
    /// Whether this is a strict comparison (excluding equal)
    pub fn is_strict(&self) -> bool {
        matches!(self, Comparison::Less | Comparison::Greater)
    }

    /// 是否包含等于
    /// Whether this includes equality
    pub fn includes_equality(&self) -> bool {
        !self.is_strict()
    }

    /// 获取反向比较运算符
    /// Get the reversed comparison operator
    ///
    /// # 示例 / Example
    ///
    /// `a < b` 的反向是 `b > a`
    /// The reverse of `a < b` is `b > a`
    pub fn reverse(&self) -> Self {
        match self {
            Comparison::Less => Comparison::Greater,
            Comparison::LessEqual => Comparison::GreaterEqual,
            Comparison::Greater => Comparison::Less,
            Comparison::GreaterEqual => Comparison::LessEqual,
            Comparison::Equal => Comparison::Equal,
        }
    }

    /// 是否为“小于”类型（< 或 ≤）
    /// Whether this is a "less than" type (< or ≤)
    pub fn is_less_type(&self) -> bool {
        matches!(self, Comparison::Less | Comparison::LessEqual)
    }

    /// 是否为“大于”类型（> 或 ≥）
    /// Whether this is a "greater than" type (> or ≥)
    pub fn is_greater_type(&self) -> bool {
        matches!(self, Comparison::Greater | Comparison::GreaterEqual)
    }
}

impl Display for Comparison {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.symbol())
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comparison_symbol() {
        assert_eq!(Comparison::Less.symbol(), "<");
        assert_eq!(Comparison::LessEqual.symbol(), "≤");
        assert_eq!(Comparison::Greater.symbol(), ">");
        assert_eq!(Comparison::GreaterEqual.symbol(), "≥");
        assert_eq!(Comparison::Equal.symbol(), "=");
    }

    #[test]
    fn test_comparison_reverse() {
        assert_eq!(Comparison::Less.reverse(), Comparison::Greater);
        assert_eq!(Comparison::LessEqual.reverse(), Comparison::GreaterEqual);
        assert_eq!(Comparison::Greater.reverse(), Comparison::Less);
        assert_eq!(Comparison::GreaterEqual.reverse(), Comparison::LessEqual);
        assert_eq!(Comparison::Equal.reverse(), Comparison::Equal);
    }

    #[test]
    fn test_comparison_is_strict() {
        assert!(Comparison::Less.is_strict());
        assert!(!Comparison::LessEqual.is_strict());
        assert!(Comparison::Greater.is_strict());
        assert!(!Comparison::GreaterEqual.is_strict());
        assert!(!Comparison::Equal.is_strict());
    }
}
