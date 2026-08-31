//! ValueWrapper - 值包装器
//! ValueWrapper - Value wrapper

use crate::algebra::concept::Infinite;
use std::cmp::Ordering;
use std::fmt;

// ============================================================================
// ValueWrapper<T> - 值包装器
// ============================================================================

/// ValueWrapper - 值包装器
/// ValueWrapper - Value wrapper
///
/// 为任意数值类型添加无穷大支持。
/// Adds infinity support to any numeric type.
///
/// # 类型参数 / Type Parameters
/// - `T`: 被包装的数值类型
/// - `T`: The wrapped numeric type
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::value_range::ValueWrapper;
///
/// // 创建有限值
/// // Create finite value
/// let finite = ValueWrapper::finite(42_i64);
/// assert!(finite.is_finite());
/// assert_eq!(finite.unwrap(), Some(&42));
///
/// // 创建正无穷
/// // Create positive infinity
/// let pos_inf = ValueWrapper::<i64>::positive_infinity();
/// assert!(pos_inf.is_positive_infinity());
///
/// // 创建负无穷
/// // Create negative infinity
/// let neg_inf = ValueWrapper::<i64>::negative_infinity();
/// assert!(neg_inf.is_negative_infinity());
/// ```
#[derive(Clone, Debug)]
pub enum ValueWrapper<T> {
    /// 正常值 / Normal value
    Finite(T),
    /// 正无穷 / Positive infinity
    PositiveInfinity,
    /// 负无穷 / Negative infinity
    NegativeInfinity,
}

impl<T> ValueWrapper<T> {
    /// 创建有限值
    /// Create a finite value
    ///
    /// # 参数 / Parameters
    /// - `value`: 有限值
    /// - `value`: The finite value
    ///
    /// # 返回 / Returns
    /// 包装后的有限值
    /// The wrapped finite value
    pub fn finite(value: T) -> Self {
        ValueWrapper::Finite(value)
    }

    /// 创建正无穷
    /// Create positive infinity
    ///
    /// # 返回 / Returns
    /// 表示正无穷的包装值
    /// The wrapper representing positive infinity
    pub fn positive_infinity() -> Self {
        ValueWrapper::PositiveInfinity
    }

    /// 创建负无穷
    /// Create negative infinity
    ///
    /// # 返回 / Returns
    /// 表示负无穷的包装值
    /// The wrapper representing negative infinity
    pub fn negative_infinity() -> Self {
        ValueWrapper::NegativeInfinity
    }

    /// 判断是否为有限值
    /// Check if value is finite
    ///
    /// # 返回 / Returns
    /// 如果为有限值返回 `true`，否则返回 `false`
    /// Returns `true` if finite, `false` otherwise
    pub fn is_finite(&self) -> bool {
        matches!(self, ValueWrapper::Finite(_))
    }

    /// 判断是否为无穷大（正或负）
    /// Check if value is infinity (positive or negative)
    ///
    /// # 返回 / Returns
    /// 如果为无穷大返回 `true`，否则返回 `false`
    /// Returns `true` if infinity, `false` otherwise
    pub fn is_infinity(&self) -> bool {
        matches!(
            self,
            ValueWrapper::PositiveInfinity | ValueWrapper::NegativeInfinity
        )
    }

    /// 判断是否为正无穷
    /// Check if value is positive infinity
    ///
    /// # 返回 / Returns
    /// 如果为正无穷返回 `true`，否则返回 `false`
    /// Returns `true` if positive infinity, `false` otherwise
    pub fn is_positive_infinity(&self) -> bool {
        matches!(self, ValueWrapper::PositiveInfinity)
    }

    /// 判断是否为负无穷
    /// Check if value is negative infinity
    ///
    /// # 返回 / Returns
    /// 如果为负无穷返回 `true`，否则返回 `false`
    /// Returns `true` if negative infinity, `false` otherwise
    pub fn is_negative_infinity(&self) -> bool {
        matches!(self, ValueWrapper::NegativeInfinity)
    }

