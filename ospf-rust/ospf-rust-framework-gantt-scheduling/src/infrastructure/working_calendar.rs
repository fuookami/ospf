//! 工作日历 / Working calendar managing unavailable times
//!
//! 管理不可用时间段，计算实际工作时间和有效时间范围。
//! Manages unavailable time periods, computes actual working times and valid time ranges.

use crate::infrastructure::{DurationRange, TimeRange, TimeWindow, distant_future, merge};
use ospf_rust_core::solver::value::SolveValue;
use time::{Duration, OffsetDateTime};

/// 实际时间结果 / Actual time result
///
/// 包含实际时间范围、工作时间、休息时间和连接时间。
/// Contains the actual time range, working times, break times, and connection times.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActualTime {
    /// 实际时间范围 / Actual time range
    pub time: TimeRange,
    /// 工作时间段列表 / Working time ranges
    pub working_times: Vec<TimeRange>,
    /// 休息时间段列表 / Break time ranges
    pub break_times: Vec<TimeRange>,
    /// 连接时间段列表 / Connection time ranges
    pub connection_times: Vec<TimeRange>,
}

impl ActualTime {
    /// 工作时长 / Working duration
    pub fn working_duration(&self) -> Duration {
        self.working_times.iter().map(|t| t.duration()).sum()
    }

    /// 休息时长 / Break duration
    pub fn break_duration(&self) -> Duration {
        self.break_times.iter().map(|t| t.duration()).sum()
    }

    /// 连接时长 / Connection duration
    pub fn connection_duration(&self) -> Duration {
        self.connection_times.iter().map(|t| t.duration()).sum()
    }
}

/// 有效时间结果 / Valid times result
///
/// 包含有效时间段、休息时间和连接时间。
/// Contains valid time ranges, break times, and connection times.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidTimes {
    /// 有效时间段列表 / Valid time ranges
    pub times: Vec<TimeRange>,
    /// 休息时间段列表 / Break time ranges
    pub break_times: Vec<TimeRange>,
    /// 连接时间段列表 / Connection time ranges
    pub connection_times: Vec<TimeRange>,
}

impl ValidTimes {
    /// 有效时长 / Valid duration
    pub fn duration(&self) -> Duration {
        self.times.iter().map(|t| t.duration()).sum()
    }

    /// 休息时长 / Break duration
    pub fn break_duration(&self) -> Duration {
        self.break_times.iter().map(|t| t.duration()).sum()
    }

    /// 连接时长 / Connection duration
    pub fn connection_duration(&self) -> Duration {
        self.connection_times.iter().map(|t| t.duration()).sum()
    }
}

/// 工作日历 / Working calendar
///
/// 管理不可用时间段，提供实际时间计算和有效时间范围计算。
/// Manages unavailable time periods, provides actual time calculation
/// and valid time range computation.
///
/// # 泛型参数 / Type Parameters
///
/// - `V`: 数值类型，必须实现 `SolveValue`
/// - `V`: Numeric type, must implement `SolveValue`
#[derive(Debug, Clone)]
pub struct WorkingCalendar<V: SolveValue> {
    /// 时间窗口 / Time window
    pub time_window: TimeWindow<V>,
    /// 不可用时间列表（已排序）/ Sorted unavailable times
    pub unavailable_times: Vec<TimeRange>,
}

impl<V: SolveValue> WorkingCalendar<V> {
    /// 创建新的工作日历 / Create new working calendar
    pub fn new(time_window: TimeWindow<V>, unavailable_times: Vec<TimeRange>) -> Self {
        let mut sorted = unavailable_times;
        sorted.sort_by_key(|r| r.start);
        Self {
            time_window,
            unavailable_times: sorted,
        }
    }

    /// 获取合并后的不可用时间 / Get merged unavailable times
    ///
    /// 将额外的不可用时间与实例的不可用时间合并。
    /// Merges additional unavailable times with the instance's unavailable times.
    pub fn merged_unavailable_times(&self, extra: &[TimeRange]) -> Vec<TimeRange> {
        let mut all = self.unavailable_times.clone();
        all.extend_from_slice(extra);
        merge(&all)
    }

