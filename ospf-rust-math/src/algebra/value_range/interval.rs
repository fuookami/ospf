//! Interval - 开闭性质抽象
//! Interval - Openness abstraction

use std::fmt;

// ============================================================================
// IntervalKind Trait - 开闭性质抽象
// ============================================================================

/// IntervalKind - 开闭性质抽象 trait
/// IntervalKind - Openness abstraction trait
///
/// 定义区间的开闭性质，用于判断边界是否包含在区间内。
/// Defines the openness of an interval, used to determine if bounds are included.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::value_range::{IntervalTrait, Closed, Open, Interval};
///
/// // 编译时开闭性质
/// // Compile-time openness
/// assert!(Closed.is_closed());
/// assert!(Open.is_open());
///
/// // 运行时开闭性质
/// // Runtime openness
/// assert!(Interval::Closed.is_closed());
/// assert!(Interval::Open.is_open());
/// ```
pub trait IntervalTrait: Copy + Clone + PartialEq {
    /// 是否为闭区间（包含边界值）
    /// Whether it's a closed interval (includes boundary value)
    ///
    /// # 返回 / Returns
    /// 如果为闭区间返回 `true`，否则返回 `false`
    /// Returns `true` if closed interval, `false` otherwise
    fn is_closed(&self) -> bool;

    /// 是否为开区间（不包含边界值）
    /// Whether it's an open interval (excludes boundary value)
    ///
    /// # 返回 / Returns
    /// 如果为开区间返回 `true`，否则返回 `false`
    /// Returns `true` if open interval, `false` otherwise
    fn is_open(&self) -> bool {
        !self.is_closed()
    }

    /// 下界符号表示
    /// Lower bound sign representation
    ///
    /// # 返回 / Returns
    /// 下界的符号表示：`"["` 表示闭区间，`"("` 表示开区间
    /// Sign representation for lower bound: `"["` for closed, `"("` for open
    fn lower_sign(&self) -> &'static str;

    /// 上界符号表示
    /// Upper bound sign representation
    ///
    /// # 返回 / Returns
    /// 上界的符号表示：`"]"` 表示闭区间，`")"` 表示开区间
    /// Sign representation for upper bound: `"]"` for closed, `")"` for open
    fn upper_sign(&self) -> &'static str;

    /// 并集运算时的开闭性质
    /// Openness for union operation
    ///
    /// # 参数 / Parameters
    /// - `other`: 另一个区间的开闭性质
    /// - `other`: The openness of another interval
    ///
    /// # 返回 / Returns
    /// 并集后的开闭性质
    /// The openness after union
    fn union(&self, other: &Self) -> Self;

    /// 交集运算时的开闭性质
    /// Openness for intersection operation
    ///
    /// # 参数 / Parameters
    /// - `other`: 另一个区间的开闭性质
    /// - `other`: The openness of another interval
    ///
    /// # 返回 / Returns
    /// 交集后的开闭性质
    /// The openness after intersection
    fn intersect(&self, other: &Self) -> Self;
}

// ============================================================================
// Closed - 闭区间标记类型
// ============================================================================

/// Closed - 闭区间标记类型
/// Closed - Closed interval marker type
///
/// 表示闭区间，边界值包含在区间内。
/// Represents a closed interval, boundary values are included.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::value_range::{IntervalTrait, Closed};
///
/// assert!(Closed.is_closed());
/// assert!(!Closed.is_open());
/// assert_eq!(Closed.lower_sign(), "[");
/// assert_eq!(Closed.upper_sign(), "]");
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Closed;

impl IntervalTrait for Closed {
    fn is_closed(&self) -> bool {
        true
    }

    fn lower_sign(&self) -> &'static str {
        "["
    }

    fn upper_sign(&self) -> &'static str {
        "]"
    }

    fn union(&self, _other: &Self) -> Self {
        Closed
    }

    fn intersect(&self, other: &Self) -> Self {
        *other
    }
}

impl fmt::Display for Closed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "closed")
    }
}

impl Default for Closed {
    fn default() -> Self {
        Closed
    }
}

// ============================================================================
// Open - 开区间标记类型
// ============================================================================

/// Open - 开区间标记类型
/// Open - Open interval marker type
///
/// 表示开区间，边界值不包含在区间内。
/// Represents an open interval, boundary values are excluded.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::value_range::{IntervalTrait, Open};
///
/// assert!(Open.is_open());
/// assert!(!Open.is_closed());
/// assert_eq!(Open.lower_sign(), "(");
/// assert_eq!(Open.upper_sign(), ")");
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Open;

