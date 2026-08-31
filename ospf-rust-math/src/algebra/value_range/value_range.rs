//! ValueRange - 值空间/区间
//! ValueRange - Value range / interval

use super::bound::Bound;
use super::interval::IntervalTrait;
use super::value_wrapper::ValueWrapper;
use crate::algebra::concept::{Bounded, Fixed};
use crate::operator::Contains;
use std::fmt;

// ============================================================================
// ValueRange<T, IL, IU> - 值空间/区间
// ============================================================================

/// ValueRange - 值空间/区间
/// ValueRange - Value range / interval
///
/// 表示一个数值区间，支持下界和上界，以及开闭性质。
/// Represents a numeric interval, supporting lower and upper bounds with openness.
///
/// # 类型参数 / Type Parameters
/// - `T`: 数值类型
/// - `IL`: 下界开闭性质类型（编译时 `Closed`/`Open` 或运行时 `Interval`）
/// - `IU`: 上界开闭性质类型（编译时 `Closed`/`Open` 或运行时 `Interval`）
/// - `T`: The numeric type
/// - `IL`: Lower bound openness type (compile-time `Closed`/`Open` or runtime `Interval`)
/// - `IU`: Upper bound openness type (compile-time `Closed`/`Open` or runtime `Interval`)
///
/// # 示例 / Examples
///
/// ## 编译时开闭性质（零开销）
/// ## Compile-time openness (zero overhead)
///
/// ```
/// use ospf_rust_math::algebra::value_range::{ValueRange, Bound, ValueWrapper, Closed, Open};
///
/// // 闭区间 [1, 10]
/// let range: ValueRange<i64, Closed, Closed> = ValueRange::new(
///     Bound::new(ValueWrapper::finite(1), Closed),
///     Bound::new(ValueWrapper::finite(10), Closed),
/// );
///
/// // 左闭右开区间 [1, 10)
/// let range: ValueRange<i64, Closed, Open> = ValueRange::new(
///     Bound::new(ValueWrapper::finite(1), Closed),
///     Bound::new(ValueWrapper::finite(10), Open),
/// );
/// ```
///
/// ## 运行时开闭性质（灵活）
/// ## Runtime openness (flexible)
///
/// ```
/// use ospf_rust_math::algebra::value_range::{ValueRange, Bound, ValueWrapper, Interval};
///
/// // [0, +∞) - 半无限区间
/// let range: ValueRange<i64> = ValueRange::new(
///     Bound::new(ValueWrapper::finite(0), Interval::Closed),
///     Bound::new(ValueWrapper::positive_infinity(), Interval::Open),
/// );
/// ```
#[derive(Clone, Debug)]
pub struct ValueRange<T, IL: IntervalTrait = super::interval::Interval, IU: IntervalTrait = super::interval::Interval> {
    /// 下界
    /// Lower bound
    lower_bound: Bound<T, IL>,
    /// 上界
    /// Upper bound
    upper_bound: Bound<T, IU>,
}

impl<T, IL: IntervalTrait, IU: IntervalTrait> ValueRange<T, IL, IU> {
    /// 创建新的值区间
    /// Create a new value range
    ///
    /// # 参数 / Parameters
    /// - `lower_bound`: 下界
    /// - `upper_bound`: 上界
    ///
    /// # 返回 / Returns
    /// 新的值区间实例
    /// New value range instance
    pub fn new(lower_bound: Bound<T, IL>, upper_bound: Bound<T, IU>) -> Self {
        Self {
            lower_bound,
            upper_bound,
        }
    }

    /// 获取下界
    /// Get the lower bound
    ///
    /// # 返回 / Returns
    /// 下界的引用
    /// Reference to the lower bound
    pub fn lower_bound(&self) -> &Bound<T, IL> {
        &self.lower_bound
    }

