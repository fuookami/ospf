//! 机组成员模型模块 / Crew member model module.
use crate::framework::demo4::infrastructure::{WorkerNo, CrewManRankNo};

/// 机组成员等级 / Crew member rank
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CrewManRank {
    /// 等级编号 / Rank number
    pub no: CrewManRankNo,
}

/// 机组成员 / Crew member
#[derive(Debug, Clone)]
pub struct CrewMan {
    /// 工人编号 / Worker number
    pub worker_no: WorkerNo,
    /// 姓名 / Name
    pub name: String,
    /// 显示名称 / Display name
    pub display_name: Option<String>,
    /// 国籍 / Nationality
    pub nationality: String,
}

impl PartialEq for CrewMan {
    fn eq(&self, other: &Self) -> bool { self.worker_no == other.worker_no }
}
impl Eq for CrewMan {}
