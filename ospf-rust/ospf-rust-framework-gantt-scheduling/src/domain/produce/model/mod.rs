//! 产出与消耗模型 / Produce and consumption models
//!
//! 包含物料类型、需求/储备、生产任务和使用量组件。
//! Contains material types, demand/reserves, production task, and usage components.

pub mod demand;
pub mod material;
pub mod production_task;
pub mod usage;

pub use demand::{MaterialDemand, MaterialReserves};
pub use material::{MaterialTrait, Product, RawMaterial, SemiProduct};
pub use production_task::ProductionTaskTrait;
pub use usage::{ConsumptionUsage, ProduceUsage};