impl IntervalTrait for Open {
    fn is_closed(&self) -> bool {
        false
    }

    fn lower_sign(&self) -> &'static str {
        "("
    }

    fn upper_sign(&self) -> &'static str {
        ")"
    }

    fn union(&self, other: &Self) -> Self {
        *other
    }

    fn intersect(&self, _other: &Self) -> Self {
        Open
    }
}

impl fmt::Display for Open {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "open")
    }
}

impl Default for Open {
    fn default() -> Self {
        Open
    }
}

// ============================================================================
// Interval - 运行时开闭性质枚举
// ============================================================================

/// Interval - 运行时开闭性质枚举
/// Interval - Runtime openness enum
///
/// 在运行时确定区间的开闭性质。
/// Determines interval openness at runtime.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::algebra::value_range::{IntervalTrait, Interval};
///
/// let closed = Interval::Closed;
/// let open = Interval::Open;
///
/// assert!(closed.is_closed());
/// assert!(open.is_open());
///
/// // 并集和交集运算
/// // Union and intersection operations
/// assert_eq!(closed.union(&open), Interval::Closed);
/// assert_eq!(closed.intersect(&open), Interval::Open);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Interval {
    /// 开区间 / Open interval
    Open,
    /// 闭区间 / Closed interval
    Closed,
}

impl IntervalTrait for Interval {
    fn is_closed(&self) -> bool {
        matches!(self, Interval::Closed)
    }

    fn lower_sign(&self) -> &'static str {
        match self {
            Interval::Open => "(",
            Interval::Closed => "[",
        }
    }

    fn upper_sign(&self) -> &'static str {
        match self {
            Interval::Open => ")",
            Interval::Closed => "]",
        }
    }

    fn union(&self, other: &Self) -> Self {
        match (self, other) {
            (Interval::Closed, _) | (_, Interval::Closed) => Interval::Closed,
            _ => Interval::Open,
        }
    }

    fn intersect(&self, other: &Self) -> Self {
        match (self, other) {
            (Interval::Open, _) | (_, Interval::Open) => Interval::Open,
            _ => Interval::Closed,
        }
    }
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Interval::Open => write!(f, "open"),
            Interval::Closed => write!(f, "closed"),
        }
    }
}

impl Default for Interval {
    fn default() -> Self {
        Interval::Closed
    }
}

// ============================================================================
// 辅助函数 / Helper functions
// ============================================================================

/// 判断值是否在边界上（考虑开闭性质）
/// Check if value is on boundary (considering openness)
///
/// # 参数 / Parameters
/// - `value`: 要检查的值
/// - `boundary`: 边界值
/// - `is_closed`: 是否为闭区间
///
/// # 返回 / Returns
/// 如果值在边界上且区间为闭区间返回 `true`，否则返回 `false`
/// Returns `true` if value is on boundary and interval is closed, `false` otherwise
#[inline]
pub fn is_on_boundary<T: PartialEq>(value: &T, boundary: &T, is_closed: bool) -> bool {
    value == boundary && is_closed
}

/// 判断值是否严格小于边界（考虑开闭性质）
/// Check if value is strictly less than boundary (considering openness)
///
/// # 参数 / Parameters
/// - `value`: 要检查的值
/// - `boundary`: 边界值
/// - `is_closed`: 是否为闭区间
///
/// # 返回 / Returns
/// 返回比较结果
/// Returns comparison result
#[inline]
pub fn is_below_boundary<T: PartialOrd>(value: &T, boundary: &T, is_closed: bool) -> bool {
    if value < boundary {
        true
    } else if value == boundary {
        !is_closed
    } else {
        false
    }
}