    /// 计算实际时间点 / Calculate actual time point
    ///
    /// 考虑不可用时间和连接时间，计算给定时间点实际可用的最早时间。
    /// Considering unavailable times and connection times, computes the earliest
    /// available time for the given time point.
    ///
    /// # 参数 / Parameters
    ///
    /// - `time`: 目标时间点 / Target time point
    /// - `extra_unavailable`: 额外不可用时间 / Extra unavailable times
    /// - `before_connection_time`: 不可用时段前的连接时间 / Connection time before unavailable period
    /// - `after_connection_time`: 不可用时段后的连接时间 / Connection time after unavailable period
    pub fn actual_time_at(
        &self,
        time: OffsetDateTime,
        extra_unavailable: &[TimeRange],
        before_connection_time: Option<DurationRange>,
        after_connection_time: Option<DurationRange>,
    ) -> OffsetDateTime {
        let merged = self.merged_unavailable_times(extra_unavailable);
        if merged.is_empty() {
            return time;
        }

        let before_lb = before_connection_time
            .map(|ct| ct.lower)
            .unwrap_or(Duration::ZERO);
        let after_lb = after_connection_time
            .map(|ct| ct.lower)
            .unwrap_or(Duration::ZERO);

        let mut current_time = time;

        for unavail in &merged {
            // Check if time is safely before this unavailable period
            let boundary = if before_lb > Duration::ZERO {
                unavail.start - before_lb
            } else {
                unavail.start
            };

            if current_time < boundary {
                return current_time;
            }

            // Push current_time past the unavailable period + after connection time
            let after = unavail.end + after_lb;
            current_time = current_time.max(after);
        }

        current_time
    }

