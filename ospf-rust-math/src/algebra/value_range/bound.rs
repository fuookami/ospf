//! Bound - 边界
//! Bound - Boundary

use super::interval::{Interval, IntervalTrait};
use super::value_wrapper::ValueWrapper;
use crate::operator::tolerance::{Tolerance, TolerancedEq, TolerancedOrd};
use std::fmt;

// ============================================================================
// Bound<T, I> - 边界
// ============================================================================

/// Bound - 边界
/// Bound - Boundary
///
/// 表示区间的单个边界，包含值和开闭性质。
/// Represents a single boundary of an interval, containing value and openness.
///
/// # 类型参数 / Type Parameters
/// - `T`: 边界值的类型
/// - `I`: 开闭性质类型（编译时 `Closed`/`Open` 或运行时 `Interval`）
/// - `T`: The boundary value type
/// - `I`: The openness type (compile-time `Closed`/`Open` or runtime `Interval`)
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::value_range::{Bound, ValueWrapper, Interval, Closed, Open};
///
/// // 编译时开闭性质
/// // Compile-time openness
/// let closed_bound = Bound::new(ValueWrapper::finite(10_i64), Closed);
/// assert!(closed_bound.is_closed());
/// assert_eq!(closed_bound.value().unwrap(), Some(&10));
///
/// // 运行时开闭性质
/// // Runtime openness
/// let open_bound: Bound<i64, Interval> = Bound::new(ValueWrapper::finite(10_i64), Interval::Open);
/// assert!(open_bound.is_open());
/// ```
#[derive(Clone, Debug)]
pub struct Bound<T, I: IntervalTrait = Interval> {
    /// 边界值
    /// Boundary value
    value: ValueWrapper<T>,
    /// 开闭性质
    /// Openness
    interval: I,
}

impl<T, I: IntervalTrait> Bound<T, I> {
    /// 创建新的边界
    /// Create a new boundary
    ///
    /// # 参数 / Parameters
    /// - `value`: 边界值
    /// - `interval`: 开闭性质
    ///
    /// # 返回 / Returns
    /// 新的边界实例
    /// New boundary instance
    pub fn new(value: ValueWrapper<T>, interval: I) -> Self {
        Self { value, interval }
    }

    /// 创建闭区间边界
    /// Create a closed boundary
    ///
    /// # 参数 / Parameters
    /// - `value`: 边界值
    ///
    /// # 返回 / Returns
    /// 闭区间边界
    /// Closed boundary
    pub fn closed(value: ValueWrapper<T>) -> Self
    where
        I: Default,
    {
        Self {
            value,
            interval: I::default(),
        }
    }

    /// 获取边界值
    /// Get the boundary value
    ///
    /// # 返回 / Returns
    /// 边界值的引用
    /// Reference to the boundary value
    pub fn value(&self) -> &ValueWrapper<T> {
        &self.value
    }

    /// 获取边界值（按值移动）
    /// Take ownership of the boundary value
    pub fn into_value(self) -> ValueWrapper<T> {
        self.value
    }

    /// 获取开闭性质
    /// Get the openness
    ///
    /// # 返回 / Returns
    /// 开闭性质
    /// The openness
    pub fn interval(&self) -> I {
        self.interval
    }

    /// 判断是否为闭区间边界
    /// Check if it's a closed boundary
    ///
    /// # 返回 / Returns
    /// 如果为闭区间返回 `true`，否则返回 `false`
    /// Returns `true` if closed, `false` otherwise
    pub fn is_closed(&self) -> bool {
        self.interval.is_closed()
    }

    /// 判断是否为开区间边界
    /// Check if it's an open boundary
    ///
    /// # 返回 / Returns
    /// 如果为开区间返回 `true`，否则返回 `false`
    /// Returns `true` if open, `false` otherwise
    pub fn is_open(&self) -> bool {
        self.interval.is_open()
    }

    /// 判断边界值是否为有限值
    /// Check if boundary value is finite
    ///
    /// # 返回 / Returns
    /// 如果边界值为有限值返回 `true`，否则返回 `false`
    /// Returns `true` if finite, `false` otherwise
    pub fn is_finite(&self) -> bool {
        self.value.is_finite()
    }

    /// 判断边界值是否为无穷大
    /// Check if boundary value is infinity
    ///
    /// # 返回 / Returns
    /// 如果边界值为无穷大返回 `true`，否则返回 `false`
    /// Returns `true` if infinity, `false` otherwise
    pub fn is_infinity(&self) -> bool {
        self.value.is_infinity()
    }

