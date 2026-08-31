//! 层分配模型 / Layer assignment models

/// 不精确赋值占位 / Imprecise assignment placeholder
#[derive(Debug, Clone, Default)]
pub struct ImpreciseAssignment;

/// 精确赋值占位 / Precise assignment placeholder
#[derive(Debug, Clone, Default)]
pub struct PreciseAssignment;

/// 负载模型占位 / Load model placeholder
#[derive(Debug, Clone, Default)]
pub struct Load;

/// 容量模型占位 / Capacity model placeholder
#[derive(Debug, Clone, Default)]
pub struct Capacity;

/// 层聚合占位 / Layer aggregation placeholder
#[derive(Debug, Clone, Default)]
pub struct LayerAggregation;
