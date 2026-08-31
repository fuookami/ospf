//! 推荐重量均衡领域服务 / Recommended weight equalization domain service.
pub mod limits;

use std::error::Error;
use ospf_rust_core::model::MetaModel;
use crate::framework::demo2::domain::stowage::model::{LoadVariables, Position};

/// 生成推荐重量均衡管线 / Generate recommended weight equalization pipelines
pub fn generate_pipelines(
    model: &mut MetaModel<f64>,
    x_idx: &[Vec<usize>],
    cargo_weights: &[f64],
    cargo_priorities: &[u8],
    position_count: usize,
    load_vars: &LoadVariables,
    positions: &[Position],
) -> Result<(), Box<dyn Error>> {
    limits::apply_item_order_limit(model, x_idx, cargo_weights, position_count)?;
    limits::apply_priority_appointment_limit(model, x_idx, cargo_priorities, position_count)?;
    limits::apply_recommended_weight_equalization_limit(model, load_vars, positions)?;
    Ok(())
}
