//! 路由约束和目标模块 / Route constraints and objectives module

/// 节点分配约束 / Node assignment constraints
pub mod node_assignment_constraint;
/// 服务分配约束 / Service assignment constraints
pub mod service_assignment_constraint;
/// 服务成本目标 / Service cost objective
pub mod service_cost_objective;

/// 应用节点分配约束 / Apply node assignment constraints
pub use node_assignment_constraint::apply_node_assignment_constraints;
/// 应用服务分配约束 / Apply service assignment constraints
pub use service_assignment_constraint::apply_service_assignment_constraints;
/// 应用服务成本目标 / Apply service cost objective
pub use service_cost_objective::apply_service_cost_objective;
