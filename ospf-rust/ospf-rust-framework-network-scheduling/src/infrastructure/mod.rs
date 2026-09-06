//! 通用网络基础设施 / Generic network infrastructure.
//!
//! 这里不包含 depot、customer、vehicle、route 或时间窗语义。
//! This layer deliberately contains no depot, customer, vehicle, route, or time-window semantics.

mod capacity;
mod cost;
mod flow;
mod id;
mod network;
mod solver_value_adapter;

pub use capacity::{CapacityBounds, Flow, NodeBalance};
pub use cost::NetworkCost;
pub use flow::{FlowIndex, FlowValue};
pub use id::{NetworkArcId, NetworkNodeId};
pub use network::{NetworkArc, NetworkGraph, NetworkGraphBuilder, NetworkNode};
pub use solver_value_adapter::NetworkSchedulingSolverValueAdapter;
