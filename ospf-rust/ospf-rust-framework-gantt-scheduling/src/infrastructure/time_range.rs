//! 时间范围 / Time range with half-open interval semantics
//!
//! 表示半开区间 `[start, end)` 的时间范围。
//! Represents a half-open interval `[start, end)` of time.
//!
//! # 核心语义 / Core Semantics
//!
//! - 半开区间：包含 `start`，不包含 `end`
//! - 半开区间: contains `start`, excludes `end`
//! - 空范围：当 `start >= end` 时为空
//! - Empty range: when `start >= end`
//! - `DISTANT_PAST` / `DISTANT_FUTURE`：表示无限远的时间边界
//! - `DISTANT_PAST` / `DISTANT_FUTURE`: represents infinitely distant time boundaries

use crate::infrastructure::DurationRange;
use time::{Date, Duration, Month, OffsetDateTime, Time};

/// 遥远的过去 / Distant past
///
/// 对齐 Kotlin `Instant.DISTANT_PAST`（年份 -100000000）。
/// Aligned with Kotlin `Instant.DISTANT_PAST` (year -100000000).
///
/// 使用年份 -9999 作为 `time` crate 支持的最远过去值。
/// Uses year -9999 as the furthest past value supported by the `time` crate.
pub fn distant_past() -> OffsetDateTime {
    Date::from_calendar_date(-9999, Month::January, 1)
        .expect("valid date")
        .with_time(Time::MIDNIGHT)
        .assume_utc()
}

/// 遥远的未来 / Distant future
///
/// 对齐 Kotlin `Instant.DISTANT_FUTURE`（年份 +100000000）。
/// Aligned with Kotlin `Instant.DISTANT_FUTURE` (year +100000000).
///
/// 使用年份 9999 作为 `time` crate 支持的最远未来值。
/// Uses year 9999 as the furthest future value supported by the `time` crate.
pub fn distant_future() -> OffsetDateTime {
    Date::from_calendar_date(9999, Month::December, 31)
        .expect("valid date")
        .with_time(Time::from_hms(23, 59, 59).expect("valid time"))
        .assume_utc()
}

/// 拆分结果 / Split result
///
/// 包含工作时间段和休息时间段。
/// Contains work time ranges and break time ranges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitTimeRanges {
    /// 工作时间段 / Work time ranges
    pub times: Vec<TimeRange>,
    /// 休息时间段 / Break time ranges
    pub break_times: Vec<TimeRange>,
}

/// 时间范围，半开区间 `[start, end)` / Time range, half-open interval `[start, end)`
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_framework_gantt_scheduling::infrastructure::TimeRange;
/// use time::macros::datetime;
///
/// let range = TimeRange::new(datetime!(2020-08-30 08:00 UTC), datetime!(2020-08-30 18:00 UTC));
/// assert!(!range.is_empty());
/// assert_eq!(range.duration(), time::Duration::hours(10));
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TimeRange {
    /// 开始时间 / Start time
    pub start: OffsetDateTime,
    /// 结束时间 / End time
    pub end: OffsetDateTime,
}

impl TimeRange {
    /// 创建新的时间范围 / Create new time range
    ///
    /// 不检查 `start <= end`，空范围通过 `is_empty()` 判断。
    /// Does not enforce `start <= end`; empty ranges are detected via `is_empty()`.
    pub fn new(start: OffsetDateTime, end: OffsetDateTime) -> Self {
        Self { start, end }
    }

    /// 创建全时间范围 / Create the full time range
    ///
    /// `[distant_past(), distant_future())`。
    /// `[distant_past(), distant_future())`.
    pub fn full() -> Self {
        Self {
            start: distant_past(),
            end: distant_future(),
        }
    }

    /// 创建空范围 / Create an empty range
    pub fn empty() -> Self {
        Self {
            start: distant_future(),
            end: distant_past(),
        }
    }

