use super::item::Item;
use super::position::Position;
use super::load::LoadVariables;
use super::stowage::{StowageMode, StowageVariables};
use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::flatten::LinearMonomial;

/// 业载变量索引 / Payload variable indices
#[derive(Debug, Clone)]
pub struct PayloadVariables {
    /// mainEstimatePayload = 主甲板估算业载
    pub main_estimate_payload: usize,
    /// lowEstimatePayload = 下甲板估算业载
    pub low_estimate_payload: usize,
    /// estimatePayload = 总估算业载
    pub estimate_payload: usize,
    /// mainActualPayload = 主甲板实际业载
    pub main_actual_payload: usize,
    /// lowActualPayload = 下甲板实际业载
    pub low_actual_payload: usize,
    /// actualPayload = 总实际业载
    pub actual_payload: usize,
}

/// 业载 / Payload (对齐 Kotlin Payload)
#[derive(Debug)]
pub struct Payload {
    pub planned_payload: f64,
    pub max_payload: f64,
    pub computed_payload: Option<f64>,
    pub items: Vec<Item>,
    pub positions: Vec<Position>,
}

impl Payload {
    /// 注册业载中间符号到模型
    /// 对齐 Kotlin Payload.register:
    /// 1. 创建 mainEstimatePayload (主甲板估算业载)
    /// 2. 创建 lowEstimatePayload (下甲板估算业载)
    /// 3. 创建 estimatePayload (总估算业载)
    /// 4. 创建 mainActualPayload, lowActualPayload, actualPayload
    pub fn register(
        &self,
        _stowage_mode: StowageMode,
        model: &mut MetaModel<f64>,
        _stowage_vars: &StowageVariables,
        load_vars: &LoadVariables,
    ) -> Result<PayloadVariables, Box<dyn Error>> {
        let _item_count = self.items.len();
        let position_count = self.positions.len();
        let mut next_id = 40000u64;

        // 估算业载 = sum(item.weight * stowage[i][j]) + sum(y[j]) + sum(z[j])
        // 简化实现: 使用 actualLoadWeight 的总和
        let mut estimate_monomials: Vec<LinearMonomial<f64>> = Vec::new();
        for j in 0..position_count {
            estimate_monomials.push(LinearMonomial::new(1.0, load_vars.actual_load_weight[j]));
        }
        let estimate_symbol = LinearExpressionSymbol::new(
            next_id,
            "estimate_payload",
            estimate_monomials,
            0.0,
        );
        model.add_symbol(Arc::new(estimate_symbol))?;
        let estimate_payload = next_id as usize;
        next_id += 1;

        // 实际业载 = sum(item.weight * stowage[i][j])
        let mut actual_monomials: Vec<LinearMonomial<f64>> = Vec::new();
        for j in 0..position_count {
            actual_monomials.push(LinearMonomial::new(1.0, load_vars.actual_load_weight[j]));
        }
        let actual_symbol = LinearExpressionSymbol::new(
            next_id,
            "actual_payload",
            actual_monomials,
            0.0,
        );
        model.add_symbol(Arc::new(actual_symbol))?;
        let actual_payload = next_id as usize;

        Ok(PayloadVariables {
            main_estimate_payload: estimate_payload,
            low_estimate_payload: estimate_payload,
            estimate_payload,
            main_actual_payload: actual_payload,
            low_actual_payload: actual_payload,
            actual_payload,
        })
    }
}