    /// 计算实际时间范围 / Calculate actual time range
    ///
    /// 考虑不可用时间、连接时间和休息时间，计算给定时间范围的实际工作时间。
    /// Considering unavailable times, connection times, and break times,
    /// computes the actual working time for the given time range.
    ///
    /// # 参数 / Parameters
    ///
    /// - `time`: 目标时间范围 / Target time range
    /// - `extra_unavailable`: 额外不可用时间 / Extra unavailable times
    /// - `before_connection_time`: 不可用时段前的连接时间 / Connection time before unavailable period
    /// - `after_connection_time`: 不可用时段后的连接时间 / Connection time after unavailable period
    /// - `break_time`: 休息时间配置 (工作间隔, 休息时长) / Break time config (work interval, break duration)
    pub fn actual_time_range(
        &self,
        time: &TimeRange,
        extra_unavailable: &[TimeRange],
        before_connection_time: Option<DurationRange>,
        after_connection_time: Option<DurationRange>,
        break_time: Option<(DurationRange, Duration)>,
    ) -> ActualTime {
        let merged = self.merged_unavailable_times(extra_unavailable);

        // No unavailable times
        if merged.is_empty() {
            return if let Some((unit, bt)) = break_time {
                let split =
                    time.split_by_duration(&unit, Duration::ZERO, Some(time.duration()), Some(bt));
                ActualTime {
                    time: TimeRange::new(
                        time.start,
                        time.start
                            + split.times.iter().map(|t| t.duration()).sum::<Duration>()
                            + split
                                .break_times
                                .iter()
                                .map(|t| t.duration())
                                .sum::<Duration>(),
                    ),
                    working_times: split.times,
                    break_times: split.break_times,
                    connection_times: vec![],
                }
            } else {
                ActualTime {
                    time: *time,
                    working_times: vec![*time],
                    break_times: vec![],
                    connection_times: vec![],
                }
            };
        }

        let mut working_times = Vec::new();
        let mut break_times = Vec::new();
        let mut connection_times = Vec::new();
        let mut current_time = time.start;
        let mut total_duration = Duration::ZERO;
        let target_duration = time.duration();

        let before_lb = before_connection_time
            .map(|ct| ct.lower)
            .unwrap_or(Duration::ZERO);
        let before_ub = before_connection_time
            .map(|ct| ct.upper)
            .unwrap_or(Duration::ZERO);
        let after_lb = after_connection_time
            .map(|ct| ct.lower)
            .unwrap_or(Duration::ZERO);
        let after_ub = after_connection_time
            .map(|ct| ct.upper)
            .unwrap_or(Duration::ZERO);

        // Find starting index: last unavailable period that ended before or at current_time
        let mut last_idx = find_last_ended_before_or_at(&merged, current_time);
        let next_idx = |last: Option<usize>| -> Option<usize> {
            match last {
                None => Some(0),
                Some(i) if i + 1 < merged.len() => Some(i + 1),
                _ => None,
            }
        };

        while total_duration < target_duration {
            // Check if we're inside an unavailable period
            if let Some(ni) = next_idx(last_idx)
                && merged[ni].contains_instant(current_time)
            {
                current_time = merged[ni].end;
                last_idx = Some(ni);
                continue;
            }

            // Terminal condition
            if next_idx(last_idx).is_none() && current_time == distant_future() {
                break;
            }

            // Calculate this end time (before the next unavailable period's connection time)
            let this_end_time = if let Some(ni) = next_idx(last_idx) {
                if before_ub > Duration::ZERO {
                    merged[ni].start.saturating_sub(before_ub)
                } else {
                    merged[ni].start
                }
            } else {
                distant_future()
            };

            // Add after-connection time if needed
            if let Some(li) = last_idx
                && after_lb > Duration::ZERO
                && current_time <= merged[li].end
            {
                let conn_end = current_time.saturating_add(after_ub);
                let conn = TimeRange::new(current_time, conn_end);
                connection_times.push(conn);
                current_time = conn_end;
            }

            // Create base time for this segment
            // Note: time.end is the initial estimate, not a hard cap.
            // Work can extend beyond time.end to complete target_duration.
            let base_time = TimeRange::new(current_time, this_end_time);

            if base_time.is_empty() {
                // Advance past the unavailable period
                if let Some(ni) = next_idx(last_idx) {
                    current_time = merged[ni].end;
                    last_idx = Some(ni);
                } else {
                    break;
                }
                continue;
            }

            // Process working time within this segment
            if let Some((unit, bt)) = break_time {
                let offset = Duration::ZERO;
                let max_dur = target_duration - total_duration;
                let split = base_time.split_by_duration(&unit, offset, Some(max_dur), Some(bt));

                for wt in &split.times {
                    total_duration += wt.duration();
                }
                working_times.extend(split.times);
                break_times.extend(split.break_times);

                // Update current_time to the end of the last working/break segment
                let max_end = working_times
                    .iter()
                    .chain(break_times.iter())
                    .map(|t| t.end)
                    .max()
                    .unwrap_or(current_time);
                current_time = max_end;
            } else {
                let duration = base_time.duration().min(target_duration - total_duration);
                if duration > Duration::ZERO {
                    let work_end = current_time.saturating_add(duration);
                    let work = TimeRange::new(current_time, work_end);
                    working_times.push(work);
                    total_duration += duration;
                    current_time = work_end;
                }
            }

            // Add before-connection time if there's more work to do
            if total_duration < target_duration
                && before_lb > Duration::ZERO
                && let Some(_ni) = next_idx(last_idx)
            {
                let conn_end = this_end_time.saturating_add(before_ub);
                let conn = TimeRange::new(this_end_time, conn_end);
                connection_times.push(conn);
            }

            // Advance past the unavailable period
            if let Some(ni) = next_idx(last_idx) {
                if total_duration < target_duration {
                    current_time = merged[ni].end;
                    last_idx = Some(ni);
                }
            } else {
                break;
            }
        }

        let actual_end = working_times
            .iter()
            .chain(break_times.iter())
            .chain(connection_times.iter())
            .map(|t| t.end)
            .max()
            .unwrap_or(time.end);

        ActualTime {
            time: TimeRange::new(time.start, actual_end),
            working_times,
            break_times,
            connection_times,
        }
    }

