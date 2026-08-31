use time::OffsetDateTime;
use super::super::model::Link;

/// 最小出发时间计算器 / Minimum departure time calculator
/// 对齐 Kotlin MinimumDepartureTimeCalculator
pub struct MinimumDepartureTimeCalculator;

impl MinimumDepartureTimeCalculator {
    pub fn calculate(
        &self,
        arrival_time: OffsetDateTime,
        connection_time: time::Duration,
    ) -> OffsetDateTime {
        arrival_time + connection_time
    }
}
