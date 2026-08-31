//! Demo5 输入、参数和 solver 选择 / Demo5 input, parameters, and solver selection.

mod adapter;
mod demo17;
mod dto;
mod semantic_parameter;
mod solomon;
mod solver;

pub use adapter::instance_from_solomon;
pub use demo17::{all100_instance, first25_instance, proof100_instance, small_instance};
pub use dto::{Demo5Input, SolomonData, SolomonNodeData, SolomonVehicleData};
pub use semantic_parameter::SemanticParameter;
pub use solomon::parse_solomon;
pub use solver::{Demo5Solver, native_solver_available};