    /// 获取上界
    /// Get the upper bound
    ///
    /// # 返回 / Returns
    /// 上界的引用
    /// Reference to the upper bound
    pub fn upper_bound(&self) -> &Bound<T, IU> {
        &self.upper_bound
    }

    /// 判断下界是否为闭区间
    /// Check if lower bound is closed
    ///
    /// # 返回 / Returns
    /// 如果下界为闭区间返回 `true`，否则返回 `false`
    /// Returns `true` if lower bound is closed, `false` otherwise
    pub fn is_lower_closed(&self) -> bool {
        self.lower_bound.is_closed()
    }

    /// 判断上界是否为闭区间
    /// Check if upper bound is closed
    ///
    /// # 返回 / Returns
    /// 如果上界为闭区间返回 `true`，否则返回 `false`
    /// Returns `true` if upper bound is closed, `false` otherwise
    pub fn is_upper_closed(&self) -> bool {
        self.upper_bound.is_closed()
    }

    /// 判断下界是否为开区间
    /// Check if lower bound is open
    ///
    /// # 返回 / Returns
    /// 如果下界为开区间返回 `true`，否则返回 `false`
    /// Returns `true` if lower bound is open, `false` otherwise
    pub fn is_lower_open(&self) -> bool {
        self.lower_bound.is_open()
    }

    /// 判断上界是否为开区间
    /// Check if upper bound is open
    ///
    /// # 返回 / Returns
    /// 如果上界为开区间返回 `true`，否则返回 `false`
    /// Returns `true` if upper bound is open, `false` otherwise
    pub fn is_upper_open(&self) -> bool {
        self.upper_bound.is_open()
    }
}

impl<T: PartialOrd, IL: IntervalTrait, IU: IntervalTrait> ValueRange<T, IL, IU> {
    /// 判断值是否在区间内
    /// Check if value is within the range
    ///
    /// # 参数 / Parameters
    /// - `value`: 要检查的值
    /// - `value`: The value to check
    ///
    /// # 返回 / Returns
    /// 如果值在区间内返回 `true`，否则返回 `false`
    /// Returns `true` if value is within range, `false` otherwise
    pub fn contains_value(&self, value: &ValueWrapper<T>) -> bool {
        self.lower_bound.is_above(value) && self.upper_bound.is_below(value)
    }
}

// ============================================================================
// Bounded trait 实现 / Bounded trait implementation
// ============================================================================

impl<T, IL: IntervalTrait, IU: IntervalTrait> Bounded for ValueRange<T, IL, IU> {
    fn is_bounded() -> bool {
        // ValueRange 本身是否是有界的取决于其边界是否是有限值
        // 但这个方法返回的是类型的性质，不是实例的性质
        // 对于 ValueRange，我们总是返回 true，因为它总是有一个定义的边界
        // ValueRange's boundedness depends on whether its bounds are finite
        // But this method returns the type's property, not the instance's property
        // For ValueRange, we always return true since it always has defined bounds
        true
    }
}

// ============================================================================
// Fixed trait 实现 / Fixed trait implementation
// ============================================================================

impl<T: PartialEq, IL: IntervalTrait, IU: IntervalTrait> Fixed for ValueRange<T, IL, IU> {
    fn is_fixed() -> bool {
        // 类型层面上，ValueRange 不是固定的
        // 实例层面上，只有当上下界相等且都是闭区间时才固定
        // At type level, ValueRange is not fixed
        // At instance level, it's fixed only when bounds are equal and both closed
        false
    }
}

impl<T: PartialEq, IL: IntervalTrait, IU: IntervalTrait> ValueRange<T, IL, IU> {
    /// 判断区间是否退化为单点
    /// Check if the range degenerates to a single point
    ///
    /// # 返回 / Returns
    /// 如果区间退化为单点返回 `true`，否则返回 `false`
    /// Returns `true` if degenerates to a point, `false` otherwise
    pub fn is_degenerate(&self) -> bool {
        // 上下界相等且都是闭区间时，区间退化为单点
        // When bounds are equal and both closed, the range degenerates to a point
        self.lower_bound.value() == self.upper_bound.value()
            && self.lower_bound.is_closed()
            && self.upper_bound.is_closed()
    }

