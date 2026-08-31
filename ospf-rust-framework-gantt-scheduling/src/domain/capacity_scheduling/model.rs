//! 产能排程模型 / Capacity scheduling models

/// 产能列占位 / Capacity column placeholder
#[derive(Debug, Clone, Default)]
pub struct CapacityColumn;

/// 产能编译占位 / Capacity compilation placeholder
#[derive(Debug, Clone, Default)]
pub struct CapacityCompilation;

/// 产能排程解占位 / Capacity scheduling solution placeholder
#[derive(Debug, Clone, Default)]
pub struct CapacitySchedulingSolution;

/// 生产动作 trait 占位 / Production action trait placeholder
pub trait ProductionActionTrait: Send + Sync {}