    /// 计算有效时间范围 / Calculate valid time ranges
    ///
    /// 计算给定时间范围内的所有有效工作时间段。
    /// Computes all valid working time ranges within the given time range.
    pub fn valid_times(
        &self,
        time: &TimeRange,
        extra_unavailable: &[TimeRange],
        before_connection_time: Option<DurationRange>,
        after_connection_time: Option<DurationRange>,
        max_duration: Option<Duration>,
        break_time: Option<(DurationRange, Duration)>,
    ) -> ValidTimes {
        let merged = self.merged_unavailable_times(extra_unavailable);

        // No unavailable times
        if merged.is_empty() {
            let effective_time = if let Some(max_d) = max_duration {
                TimeRange::new(time.start, (time.start + max_d).min(time.end))
            } else {
                *time
            };

            return if let Some((unit, bt)) = break_time {
                let split =
                    effective_time.split_by_duration(&unit, Duration::ZERO, max_duration, Some(bt));
                ValidTimes {
                    times: split.times,
                    break_times: split.break_times,
                    connection_times: vec![],
                }
            } else {
                ValidTimes {
                    times: vec![effective_time],
                    break_times: vec![],
                    connection_times: vec![],
                }
            };
        }

        let mut valid = Vec::new();
        let mut break_times = Vec::new();
        let mut connection_times = Vec::new();
        let mut current_time = time.start;

        let before_ub = before_connection_time
            .map(|ct| ct.upper)
            .unwrap_or(Duration::ZERO);
        let after_ub = after_connection_time
            .map(|ct| ct.upper)
            .unwrap_or(Duration::ZERO);

        let mut last_idx = find_last_ended_before_or_at(&merged, current_time);
        let mut total_duration = Duration::ZERO;

        let next_idx = |last: Option<usize>| -> Option<usize> {
            match last {
                None => Some(0),
                Some(i) if i + 1 < merged.len() => Some(i + 1),
                _ => None,
            }
        };

        while current_time < time.end {
            // Check if inside an unavailable period
            if let Some(ni) = next_idx(last_idx)
                && merged[ni].contains_instant(current_time)
            {
                current_time = merged[ni].end;
                last_idx = Some(ni);
                continue;
            }

            // Terminal condition
            if next_idx(last_idx).is_none() && current_time == distant_future() {
                break;
            }

            // Calculate end time for this segment
            let this_end = if let Some(ni) = next_idx(last_idx) {
                if before_ub > Duration::ZERO {
                    merged[ni].start.saturating_sub(before_ub)
                } else {
                    merged[ni].start
                }
            } else {
                time.end
            };

            let this_end = this_end.min(time.end);

            // Add after-connection time
            if let Some(li) = last_idx
                && after_ub > Duration::ZERO
                && current_time <= merged[li].end
            {
                let conn_end = current_time.saturating_add(after_ub);
                let conn = TimeRange::new(current_time, conn_end);
                connection_times.push(conn);
                current_time = conn_end;
            }

            if current_time >= this_end {
                if let Some(ni) = next_idx(last_idx) {
                    current_time = merged[ni].end;
                    last_idx = Some(ni);
                } else {
                    break;
                }
                continue;
            }

            // Create segment
            let mut segment_end = this_end;
            if let Some(max_d) = max_duration {
                let remaining = max_d - total_duration;
                segment_end = segment_end.min(current_time.saturating_add(remaining));
            }

            let base_time = TimeRange::new(current_time, segment_end);
            if base_time.is_empty() {
                if let Some(ni) = next_idx(last_idx) {
                    current_time = merged[ni].end;
                    last_idx = Some(ni);
                } else {
                    break;
                }
                continue;
            }

            // Process segment
            if let Some((unit, bt)) = break_time {
                let split = base_time.split_by_duration(
                    &unit,
                    Duration::ZERO,
                    max_duration.map(|md| md - total_duration),
                    Some(bt),
                );
                total_duration += split.times.iter().map(|t| t.duration()).sum::<Duration>();
                valid.extend(split.times);
                break_times.extend(split.break_times);
            } else {
                total_duration += base_time.duration();
                valid.push(base_time);
            }

            // Check max_duration completion
            if let Some(max_d) = max_duration
                && total_duration >= max_d
            {
                break;
            }

            // Add before-connection time
            if before_ub > Duration::ZERO
                && let Some(_ni) = next_idx(last_idx)
            {
                let conn_end = this_end.saturating_add(before_ub);
                connection_times.push(TimeRange::new(this_end, conn_end));
            }

            // Advance
            if let Some(ni) = next_idx(last_idx) {
                current_time = merged[ni].end;
                last_idx = Some(ni);
            } else {
                break;
            }
        }

        ValidTimes {
            times: valid,
            break_times,
            connection_times,
        }
    }
}