    /// 获取内部值（如果是有限值）
    /// Get the inner value (if finite)
    ///
    /// # 返回 / Returns
    /// 如果为有限值返回 `Some(&value)`，否则返回 `None`
    /// Returns `Some(&value)` if finite, `None` otherwise
    pub fn unwrap(&self) -> Option<&T> {
        match self {
            ValueWrapper::Finite(value) => Some(value),
            _ => None,
        }
    }

    /// 映射内部值（如果是有限值）
    /// Map the inner value (if finite)
    ///
    /// # 参数 / Parameters
    /// - `f`: 映射函数
    /// - `f`: The mapping function
    ///
    /// # 返回 / Returns
    /// 如果为有限值返回映射后的新值，否则保持无穷大状态
    /// Returns mapped value if finite, preserves infinity status otherwise
    pub fn map<U, F>(self, f: F) -> ValueWrapper<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            ValueWrapper::Finite(value) => ValueWrapper::Finite(f(value)),
            ValueWrapper::PositiveInfinity => ValueWrapper::PositiveInfinity,
            ValueWrapper::NegativeInfinity => ValueWrapper::NegativeInfinity,
        }
    }
}

// ============================================================================
// PartialEq 实现 / PartialEq implementation
// ============================================================================

impl<T: PartialEq> PartialEq for ValueWrapper<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ValueWrapper::Finite(a), ValueWrapper::Finite(b)) => a == b,
            (ValueWrapper::PositiveInfinity, ValueWrapper::PositiveInfinity) => true,
            (ValueWrapper::NegativeInfinity, ValueWrapper::NegativeInfinity) => true,
            _ => false,
        }
    }
}

impl<T: Eq> Eq for ValueWrapper<T> {}

// ============================================================================
// PartialOrd 实现 / PartialOrd implementation
// ============================================================================

impl<T: PartialOrd> PartialOrd for ValueWrapper<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (ValueWrapper::NegativeInfinity, ValueWrapper::NegativeInfinity) => {
                Some(Ordering::Equal)
            }
            (ValueWrapper::NegativeInfinity, _) => Some(Ordering::Less),
            (_, ValueWrapper::NegativeInfinity) => Some(Ordering::Greater),
            (ValueWrapper::PositiveInfinity, ValueWrapper::PositiveInfinity) => {
                Some(Ordering::Equal)
            }
            (ValueWrapper::PositiveInfinity, _) => Some(Ordering::Greater),
            (_, ValueWrapper::PositiveInfinity) => Some(Ordering::Less),
            (ValueWrapper::Finite(a), ValueWrapper::Finite(b)) => a.partial_cmp(b),
        }
    }
}

// ============================================================================
// Display 实现 / Display implementation
// ============================================================================

impl<T: fmt::Display> fmt::Display for ValueWrapper<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueWrapper::Finite(value) => write!(f, "{}", value),
            ValueWrapper::PositiveInfinity => write!(f, "+∞"),
            ValueWrapper::NegativeInfinity => write!(f, "-∞"),
        }
    }
}

// ============================================================================
// Default 实现 / Default implementation
// ============================================================================

impl<T: Default> Default for ValueWrapper<T> {
    fn default() -> Self {
        ValueWrapper::Finite(T::default())
    }
}

// ============================================================================
// 从原生类型转换 / From native type conversions
// ============================================================================

impl<T> From<T> for ValueWrapper<T> {
    fn from(value: T) -> Self {
        ValueWrapper::Finite(value)
    }
}

// ============================================================================
// Infinite trait 实现 / Infinite trait implementation
// ============================================================================

impl<T: Infinite> Infinite for ValueWrapper<T> {
    fn infinity() -> Option<Self> {
        Some(ValueWrapper::PositiveInfinity)
    }

    fn negative_infinity() -> Option<Self> {
        Some(ValueWrapper::NegativeInfinity)
    }

    fn is_infinity(&self) -> bool {
        self.is_infinity()
    }

