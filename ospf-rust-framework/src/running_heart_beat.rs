//! 运行心跳（Kotlin 对齐占位） / Running heartbeat (Kotlin-aligned placeholder)

use std::time::{Duration, Instant};

/// 运行心跳，用于跟踪运行时长和心跳计数 / Running heartbeat for tracking uptime and tick count
#[derive(Debug, Clone)]
pub struct RunningHeartBeat {
    started_at: Instant,
    last_tick_at: Instant,
    tick_count: usize,
}

impl RunningHeartBeat {
    /// 创建新的运行心跳实例 / Create a new running heartbeat instance
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            started_at: now,
            last_tick_at: now,
            tick_count: 0,
        }
    }

    /// 记录一次心跳 / Record a heartbeat tick
    pub fn tick(&mut self) {
        self.last_tick_at = Instant::now();
        self.tick_count += 1;
    }

    /// 获取自启动以来的运行时长 / Get the duration since the heartbeat was started
    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// 获取心跳计数 / Get the total number of heartbeat ticks recorded
    pub fn tick_count(&self) -> usize {
        self.tick_count
    }
}

impl Default for RunningHeartBeat {
    fn default() -> Self {
        Self::new()
    }
}