    /// 判断边界值是否为正无穷
    /// Check if boundary value is positive infinity
    ///
    /// # 返回 / Returns
    /// 如果边界值为正无穷返回 `true`，否则返回 `false`
    /// Returns `true` if positive infinity, `false` otherwise
    pub fn is_positive_infinity(&self) -> bool {
        self.value.is_positive_infinity()
    }

    /// 判断边界值是否为负无穷
    /// Check if boundary value is negative infinity
    ///
    /// # 返回 / Returns
    /// 如果边界值为负无穷返回 `true`，否则返回 `false`
    /// Returns `true` if negative infinity, `false` otherwise
    pub fn is_negative_infinity(&self) -> bool {
        self.value.is_negative_infinity()
    }

    /// 映射边界值
    /// Map the boundary value
    ///
    /// # 参数 / Parameters
    /// - `f`: 映射函数
    /// - `f`: The mapping function
    ///
    /// # 返回 / Returns
    /// 映射后的新边界
    /// New boundary after mapping
    pub fn map<U, F>(self, f: F) -> Bound<U, I>
    where
        F: FnOnce(T) -> U,
    {
        Bound {
            value: self.value.map(f),
            interval: self.interval,
        }
    }
}

// ============================================================================
// PartialEq 实现 / PartialEq implementation
// ============================================================================

impl<T: PartialEq, I: IntervalTrait> PartialEq for Bound<T, I> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.interval == other.interval
    }
}

impl<T: Eq, I: IntervalTrait + Eq> Eq for Bound<T, I> {}

// ============================================================================
// PartialOrd 实现 / PartialOrd implementation
// ============================================================================

impl<T: PartialOrd, I: IntervalTrait> PartialOrd for Bound<T, I> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

// ============================================================================
// Display 实现 / Display implementation
// ============================================================================

impl<T: fmt::Display, I: IntervalTrait> fmt::Display for Bound<T, I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.interval.lower_sign(), self.value)
    }
}

// ============================================================================
// Default 实现 / Default implementation
// ============================================================================

impl<T: Default, I: IntervalTrait + Default> Default for Bound<T, I> {
    fn default() -> Self {
        Self {
            value: ValueWrapper::finite(T::default()),
            interval: I::default(),
        }
    }
}

// ============================================================================
// TolerancedEq 实现 / TolerancedEq implementation
// ============================================================================

impl<T: TolerancedEq, I: IntervalTrait + PartialEq> TolerancedEq for Bound<T, I> {
    type Value = T::Value;

    fn eq_within(&self, other: &Self, tolerance: &Tolerance<Self::Value>) -> bool {
        // 值相等（使用 tolerance）且开闭性质相同
        // Values are equal (using tolerance) and openness is the same
        self.value.eq_within(&other.value, tolerance) && self.interval == other.interval
    }
}

// ============================================================================
// 辅助方法 / Helper methods
// ============================================================================

impl<T: PartialOrd, I: IntervalTrait> Bound<T, I> {
    /// 判断值是否在边界正确的一侧（用于下界）
    /// Check if value is on the correct side of boundary (for lower bound)
    ///
    /// 对于下界，值应该 >= 边界值（闭区间）或 > 边界值（开区间）
    /// For lower bound, value should be >= boundary (closed) or > boundary (open)
    ///
    /// # 参数 / Parameters
    /// - `value`: 要检查的值
    /// - `value`: The value to check
    ///
    /// # 返回 / Returns
    /// 如果值在下界正确的一侧返回 `true`，否则返回 `false`
    /// Returns `true` if value is on correct side, `false` otherwise
    pub fn is_above(&self, value: &ValueWrapper<T>) -> bool {
        match (&self.value, value) {
            // 负无穷下界：有限值和正无穷在正确的一侧，负无穷取决于开闭性质
            // Negative infinity lower bound: finite values and positive infinity are on correct side
            // Negative infinity depends on openness
            (ValueWrapper::NegativeInfinity, ValueWrapper::NegativeInfinity) => self.is_closed(),
            (ValueWrapper::NegativeInfinity, _) => true,
            // 正无穷下界：没有值在正确的一侧（闭区间时只有正无穷自身）
            // Positive infinity lower bound: no value is on correct side (only +∞ itself when closed)
            (ValueWrapper::PositiveInfinity, ValueWrapper::PositiveInfinity) => self.is_closed(),
            (ValueWrapper::PositiveInfinity, _) => false,
            // 有限值下界
            // Finite lower bound
            (ValueWrapper::Finite(bound), ValueWrapper::Finite(v)) => {
                if self.is_closed() {
                    v >= bound
                } else {
                    v > bound
                }
            }
            // 有限值下界 vs 无穷大值
            // Finite lower bound vs infinity value
            (ValueWrapper::Finite(_), ValueWrapper::PositiveInfinity) => true,
            (ValueWrapper::Finite(_), ValueWrapper::NegativeInfinity) => false,
        }
    }

