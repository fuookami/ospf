use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};

/// 物品顺序限制 / Item order limit
/// 对齐 Kotlin ItemOrderLimit
pub fn apply_item_order_limit(
    _model: &mut MetaModel<f64>,
    _x_idx: &[Vec<usize>],
    _cargo_weights: &[f64],
    _position_count: usize,
) -> Result<(), Box<dyn Error>> {
    // 同一目的地的货物应装载在相邻舱位
    // 简化实现: 不添加硬约束
    Ok(())
}

/// 优先预约限制 / Priority appointment limit
/// 对齐 Kotlin PriorityAppointmentLimit
pub fn apply_priority_appointment_limit(
    model: &mut MetaModel<f64>,
    x_idx: &[Vec<usize>],
    cargo_priorities: &[u8],
    position_count: usize,
) -> Result<(), Box<dyn Error>> {
    // 高优先级货物必须装载
    for c in 0..cargo_priorities.len() {
        if cargo_priorities[c] >= 8 {
            let coefficients: Vec<(usize, f64)> = (0..position_count)
                .filter_map(|p| {
                    if c < x_idx.len() && p < x_idx[c].len() {
                        Some((x_idx[c][p], 1.0))
                    } else {
                        None
                    }
                })
                .collect();
            if !coefficients.is_empty() {
                model.add_linear_constraint(
                    &coefficients,
                    ConstraintRelation::Equal,
                    1.0,
                    &format!("rwe_priority_appointment_{}", c),
                )?;
            }
        }
    }
    Ok(())
}

/// 推荐重量均衡限制 / Recommended weight equalization limit
/// 对齐 Kotlin RecommendedWeightEqualizationLimit
pub fn apply_recommended_weight_equalization_limit(
    model: &mut MetaModel<f64>,
    x_idx: &[Vec<usize>],
    cargo_weights: &[f64],
    position_count: usize,
) -> Result<(), Box<dyn Error>> {
    // 各舱位装载重量应均衡
    let total_weight: f64 = cargo_weights.iter().sum();
    if position_count > 0 {
        let avg_weight = total_weight / position_count as f64;
        let max_weight = avg_weight * 1.5;
        for p in 0..position_count {
            let coefficients: Vec<(usize, f64)> = (0..cargo_weights.len())
                .filter_map(|c| {
                    if c < x_idx.len() && p < x_idx[c].len() {
                        Some((x_idx[c][p], cargo_weights[c]))
                    } else {
                        None
                    }
                })
                .collect();
            if !coefficients.is_empty() {
                model.add_linear_constraint(
                    &coefficients,
                    ConstraintRelation::LessEqual,
                    max_weight,
                    &format!("rwe_weight_equalization_{}", p),
                )?;
            }
        }
    }
    Ok(())
}
