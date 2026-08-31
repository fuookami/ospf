//! 模型注册辅助函数 / Model registration helper functions
//!
//! 提供变量注册和目标构造的共享逻辑，供 Application 层调用。
//! Provides shared logic for variable registration and objective construction, called by the Application layer.

use std::error::Error;
use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::variable::{BinaryVariableItem, UContinuousVariableItem, VariableId};

use crate::framework_demo::demo2::infrastructure::dto::Demo2Request;
use super::pipeline_mode::Demo2PipelineMode;

/// 变量注册结果 / Variable registration result
///
/// 包含注册后得到的变量索引和可选的偏差变量。
/// Contains variable indices after registration and an optional deviation variable.
pub struct RegistrationResult {
    /// 决策变量索引 x[c][p] / Decision variable indices x[c][p]
    pub x_idx: Vec<Vec<usize>>,
    /// 偏差变量索引（仅预分配和重量推荐模式） / Deviation variable index (only for predistribution and weight recommendation modes)
    pub z: Option<usize>,
}

/// 注册决策变量 / Register decision variables
///
/// 根据模式创建二元决策变量和可选的偏差变量。
/// Creates binary decision variables and optional deviation variable based on mode.
pub fn register_variables(
    request: &Demo2Request,
    model: &mut MetaModel<f64>,
    var_prefix: &str,
    mode: Demo2PipelineMode,
) -> Result<RegistrationResult, Box<dyn Error>> {
    let mut x_idx = vec![vec![0usize; request.positions.len()]; request.cargos.len()];
    for c in 0..request.cargos.len() {
        for p in 0..request.positions.len() {
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

    Ok(RegistrationResult { x_idx, z })
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
    let RegistrationResult { x_idx, z } = registration;

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
            let sub_var = BinaryVariableItem::create(
                shared_id,
                &format!("{}_bs_{}_{}", var_prefix, c, p),
            );
            x_idx_master[c][p] = master_model.register_variable(master_var)?;
            x_idx_sub[c][p] = sub_model.register_variable(sub_var)?;
            fixed_variable_ids.push(shared_id);
        }
    }

    Ok((x_idx_master, x_idx_sub, fixed_variable_ids))
}
