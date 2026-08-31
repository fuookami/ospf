//! 变量类型定义
//! Variable Type Definitions

use super::VariableRange;
use std::fmt::Debug;

// ============================================================================
// VariableType - 变量类型枚举
// ============================================================================

/// 变量类型枚举 / Variable Type Enum
///
/// 表示优化问题中决策变量的类型。
/// Represents the type of decision variables in optimization problems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum VariableType {
    /// 二进制变量 (0 或 1) / Binary variable (0 or 1)
    Binary,
    /// 三元变量 (0, 1, 或 2) / Ternary variable (0, 1, or 2)
    Ternary,
    /// 平衡三元变量 (-1, 0, 或 1) / Balanced ternary variable (-1, 0, or 1)
    BalancedTernary,
    /// 百分比变量 [0, 1] / Percentage variable [0, 1]
    Percentage,
    /// 整数变量 / Integer variable
    Integer,
    /// 无符号整数变量 / Unsigned integer variable
    UInteger,
    /// 连续变量 / Continuous variable
    Continuous,
    /// 无符号连续变量 / Unsigned continuous variable
    UContinuous,
}

impl VariableType {
    /// 是否为整数类型变量 / Whether it's an integer type variable
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            VariableType::Binary
                | VariableType::Ternary
                | VariableType::BalancedTernary
                | VariableType::Integer
                | VariableType::UInteger
        )
    }

    /// 是否为有符号变量 / Whether it's a signed variable
    pub fn is_signed(&self) -> bool {
        matches!(
            self,
            VariableType::BalancedTernary | VariableType::Integer | VariableType::Continuous
        )
    }

    /// 是否为二进制变量 / Whether it's a binary variable
    pub fn is_binary(&self) -> bool {
        matches!(self, VariableType::Binary)
    }

    /// 是否为连续变量 / Whether it's a continuous variable
    pub fn is_continuous(&self) -> bool {
        matches!(
            self,
            VariableType::Percentage | VariableType::Continuous | VariableType::UContinuous
        )
    }

    /// 获取类型名称 / Get type name
    pub fn type_name(&self) -> &'static str {
        match self {
            VariableType::Binary => "Binary",
            VariableType::Ternary => "Ternary",
            VariableType::BalancedTernary => "BalancedTernary",
            VariableType::Percentage => "Percentage",
            VariableType::Integer => "Integer",
            VariableType::UInteger => "UInteger",
            VariableType::Continuous => "Continuous",
            VariableType::UContinuous => "UContinuous",
        }
    }
}

// ============================================================================
// VariableTypeTrait - 变量类型标记 Trait
// ============================================================================

/// 变量类型标记 Trait / Variable Type Marker Trait
///
/// 为变量类型提供元信息，包括值类型、边界、默认范围等。
/// Provides metadata for variable types, including value type, bounds, default range, etc.
///
/// # 类型参数 / Type Parameters
///
/// 无类型参数，关联类型 `Value` 定义值类型。
/// No type parameters, associated type `Value` defines the value type.
///
/// # 示例 / Examples
///
/// ```rust
/// use ospf_rust_core::variable::{VariableTypeTrait, Binary, VariableRange};
///
/// // 获取二进制变量的默认范围 / Get default range for binary variable
/// let range = Binary::default_range();
/// assert_eq!(range.lower_bound, Some(0.0));
/// assert_eq!(range.upper_bound, Some(1.0));
///
/// // 检查值是否在范围内 / Check if value is valid
/// assert!(Binary::is_valid_value(&0.5));
/// assert!(!Binary::is_valid_value(&2.0));
/// ```
pub trait VariableTypeTrait: Clone + Debug + Default + Send + Sync + 'static {
    /// 值类型 / Value type
    type Value: Clone + Debug + PartialOrd + Send + Sync + 'static;

    /// 获取变量类型枚举 / Get variable type enum
    fn var_type() -> VariableType;

    /// 获取最小值 / Get minimum value
    ///
    /// 返回 `None` 表示无下界。
    /// Returns `None` for no lower bound.
    fn min_value() -> Option<Self::Value>;

    /// 获取最大值 / Get maximum value
    ///
    /// 返回 `None` 表示无上界。
    /// Returns `None` for no upper bound.
    fn max_value() -> Option<Self::Value>;

    /// 获取默认取值范围 / Get default range
    ///
    /// 默认实现基于 `min_value` 和 `max_value`。
    /// Default implementation based on `min_value` and `max_value`.
    fn default_range() -> VariableRange<Self::Value> {
        VariableRange {
            lower_bound: Self::min_value(),
            upper_bound: Self::max_value(),
        }
    }

    /// 检查值是否有效 / Check if value is valid
    ///
    /// 检查值是否在允许的取值范围内。
    /// Checks if value is within allowed range.
    fn is_valid_value(value: &Self::Value) -> bool {
        let range = Self::default_range();
        let lower_ok = range.lower_bound.as_ref().is_none_or(|lb| value >= lb);
        let upper_ok = range.upper_bound.as_ref().is_none_or(|ub| value <= ub);
        lower_ok && upper_ok
    }

    /// 是否为整数变量 / Whether it's an integer variable
    fn is_integer() -> bool {
        false
    }

    /// 是否为有符号变量 / Whether it's a signed variable
    fn is_signed() -> bool {
        true
    }

    /// 获取类型名称 / Get type name
    fn type_name() -> &'static str;
}

