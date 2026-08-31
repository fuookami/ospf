//! 业载最大化约束限制 / Payload maximization constraint limits.
use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};

/// 最大业载限制 / Max payload limit
/// 对齐 Kotlin MaxPayloadLimit
///
/// 限制总业载不超过最大业载 / Constrain total payload not to exceed maximum payload
pub fn apply_max_payload_limit(
    model: &mut MetaModel<f64>,
    max_payload: f64,
    x_idx: &[Vec<usize>],
    cargo_weights: &[f64],
    position_count: usize,
) -> Result<(), Box<dyn Error>> {
    // 总业载 = sum(cargo.weight * x[c][p])
    // 限制总业载 <= max_payload
    let mut coefficients: Vec<(usize, f64)> = Vec::new();
    for c in 0..cargo_weights.len() {
        for p in 0..position_count {
            if c < x_idx.len() && p < x_idx[c].len() {
                coefficients.push((x_idx[c][p], cargo_weights[c]));
            }
        }
    }
    if !coefficients.is_empty() {
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::LessEqual,
            max_payload,
            "payload_maximization_max_payload",
        )?;
    }
    Ok(())
}
