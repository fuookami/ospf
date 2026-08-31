use time::Duration;

/// 链接类型 / Link type (对齐 Kotlin Link sealed class)
#[derive(Debug, Clone)]
pub enum LinkType {
    Connecting { min_connection_time: Duration },
    Stopover { stopover_time: Duration },
    ConnectionTimeIgnoring,
}

/// 链接 / Link
#[derive(Debug, Clone)]
pub struct Link {
    pub from_task: String,
    pub to_task: String,
    pub link_type: LinkType,
}

impl Link {
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
    pub links: Vec<Link>,
}

impl LinkMap {
    pub fn links_after(&self, task_id: &str) -> Vec<&Link> {
        self.links.iter().filter(|l| l.from_task == task_id).collect()
    }

    pub fn links_before(&self, task_id: &str) -> Vec<&Link> {
        self.links.iter().filter(|l| l.to_task == task_id).collect()
    }
}
