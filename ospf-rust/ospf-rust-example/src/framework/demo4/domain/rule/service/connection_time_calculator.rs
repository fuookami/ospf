//! 航段连接时间计算器 / Flight connection time calculator

use super::super::model::Link;
use time::Duration;

/// 连接时间计算器 / Connection time calculator
/// 对齐 Kotlin ConnectionTimeCalculator
pub struct ConnectionTimeCalculator;

impl ConnectionTimeCalculator {
    /// 根据链接列表计算各航段间的连接时间 / Calculate connection times between flight legs from link list
    pub fn calculate(&self, links: &[Link]) -> Vec<(String, String, Duration)> {
        links
            .iter()
            .map(|link| {
                (
                    link.from_task.clone(),
                    link.to_task.clone(),
                    link.min_connection_time(),
                )
            })
            .collect()
    }
}
