//! 旅客领域服务 / Passenger domain service.
/// 旅客约束模块 / Passenger constraints module
pub mod limits;

use super::Aggregation;
use ospf_rust_core::model::MetaModel;
use std::error::Error;

/// 旅客管道列表生成器 / Passenger pipeline list generator
/// 对齐 Kotlin passenger PipelineListGenerator / Aligned with Kotlin passenger PipelineListGenerator
pub fn generate_pipelines(
    aggregation: &Aggregation,
    model: &mut MetaModel<f64>,
) -> Result<(), Box<dyn Error>> {
    limits::apply_passenger_flight_capacity_constraint(model, &aggregation.amounts, &[])?;
    limits::apply_passenger_route_cancel_constraint(model, &aggregation.cancels)?;
    limits::apply_passenger_flight_change_constraint(model, &aggregation.changes)?;
    limits::apply_passenger_cancel_minimization(model, &aggregation.cancels)?;
    limits::apply_passenger_class_change_minimization(model, &aggregation.changes)?;
    limits::apply_passenger_flight_change_minimization(model, &aggregation.changes)?;
    Ok(())
}
