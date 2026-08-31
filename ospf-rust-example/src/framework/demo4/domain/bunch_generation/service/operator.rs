use time::Duration;

/// 可行性判断器 / Feasibility judger (对齐 FSRA Operator.kt FeasibilityJudger)
pub type FeasibilityJudger = Box<dyn Fn(&str, Option<&str>, &str) -> bool + Send + Sync>;

/// 连接时间计算器 / Connection time calculator
pub type ConnectionTimeCalculator = Box<dyn Fn(&str, &str) -> Duration + Send + Sync>;

/// 最小出发时间计算器 / Minimum departure time calculator
pub type MinimumDepartureTimeCalculator = Box<dyn Fn(&str, &str) -> OffsetDateTime + Send + Sync>;

/// 成本计算器 / Cost calculator
pub type CostCalculator = Box<dyn Fn(&str, Option<&str>, &str) -> f64 + Send + Sync>;

/// 总成本计算器 / Total cost calculator
pub type TotalCostCalculator = Box<dyn Fn(&[String]) -> f64 + Send + Sync>;

use time::OffsetDateTime;