    /// 是否为空范围 / Whether this is an empty range
    ///
    /// 当 `start >= end` 时为空。
    /// Empty when `start >= end`.
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    /// 持续时间 / Duration
    ///
    /// 返回 `end - start`。空范围返回零或负持续时间。
    /// Returns `end - start`. Empty ranges return zero or negative duration.
    pub fn duration(&self) -> Duration {
        self.end - self.start
    }

    /// 前方范围 / Front range
    ///
    /// 返回 `[distant_past(), start)`，当 `start == distant_past()` 时返回 `None`。
    /// Returns `[distant_past(), start)`, or `None` when `start == distant_past()`.
    pub fn front(&self) -> Option<TimeRange> {
        if self.start == distant_past() {
            None
        } else {
            Some(TimeRange {
                start: distant_past(),
                end: self.start,
            })
        }
    }

    /// 后方范围 / Back range
    ///
    /// 返回 `[end, distant_future())`，当 `end == distant_future()` 时返回 `None`。
    /// Returns `[end, distant_future())`, or `None` when `end == distant_future()`.
    pub fn back(&self) -> Option<TimeRange> {
        if self.end == distant_future() {
            None
        } else {
            Some(TimeRange {
                start: self.end,
                end: distant_future(),
            })
        }
    }

    /// 与另一范围之间的前方间隙 / Front gap between this and another range
    ///
    /// 返回 `[other.end, self.start)`，当 `other.end >= self.start` 时返回 `None`。
    /// Returns `[other.end, self.start)`, or `None` when `other.end >= self.start`.
    pub fn front_between(&self, other: &TimeRange) -> Option<TimeRange> {
        if other.end < self.start {
            Some(TimeRange {
                start: other.end,
                end: self.start,
            })
        } else {
            None
        }
    }

    /// 与另一范围之间的后方间隙 / Back gap between this and another range
    ///
    /// 返回 `[self.end, other.start)`，当 `other.start <= self.end` 时返回 `None`。
    /// Returns `[self.end, other.start)`, or `None` when `other.start <= self.end`.
    pub fn back_between(&self, other: &TimeRange) -> Option<TimeRange> {
        if other.start > self.end {
            Some(TimeRange {
                start: self.end,
                end: other.start,
            })
        } else {
            None
        }
    }

    // ========================================================================
    // 包含检查 / Contains checks
    // ========================================================================

    /// 是否包含时间点 / Check if contains instant
    ///
    /// 半开区间语义：`start <= instant && instant < end`。
    /// Half-open interval semantics: `start <= instant && instant < end`.
    pub fn contains_instant(&self, instant: OffsetDateTime) -> bool {
        self.start <= instant && instant < self.end
    }

    /// 是否包含另一时间范围 / Check if contains another time range
    ///
    /// 当 `self.start <= other.start && other.end <= self.end` 时返回 `true`。
    /// Returns `true` when `self.start <= other.start && other.end <= self.end`.
    pub fn contains_range(&self, other: &TimeRange) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    // ========================================================================
    // 交集 / Intersection
    // ========================================================================

    /// 是否有交集 / Check if intersects with another range
    ///
    /// 条件：`self.start < other.end && other.start < self.end`。
    /// Condition: `self.start < other.end && other.start < self.end`.
    pub fn intersects(&self, other: &TimeRange) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// 计算交集 / Compute intersection
    ///
    /// 返回 `Some([max(self.start, other.start), min(self.end, other.end)))`，
    /// 无交集时返回 `None`。
    /// Returns `Some([max(self.start, other.start), min(self.end, other.end)))`,
    /// or `None` if no intersection.
    pub fn intersection(&self, other: &TimeRange) -> Option<TimeRange> {
        let max_start = self.start.max(other.start);
        let min_end = self.end.min(other.end);
        if min_end <= max_start {
            None
        } else {
            Some(TimeRange {
                start: max_start,
                end: min_end,
            })
        }
    }

    // ========================================================================
    // 差集 / Difference
    // ========================================================================

