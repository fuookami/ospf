use crate::framework::demo4::infrastructure::{WorkerNo, PilotRankNo, PilotCode};

/// 飞行员等级 / Pilot rank
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PilotRank {
    pub no: PilotRankNo,
}

/// 飞行员 / Pilot
#[derive(Debug, Clone)]
pub struct Pilot {
    pub worker_no: WorkerNo,
    pub code: PilotCode,
    pub name: String,
    pub display_name: Option<String>,
    pub nationality: String,
}

impl PartialEq for Pilot {
    fn eq(&self, other: &Self) -> bool { self.worker_no == other.worker_no }
}
impl Eq for Pilot {}