    /// 获取退化点值（如果存在）
    /// Get the degenerate point value (if exists)
    ///
    /// # 返回 / Returns
    /// 如果区间退化为单点返回 `Some(value)`，否则返回 `None`
    /// Returns `Some(value)` if degenerates to a point, `None` otherwise
    pub fn degenerate_value(&self) -> Option<&T> {
        if self.is_degenerate() {
            self.lower_bound.value().unwrap()
        } else {
            None
        }
    }
}

// ============================================================================
// Contains trait 实现 / Contains trait implementation
// ============================================================================

impl<T: PartialOrd, IL: IntervalTrait, IU: IntervalTrait> Contains<ValueWrapper<T>> for ValueRange<T, IL, IU> {
    fn contains(&self, value: &ValueWrapper<T>) -> bool {
        self.contains_value(value)
    }
}

impl<T: PartialOrd + Clone, IL: IntervalTrait, IU: IntervalTrait> Contains<T> for ValueRange<T, IL, IU> {
    fn contains(&self, value: &T) -> bool {
        self.contains_value(&ValueWrapper::finite(value.clone()))
    }
}

// ============================================================================
// PartialEq 实现 / PartialEq implementation
// ============================================================================

impl<T: PartialEq, IL: IntervalTrait, IU: IntervalTrait> PartialEq for ValueRange<T, IL, IU>
where
    IL: PartialEq,
    IU: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.lower_bound == other.lower_bound && self.upper_bound == other.upper_bound
    }
}

impl<T: Eq, IL: IntervalTrait + Eq, IU: IntervalTrait + Eq> Eq for ValueRange<T, IL, IU> {}

// ============================================================================
// Display 实现 / Display implementation
// ============================================================================

impl<T: fmt::Display, IL: IntervalTrait, IU: IntervalTrait> fmt::Display for ValueRange<T, IL, IU> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}, {}{}",
            self.lower_bound.interval().lower_sign(),
            self.lower_bound.value(),
            self.upper_bound.value(),
            self.upper_bound.interval().upper_sign()
        )
    }
}

// ============================================================================
// Default 实现 / Default implementation
// ============================================================================

impl<T: Default, IL: IntervalTrait + Default, IU: IntervalTrait + Default> Default for ValueRange<T, IL, IU> {
    fn default() -> Self {
        Self {
            lower_bound: Bound::default(),
            upper_bound: Bound::default(),
        }
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::value_range::{Closed, Open, Interval};

    // ========================================================================
    // 基本功能测试 / Basic functionality tests
    // ========================================================================

    #[test]
    fn test_value_range_closed() {
        let range: ValueRange<i64, Closed, Closed> = ValueRange::new(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(10), Closed),
        );

        assert!(range.is_lower_closed());
        assert!(range.is_upper_closed());
        assert!(!range.is_lower_open());
        assert!(!range.is_upper_open());
    }

    #[test]
    fn test_value_range_open() {
        let range: ValueRange<i64, Open, Open> = ValueRange::new(
            Bound::new(ValueWrapper::finite(1), Open),
            Bound::new(ValueWrapper::finite(10), Open),
        );

        assert!(range.is_lower_open());
        assert!(range.is_upper_open());
        assert!(!range.is_lower_closed());
        assert!(!range.is_upper_closed());
    }

    #[test]
    fn test_value_range_mixed_compile_time() {
        // [1, 10) - 左闭右开
        let range: ValueRange<i64, Closed, Open> = ValueRange::new(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(10), Open),
        );

        assert!(range.is_lower_closed());
        assert!(range.is_upper_open());

        // (1, 10] - 左开右闭
        let range: ValueRange<i64, Open, Closed> = ValueRange::new(
            Bound::new(ValueWrapper::finite(1), Open),
            Bound::new(ValueWrapper::finite(10), Closed),
        );

