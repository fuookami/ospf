//! 机组模型模块 / Crew model module.
use super::pilot::{Pilot, PilotRank};
use super::crew_man::{CrewMan, CrewManRank};

/// 机组成员类型 / Crew type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CrewType {
    /// 操作员 / Operator
    Operator,
    /// 乘务员 / Attendant
    Attendant,
    /// 其他 / Other
    Other,
}

/// 机组成员（密封接口）/ Crew member (sealed interface)
#[derive(Debug, Clone)]
pub enum CrewMember {
    /// 飞行员 / Pilot
    Pilot {
        /// 机组类型 / Crew type
        crew_type: CrewType,
        /// 飞行员等级 / Pilot rank
        rank: PilotRank,
        /// 飞行员信息 / Pilot info
        pilot: Pilot,
    },
    /// 非飞行员 / Non-pilot crew member
    NotPilot {
        /// 机组类型 / Crew type
        crew_type: CrewType,
        /// 机组成员等级 / Crew member rank
        rank: CrewManRank,
        /// 机组成员信息 / Crew member info
        crew_man: CrewMan,
    },
}

/// 机组 / Crew
#[derive(Debug, Clone)]
pub struct Crew {
    /// 航班标识 / Flight identifier
    pub flight_id: String,
    /// 成员列表 / Member list
    pub members: Vec<CrewMember>,
}

/// 机组排班 / Crew schedule
#[derive(Debug, Clone)]
pub struct CrewSchedule {
    /// 机组成员 / Crew member
    pub crew_man: CrewMan,
    /// 航班列表 / Flight list
    pub flights: Vec<String>,
}

/// 中转时间 / Transit time
#[derive(Debug, Clone)]
pub struct TransitTime {
    /// 出发机场 / Departure airport
    pub from_airport: String,
    /// 到达机场 / Arrival airport
    pub to_airport: String,
    /// 中转时长 / Transit duration
    pub duration: time::Duration,
}
