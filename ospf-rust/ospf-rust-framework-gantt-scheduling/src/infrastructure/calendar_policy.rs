//! 工作日历扩展策略 / Working calendar extension policy
//!
//! 将复杂班次、额外停机、条件连接时间和休息规则封装为可注入策略。
//! Encapsulates complex shifts, extra downtime, conditional connection times,
//! and break rules as injectable policies.

use time::Duration;

use crate::infrastructure::{ActualTime, DurationRange, TimeRange, WorkingCalendar};

/// 日历查询 / Calendar query
#[derive(Debug, Clone)]
pub struct CalendarQuery {
    /// 目标时间 / Target time
    pub time: TimeRange,
    /// 额外不可用时间 / Extra unavailable ranges
    pub extra_unavailable: Vec<TimeRange>,
    /// 前置连接时间 / Before-connection time
    pub before_connection_time: Option<DurationRange>,
    /// 后置连接时间 / After-connection time
    pub after_connection_time: Option<DurationRange>,
    /// 休息规则 / Break rule
    pub break_time: Option<(DurationRange, Duration)>,
}

impl CalendarQuery {
    /// 创建日历查询 / Create calendar query
    pub fn new(time: TimeRange) -> Self {
        Self {
            time,
            extra_unavailable: Vec::new(),
            before_connection_time: None,
            after_connection_time: None,
            break_time: None,
        }
    }

    /// 添加额外不可用时间 / Add extra unavailable ranges
    pub fn with_extra_unavailable(mut self, extra_unavailable: Vec<TimeRange>) -> Self {
        self.extra_unavailable = extra_unavailable;
        self
    }

    /// 添加连接时间 / Add connection time
    pub fn with_connection_time(
        mut self,
        before_connection_time: Option<DurationRange>,
        after_connection_time: Option<DurationRange>,
    ) -> Self {
        self.before_connection_time = before_connection_time;
        self.after_connection_time = after_connection_time;
        self
    }

    /// 添加休息规则 / Add break rule
    pub fn with_break_time(mut self, break_time: Option<(DurationRange, Duration)>) -> Self {
        self.break_time = break_time;
        self
    }
}

/// 日历扩展策略 / Calendar extension policy
pub trait CalendarPolicy<V>: Send + Sync
where
    V: ospf_rust_core::solver::value::SolveValue,
{
    /// 计算实际时间 / Calculate actual time
    fn actual_time(&self, calendar: &WorkingCalendar<V>, query: &CalendarQuery) -> ActualTime;
}

/// 默认日历策略 / Default calendar policy
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultCalendarPolicy;

impl<V> CalendarPolicy<V> for DefaultCalendarPolicy
where
    V: ospf_rust_core::solver::value::SolveValue,
{
    fn actual_time(&self, calendar: &WorkingCalendar<V>, query: &CalendarQuery) -> ActualTime {
        calendar.actual_time_range(
            &query.time,
            &query.extra_unavailable,
            query.before_connection_time,
            query.after_connection_time,
            query.break_time,
        )
    }
}

/// 组合日历策略 / Composite calendar policy
///
/// 用固定额外不可用时间和固定连接规则表达复杂班次组合的最小扩展入口。
/// Provides a minimal extension point for complex shift combinations through
/// fixed extra downtime and fixed connection rules.
#[derive(Debug, Clone, Default)]
pub struct CompositeCalendarPolicy {
    /// 固定额外不可用时间 / Fixed extra unavailable ranges
    pub extra_unavailable: Vec<TimeRange>,
    /// 固定前置连接时间 / Fixed before-connection time
    pub before_connection_time: Option<DurationRange>,
    /// 固定后置连接时间 / Fixed after-connection time
    pub after_connection_time: Option<DurationRange>,
    /// 固定休息规则 / Fixed break rule
    pub break_time: Option<(DurationRange, Duration)>,
}

impl CompositeCalendarPolicy {
    /// 创建组合日历策略 / Create composite calendar policy
    pub fn new() -> Self {
        Self::default()
    }

    /// 增加不可用时间 / Add unavailable ranges
    pub fn with_extra_unavailable(mut self, extra_unavailable: Vec<TimeRange>) -> Self {
        self.extra_unavailable = extra_unavailable;
        self
    }

    /// 设置连接时间 / Set connection time
    pub fn with_connection_time(
        mut self,
        before_connection_time: Option<DurationRange>,
        after_connection_time: Option<DurationRange>,
    ) -> Self {
        self.before_connection_time = before_connection_time;
        self.after_connection_time = after_connection_time;
        self
    }

    /// 设置休息规则 / Set break rule
    pub fn with_break_time(mut self, break_time: Option<(DurationRange, Duration)>) -> Self {
        self.break_time = break_time;
        self
    }
}

impl<V> CalendarPolicy<V> for CompositeCalendarPolicy
where
    V: ospf_rust_core::solver::value::SolveValue,
{
    fn actual_time(&self, calendar: &WorkingCalendar<V>, query: &CalendarQuery) -> ActualTime {
        let mut extra = self.extra_unavailable.clone();
        extra.extend_from_slice(&query.extra_unavailable);
        let query = CalendarQuery {
            time: query.time,
            extra_unavailable: extra,
            before_connection_time: query.before_connection_time.or(self.before_connection_time),
            after_connection_time: query.after_connection_time.or(self.after_connection_time),
            break_time: query.break_time.or(self.break_time),
        };
        DefaultCalendarPolicy.actual_time(calendar, &query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::TimeWindow;
    use time::macros::datetime;

    fn h(hour: i64) -> time::OffsetDateTime {
        datetime!(2020-08-30 00:00 UTC) + Duration::hours(hour)
    }

    #[test]
    fn test_composite_calendar_policy_adds_shift_break_and_connection() {
        let window = TimeWindow::hours(TimeRange::new(h(8), h(20)), 0.0, true, 1.0);
        let calendar = WorkingCalendar::<f64>::new(window, vec![]);
        let policy = CompositeCalendarPolicy::new()
            .with_extra_unavailable(vec![TimeRange::new(h(10), h(12))])
            .with_connection_time(None, Some(DurationRange::fixed(Duration::minutes(30))));
        let query = CalendarQuery::new(TimeRange::new(h(8), h(12)));

        let actual = policy.actual_time(&calendar, &query);

        assert_eq!(actual.working_duration(), Duration::hours(4));
        assert!(actual.connection_duration() >= Duration::minutes(30));
        assert!(actual.time.end >= h(14));
    }
}