    fn is_positive_infinity(&self) -> bool {
        self.is_positive_infinity()
    }

    fn is_negative_infinity(&self) -> bool {
        self.is_negative_infinity()
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // ValueWrapper 测试 / ValueWrapper tests
    // ========================================================================

    #[test]
    fn test_value_wrapper_finite() {
        let wrapper = ValueWrapper::finite(42_i64);
        assert!(wrapper.is_finite());
        assert!(!wrapper.is_infinity());
        assert_eq!(wrapper.unwrap(), Some(&42));
    }

    #[test]
    fn test_value_wrapper_positive_infinity() {
        let wrapper = ValueWrapper::<i64>::positive_infinity();
        assert!(!wrapper.is_finite());
        assert!(wrapper.is_infinity());
        assert!(wrapper.is_positive_infinity());
        assert!(!wrapper.is_negative_infinity());
        assert_eq!(wrapper.unwrap(), None);
    }

    #[test]
    fn test_value_wrapper_negative_infinity() {
        let wrapper = ValueWrapper::<i64>::negative_infinity();
        assert!(!wrapper.is_finite());
        assert!(wrapper.is_infinity());
        assert!(!wrapper.is_positive_infinity());
        assert!(wrapper.is_negative_infinity());
        assert_eq!(wrapper.unwrap(), None);
    }

    #[test]
    fn test_value_wrapper_eq() {
        let a = ValueWrapper::finite(42_i64);
        let b = ValueWrapper::finite(42_i64);
        let c = ValueWrapper::finite(43_i64);
        let pos_inf = ValueWrapper::<i64>::positive_infinity();
        let neg_inf = ValueWrapper::<i64>::negative_infinity();

        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, pos_inf);
        assert_ne!(pos_inf, neg_inf);
        assert_eq!(
            pos_inf,
            ValueWrapper::<i64>::positive_infinity()
        );
    }

    #[test]
    fn test_value_wrapper_ord() {
        let a = ValueWrapper::finite(42_i64);
        let b = ValueWrapper::finite(43_i64);
        let pos_inf = ValueWrapper::<i64>::positive_infinity();
        let neg_inf = ValueWrapper::<i64>::negative_infinity();

        assert!(a < b);
        assert!(neg_inf < a);
        assert!(a < pos_inf);
        assert!(neg_inf < pos_inf);
    }

    #[test]
    fn test_value_wrapper_display() {
        let finite = ValueWrapper::finite(42_i64);
        let pos_inf = ValueWrapper::<i64>::positive_infinity();
        let neg_inf = ValueWrapper::<i64>::negative_infinity();

        assert_eq!(format!("{}", finite), "42");
        assert_eq!(format!("{}", pos_inf), "+∞");
        assert_eq!(format!("{}", neg_inf), "-∞");
    }

    #[test]
    fn test_value_wrapper_map() {
        let finite = ValueWrapper::finite(42_i64);
        let mapped = finite.map(|x| x * 2);
        assert_eq!(mapped.unwrap(), Some(&84));

        let pos_inf = ValueWrapper::<i64>::positive_infinity();
        let mapped_inf = pos_inf.map(|x: i64| x * 2);
        assert!(mapped_inf.is_positive_infinity());
    }

    #[test]
    fn test_value_wrapper_from() {
        let wrapper: ValueWrapper<i64> = ValueWrapper::from(42);
        assert!(wrapper.is_finite());
        assert_eq!(wrapper.unwrap(), Some(&42));
    }

    #[test]
    fn test_value_wrapper_infinite_trait() {
        assert!(ValueWrapper::<i64>::infinity().is_some());
        assert!(ValueWrapper::<i64>::negative_infinity().is_some());

        let pos_inf = ValueWrapper::<i64>::infinity().unwrap();
        let neg_inf = ValueWrapper::<i64>::negative_infinity().unwrap();

        assert!(pos_inf.is_positive_infinity());
        assert!(neg_inf.is_negative_infinity());
    }
}