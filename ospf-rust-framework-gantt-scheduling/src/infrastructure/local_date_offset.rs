//! 本地日期偏移 / Local date offset for cross-day boundary calculation
//!
//! 用于处理跨日边界场景：当工作时间超过午夜时，逻辑日期可能与日历日期不同。
//! Handles cross-day boundary scenarios: when working hours extend past midnight,
//! the logical date may differ from the calendar date.

use time::{Date, Time, Duration};

/// 本地日期偏移 / Local date offset
///
/// 定义了逻辑日期切换点。当日内时间早于 `offset` 时，逻辑日期为日历日期前一天。
/// Defines the logical date switch point. When the time of day is before `offset`,
/// the logical date is the previous calendar date.
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_framework_gantt_scheduling::infrastructure::LocalDateOffset;
/// use time::macros::{time, date};
///
/// let offset = LocalDateOffset::new(time!(08:00));
/// // 09:00 在 08:00 之后 -> 同一天
/// // 09:00 is after 08:00 -> same day
/// assert_eq!(offset.date(date!(2020-08-30), time!(09:00)), date!(2020-08-30));
///
/// // 07:00 在 08:00 之前 -> 前一天
/// // 07:00 is before 08:00 -> previous day
/// assert_eq!(offset.date(date!(2020-08-30), time!(07:00)), date!(2020-08-29));
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalDateOffset {
    /// 日偏移时间 / Daily offset time
    ///
    /// 逻辑日期切换点。时间早于此值时属于前一天。
    /// Logical date switch point. Times before this value belong to the previous day.
    pub offset: Time,
}

impl LocalDateOffset {
    /// 创建新的日期偏移 / Create new date offset
    pub fn new(offset: Time) -> Self {
        Self { offset }
    }

    /// 根据日期和时间计算所属逻辑日期 / Calculate logical date from date and time
    ///
    /// 当 `time < self.offset` 时，逻辑日期为 `date - 1 day`。
    /// When `time < self.offset`, logical date is `date - 1 day`.
    pub fn date(&self, date: Date, time: Time) -> Date {
        if time < self.offset {
            date - Duration::days(1)
        } else {
            date
        }
    }
}

impl Default for LocalDateOffset {
    fn default() -> Self {
        Self {
            // 默认午夜零点，即不做日期偏移
            // Default midnight, i.e., no date offset
            offset: Time::MIDNIGHT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::{date, time};

    #[test]
    fn test_date_after_offset() {
        let offset = LocalDateOffset::new(time!(08:00));
        let result = offset.date(date!(2020-08-30), time!(09:00));
        assert_eq!(result, date!(2020-08-30));
    }

    #[test]
    fn test_date_before_offset() {
        let offset = LocalDateOffset::new(time!(08:00));
        let result = offset.date(date!(2020-08-30), time!(07:00));
        assert_eq!(result, date!(2020-08-29));
    }

    #[test]
    fn test_date_at_offset() {
        let offset = LocalDateOffset::new(time!(08:00));
        let result = offset.date(date!(2020-08-30), time!(08:00));
        // Equal to offset -> same day
        assert_eq!(result, date!(2020-08-30));
    }

    #[test]
    fn test_default_no_offset() {
        let offset = LocalDateOffset::default();
        let result = offset.date(date!(2020-08-30), time!(00:00));
        assert_eq!(result, date!(2020-08-30));
    }
}
