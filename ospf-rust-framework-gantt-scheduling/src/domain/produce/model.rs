//! 产出与消耗模型 / Produce and consumption models

/// 产出占位 / Produce placeholder
#[derive(Debug, Clone, Default)]
pub struct Produce;

/// 消耗占位 / Consumption placeholder
#[derive(Debug, Clone, Default)]
pub struct Consumption;

/// 生产任务占位 / Production task placeholder
#[derive(Debug, Clone, Default)]
pub struct ProductionTask;

/// 产出松弛占位 / Produce slack placeholder
#[derive(Debug, Clone, Default)]
pub struct ProduceSlack;
