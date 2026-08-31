//! 推荐重量均衡约束限制 / Recommended weight equalization constraint limits.
use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::stowage::model::{LoadVariables, Position};

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
///
/// Kotlin 语义: 对于每对位置 (j1, j2)，当两者都需要推荐重量时:
///   z[j1] <= z[j2] + mlw[j1] * actualLoaded[j2]  (硬约束)
///   z[j2] <= z[j1] + mlw[j2] * actualLoaded[j1]  (硬约束)
/// 其中 z[j] 是位置 j 的推荐重量变量，mlw[j] 是最大装载重量，
/// actualLoaded[j] 是位置 j 的实际装载指示变量（0 或 1）。
///
/// 线性约束形式:
///   z[j1] - z[j2] - mlw[j1] * actualLoaded[j2] <= 0
///   z[j2] - z[j1] - mlw[j2] * actualLoaded[j1] <= 0
pub fn apply_recommended_weight_equalization_limit(
    model: &mut MetaModel<f64>,
    load_vars: &LoadVariables,
    positions: &[Position],
) -> Result<(), Box<dyn Error>> {
    let position_count = positions.len();
    if position_count < 2 {
        return Ok(());
    }

    for j1 in 0..position_count {
        if !positions[j1].status.recommended_weight_needed {
            continue;
        }
        for j2 in (j1 + 1)..position_count {
            if !positions[j2].status.recommended_weight_needed {
                continue;
            }

            let mlw1 = positions[j1].max_load_weight;
            let mlw2 = positions[j2].max_load_weight;

            // z[j1] - z[j2] - mlw1 * actualLoaded[j2] <= 0
            let coefficients1: Vec<(usize, f64)> = vec![
                (load_vars.z[j1], 1.0),
                (load_vars.z[j2], -1.0),
                (load_vars.actual_loaded[j2], -mlw1),
            ];
            model.add_linear_constraint(
                &coefficients1,
                ConstraintRelation::LessEqual,
                0.0,
                &format!("rwe_weight_equalization_{}_{}", j1, j2),
            )?;

            // z[j2] - z[j1] - mlw2 * actualLoaded[j1] <= 0
            let coefficients2: Vec<(usize, f64)> = vec![
                (load_vars.z[j2], 1.0),
                (load_vars.z[j1], -1.0),
                (load_vars.actual_loaded[j1], -mlw2),
            ];
            model.add_linear_constraint(
                &coefficients2,
                ConstraintRelation::LessEqual,
                0.0,
                &format!("rwe_weight_equalization_{}_{}", j2, j1),
            )?;
        }
    }

    Ok(())
}