    /// 计算差集 / Compute difference
    ///
    /// 返回从 `self` 中移除 `other` 后剩余的范围列表。
    /// Returns the remaining ranges after removing `other` from `self`.
    pub fn difference(&self, other: &TimeRange) -> Vec<TimeRange> {
        let intersection = match self.intersection(other) {
            Some(i) => i,
            None => return vec![*self],
        };

        // Intersection covers the entire range
        if intersection.start <= self.start && intersection.end >= self.end {
            return vec![];
        }

        let mut result = Vec::with_capacity(2);

        // Front part: [self.start, intersection.start)
        if intersection.start > self.start {
            result.push(TimeRange {
                start: self.start,
                end: intersection.start,
            });
        }

        // Back part: [intersection.end, self.end)
        if intersection.end < self.end {
            result.push(TimeRange {
                start: intersection.end,
                end: self.end,
            });
        }

        result
    }

    /// 计算差集（多个范围）/ Compute difference with multiple ranges
    ///
    /// 从 `self` 中依次移除 `others` 中的所有范围。
    /// Removes all ranges in `others` from `self`.
    pub fn difference_with_many(&self, others: &[TimeRange]) -> Vec<TimeRange> {
        if others.is_empty() {
            return vec![*self];
        }

        // Collect and merge intersections with this range
        let mut intersections: Vec<TimeRange> =
            others.iter().filter_map(|o| self.intersection(o)).collect();

        if intersections.is_empty() {
            return vec![*self];
        }

        // Sort and merge overlapping intersections
        intersections.sort_by_key(|r| r.start);
        let merged = merge(&intersections);

        if merged.is_empty() {
            return vec![*self];
        }

        // Walk through gaps between merged intersections
        let mut result = Vec::with_capacity(merged.len() + 1);
        let mut current = self.start;

        for range in &merged {
            if range.start > current {
                result.push(TimeRange {
                    start: current,
                    end: range.start,
                });
            }
            current = current.max(range.end);
        }

        if current < self.end {
            result.push(TimeRange {
                start: current,
                end: self.end,
            });
        }

        result
    }

    // ========================================================================
    // 拆分 / Split
    // ========================================================================

    /// 按时间点拆分 / Split at instants
    ///
    /// 在给定时间点处拆分此范围，返回拆分后的范围列表。
    /// Splits this range at the given instants, returning the resulting ranges.
    ///
    /// 只有严格位于范围内部的时间点才会产生拆分。
    /// Only instants strictly inside the range produce splits.
    pub fn split_at(&self, instants: &[OffsetDateTime]) -> Vec<TimeRange> {
        // Collect and deduplicate valid split points (strictly inside the range)
        let mut split_points: Vec<OffsetDateTime> = instants
            .iter()
            .filter(|&&t| t != self.start && self.contains_instant(t))
            .copied()
            .collect();
        split_points.sort();
        split_points.dedup();

        if split_points.is_empty() {
            return vec![*self];
        }

        let mut result = Vec::with_capacity(split_points.len() + 1);
        let mut current = self.start;

        for point in split_points {
            if current < point {
                result.push(TimeRange {
                    start: current,
                    end: point,
                });
            }
            current = point;
        }

        if current < self.end {
            result.push(TimeRange {
                start: current,
                end: self.end,
            });
        }

        result
    }

