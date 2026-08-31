//! 压舱物模型 / Ballast model
use super::load::LoadVariables;
use super::position::Position;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::flatten::LinearMonomial;
use ospf_rust_core::variable::UContinuousVariableItem;
use std::error::Error;
use std::sync::Arc;

/// 压舱物变量索引 / Ballast variable indices
#[derive(Debug, Clone)]
pub struct BallastVariables {
    /// 压舱物重量变量索引 / Ballast weight variable index
    pub ballast_weight: usize,
    /// 可放置压舱物的舱位索引 / Indices of positions where ballast can be placed
    pub ballast_positions: Vec<usize>,
}

/// 压舱物 / Ballast (对齐 Kotlin Ballast)
#[derive(Debug)]
pub struct Ballast {
    /// 可放置压舱物的舱位列表 / Positions where ballast can be placed
    pub ballast_positions: Vec<Position>,
    /// 最小压舱物重量 / Minimum ballast weight
    pub min_ballast_weight: Option<f64>,
    /// 建议压舱物重量 / Advice ballast weight
    pub advice_ballast_weight: Option<f64>,
}

impl Ballast {
    /// 注册压舱物变量到模型
    /// 对齐 Kotlin Ballast.register:
    /// - 创建 ballastWeight 变量
    /// - 设置 min/max 范围
    pub fn register(
        &self,
        _load_vars: &LoadVariables,
        model: &mut MetaModel<f64>,
    ) -> Result<BallastVariables, Box<dyn Error>> {
        // 注册压舱物重量变量
        let var = UContinuousVariableItem::auto("ballast_weight");
        let ballast_weight_idx = model.register_variable(var)?;

        Ok(BallastVariables {
            ballast_weight: ballast_weight_idx,
            ballast_positions: self
                .ballast_positions
                .iter()
                .map(|p| p.max_load_amount as usize)
                .collect(),
        })
    }
}
