use super::item::Item;
use super::position::Position;
use super::stowage::{Stowage, StowageVariables};
use super::super::shared::units::{quantity_value_in_unit, weight_unit};
use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::{
    BinaryzationFunction, IfFunction, SameAsFunction, SlackFunction,
};
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
    /// full[j] = 舱位 j 是否装满 (BinaryzationFunction result variable solver index)
    pub full: Vec<usize>,
    /// estimateLoadWeight[j] = 舱位 j 的估算装载重量
    pub estimate_load_weight: Vec<usize>,
    /// actualLoadWeight[j] = 舱位 j 的实际装载重量
    pub actual_load_weight: Vec<usize>,
    /// predicateLoadWeightSlack[j] = |estimateLoadWeight - actualLoadWeight| (SlackFunction result variable solver index)
    pub predicate_load_weight_slack: Vec<usize>,
    /// y_same_as[j] = SameAsFunction(y, actualLoadWeight) result variable solver index
    pub y_same_as: Vec<usize>,
    /// z_same_as[j] = SameAsFunction(z, estimateLoadWeight) result variable solver index
    pub z_same_as: Vec<usize>,
    /// y_if[j] = IfFunction(condition, then_expr, else_expr) result variable solver index
    pub y_if: Vec<usize>,
}

/// 装载量 / Load (对齐 Kotlin Load)
#[derive(Debug)]
pub struct Load {
    pub items: Vec<Item>,
    pub positions: Vec<Position>,
}

impl Load {
    /// 注册装载量变量和中间符号到模型
    ///
    /// 对齐 Kotlin Load.register，使用 FunctionSymbol 替代手写线性化：
    /// 1. 创建 y[j] 预测装载重量变量
    /// 2. 创建 z[j] 推荐装载重量变量
    /// 3. 创建 loadAmount[j] 装载数量中间符号 (LinearExpressionSymbol)
    /// 4. 创建 full[j] 是否装满中间符号 (BinaryzationFunction, 替代原"连续近似")
    /// 5. 创建 estimateLoadWeight[j] 估算装载重量中间符号 (LinearExpressionSymbol)
    /// 6. 创建 actualLoadWeight[j] 实际装载重量中间符号 (LinearExpressionSymbol)
    /// 7. 创建 predicateLoadWeightSlack[j] 松弛函数 (SlackFunction)
    /// 8. 创建 y_same_as[j] / z_same_as[j] 相同性检查 (SameAsFunction)
    /// 9. 创建 y_if[j] 条件中间符号 (IfFunction)
    ///
    /// Kotlin-Rust 映射 / Kotlin-Rust Mapping:
    /// - `predicateLoadWeightSlack[j]` -> `SlackFunction(|estimateLoadWeight - actualLoadWeight|)`
    /// - `full[j]`                     -> `BinaryzationFunction(loadAmount >= mla)`
    /// - `y_same_as[j]`                -> `SameAsFunction(y, actualLoadWeight)`
    /// - `z_same_as[j]`                -> `SameAsFunction(z, estimateLoadWeight)`
    /// - `y_if[j]`                     -> `IfFunction(loadAmount >= 1, y_same_as, y)`
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

        // 3. 创建 loadAmount[j] 中间符号 (LinearExpressionSymbol, 保留)
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

        // 4. 创建 full[j] 中间符号 (BinaryzationFunction, 替代原"连续近似")
        // Kotlin: val full = BinaryzationFunction(loadAmount[j], threshold = mla)
        // full[j] = 1 if loadAmount[j] >= mla, else 0
        let mut full_idx = vec![0usize; position_count];
        for (j, position) in self.positions.iter().enumerate() {
            let mla = position.max_load_amount as f64;
            let load_amount_linear = Linear::new(
                vec![LinearMonomial::new(1.0, load_amount_idx[j])],
                0.0,
            );
            let bin_fn = BinaryzationFunction::with_threshold(
                next_id,
                &format!("full_{}", position.id),
                load_amount_linear,
                mla,
            );
            let result_idx = bin_fn.result_variable().index();
            model.add_symbol(Arc::new(bin_fn))?;
            full_idx[j] = result_idx;
            next_id += 1;
        }