    /// 按持续时间单元拆分 / Split by duration unit
    ///
    /// 按给定持续时间范围 `[unit.lower, unit.upper]` 拆分此时间范围，
    /// 每个工作段目标时长为 `unit.lower`，最大不超过 `unit.upper`。
    /// Splits this time range by the given duration range `[unit.lower, unit.upper]`,
    /// targeting `unit.lower` per work segment, max `unit.upper`.
    ///
    /// # 参数 / Parameters
    ///
    /// - `unit`: 持续时间范围 `[lower, upper]` / Duration range `[lower, upper]`
    /// - `current_duration`: 已累积时长 / Already accumulated duration
    /// - `max_duration`: 最大工作时长限制 / Max work duration cap
    /// - `break_time`: 段间休息时长 / Break duration between segments
    pub fn split_by_duration(
        &self,
        unit: &DurationRange,
        current_duration: Duration,
        max_duration: Option<Duration>,
        break_time: Option<Duration>,
    ) -> SplitTimeRanges {
        let mut times = Vec::new();
        let mut break_times = Vec::new();

        if self.is_empty() {
            return SplitTimeRanges { times, break_times };
        }

        let total_range_duration = self.duration();
        let mut remaining = total_range_duration;
        let mut pos = self.start;
        let mut accumulated = current_duration;
        let zero = Duration::ZERO;

        while remaining > zero {
            let target = unit.lower.saturating_sub(accumulated).max(zero);

            if target.is_zero() && accumulated >= unit.lower {
                // Already accumulated enough, check if we need a break
                if let Some(bt) = break_time
                    && accumulated >= unit.upper
                {
                    // Insert break and reset
                    break_times.push(TimeRange::new(pos, pos + bt));
                    pos += bt;
                    remaining = remaining.saturating_sub(bt);
                    accumulated = zero;
                    continue;
                }

                // Remaining fits within upper bound
                let remaining_to_upper = unit.upper.saturating_sub(accumulated);
                let segment = remaining.min(remaining_to_upper);

                if segment > zero {
                    times.push(TimeRange::new(pos, pos + segment));
                    pos += segment;
                    remaining = remaining.saturating_sub(segment);
                }

                if remaining.is_zero() {
                    break;
                }

                // Need a break before next segment
                if let Some(bt) = break_time {
                    break_times.push(TimeRange::new(pos, pos + bt));
                    pos += bt;
                    remaining = remaining.saturating_sub(bt);
                    accumulated = zero;
                } else {
                    accumulated += segment;
                }
                continue;
            }

            // Calculate segment duration
            let mut segment = target.min(remaining);

            // Apply max_duration cap
            if let Some(max_d) = max_duration {
                let total_work: Duration = times.iter().map(|t| t.duration()).sum();
                let remaining_work = max_d.saturating_sub(total_work);
                segment = segment.min(remaining_work);
            }

            if segment.is_zero() {
                break;
            }

            times.push(TimeRange::new(pos, pos + segment));
            pos += segment;
            remaining = remaining.saturating_sub(segment);
            accumulated += segment;

            // Insert break if configured and more work remains
            if let Some(bt) = break_time
                && remaining > zero
                && accumulated >= unit.lower
            {
                break_times.push(TimeRange::new(pos, pos + bt));
                pos += bt;
                remaining = remaining.saturating_sub(bt);
                accumulated = zero;
            }
        }

        SplitTimeRanges { times, break_times }
    }

    // ========================================================================
    // 连续性 / Continuity
    // ========================================================================

    /// 在 `other` 之前连续 / Continuous before `other`
    ///
    /// 当 `self.end == other.start` 时返回 `true`。
    /// Returns `true` when `self.end == other.start`.
    pub fn continuous_before(&self, other: &TimeRange) -> bool {
        self.end == other.start
    }

    /// 在 `other` 之后连续 / Continuous after `other`
    ///
    /// 当 `self.start == other.end` 时返回 `true`。
    /// Returns `true` when `self.start == other.end`.
    pub fn continuous_after(&self, other: &TimeRange) -> bool {
        self.start == other.end
    }

    /// 与 `other` 相邻 / Continuous with `other`
    ///
    /// 当 `continuous_before(other) || continuous_after(other)` 时返回 `true`。
    /// Returns `true` when `continuous_before(other) || continuous_after(other)`.
    pub fn continuous_with(&self, other: &TimeRange) -> bool {
        self.continuous_before(other) || self.continuous_after(other)
    }

    // ========================================================================
    // 平移 / Shift
    // ========================================================================

    /// 时间范围平移 / Shift time range
    ///
    /// 将开始和结束时间同时平移 `duration`。
    /// Shifts both start and end by `duration`.
    pub fn shift(&self, duration: Duration) -> Self {
        TimeRange {
            start: self.start + duration,
            end: self.end + duration,
        }
    }
}

impl Default for TimeRange {
    fn default() -> Self {
        Self::empty()
    }
}

// ============================================================================
// 列表扩展函数 / List extension functions
// ============================================================================