// ============================================================================
// 变量类型标记实现 / Variable Type Marker Implementations
// ============================================================================

/// 二进制变量标记类型 / Binary variable marker type
///
/// 取值范围: {0, 1}
/// Value range: {0, 1}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Binary;

impl VariableTypeTrait for Binary {
    type Value = f64;

    fn var_type() -> VariableType {
        VariableType::Binary
    }
    fn min_value() -> Option<Self::Value> {
        Some(0.0)
    }
    fn max_value() -> Option<Self::Value> {
        Some(1.0)
    }
    fn is_integer() -> bool {
        true
    }
    fn type_name() -> &'static str {
        "Binary"
    }
}

/// 三元变量标记类型 / Ternary variable marker type
///
/// 取值范围: {0, 1, 2}
/// Value range: {0, 1, 2}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Ternary;

impl VariableTypeTrait for Ternary {
    type Value = f64;

    fn var_type() -> VariableType {
        VariableType::Ternary
    }
    fn min_value() -> Option<Self::Value> {
        Some(0.0)
    }
    fn max_value() -> Option<Self::Value> {
        Some(2.0)
    }
    fn is_integer() -> bool {
        true
    }
    fn type_name() -> &'static str {
        "Ternary"
    }
}

/// 平衡三元变量标记类型 / Balanced ternary variable marker type
///
/// 取值范围: {-1, 0, 1}
/// Value range: {-1, 0, 1}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BalancedTernary;

impl VariableTypeTrait for BalancedTernary {
    type Value = f64;

    fn var_type() -> VariableType {
        VariableType::BalancedTernary
    }
    fn min_value() -> Option<Self::Value> {
        Some(-1.0)
    }
    fn max_value() -> Option<Self::Value> {
        Some(1.0)
    }
    fn is_integer() -> bool {
        true
    }
    fn type_name() -> &'static str {
        "BalancedTernary"
    }
}

/// 百分比变量标记类型 / Percentage variable marker type
///
/// 取值范围: [0, 1]
/// Value range: [0, 1]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Percentage;

impl VariableTypeTrait for Percentage {
    type Value = f64;

    fn var_type() -> VariableType {
        VariableType::Percentage
    }
    fn min_value() -> Option<Self::Value> {
        Some(0.0)
    }
    fn max_value() -> Option<Self::Value> {
        Some(1.0)
    }
    fn is_integer() -> bool {
        false
    }
    fn type_name() -> &'static str {
        "Percentage"
    }
}

/// 整数变量标记类型 / Integer variable marker type
///
/// 取值范围: (-∞, +∞)，默认范围可自定义
/// Value range: (-∞, +∞), default range can be customized
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Integer;

impl VariableTypeTrait for Integer {
    type Value = f64;

    fn var_type() -> VariableType {
        VariableType::Integer
    }
    fn min_value() -> Option<Self::Value> {
        None
    }
    fn max_value() -> Option<Self::Value> {
        None
    }
    fn is_integer() -> bool {
        true
    }
    fn type_name() -> &'static str {
        "Integer"
    }
}

/// 无符号整数变量标记类型 / Unsigned integer variable marker type
///
/// 取值范围: [0, +∞)
/// Value range: [0, +∞)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct UInteger;

impl VariableTypeTrait for UInteger {
    type Value = f64;

