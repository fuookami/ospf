//! 时间槽接口 / Time slot trait
//!
//! 表示具有时间范围的可切片对象。
//! Represents a sliceable object with a time range.

use super::TimeRange;
use time::{Duration, OffsetDateTime};

/// 时间槽接口 / Time slot interface
///
/// 表示具有时间范围的可切片对象，支持按时间范围提取子槽。
/// Represents a sliceable object with a time range, supporting sub-slot extraction.
pub trait TimeSlot: Send + Sync {
    /// 获取时间范围 / Get time range
    fn time(&self) -> &TimeRange;

    /// 获取开始时间 / Get start time
    fn start(&self) -> OffsetDateTime {
        self.time().start
    }

    /// 获取结束时间 / Get end time
    fn end(&self) -> OffsetDateTime {
        self.time().end
    }

    /// 获取持续时间 / Get duration
    fn duration(&self) -> Duration {
        self.time().duration()
    }

    /// 获取在给定时间范围内的子槽 / Get sub-slot within given time range
    ///
    /// 返回 `Some(self ∩ sub_time)`，无交集时返回 `None`。
    /// Returns `Some(self ∩ sub_time)`, or `None` if no intersection.
    fn sub_of(&self, sub_time: &TimeRange) -> Option<Self>
    where
        Self: Sized;
}