/// 合并重叠或相邻的时间范围 / Merge overlapping or adjacent time ranges
///
/// 将已排序或未排序的时间范围列表合并，消除重叠和相邻间隙。
/// Merges a list of time ranges (sorted or unsorted), eliminating overlaps and adjacency.
pub fn merge(time_ranges: &[TimeRange]) -> Vec<TimeRange> {
    if time_ranges.is_empty() {
        return vec![];
    }

    let mut sorted: Vec<TimeRange> = time_ranges.to_vec();
    sorted.sort_by_key(|r| r.start);

    let mut result = Vec::with_capacity(sorted.len());
    let mut current = sorted[0];

    for next in sorted.iter().skip(1) {
        if next.start <= current.end {
            // Overlapping or adjacent: extend current
            current.end = current.end.max(next.end);
        } else {
            // Gap: finalize current and start new
            result.push(current);
            current = *next;
        }
    }
    result.push(current);

    result
}

/// 二分查找下界 / Binary search lower bound
///
/// 找到第一个可能与 `time` 有交集的元素索引。
/// Finds the first index whose range could intersect with `time`.
fn find_lower_bound(time_ranges: &[TimeRange], time: &TimeRange) -> usize {
    if time_ranges.is_empty() {
        return 0;
    }

    let first = &time_ranges[0];
    let last = &time_ranges[time_ranges.len() - 1];

    if time.start <= first.start {
        return 0;
    }
    if time.start >= last.end {
        return time_ranges.len();
    }

    let mut lo: usize = 0;
    let mut hi: usize = time_ranges.len();

    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let mid_range = &time_ranges[mid];

        if time.start < mid_range.start {
            hi = mid;
        } else if time.start >= mid_range.end {
            lo = mid + 1;
        } else {
            // time.start is within mid_range
            return mid;
        }
    }

    lo
}

/// 二分查找上界 / Binary search upper bound
///
/// 找到最后一个可能与 `time` 有交集的元素索引 + 1。
/// Finds the index after the last element that could intersect with `time`.
fn find_upper_bound(time_ranges: &[TimeRange], time: &TimeRange) -> usize {
    if time_ranges.is_empty() {
        return 0;
    }

    let first = &time_ranges[0];
    let last = &time_ranges[time_ranges.len() - 1];

    if time.end <= first.start {
        return 0;
    }
    if time.end >= last.end {
        return time_ranges.len();
    }

    let mut lo: usize = 0;
    let mut hi: usize = time_ranges.len();

    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let mid_range = &time_ranges[mid];

        if time.end < mid_range.start {
            hi = mid;
        } else if time.end >= mid_range.end {
            lo = mid + 1;
        } else {
            // time.end is within mid_range
            return mid + 1;
        }
    }

    lo
}

/// 查找与给定时间范围相交的所有范围 / Find all ranges intersecting with given time range
///
/// 使用二分查找高效定位交集范围，要求输入列表已按 `start` 排序。
/// Uses binary search to efficiently locate intersecting ranges.
/// Input list must be sorted by `start`.
pub fn find(time_ranges: &[TimeRange], time: &TimeRange) -> Vec<TimeRange> {
    if time_ranges.is_empty() || time.is_empty() {
        return vec![];
    }

    let lo = find_lower_bound(time_ranges, time);
    let hi = find_upper_bound(time_ranges, time);

    if lo >= hi {
        return vec![];
    }

    time_ranges[lo..hi].to_vec()
}

/// 查找从给定时间点开始的所有范围 / Find all ranges from a given instant
///
/// 等价于 `find(time_ranges, TimeRange { start: from, end: distant_future() })`。
/// Equivalent to `find(time_ranges, TimeRange { start: from, end: distant_future() })`.
pub fn find_from(time_ranges: &[TimeRange], from: OffsetDateTime) -> Vec<TimeRange> {
    find(
        time_ranges,
        &TimeRange {
            start: from,
            end: distant_future(),
        },
    )
}

