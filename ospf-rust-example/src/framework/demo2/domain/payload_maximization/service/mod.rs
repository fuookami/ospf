//! 业载最大化领域服务 / Payload maximization domain service.
pub mod limits;

use super::Aggregation;
use ospf_rust_core::model::MetaModel;
use std::error::Error;

/// 生成业载最大化管线 / Generate payload maximization pipelines
pub fn generate_pipelines(
    aggregation: &Aggregation,
    model: &mut MetaModel<f64>,
    x_idx: &[Vec<usize>],
    cargo_weights: &[f64],
    position_count: usize,
) -> Result<(), Box<dyn Error>> {
    limits::apply_max_payload_limit(
        model,
        aggregation.payload_estimate,
        x_idx,
        cargo_weights,
        position_count,
    )?;
    Ok(())
}
