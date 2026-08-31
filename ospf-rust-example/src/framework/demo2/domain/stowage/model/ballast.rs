use super::position::Position;
use super::load::LoadVariables;
use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::flatten::LinearMonomial;
use ospf_rust_core::variable::UContinuousVariableItem;

/// 压舱物变量索引 / Ballast variable indices
#[derive(Debug, Clone)]
pub struct BallastVariables {
    /// ballastWeight = 压舱物重量变量
    pub ballast_weight: usize,
    /// ballastPositions = 可以放置压舱物的舱位索引
    pub ballast_positions: Vec<usize>,
}

/// 压舱物 / Ballast (对齐 Kotlin Ballast)
#[derive(Debug)]
pub struct Ballast {
    pub ballast_positions: Vec<Position>,
    pub min_ballast_weight: Option<f64>,
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
            ballast_positions: self.ballast_positions.iter().map(|p| p.max_load_amount as usize).collect(),
        })
    }
}