/// 查找直到给定时间点的所有范围 / Find all ranges until a given instant
///
/// 等价于 `find(time_ranges, TimeRange { start: distant_past(), end: until })`。
/// Equivalent to `find(time_ranges, TimeRange { start: distant_past(), end: until })`.
pub fn find_until(time_ranges: &[TimeRange], until: OffsetDateTime) -> Vec<TimeRange> {
    find(
        time_ranges,
        &TimeRange {
            start: distant_past(),
            end: until,
        },
    )
}

/// 获取指定索引处的前方间隙 / Get front gap at index
///
/// - 索引 0：返回第一个范围的前方范围（`start == distant_past()` 时返回 `None`）
/// - 其他索引：返回与前一范围之间的间隙
///
/// - Index 0: returns the front range of the first element (None if `start == distant_past()`)
/// - Other indices: returns the gap with the previous element
pub fn front_at(time_ranges: &[TimeRange], index: usize) -> Option<TimeRange> {
    if index >= time_ranges.len() {
        return None;
    }
    if index == 0 {
        time_ranges[0].front()
    } else {
        time_ranges[index].front_between(&time_ranges[index - 1])
    }
}

/// 获取指定索引处的后方间隙 / Get back gap at index
///
/// - 最后一个索引：返回最后一个范围的后方范围（`end == distant_future()` 时返回 `None`）
/// - 其他索引：返回与后一范围之间的间隙
///
/// - Last index: returns the back range of the last element (None if `end == distant_future()`)
/// - Other indices: returns the gap with the next element
pub fn back_at(time_ranges: &[TimeRange], index: usize) -> Option<TimeRange> {
    if index >= time_ranges.len() {
        return None;
    }
    if index == time_ranges.len() - 1 {
        time_ranges[index].back()
    } else {
        time_ranges[index].back_between(&time_ranges[index + 1])
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    // ========================================================================
    // TimeRange 差集测试 / TimeRange difference tests
    // (aligned with Kotlin TimeRangeDifferenceTest)
    // ========================================================================

    fn h(hour: i8) -> OffsetDateTime {
        datetime!(2020-08-30 00:00 UTC) + Duration::hours(hour as i64)
    }

    #[test]
    fn test_difference_split_in_middle() {
        // [08:00, 18:00) - [12:00, 14:00) = [08:00, 12:00), [14:00, 18:00)
        let base = TimeRange::new(h(8), h(18));
        let other = TimeRange::new(h(12), h(14));
        let result = base.difference(&other);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], TimeRange::new(h(8), h(12)));
        assert_eq!(result[1], TimeRange::new(h(14), h(18)));
    }

    #[test]
    fn test_difference_overlap_at_start() {
        // [08:00, 18:00) - [06:00, 12:00) = [12:00, 18:00)
        let base = TimeRange::new(h(8), h(18));
        let other = TimeRange::new(h(6), h(12));
        let result = base.difference(&other);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], TimeRange::new(h(12), h(18)));
    }

    #[test]
    fn test_difference_overlap_at_end() {
        // [08:00, 18:00) - [12:00, 20:00) = [08:00, 12:00)
        let base = TimeRange::new(h(8), h(18));
        let other = TimeRange::new(h(12), h(20));
        let result = base.difference(&other);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], TimeRange::new(h(8), h(12)));
    }

    #[test]
    fn test_difference_completely_covered() {
        // [08:00, 18:00) - [06:00, 20:00) = empty
        let base = TimeRange::new(h(8), h(18));
        let other = TimeRange::new(h(6), h(20));
        let result = base.difference(&other);
        assert!(result.is_empty());
    }

    #[test]
    fn test_difference_no_overlap_before() {
        // [08:00, 18:00) - [00:00, 06:00) = [08:00, 18:00)
        let base = TimeRange::new(h(8), h(18));
        let other = TimeRange::new(h(0), h(6));
        let result = base.difference(&other);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], base);
    }

    #[test]
    fn test_difference_no_overlap_after() {
        // [08:00, 18:00) - [20:00, 24:00) = [08:00, 18:00)
        let base = TimeRange::new(h(8), h(18));
        let other = TimeRange::new(h(20), h(24));
        let result = base.difference(&other);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], base);
    }

    #[test]
    fn test_difference_with_many_multiple_exclusions() {
        // [08:00, 18:00) - {[10:00, 12:00), [14:00, 16:00)}
        // = [08:00, 10:00), [12:00, 14:00), [16:00, 18:00)
        let base = TimeRange::new(h(8), h(18));
        let others = vec![TimeRange::new(h(10), h(12)), TimeRange::new(h(14), h(16))];
        let result = base.difference_with_many(&others);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], TimeRange::new(h(8), h(10)));
        assert_eq!(result[1], TimeRange::new(h(12), h(14)));
        assert_eq!(result[2], TimeRange::new(h(16), h(18)));
    }

    #[test]
    fn test_difference_with_many_overlapping_exclusions() {
        // [08:00, 18:00) - {[12:00, 14:00), [13:00, 15:00)}
        // = [08:00, 12:00), [15:00, 18:00) (overlapping exclusions merged)
        let base = TimeRange::new(h(8), h(18));
        let others = vec![TimeRange::new(h(12), h(14)), TimeRange::new(h(13), h(15))];
        let result = base.difference_with_many(&others);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], TimeRange::new(h(8), h(12)));
        assert_eq!(result[1], TimeRange::new(h(15), h(18)));
    }

    // ========================================================================
    // TimeRange 查找测试 / TimeRange find tests
    // (aligned with Kotlin TimeRangeFindTest)
    // ========================================================================

    fn day(n: i64, hour: i8) -> OffsetDateTime {
        datetime!(2020-08-30 00:00 UTC) + Duration::days(n) + Duration::hours(hour as i64)
    }

    #[test]
    fn test_find_no_overlap() {
        // Calendar: [30 08:00, 30 18:00), [31 08:00, 31 18:00)
        let calendar = vec![
            TimeRange::new(day(0, 8), day(0, 18)),
            TimeRange::new(day(1, 8), day(1, 18)),
        ];
        // find([30 19:00, 31 07:00)) -> empty
        let result = find(&calendar, &TimeRange::new(day(0, 19), day(1, 7)));
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_single_result() {
        let calendar = vec![
            TimeRange::new(day(0, 8), day(0, 18)),
            TimeRange::new(day(1, 8), day(1, 18)),
        ];
        // find([30 07:00, 31 07:00)) -> 1 result (first day)
        let result = find(&calendar, &TimeRange::new(day(0, 7), day(1, 7)));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], calendar[0]);
    }

    #[test]
    fn test_find_two_results() {
        let calendar = vec![
            TimeRange::new(day(0, 8), day(0, 18)),
            TimeRange::new(day(1, 8), day(1, 18)),
        ];
        // find([30 07:00, 31 19:00)) -> 2 results
        let result = find(&calendar, &TimeRange::new(day(0, 7), day(1, 19)));
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_find_from() {
        let calendar = vec![
            TimeRange::new(day(0, 8), day(0, 18)),
            TimeRange::new(day(1, 8), day(1, 18)),
        ];
        // findFrom(31 19:00) -> 0 results (after both calendar ranges)
        let result = find_from(&calendar, day(1, 19));
        assert!(result.is_empty());

        // findFrom(31 07:00) -> 1 result (before second calendar range)
        let result = find_from(&calendar, day(1, 7));
        assert_eq!(result.len(), 1);

        // findFrom(31 09:00) -> 1 result (within second calendar range)
        let result = find_from(&calendar, day(1, 9));
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_find_from_next_day() {
        let calendar = vec![
            TimeRange::new(day(0, 8), day(0, 18)),
            TimeRange::new(day(1, 8), day(1, 18)),
        ];
        // findFrom(29 09:00) -> 2 results
        let result = find_from(&calendar, day(-1, 9));
        assert_eq!(result.len(), 2);
    }

    // ========================================================================
    // 合并测试 / Merge tests
    // ========================================================================

    #[test]
    fn test_merge_overlapping() {
        let ranges = vec![
            TimeRange::new(h(8), h(12)),
            TimeRange::new(h(10), h(14)),
            TimeRange::new(h(16), h(18)),
        ];
        let result = merge(&ranges);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], TimeRange::new(h(8), h(14)));
        assert_eq!(result[1], TimeRange::new(h(16), h(18)));
    }

    #[test]
    fn test_merge_adjacent() {
        let ranges = vec![TimeRange::new(h(8), h(10)), TimeRange::new(h(10), h(12))];
        let result = merge(&ranges);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], TimeRange::new(h(8), h(12)));
    }

    // ========================================================================
    // 包含/交集/连续性测试 / Contains/Intersection/Continuity tests
    // ========================================================================

    #[test]
    fn test_contains_instant() {
        let range = TimeRange::new(h(8), h(18));
        assert!(range.contains_instant(h(8))); // start is included
        assert!(range.contains_instant(h(12)));
        assert!(!range.contains_instant(h(18))); // end is excluded
        assert!(!range.contains_instant(h(7)));
    }

    #[test]
    fn test_intersection() {
        let a = TimeRange::new(h(8), h(14));
        let b = TimeRange::new(h(10), h(18));
        let result = a.intersection(&b);
        assert_eq!(result, Some(TimeRange::new(h(10), h(14))));

        // No intersection
        let c = TimeRange::new(h(18), h(20));
        assert!(a.intersection(&c).is_none());
    }

    #[test]
    fn test_continuous_before_and_after() {
        let a = TimeRange::new(h(8), h(12));
        let b = TimeRange::new(h(12), h(16));
        assert!(a.continuous_before(&b));
        assert!(b.continuous_after(&a));
        assert!(a.continuous_with(&b));
    }

    #[test]
    fn test_split_at() {
        let range = TimeRange::new(h(8), h(16));
        let result = range.split_at(&[h(10), h(12), h(14)]);
        assert_eq!(result.len(), 4);
        assert_eq!(result[0], TimeRange::new(h(8), h(10)));
        assert_eq!(result[1], TimeRange::new(h(10), h(12)));
        assert_eq!(result[2], TimeRange::new(h(12), h(14)));
        assert_eq!(result[3], TimeRange::new(h(14), h(16)));
    }

    #[test]
    fn test_shift() {
        let range = TimeRange::new(h(8), h(18));
        let shifted = range.shift(Duration::hours(2));
        assert_eq!(shifted, TimeRange::new(h(10), h(20)));
    }

    #[test]
    fn test_front_at_and_back_at() {
        let ranges = vec![TimeRange::new(h(8), h(12)), TimeRange::new(h(14), h(18))];
        // frontAt(1) = gap between ranges[0] and ranges[1]
        let front = front_at(&ranges, 1);
        assert_eq!(front, Some(TimeRange::new(h(12), h(14))));

        // backAt(0) = gap between ranges[0] and ranges[1]
        let back = back_at(&ranges, 0);
        assert_eq!(back, Some(TimeRange::new(h(12), h(14))));
    }

    // ========================================================================
    // 边界值测试 / Boundary value tests
    // ========================================================================

    #[test]
    fn test_empty_range_operations() {
        let empty = TimeRange::empty();
        assert!(empty.is_empty());
        assert_eq!(empty.split_at(&[h(10)]), vec![empty]);

        // Intersection with empty range
        let other = TimeRange::new(h(8), h(12));
        assert!(empty.intersection(&other).is_none());

        // Difference from empty range
        assert!(empty.difference(&other).is_empty() || empty.difference(&other)[0].is_empty());
    }

    #[test]
    fn test_full_range_contains_distant_past() {
        let full = TimeRange::full();
        assert!(full.contains_instant(distant_past()));
        assert!(!full.contains_instant(distant_future())); // half-open: excludes end
    }

    #[test]
    fn test_default_is_empty() {
        let default: TimeRange = TimeRange::default();
        assert!(default.is_empty());
    }

    #[test]
    fn test_difference_with_many_empty_input() {
        let base = TimeRange::new(h(8), h(18));
        let result = base.difference_with_many(&[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], base);
    }
}