    /// 判断值是否在边界正确的一侧（用于上界）
    /// Check if value is on the correct side of boundary (for upper bound)
    ///
    /// 对于上界，值应该 <= 边界值（闭区间）或 < 边界值（开区间）
    /// For upper bound, value should be <= boundary (closed) or < boundary (open)
    ///
    /// # 参数 / Parameters
    /// - `value`: 要检查的值
    /// - `value`: The value to check
    ///
    /// # 返回 / Returns
    /// 如果值在上界正确的一侧返回 `true`，否则返回 `false`
    /// Returns `true` if value is on correct side, `false` otherwise
    pub fn is_below(&self, value: &ValueWrapper<T>) -> bool {
        match (&self.value, value) {
            // 正无穷上界：有限值和负无穷在正确的一侧，正无穷取决于开闭性质
            // Positive infinity upper bound: finite values and negative infinity are on correct side
            // Positive infinity depends on openness
            (ValueWrapper::PositiveInfinity, ValueWrapper::PositiveInfinity) => self.is_closed(),
            (ValueWrapper::PositiveInfinity, _) => true,
            // 负无穷上界：没有值在正确的一侧（闭区间时只有负无穷自身）
            // Negative infinity upper bound: no value is on correct side (only -∞ itself when closed)
            (ValueWrapper::NegativeInfinity, ValueWrapper::NegativeInfinity) => self.is_closed(),
            (ValueWrapper::NegativeInfinity, _) => false,
            // 有限值上界
            // Finite upper bound
            (ValueWrapper::Finite(bound), ValueWrapper::Finite(v)) => {
                if self.is_closed() {
                    v <= bound
                } else {
                    v < bound
                }
            }
            // 有限值上界 vs 无穷大值
            // Finite upper bound vs infinity value
            (ValueWrapper::Finite(_), ValueWrapper::PositiveInfinity) => false,
            (ValueWrapper::Finite(_), ValueWrapper::NegativeInfinity) => true,
        }
    }
}

// ============================================================================
// Tolerance 版本辅助方法 / Tolerance version helper methods
// ============================================================================

impl<T: TolerancedOrd, I: IntervalTrait> Bound<T, I> {
    /// 判断值是否在边界正确的一侧（用于下界，带精度容差）
    /// Check if value is on the correct side of boundary (for lower bound, with tolerance)
    ///
    /// 对于下界，值应该 >= 边界值（闭区间）或 > 边界值（开区间）
    /// For lower bound, value should be >= boundary (closed) or > boundary (open)
    ///
    /// # 参数 / Parameters
    /// - `value`: 要检查的值
    /// - `value`: The value to check
    /// - `tolerance`: 精度容差
    ///
    /// # 返回 / Returns
    /// 如果值在下界正确的一侧返回 `true`，否则返回 `false`
    /// Returns `true` if value is on correct side, `false` otherwise
    pub fn is_above_within(
        &self,
        value: &ValueWrapper<T>,
        tolerance: &Tolerance<T::Value>,
    ) -> bool {
        match (&self.value, value) {
            // 负无穷下界：任何值都在正确的一侧
            // Negative infinity lower bound: any value is on correct side
            (ValueWrapper::NegativeInfinity, _) => true,
            // 正无穷下界：没有值在正确的一侧
            // Positive infinity lower bound: no value is on correct side
            (ValueWrapper::PositiveInfinity, _) => false,
            // 有限值下界
            // Finite lower bound
            (ValueWrapper::Finite(bound), ValueWrapper::Finite(v)) => {
                self.interval.is_above_boundary_within(v, bound, tolerance)
            }
            // 有限值下界 vs 无穷大值
            // Finite lower bound vs infinity value
            (ValueWrapper::Finite(_), ValueWrapper::PositiveInfinity) => true,
            (ValueWrapper::Finite(_), ValueWrapper::NegativeInfinity) => false,
        }
    }

