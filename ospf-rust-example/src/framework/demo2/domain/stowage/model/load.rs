use super::item::Item;
use super::position::Position;
use super::stowage::{Stowage, StowageVariables};
use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::flatten::LinearMonomial;
use ospf_rust_core::variable::UContinuousVariableItem;

/// 装载量变量索引 / Load variable indices
#[derive(Debug, Clone)]
pub struct LoadVariables {
    /// y[j] = 舱位 j 的预测装载重量
    pub y: Vec<usize>,
    /// z[j] = 舱位 j 的推荐装载重量
    pub z: Vec<usize>,
    /// loadAmount[j] = 舱位 j 的装载数量
    pub load_amount: Vec<usize>,
    /// full[j] = 舱位 j 是否装满
    pub full: Vec<usize>,
    /// estimateLoadWeight[j] = 舱位 j 的估算装载重量
    pub estimate_load_weight: Vec<usize>,
    /// actualLoadWeight[j] = 舱位 j 的实际装载重量
    pub actual_load_weight: Vec<usize>,
}

/// 装载量 / Load (对齐 Kotlin Load)
#[derive(Debug)]
pub struct Load {
    pub items: Vec<Item>,
    pub positions: Vec<Position>,
}

impl Load {
    /// 注册装载量变量和中间符号到模型
    /// 对齐 Kotlin Load.register:
    /// 1. 创建 y[j] 预测装载重量变量
    /// 2. 创建 z[j] 推荐装载重量变量
    /// 3. 创建 loadAmount[j] 装载数量中间符号
    /// 4. 创建 full[j] 是否装满中间符号
    /// 5. 创建 estimateLoadWeight[j] 估算装载重量中间符号
    /// 6. 创建 actualLoadWeight[j] 实际装载重量中间符号
    pub fn register(
        &self,
        model: &mut MetaModel<f64>,
        stowage_vars: &StowageVariables,
    ) -> Result<LoadVariables, Box<dyn Error>> {
        let item_count = self.items.len();
        let position_count = self.positions.len();
        let mut next_id = 30000u64;

        // 1. 注册 y[j] 预测装载重量变量
        let mut y_idx = vec![0usize; position_count];
        for (j, position) in self.positions.iter().enumerate() {
            let var = UContinuousVariableItem::auto(&format!("y_{}", position.id));
            y_idx[j] = model.register_variable(var)?;
        }

        // 2. 注册 z[j] 推荐装载重量变量
        let mut z_idx = vec![0usize; position_count];
        for (j, position) in self.positions.iter().enumerate() {
            let var = UContinuousVariableItem::auto(&format!("z_{}", position.id));
            z_idx[j] = model.register_variable(var)?;
        }

        // 3. 创建 loadAmount[j] 中间符号
        // loadAmount[j] = sum(stowage[i][j] for all items i)
        let mut load_amount_idx = vec![0usize; position_count];
        for (j, position) in self.positions.iter().enumerate() {
            let monomials: Vec<LinearMonomial<f64>> = (0..item_count)
                .map(|i| LinearMonomial::new(1.0, stowage_vars.stowage[i][j]))
                .collect();
            let symbol = LinearExpressionSymbol::new(
                next_id,
                &format!("load_amount_{}", position.id),
                monomials,
                0.0,
            );
            model.add_symbol(Arc::new(symbol))?;
            load_amount_idx[j] = next_id as usize;
            next_id += 1;
        }

        // 4. 创建 full[j] 中间符号
        // full[j] = 1 if loadAmount[j] >= mla, else 0
        // 简化实现: full[j] = loadAmount[j] / mla (连续近似)
        let mut full_idx = vec![0usize; position_count];
        for (j, position) in self.positions.iter().enumerate() {
            let mla = position.max_load_amount as f64;
            let coefficient = if mla > 0.0 { 1.0 / mla } else { 0.0 };
            let monomials = vec![LinearMonomial::new(coefficient, load_amount_idx[j])];
            let symbol = LinearExpressionSymbol::new(
                next_id,
                &format!("full_{}", position.id),
                monomials,
                0.0,
            );
            model.add_symbol(Arc::new(symbol))?;
            full_idx[j] = next_id as usize;
            next_id += 1;
        }

        // 5. 创建 estimateLoadWeight[j] 中间符号
        // estimateLoadWeight[j] = sum(item.weight * stowage[i][j]) + y[j] + z[j]
        let mut estimate_load_weight_idx = vec![0usize; position_count];
        for (j, position) in self.positions.iter().enumerate() {
            let mut monomials: Vec<LinearMonomial<f64>> = Vec::new();
            for (i, item) in self.items.iter().enumerate() {
                monomials.push(LinearMonomial::new(item.weight, stowage_vars.stowage[i][j]));
            }
            monomials.push(LinearMonomial::new(1.0, y_idx[j]));
            monomials.push(LinearMonomial::new(1.0, z_idx[j]));
            let symbol = LinearExpressionSymbol::new(
                next_id,
                &format!("estimate_load_weight_{}", position.id),
                monomials,
                0.0,
            );
            model.add_symbol(Arc::new(symbol))?;
            estimate_load_weight_idx[j] = next_id as usize;
            next_id += 1;
        }

        // 6. 创建 actualLoadWeight[j] 中间符号
        // actualLoadWeight[j] = sum(item.weight * stowage[i][j])
        let mut actual_load_weight_idx = vec![0usize; position_count];
        for (j, position) in self.positions.iter().enumerate() {
            let monomials: Vec<LinearMonomial<f64>> = (0..item_count)
                .map(|i| LinearMonomial::new(self.items[i].weight, stowage_vars.stowage[i][j]))
                .collect();
            let symbol = LinearExpressionSymbol::new(
                next_id,
                &format!("actual_load_weight_{}", position.id),
                monomials,
                0.0,
            );
            model.add_symbol(Arc::new(symbol))?;
            actual_load_weight_idx[j] = next_id as usize;
            next_id += 1;
        }

        Ok(LoadVariables {
            y: y_idx,
            z: z_idx,
            load_amount: load_amount_idx,
            full: full_idx,
            estimate_load_weight: estimate_load_weight_idx,
            actual_load_weight: actual_load_weight_idx,
        })
    }
}