        assert!(range.is_lower_open());
        assert!(range.is_upper_closed());
    }

    #[test]
    fn test_value_range_runtime_interval() {
        let range: ValueRange<i64> = ValueRange::new(
            Bound::new(ValueWrapper::finite(1), Interval::Closed),
            Bound::new(ValueWrapper::finite(10), Interval::Open),
        );

        assert!(range.is_lower_closed());
        assert!(range.is_upper_open());
    }

    // ========================================================================
    // contains 测试 / contains tests
    // ========================================================================

    #[test]
    fn test_contains_closed() {
        let range: ValueRange<i64, Closed, Closed> = ValueRange::new(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(10), Closed),
        );

        // 闭区间 [1, 10]
        assert!(range.contains_value(&ValueWrapper::finite(1))); // 边界值
        assert!(range.contains_value(&ValueWrapper::finite(5))); // 中间值
        assert!(range.contains_value(&ValueWrapper::finite(10))); // 边界值
        assert!(!range.contains_value(&ValueWrapper::finite(0))); // 小于下界
        assert!(!range.contains_value(&ValueWrapper::finite(11))); // 大于上界
    }

    #[test]
    fn test_contains_open() {
        let range: ValueRange<i64, Open, Open> = ValueRange::new(
            Bound::new(ValueWrapper::finite(1), Open),
            Bound::new(ValueWrapper::finite(10), Open),
        );

        // 开区间 (1, 10)
        assert!(!range.contains_value(&ValueWrapper::finite(1))); // 边界值不包含
        assert!(range.contains_value(&ValueWrapper::finite(5))); // 中间值
        assert!(!range.contains_value(&ValueWrapper::finite(10))); // 边界值不包含
        assert!(!range.contains_value(&ValueWrapper::finite(0))); // 小于下界
        assert!(!range.contains_value(&ValueWrapper::finite(11))); // 大于上界
    }

    #[test]
    fn test_contains_mixed_compile_time() {
        // [1, 10) - 左闭右开
        let range: ValueRange<i64, Closed, Open> = ValueRange::new(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(10), Open),
        );

        assert!(range.contains_value(&ValueWrapper::finite(1))); // 左边界包含
        assert!(!range.contains_value(&ValueWrapper::finite(10))); // 右边界不包含
        assert!(range.contains_value(&ValueWrapper::finite(5))); // 中间值

        // (1, 10] - 左开右闭
        let range: ValueRange<i64, Open, Closed> = ValueRange::new(
            Bound::new(ValueWrapper::finite(1), Open),
            Bound::new(ValueWrapper::finite(10), Closed),
        );

        assert!(!range.contains_value(&ValueWrapper::finite(1))); // 左边界不包含
        assert!(range.contains_value(&ValueWrapper::finite(10))); // 右边界包含
        assert!(range.contains_value(&ValueWrapper::finite(5))); // 中间值
    }

    #[test]
    fn test_contains_mixed_runtime() {
        let range: ValueRange<i64> = ValueRange::new(
            Bound::new(ValueWrapper::finite(1), Interval::Closed),
            Bound::new(ValueWrapper::finite(10), Interval::Open),
        );

        // 半开半闭区间 [1, 10)
        assert!(range.contains_value(&ValueWrapper::finite(1))); // 闭区间边界值
        assert!(range.contains_value(&ValueWrapper::finite(5))); // 中间值
        assert!(!range.contains_value(&ValueWrapper::finite(10))); // 开区间边界值不包含
    }

    #[test]
    fn test_contains_infinity() {
        let range: ValueRange<i64> = ValueRange::new(
            Bound::new(ValueWrapper::finite(0), Interval::Closed),
            Bound::new(ValueWrapper::positive_infinity(), Interval::Open),
        );

        // [0, +∞)
        assert!(range.contains_value(&ValueWrapper::finite(0)));
        assert!(range.contains_value(&ValueWrapper::finite(100)));
        assert!(!range.contains_value(&ValueWrapper::finite(-1)));
        assert!(!range.contains_value(&ValueWrapper::positive_infinity())); // 开区间不包含 +∞
    }

    // ========================================================================
    // is_degenerate 测试 / is_degenerate tests
    // ========================================================================

    #[test]
    fn test_is_degenerate() {
        // 退化为单点 [5, 5]
        let degenerate: ValueRange<i64, Closed, Closed> = ValueRange::new(
            Bound::new(ValueWrapper::finite(5), Closed),
            Bound::new(ValueWrapper::finite(5), Closed),
        );
        assert!(degenerate.is_degenerate());
        assert_eq!(degenerate.degenerate_value(), Some(&5));

        // 非退化 [1, 10]
        let non_degenerate: ValueRange<i64, Closed, Closed> = ValueRange::new(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(10), Closed),
        );
        assert!(!non_degenerate.is_degenerate());
        assert_eq!(non_degenerate.degenerate_value(), None);

        // 值相等但开区间 (5, 5) - 不包含任何值，但不是退化
        let open_same: ValueRange<i64, Open, Open> = ValueRange::new(
            Bound::new(ValueWrapper::finite(5), Open),
            Bound::new(ValueWrapper::finite(5), Open),
        );
        assert!(!open_same.is_degenerate()); // 开区间不退化

        // 值相等但混合开闭 [5, 5) - 不是退化
        let mixed: ValueRange<i64, Closed, Open> = ValueRange::new(
            Bound::new(ValueWrapper::finite(5), Closed),
            Bound::new(ValueWrapper::finite(5), Open),
        );
        assert!(!mixed.is_degenerate()); // 混合开闭不退化
    }

    // ========================================================================
    // Display 测试 / Display tests
    // ========================================================================

    #[test]
    fn test_display() {
        let closed: ValueRange<i64, Closed, Closed> = ValueRange::new(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(10), Closed),
        );
        assert_eq!(format!("{}", closed), "[1, 10]");

        let open: ValueRange<i64, Open, Open> = ValueRange::new(
            Bound::new(ValueWrapper::finite(1), Open),
            Bound::new(ValueWrapper::finite(10), Open),
        );
        assert_eq!(format!("{}", open), "(1, 10)");

        let mixed: ValueRange<i64, Closed, Open> = ValueRange::new(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(10), Open),
        );
        assert_eq!(format!("{}", mixed), "[1, 10)");

        let infinity: ValueRange<i64> = ValueRange::new(
            Bound::new(ValueWrapper::negative_infinity(), Interval::Open),
            Bound::new(ValueWrapper::positive_infinity(), Interval::Open),
        );
        assert_eq!(format!("{}", infinity), "(-∞, +∞)");
    }

    // ========================================================================
    // 相等性测试 / Equality tests
    // ========================================================================

    #[test]
    fn test_eq() {
        let a: ValueRange<i64, Closed, Closed> = ValueRange::new(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(10), Closed),
        );
        let b: ValueRange<i64, Closed, Closed> = ValueRange::new(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(10), Closed),
        );
        let c: ValueRange<i64, Closed, Closed> = ValueRange::new(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(20), Closed),
        );

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // ========================================================================
    // Contains trait 测试 / Contains trait tests
    // ========================================================================

    #[test]
    fn test_contains_trait() {
        let range: ValueRange<i64, Closed, Closed> = ValueRange::new(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(10), Closed),
        );

        // 测试 Contains<T> 实现
        // Test Contains<T> implementation
        use crate::operator::Contains;
        assert!(range.contains(&5_i64));
        assert!(range.contains(&1_i64));
        assert!(range.contains(&10_i64));
        assert!(!range.contains(&0_i64));
        assert!(!range.contains(&11_i64));
    }
}