        // 5. 创建 estimateLoadWeight[j] 中间符号 (LinearExpressionSymbol, 保留)
        // estimateLoadWeight[j] = sum(item.weight.value * stowage[i][j]) + y[j] + z[j]
        let wu = weight_unit();
        let mut estimate_load_weight_idx = vec![0usize; position_count];
        for (j, position) in self.positions.iter().enumerate() {
            let mut monomials: Vec<LinearMonomial<f64>> = Vec::new();
            for (i, item) in self.items.iter().enumerate() {
                let w = quantity_value_in_unit(&item.weight, &wu)?;
                monomials.push(LinearMonomial::new(w, stowage_vars.stowage[i][j]));
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

        // 6. 创建 actualLoadWeight[j] 中间符号 (LinearExpressionSymbol, 保留)
        // actualLoadWeight[j] = sum(item.weight.value * stowage[i][j])
        let mut actual_load_weight_idx = vec![0usize; position_count];
        for (j, position) in self.positions.iter().enumerate() {
            let mut monomials: Vec<LinearMonomial<f64>> = Vec::with_capacity(item_count);
            for (i, item) in self.items.iter().enumerate() {
                let w = quantity_value_in_unit(&item.weight, &wu)?;
                monomials.push(LinearMonomial::new(w, stowage_vars.stowage[i][j]));
            }
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

        // 7. 创建 predicateLoadWeightSlack[j] 松弛函数 (SlackFunction)
        // Kotlin: val predicateLoadWeightSlack = SlackFunction(estimateLoadWeight, actualLoadWeight)
        // slack = |estimateLoadWeight - actualLoadWeight|
        let mut slack_idx = vec![0usize; position_count];
        for (j, position) in self.positions.iter().enumerate() {
            let left = Linear::new(
                vec![LinearMonomial::new(1.0, estimate_load_weight_idx[j])],
                0.0,
            );
            let right = Linear::new(
                vec![LinearMonomial::new(1.0, actual_load_weight_idx[j])],
                0.0,
            );
            let slack_fn = SlackFunction::new(
                next_id,
                &format!("predicate_load_weight_slack_{}", position.id),
                left,
                right,
            );
            let result_idx = slack_fn.result_variable().index();
            model.add_symbol(Arc::new(slack_fn))?;
            slack_idx[j] = result_idx;
            next_id += 1;
        }

        // 8. 创建 y_same_as[j] = SameAsFunction(y, actualLoadWeight) 中间符号
        // Kotlin: val y_same_as = SameAsFunction(y[j], actualLoadWeight[j])
        let mut y_same_as_idx = vec![0usize; position_count];
        for (j, position) in self.positions.iter().enumerate() {
            let first = Linear::new(vec![LinearMonomial::new(1.0, y_idx[j])], 0.0);
            let second = Linear::new(
                vec![LinearMonomial::new(1.0, actual_load_weight_idx[j])],
                0.0,
            );
            let same_as_fn =
                SameAsFunction::new(next_id, &format!("y_same_as_{}", position.id), first, second, 0.0);
            let result_idx = same_as_fn.result_variable().index();
            model.add_symbol(Arc::new(same_as_fn))?;
            y_same_as_idx[j] = result_idx;
            next_id += 1;
        }

        // 9. 创建 z_same_as[j] = SameAsFunction(z, estimateLoadWeight) 中间符号
        // Kotlin: val z_same_as = SameAsFunction(z[j], estimateLoadWeight[j])
        let mut z_same_as_idx = vec![0usize; position_count];
        for (j, position) in self.positions.iter().enumerate() {
            let first = Linear::new(vec![LinearMonomial::new(1.0, z_idx[j])], 0.0);
            let second = Linear::new(
                vec![LinearMonomial::new(1.0, estimate_load_weight_idx[j])],
                0.0,
            );
            let same_as_fn =
                SameAsFunction::new(next_id, &format!("z_same_as_{}", position.id), first, second, 0.0);
            let result_idx = same_as_fn.result_variable().index();
            model.add_symbol(Arc::new(same_as_fn))?;
            z_same_as_idx[j] = result_idx;
            next_id += 1;
        }

        // 10. 创建 y_if[j] = IfFunction(loadAmount >= 1, then_=y_same_as, else_=y) 中间符号
        // Kotlin: val y_if = IfFunction(condition = loadAmount[i] geq 1, then_ = y_same_as, else_ = y[i])
        let mut y_if_idx = vec![0usize; position_count];
        for (j, position) in self.positions.iter().enumerate() {
            // condition: loadAmount[j] - 1 (nonzero when loadAmount >= 1)
            let condition = Linear::new(
                vec![LinearMonomial::new(1.0, load_amount_idx[j])],
                -1.0,
            );
            // then_expr: y_same_as result (uses result variable index)
            let then_expr = Linear::new(
                vec![LinearMonomial::new(1.0, y_same_as_idx[j])],
                0.0,
            );
            // else_expr: y[j]
            let else_expr = Linear::new(vec![LinearMonomial::new(1.0, y_idx[j])], 0.0);
            let if_fn = IfFunction::new(
                next_id,
                &format!("y_if_{}", position.id),
                condition,
                then_expr,
                else_expr,
            );
            let result_idx = if_fn.result_variable().index();
            model.add_symbol(Arc::new(if_fn))?;
            y_if_idx[j] = result_idx;
            next_id += 1;
        }

        Ok(LoadVariables {
            y: y_idx,
            z: z_idx,
            load_amount: load_amount_idx,
            full: full_idx,
            estimate_load_weight: estimate_load_weight_idx,
            actual_load_weight: actual_load_weight_idx,
            predicate_load_weight_slack: slack_idx,
            y_same_as: y_same_as_idx,
            z_same_as: z_same_as_idx,
            y_if: y_if_idx,
        })
    }
}
