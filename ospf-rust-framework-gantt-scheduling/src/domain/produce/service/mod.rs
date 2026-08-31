//! 产出与消耗服务 / Produce and consumption services
//!
//! 包含产出/消耗约束和目标 Pipeline 实现。
//! Contains produce/consumption constraint and objective pipeline implementations.

pub mod limits;

pub use limits::{
    ProduceQuantityConstraint,
    ConsumptionQuantityConstraint,
    ProduceOverQuantityMinimization,
    ProduceLessQuantityMinimization,
    ProduceQuantityMaximization,
    ProduceQuantityMinimization,
    ConsumptionOverQuantityMinimization,
    ConsumptionLessQuantityMinimization,
    ConsumptionQuantityMaximization,
    ConsumptionQuantityMinimization,
};
