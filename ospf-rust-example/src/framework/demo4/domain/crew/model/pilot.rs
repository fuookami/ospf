//! 飞行员模型模块 / Pilot model module.
use crate::framework::demo4::infrastructure::{WorkerNo, PilotRankNo, PilotCode};

/// 飞行员等级 / Pilot rank
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PilotRank {
    /// 等级编号 / Rank number
    pub no: PilotRankNo,
}

/// 飞行员 / Pilot
#[derive(Debug, Clone)]
pub struct Pilot {
    /// 工人编号 / Worker number
    pub worker_no: WorkerNo,
    /// 飞行员代码 / Pilot code
    pub code: PilotCode,
    /// 姓名 / Name
    pub name: String,
    /// 显示名称 / Display name
    pub display_name: Option<String>,
    /// 国籍 / Nationality
    pub nationality: String,
}

impl PartialEq for Pilot {
    fn eq(&self, other: &Self) -> bool { self.worker_no == other.worker_no }
}
impl Eq for Pilot {}
