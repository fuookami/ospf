use time::Duration;
use super::super::model::Link;

/// 连接时间计算器 / Connection time calculator
/// 对齐 Kotlin ConnectionTimeCalculator
pub struct ConnectionTimeCalculator;

impl ConnectionTimeCalculator {
    pub fn calculate(&self, links: &[Link]) -> Vec<(String, String, Duration)> {
        links
            .iter()
            .map(|link| {
                (link.from_task.clone(), link.to_task.clone(), link.min_connection_time())
            })
            .collect()
    }
}
