//! 模型注册辅助函数 / Model registration helper functions
//!
//! 提供变量注册和目标构造的共享逻辑，供 Application 层调用。
//! Provides shared logic for variable registration and objective construction, called by the Application layer.

use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::flatten::LinearMonomial;
use ospf_rust_core::variable::{BinaryVariableItem, UContinuousVariableItem, VariableId};

use crate::framework::demo2::infrastructure::dto::Demo2Request;
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
    /// estimateLoadWeight[p] 中间符号索引 / estimateLoadWeight intermediate symbol indices
    /// 对齐 Kotlin Load.estimateLoadWeight: sum(cargo_weight * x[c][p]) for all cargos c
    pub estimate_load_weight_idx: Vec<usize>,
    /// estimateLoaded[p] 中间符号索引 / estimateLoaded intermediate symbol indices
    /// 对齐 Kotlin Load.estimateLoaded: Binaryzation(sum(x[c][p]) for all cargos c)
    pub estimate_loaded_idx: Vec<usize>,
    /// loaded[c] 中间符号索引 / loaded intermediate symbol indices
    /// 对齐 Kotlin Stowage.loaded[i]: sum(stowage[i][j]) for all positions j
    pub loaded_idx: Vec<usize>,
}

/// 注册决策变量 / Register decision variables
///
/// 根据模式创建二元决策变量和可选的偏差变量。
/// 同时注册中间符号：estimateLoadWeight、estimateLoaded、loaded。
/// Creates binary decision variables, optional deviation variable, and intermediate symbols.
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

    // 注册 estimateLoadWeight[p] 中间符号
    // 对齐 Kotlin Load.estimateLoadWeight: sum(cargo_weight * x[c][p])
    let mut next_id = 10000u64;
    let mut estimate_load_weight_idx = vec![0usize; pos_count];
    for p in 0..pos_count {
        let monomials: Vec<LinearMonomial<f64>> = (0..cargo_count)
            .map(|c| LinearMonomial::new(request.cargos[c].weight, x_idx[c][p]))
            .collect();
        let symbol = LinearExpressionSymbol::new(
            next_id,
            &format!("estimate_load_weight_{}", p),
            monomials,
            0.0,
        );
        model.add_symbol(Arc::new(symbol))?;
        estimate_load_weight_idx[p] = next_id as usize;
        next_id += 1;
    }

    // 注册 estimateLoaded[p] 中间符号
    // 对齐 Kotlin Load.estimateLoaded: Binaryzation(sum(x[c][p]))
    // 使用 LinearExpressionSymbol 表示 sum(x[c][p])，语义为"该位置是否有装载"
    let mut estimate_loaded_idx = vec![0usize; pos_count];
    for p in 0..pos_count {
        let monomials: Vec<LinearMonomial<f64>> = (0..cargo_count)
            .map(|c| LinearMonomial::new(1.0, x_idx[c][p]))
            .collect();
        let symbol = LinearExpressionSymbol::new(
            next_id,
            &format!("estimate_loaded_{}", p),
            monomials,
            0.0,
        );
        model.add_symbol(Arc::new(symbol))?;
        estimate_loaded_idx[p] = next_id as usize;
        next_id += 1;
    }

    // 注册 loaded[c] 中间符号
    // 对齐 Kotlin Stowage.loaded[i]: sum(x[c][p] for all positions p)
    let mut loaded_idx = vec![0usize; cargo_count];
    for c in 0..cargo_count {
        let monomials: Vec<LinearMonomial<f64>> = (0..pos_count)
            .map(|p| LinearMonomial::new(1.0, x_idx[c][p]))
            .collect();
        let symbol = LinearExpressionSymbol::new(
            next_id,
            &format!("loaded_{}", c),
            monomials,
            0.0,
        );
        model.add_symbol(Arc::new(symbol))?;
        loaded_idx[c] = next_id as usize;
        next_id += 1;
    }

    Ok(RegistrationResult {
        x_idx,
        z,
        estimate_load_weight_idx,
        estimate_loaded_idx,
        loaded_idx,
    })
}

/// 中间符号索引 / Intermediate symbol indices
///
/// 包含 estimateLoadWeight、estimateLoaded、loaded 三组中间符号的索引。
/// Intermediate symbol indices for estimateLoadWeight, estimateLoaded, loaded.
pub struct IntermediateSymbolIndices {
    pub estimate_load_weight_idx: Vec<usize>,
    pub estimate_loaded_idx: Vec<usize>,
    pub loaded_idx: Vec<usize>,
}

/// 注册中间符号 / Register intermediate symbols
///
/// 给定已有的 x_idx，注册 estimateLoadWeight、estimateLoaded、loaded 中间符号。
/// 可用于 Benders 分解等需要独立注册中间符号的场景。
pub fn register_intermediate_symbols(
    request: &Demo2Request,
    model: &mut MetaModel<f64>,
    x_idx: &[Vec<usize>],
    start_id: u64,
) -> Result<IntermediateSymbolIndices, Box<dyn Error>> {
    let cargo_count = request.cargos.len();
    let pos_count = request.positions.len();
    let mut next_id = start_id;

    // estimateLoadWeight[p] = sum(cargo_weight * x[c][p])
    let mut estimate_load_weight_idx = vec![0usize; pos_count];
    for p in 0..pos_count {
        let monomials: Vec<LinearMonomial<f64>> = (0..cargo_count)
            .map(|c| LinearMonomial::new(request.cargos[c].weight, x_idx[c][p]))
            .collect();
        let symbol = LinearExpressionSymbol::new(
            next_id,
            &format!("estimate_load_weight_{}", p),
            monomials,
            0.0,
        );
        model.add_symbol(Arc::new(symbol))?;
        estimate_load_weight_idx[p] = next_id as usize;
        next_id += 1;
    }

    // estimateLoaded[p] = sum(x[c][p])
    let mut estimate_loaded_idx = vec![0usize; pos_count];
    for p in 0..pos_count {
        let monomials: Vec<LinearMonomial<f64>> = (0..cargo_count)
            .map(|c| LinearMonomial::new(1.0, x_idx[c][p]))
            .collect();
        let symbol = LinearExpressionSymbol::new(
            next_id,
            &format!("estimate_loaded_{}", p),
            monomials,
            0.0,
        );
        model.add_symbol(Arc::new(symbol))?;
        estimate_loaded_idx[p] = next_id as usize;
        next_id += 1;
    }

    // loaded[c] = sum(x[c][p])
    let mut loaded_idx = vec![0usize; cargo_count];
    for c in 0..cargo_count {
        let monomials: Vec<LinearMonomial<f64>> = (0..pos_count)
            .map(|p| LinearMonomial::new(1.0, x_idx[c][p]))
            .collect();
        let symbol = LinearExpressionSymbol::new(
            next_id,
            &format!("loaded_{}", c),
            monomials,
            0.0,
        );
        model.add_symbol(Arc::new(symbol))?;
        loaded_idx[c] = next_id as usize;
        next_id += 1;
    }

    Ok(IntermediateSymbolIndices {
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
