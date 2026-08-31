use crate::framework::demo4::infrastructure::{WorkerNo, CrewManRankNo};

/// 机组成员等级 / Crew member rank
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CrewManRank {
    pub no: CrewManRankNo,
}

/// 机组成员 / Crew member
#[derive(Debug, Clone)]
pub struct CrewMan {
    pub worker_no: WorkerNo,
    pub name: String,
    pub display_name: Option<String>,
    pub nationality: String,
}

impl PartialEq for CrewMan {
    fn eq(&self, other: &Self) -> bool { self.worker_no == other.worker_no }
}
impl Eq for CrewMan {}
