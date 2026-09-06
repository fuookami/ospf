//! 模型注册辅助函数 / Model registration helper functions
//!
//! 提供变量注册和目标构造的共享逻辑，供 Application 层调用。
//! Provides shared logic for variable registration and objective construction, called by the Application layer.

use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::variable::{BinaryVariableItem, UContinuousVariableItem, VariableId};
use std::error::Error;

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
    /// 对齐 Kotlin Load.estimateLoaded: Binaryzation(sum(x[c][p]) for all cargos c)
    pub estimate_loaded_idx: Vec<usize>,
    /// loaded[c] 派生变量索引 / loaded derived-variable indices
    /// 对齐 Kotlin Stowage.loaded[i]: sum(stowage[i][j]) for all positions j
    pub loaded_idx: Vec<usize>,
}

/// 注册决策变量 / Register decision variables
///
/// 根据模式创建二元决策变量和可选的偏差变量。
/// 同时注册派生辅助变量：estimateLoadWeight、estimateLoaded、loaded。
/// Creates binary decision variables, an optional deviation variable, and derived auxiliary variables.
pub fn register_variables(
    request: &Demo2Request,
    model: &mut MetaModel<f64>,
    var_prefix: &str,
    mode: Demo2PipelineMode,
) -> Result<RegistrationResult, Box<dyn Error>> {
    let cargo_count = request.cargos.len();
    let pos_count = request.positions.len();

    // 注册 x[c][p] 决策变量
    let mut x_idx = vec![vec![0usize; pos_count]; cargo_count];
    for c in 0..cargo_count {
        for p in 0..pos_count {
            let var_name = format!("{}_{}_{}", var_prefix, c, p);
            x_idx[c][p] = model.register_variable(BinaryVariableItem::auto(&var_name))?;
        }
    }

    let z = match mode {
        Demo2PipelineMode::Predistribution => {
            Some(model.register_variable(UContinuousVariableItem::auto("max_deviation"))?)
        }
        Demo2PipelineMode::WeightRecommendation => {
            Some(model.register_variable(UContinuousVariableItem::auto("wr_max_deviation"))?)
        }
        Demo2PipelineMode::FullLoad => None,
    };

    let derived = register_derived_variables(request, model, &x_idx)?;

    Ok(RegistrationResult {
        x_idx,
        z,
        estimate_load_weight_idx: derived.estimate_load_weight_idx,
        estimate_loaded_idx: derived.estimate_loaded_idx,
        loaded_idx: derived.loaded_idx,
    })
}

/// 派生变量索引 / Derived variable indices
///
/// 包含 estimateLoadWeight、estimateLoaded、loaded 三组辅助变量的索引。
/// Auxiliary variable indices for estimateLoadWeight, estimateLoaded, and loaded.
pub struct DerivedVariableIndices {
    /// 各位置的估算装载重量变量索引 / Estimated load-weight variable index for each position
    pub estimate_load_weight_idx: Vec<usize>,
    /// 各位置的估算装载数量变量索引 / Estimated loaded-count variable index for each position
    pub estimate_loaded_idx: Vec<usize>,
    /// 各货物的装载数量变量索引 / Loaded-count variable index for each cargo
    pub loaded_idx: Vec<usize>,
}

