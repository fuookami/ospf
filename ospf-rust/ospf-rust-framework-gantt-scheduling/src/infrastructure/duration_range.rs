//! 持续时间范围 / Duration range with lower and upper bounds
//!
//! 表示持续时间的下界和上界 [lower, upper]。
//! Represents the lower and upper bounds of a duration [lower, upper].

use time::Duration;

/// 持续时间范围，表示下界和上界 / Duration range with lower and upper bounds
///
/// 当 `lower == upper` 时表示固定持续时间值。
/// When `lower == upper`, represents a fixed duration value.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DurationRange {
    /// 下界 / Lower bound
    pub lower: Duration,
    /// 上界 / Upper bound
    pub upper: Duration,
}

impl DurationRange {
    /// 创建新的持续时间范围 / Create new duration range
    ///
    /// # Panics
    /// 当 `lower > upper` 时在 debug 模式下断言失败。
    /// Asserts in debug mode when `lower > upper`.
    pub fn new(lower: Duration, upper: Duration) -> Self {
        debug_assert!(
            lower <= upper,
            "DurationRange lower ({lower:?}) must be <= upper ({upper:?})"
        );
        Self { lower, upper }
    }

    /// 创建固定值的持续时间范围 / Create duration range with fixed value
    ///
    /// `lower` 和 `upper` 均设为 `value`。
    /// Both `lower` and `upper` are set to `value`.
    pub fn fixed(value: Duration) -> Self {
        Self {
            lower: value,
            upper: value,
        }
    }

    /// 是否为固定值 / Whether this is a fixed value
    ///
    /// 当 `lower == upper` 时返回 `true`。
    /// Returns `true` when `lower == upper`.
    pub fn is_fixed(&self) -> bool {
        self.lower == self.upper
    }

    /// 获取持续时间 / Get duration
    ///
    /// 对于固定值范围返回 `lower`，否则返回 `lower`。
    /// For fixed ranges returns `lower`, otherwise returns `lower`.
    pub fn duration(&self) -> Duration {
        self.lower
    }
}

impl From<Duration> for DurationRange {
    fn from(value: Duration) -> Self {
        Self::fixed(value)
    }
}

impl Default for DurationRange {
    fn default() -> Self {
        Self::fixed(Duration::ZERO)
    }
}