/// 查找在给定时间点之前或同时结束的最后一个不可用时段的索引
/// Find the index of the last unavailable period that ended before or at the given time
///
/// 返回 `None` 表示没有不可用时段在 `time` 之前结束。
/// Returns `None` if no unavailable period ended before `time`.
fn find_last_ended_before_or_at(merged: &[TimeRange], time: OffsetDateTime) -> Option<usize> {
    for (idx, range) in merged.iter().enumerate().rev() {
        if range.end <= time {
            return Some(idx);
        }
    }
    None
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    type TestCalendar = WorkingCalendar<f64>;

    fn h(day: i64, hour: i8) -> OffsetDateTime {
        datetime!(2020-08-30 00:00 UTC) + Duration::days(day) + Duration::hours(hour as i64)
    }

    #[test]
    fn test_actual_time_at_no_unavailable() {
        let window = TimeWindow::hours(TimeRange::new(h(0, 8), h(0, 18)), 0.0, true, 1.0);
        let calendar = TestCalendar::new(window, vec![]);
        let result = calendar.actual_time_at(h(0, 10), &[], None, None);
        assert_eq!(result, h(0, 10));
    }

    #[test]
    fn test_actual_time_at_with_unavailable() {
        let window = TimeWindow::hours(TimeRange::new(h(0, 8), h(0, 18)), 0.0, true, 1.0);
        let unavail = vec![TimeRange::new(h(0, 10), h(0, 12))];
        let calendar = TestCalendar::new(window, unavail);
        // 09:00 is before unavailable, should return 09:00
        let result = calendar.actual_time_at(h(0, 9), &[], None, None);
        assert_eq!(result, h(0, 9));

        // 11:00 is inside unavailable, should be pushed to 12:00
        let result = calendar.actual_time_at(h(0, 11), &[], None, None);
        assert_eq!(result, h(0, 12));
    }

    #[test]
    fn test_actual_time_at_with_connection_time() {
        let window = TimeWindow::hours(TimeRange::new(h(0, 8), h(0, 18)), 0.0, true, 1.0);
        let unavail = vec![TimeRange::new(h(0, 10), h(0, 12))];
        let calendar = TestCalendar::new(window, unavail);

        let after_conn = DurationRange::fixed(Duration::minutes(30));

        // 11:00 is inside unavailable + 30min after connection = 12:30
        let result = calendar.actual_time_at(h(0, 11), &[], None, Some(after_conn));
        assert_eq!(result, h(0, 12) + Duration::minutes(30));
    }

    #[test]
    fn test_actual_time_range_no_unavailable() {
        let window = TimeWindow::hours(TimeRange::new(h(0, 8), h(0, 18)), 0.0, true, 1.0);
        let calendar = TestCalendar::new(window, vec![]);
        let time = TimeRange::new(h(0, 8), h(0, 12));
        let result = calendar.actual_time_range(&time, &[], None, None, None);

        assert_eq!(result.working_times.len(), 1);
        assert_eq!(result.working_duration(), Duration::hours(4));
    }

    #[test]
    fn test_actual_time_range_with_unavailable() {
        let window = TimeWindow::hours(TimeRange::new(h(0, 8), h(0, 18)), 0.0, true, 1.0);
        let unavail = vec![TimeRange::new(h(0, 10), h(0, 11))];
        let calendar = TestCalendar::new(window, unavail);
        let time = TimeRange::new(h(0, 8), h(0, 12));
        let result = calendar.actual_time_range(&time, &[], None, None, None);

        // 4 hours of work starting at 08:00: 08-10 (2h) + 11-13 (2h) = actual time 08:00-13:00
        // The time range duration (4h) is the TOTAL work needed, not a fixed window.
        assert!(result.working_times.len() >= 2);
        assert_eq!(result.working_duration(), Duration::hours(4));
    }

    #[test]
    fn test_valid_times_no_unavailable() {
        let window = TimeWindow::hours(TimeRange::new(h(0, 8), h(0, 18)), 0.0, true, 1.0);
        let calendar = TestCalendar::new(window, vec![]);
        let time = TimeRange::new(h(0, 8), h(0, 12));
        let result = calendar.valid_times(&time, &[], None, None, None, None);

        assert_eq!(result.times.len(), 1);
        assert_eq!(result.duration(), Duration::hours(4));
    }

    #[test]
    fn test_valid_times_with_unavailable() {
        let window = TimeWindow::hours(TimeRange::new(h(0, 8), h(0, 18)), 0.0, true, 1.0);
        let unavail = vec![TimeRange::new(h(0, 10), h(0, 11))];
        let calendar = TestCalendar::new(window, unavail);
        let time = TimeRange::new(h(0, 8), h(0, 12));
        let result = calendar.valid_times(&time, &[], None, None, None, None);

        // Two valid ranges: 08-10 and 11-12
        assert_eq!(result.times.len(), 2);
        assert_eq!(result.duration(), Duration::hours(3));
    }

    #[test]
    fn test_valid_times_with_max_duration() {
        let window = TimeWindow::hours(TimeRange::new(h(0, 8), h(0, 18)), 0.0, true, 1.0);
        let calendar = TestCalendar::new(window, vec![]);
        let time = TimeRange::new(h(0, 8), h(0, 12));
        let result = calendar.valid_times(&time, &[], None, None, Some(Duration::hours(2)), None);

        assert_eq!(result.duration(), Duration::hours(2));
    }

    // ========================================================================
    // 边界值测试 / Boundary value tests
    // ========================================================================

    #[test]
    fn test_valid_times_all_unavailable() {
        let window = TimeWindow::hours(TimeRange::new(h(0, 8), h(0, 18)), 0.0, true, 1.0);
        let unavail = vec![TimeRange::new(h(0, 8), h(0, 18))];
        let calendar = TestCalendar::new(window, unavail);
        let time = TimeRange::new(h(0, 8), h(0, 18));
        let result = calendar.valid_times(&time, &[], None, None, None, None);
        assert!(result.times.is_empty());
    }

    #[test]
    fn test_actual_time_at_before_all_unavailable() {
        let window = TimeWindow::hours(TimeRange::new(h(0, 8), h(0, 18)), 0.0, true, 1.0);
        let unavail = vec![TimeRange::new(h(0, 12), h(0, 14))];
        let calendar = TestCalendar::new(window, unavail);
        // 09:00 is before the unavailable period
        let result = calendar.actual_time_at(h(0, 9), &[], None, None);
        assert_eq!(result, h(0, 9));
    }

    #[test]
    fn test_actual_time_at_after_all_unavailable() {
        let window = TimeWindow::hours(TimeRange::new(h(0, 8), h(0, 18)), 0.0, true, 1.0);
        let unavail = vec![TimeRange::new(h(0, 10), h(0, 12))];
        let calendar = TestCalendar::new(window, unavail);
        // 15:00 is after the unavailable period
        let result = calendar.actual_time_at(h(0, 15), &[], None, None);
        assert_eq!(result, h(0, 15));
    }
}
