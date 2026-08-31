//! 任务束编译模型 / Bunch compilation models

/// 任务束聚合占位 / Bunch aggregation placeholder
#[derive(Debug, Clone, Default)]
pub struct BunchAggregation;

/// 任务束调度解占位 / Bunch scheduling solution placeholder
#[derive(Debug, Clone, Default)]
pub struct BunchSchedulingSolution;

/// 基于时隙的任务束占位 / Slot-based bunch placeholder
#[derive(Debug, Clone, Default)]
pub struct SlotBasedBunch;

/// 基于时隙的产能结果占位 / Slot-based capacity result placeholder
#[derive(Debug, Clone, Default)]
pub struct SlotBasedCapacityResult;
