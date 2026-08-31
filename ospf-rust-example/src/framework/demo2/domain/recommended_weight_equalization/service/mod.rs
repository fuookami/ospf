pub mod limits;

use std::error::Error;
use ospf_rust_core::model::MetaModel;

pub fn generate_pipelines(
    model: &mut MetaModel<f64>,
    x_idx: &[Vec<usize>],
    cargo_weights: &[f64],
    cargo_priorities: &[u8],
    position_count: usize,
) -> Result<(), Box<dyn Error>> {
    limits::apply_item_order_limit(model, x_idx, cargo_weights, position_count)?;
    limits::apply_priority_appointment_limit(model, x_idx, cargo_priorities, position_count)?;
    limits::apply_recommended_weight_equalization_limit(model, x_idx, cargo_weights, position_count)?;
    Ok(())
}
