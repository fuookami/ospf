//! 资源容量定义 / Resource capacity definitions
//!
//! 定义资源的容量规格，包括时间范围、数量范围和松弛限制。
//! Defines resource capacity specifications including time range, quantity range, and slack limits.

use crate::infrastructure::TimeRange;

/// 资源容量 / Resource capacity
///
/// 描述资源在特定时间范围内的可用量及其松弛限制。
/// Describes the available quantity of a resource within a specific time range and its slack limits.
#[derive(Debug, Clone)]
pub struct ResourceCapacity {
    /// 时间范围 / Time range
    pub time_range: TimeRange,
    /// 数量下界（solver 值域）/ Lower bound in solver value domain
    pub lower_bound: f64,
    /// 数量上界（solver 值域）/ Upper bound in solver value domain
    pub upper_bound: f64,
    /// 允许不足量上限 / Allowed less quantity limit
    pub less_slack_limit: Option<f64>,
    /// 允许过量上限 / Allowed over quantity limit
    pub over_slack_limit: Option<f64>,
}

impl ResourceCapacity {
    /// 创建新的资源容量 / Create new resource capacity
    pub fn new(time_range: TimeRange, lower_bound: f64, upper_bound: f64) -> Self {
        Self {
            time_range,
            lower_bound,
            upper_bound,
            less_slack_limit: None,
            over_slack_limit: None,
        }
    }

    /// 创建带松弛限制的资源容量 / Create resource capacity with slack limits
    pub fn with_slack(
        time_range: TimeRange,
        lower_bound: f64,
        upper_bound: f64,
        less_slack_limit: Option<f64>,
        over_slack_limit: Option<f64>,
    ) -> Self {
        Self {
            time_range,
            lower_bound,
            upper_bound,
            less_slack_limit,
            over_slack_limit,
        }
    }

    /// 是否允许不足量 / Whether less slack is enabled
    pub fn less_enabled(&self) -> bool {
        self.less_slack_limit.is_some_and(|v| v > 0.0)
    }

    /// 是否允许过量 / Whether over slack is enabled
    pub fn over_enabled(&self) -> bool {
        self.over_slack_limit.is_some_and(|v| v > 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::OffsetDateTime;
    use time::ext::NumericalDuration;

    fn test_time_range() -> TimeRange {
        TimeRange::new(
            OffsetDateTime::UNIX_EPOCH,
            OffsetDateTime::UNIX_EPOCH + 1.hours(),
        )
    }

    #[test]
    fn test_resource_capacity_bounds() {
        let cap = ResourceCapacity::new(test_time_range(), 5.0, 100.0);
        assert_eq!(cap.lower_bound, 5.0);
        assert_eq!(cap.upper_bound, 100.0);
        assert!(!cap.less_enabled());
        assert!(!cap.over_enabled());
    }

    #[test]
    fn test_resource_capacity_with_slack() {
        let cap =
            ResourceCapacity::with_slack(test_time_range(), 5.0, 100.0, Some(10.0), Some(20.0));
        assert!(cap.less_enabled());
        assert!(cap.over_enabled());
        assert_eq!(cap.less_slack_limit, Some(10.0));
        assert_eq!(cap.over_slack_limit, Some(20.0));
    }
}
