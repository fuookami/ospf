use super::pilot::{Pilot, PilotRank};
use super::crew_man::{CrewMan, CrewManRank};

/// 机组成员类型 / Crew type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CrewType {
    Operator,
    Attendant,
    Other,
}

/// 机组成员（密封接口）/ Crew member (sealed interface)
#[derive(Debug, Clone)]
pub enum CrewMember {
    Pilot {
        crew_type: CrewType,
        rank: PilotRank,
        pilot: Pilot,
    },
    NotPilot {
        crew_type: CrewType,
        rank: CrewManRank,
        crew_man: CrewMan,
    },
}

/// 机组 / Crew
#[derive(Debug, Clone)]
pub struct Crew {
    pub flight_id: String,
    pub members: Vec<CrewMember>,
}

/// 机组排班 / Crew schedule
#[derive(Debug, Clone)]
pub struct CrewSchedule {
    pub crew_man: CrewMan,
    pub flights: Vec<String>,
}

/// 中转时间 / Transit time
#[derive(Debug, Clone)]
pub struct TransitTime {
    pub from_airport: String,
    pub to_airport: String,
    pub duration: time::Duration,
}
