//! 带宽约束和目标模块 / Bandwidth constraints and objectives module

/// 带宽成本目标子模块 / Bandwidth cost objective submodule
pub mod bandwidth_cost_objective;
/// 需求约束子模块 / Demand constraint submodule
pub mod demand_constraint;
/// 边带宽约束子模块 / Edge bandwidth constraint submodule
pub mod edge_bandwidth_constraint;
/// 服务容量约束子模块 / Service capacity constraint submodule
pub mod service_capacity_constraint;
/// 中转节点带宽约束子模块 / Transfer node bandwidth constraint submodule
pub mod transfer_node_bandwidth_constraint;

/// 应用带宽成本目标函数 / Apply bandwidth cost objective function
pub use bandwidth_cost_objective::apply_bandwidth_cost_objective;
/// 应用需求约束 / Apply demand constraints
pub use demand_constraint::apply_demand_constraints;
/// 应用边带宽约束 / Apply edge bandwidth constraints
pub use edge_bandwidth_constraint::apply_edge_bandwidth_constraints;
/// 应用服务容量约束 / Apply service capacity constraints
pub use service_capacity_constraint::apply_service_capacity_constraints;
/// 应用中转节点带宽约束 / Apply transfer node bandwidth constraints
pub use transfer_node_bandwidth_constraint::apply_transfer_node_bandwidth_constraints;