/// 注册派生变量 / Register derived variables
///
/// 给定已有的 x_idx，注册 estimateLoadWeight、estimateLoaded、loaded 辅助变量及其定义等式。
/// Registers auxiliary variables and defining equalities for an existing x_idx, including Benders models.
pub fn register_derived_variables(
    request: &Demo2Request,
    model: &mut MetaModel<f64>,
    x_idx: &[Vec<usize>],
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
        estimate_load_weight_idx[p] = index;
    }

    // estimateLoaded[p] = sum(x[c][p])
    let mut estimate_loaded_idx = vec![0usize; pos_count];
    for p in 0..pos_count {
        let index = model.register_variable(UContinuousVariableItem::auto(&format!(
            "estimate_loaded_{}",
            p
        )))?;
        let mut terms = Vec::with_capacity(cargo_count + 1);
        terms.push((index, 1.0));
        terms.extend((0..cargo_count).map(|c| (x_idx[c][p], -1.0)));
        model.add_linear_constraint(
            &terms,
            ConstraintRelation::Equal,
            0.0,
            &format!("define_estimate_loaded_{}", p),
        )?;
        estimate_loaded_idx[p] = index;
    }

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

    match mode {
        Demo2PipelineMode::FullLoad => {
            // 最大化总装载重量 / Maximize total loaded weight
            let mut objective = vec![0.0; model.num_tokens()];
            for c in 0..request.cargos.len() {
                for p in 0..request.positions.len() {
                    objective[x_idx[c][p]] = request.cargos[c].weight;
                }
            }
            model.set_linear_objective(objective, ObjectiveCategory::Maximum);
        }
        Demo2PipelineMode::Predistribution => {
            // 最小化偏差 / Minimize deviation
            let mut objective = vec![0.0; model.num_tokens()];
            if let Some(z_idx) = z {
                objective[*z_idx] = 1.0;
            }
            model.set_linear_objective(objective, ObjectiveCategory::Minimum);
        }
        Demo2PipelineMode::WeightRecommendation => {
            // 最大化装载优先级重量 - 平衡优先级*偏差 / Maximize payload priority weight - balance priority * deviation
            let mut objective = vec![0.0; model.num_tokens()];
            for c in 0..request.cargos.len() {
                for p in 0..request.positions.len() {
                    objective[x_idx[c][p]] += request.cargos[c].weight
                        * request.weight_recommendation_objective.payload_priority;
                }
            }
            if let Some(z_idx) = z {
                objective[*z_idx] -= request.weight_recommendation_objective.balance_priority;
            }
            model.set_linear_objective(objective, ObjectiveCategory::Maximum);
        }
    }

    Ok(())
}

/// 注册 Benders 分解的变量 / Register variables for Benders decomposition
///
/// 为 master 和 sub 模型创建共享 ID 的变量。
/// Creates variables with shared IDs for master and sub models.
pub fn register_benders_variables(
    request: &Demo2Request,
    master_model: &mut MetaModel<f64>,
    sub_model: &mut MetaModel<f64>,
    var_prefix: &str,
) -> Result<(Vec<Vec<usize>>, Vec<Vec<usize>>, Vec<VariableId>), Box<dyn Error>> {
    let mut x_idx_master = vec![vec![0usize; request.positions.len()]; request.cargos.len()];
    let mut x_idx_sub = vec![vec![0usize; request.positions.len()]; request.cargos.len()];
    let mut fixed_variable_ids = Vec::with_capacity(request.cargos.len() * request.positions.len());

    for c in 0..request.cargos.len() {
        for p in 0..request.positions.len() {
            let master_var = BinaryVariableItem::auto(&format!("{}_bm_{}_{}", var_prefix, c, p));
            let shared_id = master_var.id();
            let sub_var =
                BinaryVariableItem::create(shared_id, &format!("{}_bs_{}_{}", var_prefix, c, p));
            x_idx_master[c][p] = master_model.register_variable(master_var)?;
            x_idx_sub[c][p] = sub_model.register_variable(sub_var)?;
            fixed_variable_ids.push(shared_id);
        }
    }

    Ok((x_idx_master, x_idx_sub, fixed_variable_ids))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_variable_indices_are_valid_solver_columns() {
        let request = Demo2Request::sample();
        let mut model = MetaModel::<f64>::new("demo2_derived_variable_indices");
        let registration =
            register_variables(&request, &mut model, "x", Demo2PipelineMode::FullLoad)
                .expect("derived variables should register");
        let variable_count = model.num_tokens();

        assert!(
            registration
                .estimate_load_weight_idx
                .iter()
                .chain(registration.estimate_loaded_idx.iter())
                .chain(registration.loaded_idx.iter())
                .all(|index| *index < variable_count)
        );

        let triad = model
            .try_to_linear_triad_model()
            .expect("derived variable definitions should convert");
        assert!(triad.A.rows.iter().all(|row| {
            row.entries
                .iter()
                .all(|(index, _)| *index < triad.num_variables())
        }));
    }
}
