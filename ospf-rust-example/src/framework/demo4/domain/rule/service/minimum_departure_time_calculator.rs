//! 最早起飞时间计算器 / Earliest departure time calculator

use time::OffsetDateTime;
use super::super::model::Link;

/// 最小出发时间计算器 / Minimum departure time calculator
/// 对齐 Kotlin MinimumDepartureTimeCalculator
pub struct MinimumDepartureTimeCalculator;

impl MinimumDepartureTimeCalculator {
    /// 根据到达时间和连接时间计算最小出发时间 / Calculate minimum departure time based on arrival time and connection time
    pub fn calculate(
        &self,
        arrival_time: OffsetDateTime,
        connection_time: time::Duration,
    ) -> OffsetDateTime {
        arrival_time + connection_time
    }
}
