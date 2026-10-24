//! 模型注册辅助函数 / Model registration helper functions
//!
//! 为共享装载决策注册航空派生量及目标。 / Register aviation derived values and objectives over shared loading decisions.

use std::error::Error;
use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::variable::{UContinuousVariableItem, VariableRange};
use super::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::infrastructure::dto::Demo2Request;

/// 变量注册结果 / Variable registration result
///
/// 包含注册后得到的变量索引和可选的偏差变量。
/// Contains variable indices after registration and an optional deviation variable.
pub struct RegistrationResult {
    /// 决策变量索引 x[c][p] / Decision variable indices x[c][p]
    pub x_idx: Vec<Vec<usize>>,
    /// 偏差变量索引（仅预分配和重量推荐模式） / Deviation variable index (only for predistribution and weight recommendation modes)
    pub z: Option<usize>,
    /// estimateLoadWeight[p] 派生变量索引 / estimateLoadWeight derived-variable indices
    /// 对齐 Kotlin Load.estimateLoadWeight: sum(cargo_weight * x[c][p]) for all cargos c
    pub estimate_load_weight_idx: Vec<usize>,
    /// estimateLoaded[p] 派生变量索引 / estimateLoaded derived-variable indices
    /// 对齐 Kotlin Load.estimateLoaded: shared loading occupancy
    pub estimate_loaded_idx: Vec<usize>,
    /// loaded[c] 派生变量索引 / loaded derived-variable indices
    /// 对齐 Kotlin Stowage.loaded[i]: sum(stowage[i][j]) for all positions j
    pub loaded_idx: Vec<usize>,
}

/// 派生变量索引 / Derived variable indices
///
/// 包含 estimateLoadWeight、estimateLoaded、loaded 三组辅助变量的索引。
/// Auxiliary variable indices for estimateLoadWeight, estimateLoaded, and loaded.
pub struct DerivedVariableIndices {
    /// 各位置的估算装载重量变量索引 / Estimated load-weight variable index for each position
    pub estimate_load_weight_idx: Vec<usize>,
    /// 各位置的共享整体占位变量索引 / Shared whole-position occupancy index for each position
    pub estimate_loaded_idx: Vec<usize>,
    /// 各货物的装载数量变量索引 / Loaded-count variable index for each cargo
    pub loaded_idx: Vec<usize>,
}

/// 注册派生变量 / Register derived variables
///
/// 给定已有的 x_idx，注册 estimateLoadWeight、estimateLoaded、loaded 辅助变量及其定义等式。
/// Register derived equalities over the shared sparse assignment projection for every solve strategy.
pub fn register_derived_variables(
    request: &Demo2Request,
    model: &mut MetaModel<f64>,
    x_idx: &[Vec<usize>],
    occupied_idx: &[usize],
) -> Result<DerivedVariableIndices, Box<dyn Error>> {
    let cargo_count = request.cargos.len();
    let pos_count = request.positions.len();

    // estimateLoadWeight[p] = sum(cargo_weight * x[c][p])
    let mut estimate_load_weight_idx = vec![0usize; pos_count];
    for p in 0..pos_count {
        let index = model.register_variable(UContinuousVariableItem::auto(&format!(
            "estimate_load_weight_{}",
            p
        )))?;
        let mut terms = Vec::with_capacity(cargo_count + 1);
        terms.push((index, 1.0));
        terms.extend((0..cargo_count).map(|c| (x_idx[c][p], -request.cargos[c].weight)));
        model.add_linear_constraint(
            &terms,
            ConstraintRelation::Equal,
            0.0,
            &format!("define_estimate_load_weight_{}", p),
        )?;
        model.set_variable_range_by_index(index, VariableRange::bounded(0.0, request.positions[p].max_weight))?;
        estimate_load_weight_idx[p] = index;
    }

    let estimate_loaded_idx = occupied_idx.to_vec();

    // loaded[c] = sum(x[c][p])
    let mut loaded_idx = vec![0usize; cargo_count];
    for c in 0..cargo_count {
        let index =
            model.register_variable(UContinuousVariableItem::auto(&format!("loaded_{}", c)))?;
        let mut terms = Vec::with_capacity(pos_count + 1);
        terms.push((index, 1.0));
        terms.extend((0..pos_count).map(|p| (x_idx[c][p], -1.0)));
        model.add_linear_constraint(
            &terms,
            ConstraintRelation::Equal,
            0.0,
            &format!("define_loaded_{}", c),
        )?;
        model.set_variable_range_by_index(index, VariableRange::bounded(0.0, 1.0))?;
        loaded_idx[c] = index;
    }

    Ok(DerivedVariableIndices {
        estimate_load_weight_idx,
        estimate_loaded_idx,
        loaded_idx,
    })
}

/// 构造目标函数 / Construct objective function
///
/// 根据模式设置不同的优化目标。
/// Sets different optimization objectives based on mode.
pub fn construct_objective(
    request: &Demo2Request,
    model: &mut MetaModel<f64>,
    registration: &RegistrationResult,
    mode: Demo2PipelineMode,
) -> Result<(), Box<dyn Error>> {
    let RegistrationResult { x_idx, z, .. } = registration;

    let mut objective = vec![0.0; model.num_tokens()];
    match mode {
        Demo2PipelineMode::FullLoad => {
            for c in 0..request.cargos.len() {
                for p in 0..request.positions.len() {
                    objective[x_idx[c][p]] += request.cargos[c].weight;
                }
            }
            model.set_linear_objective(objective, ObjectiveCategory::Maximum);
        }
        Demo2PipelineMode::Predistribution | Demo2PipelineMode::WeightRecommendation => {
            if let Some(column) = z {
                objective[*column] = if mode == Demo2PipelineMode::WeightRecommendation {
                    request.weight_recommendation_objective.balance_priority
                } else { 1.0 };
            }
            model.set_linear_objective(objective, ObjectiveCategory::Minimum);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_variable_indices_are_valid_solver_columns() {
        let registration = super::super::loading_context::Demo2LoadingContext::register(&Demo2Request::sample(), Demo2PipelineMode::FullLoad).unwrap();
        let model = registration.model();
        let triad = model.linear_model_snapshot().unwrap().compile_flt64().unwrap();
        assert!(triad.model.A.rows.iter().all(|row| row.entries.iter().all(|(index, _)| *index < triad.model.num_variables())));
    }
}
