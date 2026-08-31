//! 资源松弛配置 / Resource slack configuration
//!
//! 定义资源容量松弛变量的配置结构。
//! Defines the configuration structure for resource capacity slack variables.

/// 资源松弛配置 / Resource slack configuration
///
/// 描述资源容量允许的过量/不足量限制。
/// Describes the allowed over/less quantity limits for resource capacity.
#[derive(Debug, Clone)]
pub struct ResourceSlack {
    /// 允许不足量上限 / Allowed less quantity limit
    pub less_slack_limit: Option<f64>,
    /// 允许过量上限 / Allowed over quantity limit
    pub over_slack_limit: Option<f64>,
}

impl ResourceSlack {
    /// 创建新的资源松弛配置（无松弛）/ Create new resource slack configuration (no slack)
    pub fn new() -> Self {
        Self {
            less_slack_limit: None,
            over_slack_limit: None,
        }
    }

    /// 创建带限制的资源松弛配置 / Create resource slack configuration with limits
    pub fn with_limits(less_slack_limit: Option<f64>, over_slack_limit: Option<f64>) -> Self {
        Self {
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

impl Default for ResourceSlack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_slack_default() {
        let slack = ResourceSlack::default();
        assert!(!slack.less_enabled());
        assert!(!slack.over_enabled());
    }

    #[test]
    fn test_resource_slack_with_limits() {
        let slack = ResourceSlack::with_limits(Some(10.0), Some(20.0));
        assert!(slack.less_enabled());
        assert!(slack.over_enabled());
        assert_eq!(slack.less_slack_limit, Some(10.0));
        assert_eq!(slack.over_slack_limit, Some(20.0));
    }

    #[test]
    fn test_resource_slack_zero_limits() {
        let slack = ResourceSlack::with_limits(Some(0.0), Some(0.0));
        assert!(!slack.less_enabled());
        assert!(!slack.over_enabled());
    }
}