    fn var_type() -> VariableType {
        VariableType::UInteger
    }
    fn min_value() -> Option<Self::Value> {
        Some(0.0)
    }
    fn max_value() -> Option<Self::Value> {
        None
    }
    fn is_integer() -> bool {
        true
    }
    fn is_signed() -> bool {
        false
    }
    fn type_name() -> &'static str {
        "UInteger"
    }
}

/// 连续变量标记类型 / Continuous variable marker type
///
/// 取值范围: (-∞, +∞)
/// Value range: (-∞, +∞)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Continuous;

impl VariableTypeTrait for Continuous {
    type Value = f64;

    fn var_type() -> VariableType {
        VariableType::Continuous
    }
    fn min_value() -> Option<Self::Value> {
        None
    }
    fn max_value() -> Option<Self::Value> {
        None
    }
    fn is_integer() -> bool {
        false
    }
    fn type_name() -> &'static str {
        "Continuous"
    }
}

/// 无符号连续变量标记类型 / Unsigned continuous variable marker type
///
/// 取值范围: [0, +∞)
/// Value range: [0, +∞)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct UContinuous;

impl VariableTypeTrait for UContinuous {
    type Value = f64;

    fn var_type() -> VariableType {
        VariableType::UContinuous
    }
    fn min_value() -> Option<Self::Value> {
        Some(0.0)
    }
    fn max_value() -> Option<Self::Value> {
        None
    }
    fn is_integer() -> bool {
        false
    }
    fn is_signed() -> bool {
        false
    }
    fn type_name() -> &'static str {
        "UContinuous"
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_type_enum() {
        assert!(VariableType::Binary.is_integer());
        assert!(VariableType::Binary.is_binary());
        assert!(!VariableType::Binary.is_continuous());

        assert!(VariableType::Continuous.is_continuous());
        assert!(!VariableType::Continuous.is_integer());

        assert!(VariableType::Integer.is_integer());
        assert!(!VariableType::Integer.is_continuous());
    }

    #[test]
    fn test_binary_trait() {
        assert_eq!(Binary::var_type(), VariableType::Binary);
        assert_eq!(Binary::min_value(), Some(0.0));
        assert_eq!(Binary::max_value(), Some(1.0));
        assert!(Binary::is_integer());
        assert!(Binary::is_valid_value(&0.0));
        assert!(Binary::is_valid_value(&1.0));
        assert!(Binary::is_valid_value(&0.5));
        assert!(!Binary::is_valid_value(&2.0));
        assert!(!Binary::is_valid_value(&-1.0));
    }

    #[test]
    fn test_continuous_trait() {
        assert_eq!(Continuous::var_type(), VariableType::Continuous);
        assert_eq!(Continuous::min_value(), None);
        assert_eq!(Continuous::max_value(), None);
        assert!(!Continuous::is_integer());
        assert!(Continuous::is_signed());
        // 连续变量任何值都有效（无边界限制）
        assert!(Continuous::is_valid_value(&0.0));
        assert!(Continuous::is_valid_value(&-100.0));
        assert!(Continuous::is_valid_value(&100.0));
    }

    #[test]
    fn test_ucontinuous_trait() {
        assert_eq!(UContinuous::var_type(), VariableType::UContinuous);
        assert_eq!(UContinuous::min_value(), Some(0.0));
        assert_eq!(UContinuous::max_value(), None);
        assert!(!UContinuous::is_integer());
        assert!(!UContinuous::is_signed());
        assert!(UContinuous::is_valid_value(&0.0));
        assert!(UContinuous::is_valid_value(&100.0));
        assert!(!UContinuous::is_valid_value(&-1.0));
    }

    #[test]
    fn test_integer_trait() {
        assert_eq!(Integer::var_type(), VariableType::Integer);
        assert_eq!(Integer::min_value(), None);
        assert_eq!(Integer::max_value(), None);
        assert!(Integer::is_integer());
        assert!(Integer::is_signed());
    }

    #[test]
    fn test_balanced_ternary_trait() {
        assert_eq!(BalancedTernary::var_type(), VariableType::BalancedTernary);
        assert_eq!(BalancedTernary::min_value(), Some(-1.0));
        assert_eq!(BalancedTernary::max_value(), Some(1.0));
        assert!(BalancedTernary::is_integer());
        assert!(BalancedTernary::is_signed());
    }
}