/// 判断值是否严格大于边界（考虑开闭性质）
/// Check if value is strictly greater than boundary (considering openness)
///
/// # 参数 / Parameters
/// - `value`: 要检查的值
/// - `boundary`: 边界值
/// - `is_closed`: 是否为闭区间
///
/// # 返回 / Returns
/// 返回比较结果
/// Returns comparison result
#[inline]
pub fn is_above_boundary<T: PartialOrd>(value: &T, boundary: &T, is_closed: bool) -> bool {
    if value > boundary {
        true
    } else if value == boundary {
        !is_closed
    } else {
        false
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Closed 测试 / Closed tests
    // ========================================================================

    #[test]
    fn test_closed_is_closed() {
        assert!(Closed.is_closed());
        assert!(!Closed.is_open());
    }

    #[test]
    fn test_closed_signs() {
        assert_eq!(Closed.lower_sign(), "[");
        assert_eq!(Closed.upper_sign(), "]");
    }

    #[test]
    fn test_closed_union_intersect() {
        assert_eq!(Closed.union(&Closed), Closed);
        assert_eq!(Closed.intersect(&Closed), Closed);
    }

    #[test]
    fn test_closed_display() {
        assert_eq!(format!("{}", Closed), "closed");
    }

    // ========================================================================
    // Open 测试 / Open tests
    // ========================================================================

    #[test]
    fn test_open_is_open() {
        assert!(Open.is_open());
        assert!(!Open.is_closed());
    }

    #[test]
    fn test_open_signs() {
        assert_eq!(Open.lower_sign(), "(");
        assert_eq!(Open.upper_sign(), ")");
    }

    #[test]
    fn test_open_union_intersect() {
        assert_eq!(Open.union(&Open), Open);
        assert_eq!(Open.intersect(&Open), Open);
    }

    #[test]
    fn test_open_display() {
        assert_eq!(format!("{}", Open), "open");
    }

    // ========================================================================
    // Interval 测试 / Interval tests
    // ========================================================================

    #[test]
    fn test_interval_closed() {
        let closed = Interval::Closed;
        assert!(closed.is_closed());
        assert!(!closed.is_open());
        assert_eq!(closed.lower_sign(), "[");
        assert_eq!(closed.upper_sign(), "]");
    }

    #[test]
    fn test_interval_open() {
        let open = Interval::Open;
        assert!(open.is_open());
        assert!(!open.is_closed());
        assert_eq!(open.lower_sign(), "(");
        assert_eq!(open.upper_sign(), ")");
    }

    #[test]
    fn test_interval_union() {
        assert_eq!(Interval::Closed.union(&Interval::Closed), Interval::Closed);
        assert_eq!(Interval::Closed.union(&Interval::Open), Interval::Closed);
        assert_eq!(Interval::Open.union(&Interval::Closed), Interval::Closed);
        assert_eq!(Interval::Open.union(&Interval::Open), Interval::Open);
    }

    #[test]
    fn test_interval_intersect() {
        assert_eq!(
            Interval::Closed.intersect(&Interval::Closed),
            Interval::Closed
        );
        assert_eq!(Interval::Closed.intersect(&Interval::Open), Interval::Open);
        assert_eq!(Interval::Open.intersect(&Interval::Closed), Interval::Open);
        assert_eq!(Interval::Open.intersect(&Interval::Open), Interval::Open);
    }

    #[test]
    fn test_interval_display() {
        assert_eq!(format!("{}", Interval::Closed), "closed");
        assert_eq!(format!("{}", Interval::Open), "open");
    }

    #[test]
    fn test_interval_default() {
        assert_eq!(Interval::default(), Interval::Closed);
    }

    // ========================================================================
    // 辅助函数测试 / Helper function tests
    // ========================================================================

    #[test]
    fn test_is_on_boundary() {
        assert!(is_on_boundary(&5, &5, true)); // 闭区间，在边界上
        assert!(!is_on_boundary(&5, &5, false)); // 开区间，不在边界上
        assert!(!is_on_boundary(&4, &5, true)); // 不在边界上
    }

    #[test]
    fn test_is_below_boundary() {
        // 闭区间测试
        assert!(is_below_boundary(&4, &5, true)); // 4 < 5
        assert!(!is_below_boundary(&5, &5, true)); // 5 == 5，闭区间包含
        assert!(!is_below_boundary(&6, &5, true)); // 6 > 5

        // 开区间测试
        assert!(is_below_boundary(&4, &5, false)); // 4 < 5
        assert!(is_below_boundary(&5, &5, false)); // 5 == 5，开区间不包含
        assert!(!is_below_boundary(&6, &5, false)); // 6 > 5
    }

    #[test]
    fn test_is_above_boundary() {
        // 闭区间测试
        assert!(!is_above_boundary(&4, &5, true)); // 4 < 5
        assert!(!is_above_boundary(&5, &5, true)); // 5 == 5，闭区间包含
        assert!(is_above_boundary(&6, &5, true)); // 6 > 5

        // 开区间测试
        assert!(!is_above_boundary(&4, &5, false)); // 4 < 5
        assert!(is_above_boundary(&5, &5, false)); // 5 == 5，开区间不包含
        assert!(is_above_boundary(&6, &5, false)); // 6 > 5
    }
}