    /// 判断值是否在边界正确的一侧（用于上界，带精度容差）
    /// Check if value is on the correct side of boundary (for upper bound, with tolerance)
    ///
    /// 对于上界，值应该 <= 边界值（闭区间）或 < 边界值（开区间）
    /// For upper bound, value should be <= boundary (closed) or < boundary (open)
    ///
    /// # 参数 / Parameters
    /// - `value`: 要检查的值
    /// - `value`: The value to check
    /// - `tolerance`: 精度容差
    ///
    /// # 返回 / Returns
    /// 如果值在上界正确的一侧返回 `true`，否则返回 `false`
    /// Returns `true` if value is on correct side, `false` otherwise
    pub fn is_below_within(
        &self,
        value: &ValueWrapper<T>,
        tolerance: &Tolerance<T::Value>,
    ) -> bool {
        match (&self.value, value) {
            // 正无穷上界：任何值都在正确的一侧
            // Positive infinity upper bound: any value is on correct side
            (ValueWrapper::PositiveInfinity, _) => true,
            // 负无穷上界：没有值在正确的一侧
            // Negative infinity upper bound: no value is on correct side
            (ValueWrapper::NegativeInfinity, _) => false,
            // 有限值上界
            // Finite upper bound
            (ValueWrapper::Finite(bound), ValueWrapper::Finite(v)) => {
                self.interval.is_below_boundary_within(v, bound, tolerance)
            }
            // 有限值上界 vs 无穷大值
            // Finite upper bound vs infinity value
            (ValueWrapper::Finite(_), ValueWrapper::PositiveInfinity) => false,
            (ValueWrapper::Finite(_), ValueWrapper::NegativeInfinity) => true,
        }
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::value_range::{Closed, Interval, Open};

    // ========================================================================
    // 基本功能测试 / Basic functionality tests
    // ========================================================================

    #[test]
    fn test_bound_closed() {
        let bound = Bound::new(ValueWrapper::finite(10_i64), Closed);
        assert!(bound.is_closed());
        assert!(!bound.is_open());
        assert!(bound.is_finite());
        assert!(!bound.is_infinity());
        assert_eq!(bound.value().unwrap(), Some(&10));
    }

    #[test]
    fn test_bound_open() {
        let bound = Bound::new(ValueWrapper::finite(10_i64), Open);
        assert!(bound.is_open());
        assert!(!bound.is_closed());
        assert!(bound.is_finite());
    }

    #[test]
    fn test_bound_infinity() {
        let pos_inf = Bound::new(ValueWrapper::<i64>::positive_infinity(), Open);
        assert!(pos_inf.is_positive_infinity());
        assert!(pos_inf.is_infinity());
        assert!(!pos_inf.is_finite());

        let neg_inf = Bound::new(ValueWrapper::<i64>::negative_infinity(), Closed);
        assert!(neg_inf.is_negative_infinity());
        assert!(neg_inf.is_infinity());
        assert!(!neg_inf.is_finite());
    }

    #[test]
    fn test_bound_interval_runtime() {
        let closed = Bound::new(ValueWrapper::finite(10_i64), Interval::Closed);
        assert!(closed.is_closed());

        let open = Bound::new(ValueWrapper::finite(10_i64), Interval::Open);
        assert!(open.is_open());
    }

    // ========================================================================
    // 相等性测试 / Equality tests
    // ========================================================================

    #[test]
    fn test_bound_eq() {
        let a = Bound::new(ValueWrapper::finite(10_i64), Closed);
        let b = Bound::new(ValueWrapper::finite(10_i64), Closed);
        let d = Bound::new(ValueWrapper::finite(20_i64), Closed);

        assert_eq!(a, b);
        assert_ne!(a, d); // 不同值

        // 测试不同开闭性质（使用运行时 Interval）
        let closed_bound: Bound<i64, Interval> =
            Bound::new(ValueWrapper::finite(10_i64), Interval::Closed);
        let open_bound: Bound<i64, Interval> =
            Bound::new(ValueWrapper::finite(10_i64), Interval::Open);
        assert_ne!(closed_bound, open_bound);
    }

    // ========================================================================
    // 比较测试 / Comparison tests
    // ========================================================================

    #[test]
    fn test_bound_ord() {
        let a = Bound::new(ValueWrapper::finite(10_i64), Closed);
        let b = Bound::new(ValueWrapper::finite(20_i64), Closed);
        let pos_inf = Bound::new(ValueWrapper::<i64>::positive_infinity(), Closed);
        let neg_inf = Bound::new(ValueWrapper::<i64>::negative_infinity(), Closed);

        assert!(a < b);
        assert!(neg_inf < a);
        assert!(a < pos_inf);
    }

    // ========================================================================
    // is_above / is_below 测试 / is_above / is_below tests
    // ========================================================================

    #[test]
    fn test_bound_is_above_closed() {
        let bound = Bound::new(ValueWrapper::finite(10_i64), Closed);

        // 闭区间：值 >= 边界值
        assert!(bound.is_above(&ValueWrapper::finite(10_i64))); // 等于边界
        assert!(bound.is_above(&ValueWrapper::finite(15_i64))); // 大于边界
        assert!(!bound.is_above(&ValueWrapper::finite(5_i64))); // 小于边界
    }

    #[test]
    fn test_bound_is_above_open() {
        let bound = Bound::new(ValueWrapper::finite(10_i64), Open);

        // 开区间：值 > 边界值
        assert!(!bound.is_above(&ValueWrapper::finite(10_i64))); // 等于边界
        assert!(bound.is_above(&ValueWrapper::finite(15_i64))); // 大于边界
        assert!(!bound.is_above(&ValueWrapper::finite(5_i64))); // 小于边界
    }

    #[test]
    fn test_bound_is_below_closed() {
        let bound = Bound::new(ValueWrapper::finite(10_i64), Closed);

        // 闭区间：值 <= 边界值
        assert!(bound.is_below(&ValueWrapper::finite(10_i64))); // 等于边界
        assert!(bound.is_below(&ValueWrapper::finite(5_i64))); // 小于边界
        assert!(!bound.is_below(&ValueWrapper::finite(15_i64))); // 大于边界
    }

    #[test]
    fn test_bound_is_below_open() {
        let bound = Bound::new(ValueWrapper::finite(10_i64), Open);

        // 开区间：值 < 边界值
        assert!(!bound.is_below(&ValueWrapper::finite(10_i64))); // 等于边界
        assert!(bound.is_below(&ValueWrapper::finite(5_i64))); // 小于边界
        assert!(!bound.is_below(&ValueWrapper::finite(15_i64))); // 大于边界
    }

    #[test]
    fn test_bound_is_above_infinity() {
        let neg_inf_bound = Bound::new(ValueWrapper::<i64>::negative_infinity(), Open);
        let pos_inf_bound = Bound::new(ValueWrapper::<i64>::positive_infinity(), Open);

        // 负无穷下界：任何值都在正确的一侧
        assert!(neg_inf_bound.is_above(&ValueWrapper::finite(100_i64)));
        assert!(neg_inf_bound.is_above(&ValueWrapper::<i64>::positive_infinity()));

        // 正无穷下界：没有值在正确的一侧
        assert!(!pos_inf_bound.is_above(&ValueWrapper::finite(100_i64)));
        assert!(!pos_inf_bound.is_above(&ValueWrapper::<i64>::negative_infinity()));
    }

    #[test]
    fn test_bound_is_below_infinity() {
        let pos_inf_bound = Bound::new(ValueWrapper::<i64>::positive_infinity(), Open);
        let neg_inf_bound = Bound::new(ValueWrapper::<i64>::negative_infinity(), Open);

        // 正无穷上界：任何值都在正确的一侧
        assert!(pos_inf_bound.is_below(&ValueWrapper::finite(100_i64)));
        assert!(pos_inf_bound.is_below(&ValueWrapper::<i64>::negative_infinity()));

        // 负无穷上界：没有值在正确的一侧
        assert!(!neg_inf_bound.is_below(&ValueWrapper::finite(100_i64)));
        assert!(!neg_inf_bound.is_below(&ValueWrapper::<i64>::positive_infinity()));
    }

    // ========================================================================
    // Display 测试 / Display tests
    // ========================================================================

    #[test]
    fn test_bound_display() {
        let closed = Bound::new(ValueWrapper::finite(10_i64), Closed);
        let open = Bound::new(ValueWrapper::finite(10_i64), Open);
        let pos_inf = Bound::new(ValueWrapper::<i64>::positive_infinity(), Open);

        assert_eq!(format!("{}", closed), "[10");
        assert_eq!(format!("{}", open), "(10");
        assert_eq!(format!("{}", pos_inf), "(+∞");
    }

    // ========================================================================
    // map 测试 / map tests
    // ========================================================================

    #[test]
    fn test_bound_map() {
        let bound = Bound::new(ValueWrapper::finite(10_i64), Closed);
        let mapped = bound.map(|x| x * 2);

        assert_eq!(mapped.value().unwrap(), Some(&20));
        assert!(mapped.is_closed());
    }
}
