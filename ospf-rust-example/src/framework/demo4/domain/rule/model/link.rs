//! 航段连接模型 / Flight leg link model

use time::Duration;

/// 链接类型 / Link type (对齐 Kotlin Link sealed class)
#[derive(Debug, Clone)]
pub enum LinkType {
    /// 联程连接，需满足最小连接时间 / Connecting link with minimum connection time
    Connecting { min_connection_time: Duration },
    /// 经停连接，需满足经停时间 / Stopover link with stopover time
    Stopover { stopover_time: Duration },
    /// 忽略连接时间的连接 / Connection time ignoring link
    ConnectionTimeIgnoring,
}

/// 链接 / Link
#[derive(Debug, Clone)]
pub struct Link {
    /// 起始任务标识 / Source task identifier
    pub from_task: String,
    /// 目标任务标识 / Target task identifier
    pub to_task: String,
    /// 链接类型 / Link type
    pub link_type: LinkType,
}

impl Link {
    /// 获取最小连接时间 / Get minimum connection time
    pub fn min_connection_time(&self) -> Duration {
        match &self.link_type {
            LinkType::Connecting { min_connection_time } => *min_connection_time,
            LinkType::Stopover { stopover_time } => *stopover_time,
            LinkType::ConnectionTimeIgnoring => Duration::ZERO,
        }
    }
}

/// 链接映射 / Link map (对齐 Kotlin LinkMap)
#[derive(Debug, Clone)]
pub struct LinkMap {
    /// 链接列表 / Link list
    pub links: Vec<Link>,
}

impl LinkMap {
    /// 获取指定任务之后的链接 / Get links after the specified task
    pub fn links_after(&self, task_id: &str) -> Vec<&Link> {
        self.links.iter().filter(|l| l.from_task == task_id).collect()
    }

    /// 获取指定任务之前的链接 / Get links before the specified task
    pub fn links_before(&self, task_id: &str) -> Vec<&Link> {
        self.links.iter().filter(|l| l.to_task == task_id).collect()
    }
}
