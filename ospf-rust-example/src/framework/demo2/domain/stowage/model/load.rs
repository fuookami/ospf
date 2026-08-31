//! 装载量模型 / Load model
use super::super::super::shared::units::{quantity_value_in_unit, weight_unit};
use super::item::Item;
use super::position::Position;
use super::stowage::{Stowage, StowageVariables};
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::{
    BinaryzationFunction, IfFunction, OrFunction, SameAsFunction, SlackFunction,
};
use ospf_rust_core::variable::{UContinuousVariableItem, UIntegerVariableItem, VariableRange};
use std::error::Error;
use std::sync::Arc;

/// 装载量变量索引 / Load variable indices
#[derive(Debug, Clone)]
pub struct LoadVariables {
    /// 舱位预测装载重量索引 / Predicted load weight index per position
    pub y: Vec<usize>,
    /// 舱位推荐装载重量索引 / Recommended load weight index per position
    pub z: Vec<usize>,
    /// 舱位装载数量索引 / Load amount index per position
    pub load_amount: Vec<usize>,
    /// 舱位是否装满索引 / Whether position is full (BinaryzationFunction result index)
    pub full: Vec<usize>,
    /// 舱位估算装载重量索引 / Estimated load weight index per position
    pub estimate_load_weight: Vec<usize>,
    /// 舱位实际装载重量索引 / Actual load weight index per position
    pub actual_load_weight: Vec<usize>,
    /// 预测装载重量松弛索引 / Predicate load weight slack index (SlackFunction result)
    pub predicate_load_weight_slack: Vec<usize>,
    /// y 与 actualLoadWeight 相同性检查索引 / SameAsFunction(y, actualLoadWeight) result index
    pub y_same_as: Vec<usize>,
    /// z 与 estimateLoadWeight 相同性检查索引 / SameAsFunction(z, estimateLoadWeight) result index
    pub z_same_as: Vec<usize>,
    /// y 条件中间符号索引 / IfFunction condition result index for y
    pub y_if: Vec<usize>,
    /// 估算是否已装载索引 / Estimated loaded index (OrFunction result; 1 if position is estimated loaded)
    pub estimate_loaded: Vec<usize>,
    /// 实际是否已装载索引 / Actual loaded index (1 if position is actually loaded)
    pub actual_loaded: Vec<usize>,
}

/// 装载量 / Load (对齐 Kotlin Load)
#[derive(Debug)]
pub struct Load {
    /// 物品列表 / Item list
    pub items: Vec<Item>,
    /// 舱位列表 / Position list
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
    /// 10. 创建 z_if[j] 条件中间符号 (IfFunction)
    /// 11. 创建 estimateLoaded[j] = OrFunction(loadedItem, y_if, z_if)
    /// 12. 创建 actualLoaded[j] = BinaryzationFunction(loadAmount >= 1)
    ///
    /// Kotlin-Rust 映射 / Kotlin-Rust Mapping:
    /// - `predicateLoadWeightSlack[j]` -> `SlackFunction(y, loadAmount * plw_min)` 或 `SlackFunction(y, plw_min)`
    /// - `full[j]`                     -> `BinaryzationFunction(loadAmount >= mla)`
    /// - `loadedItem[j]`               -> `BinaryzationFunction(loadAmount >= 1)`
    /// - `y_same_as[j]`                -> `SameAsFunction(y, actualLoadWeight)`
    /// - `z_same_as[j]`                -> `SameAsFunction(z, estimateLoadWeight)`
    /// - `y_if[j]`                     -> `IfFunction(loadAmount >= 1, y_same_as, y)`
    /// - `z_if[j]`                     -> `IfFunction(loadAmount >= 1, z_same_as, z)`
    /// - `estimateLoaded[j]`           -> `OrFunction(full[j], y_if[j])`
    /// - `actualLoaded[j]`             -> `OrFunction(full[j], z_if[j])`
    pub fn register(
        &self,
        model: &mut MetaModel<f64>,
        stowage_vars: &StowageVariables,
    ) -> Result<LoadVariables, Box<dyn Error>> {
        let item_count = self.items.len();
        let position_count = self.positions.len();
        let mut next_id = 30000u64;

        // 1. 注册 y[j] 预测装载重量变量（仅当 predicateWeightNeeded）
        // Kotlin: if (predicateWeightNeeded) y[j] = Variable(...)
        // 类型: UContinuous，上界: position.max_load_weight
        let mut y_idx = vec![0usize; position_count];
        for (j, position) in self.positions.iter().enumerate() {
            if position.status.predicate_weight_needed {
                let var = UContinuousVariableItem::auto_with_range(
                    &format!("y_{}", position.id),
                    VariableRange::bounded(0.0, position.max_load_weight),
                );
                y_idx[j] = model.register_variable(var)?;
            }
        }

        // 2. 注册 z[j] 推荐装载重量变量（仅当 recommendedWeightNeeded）
        // Kotlin: if (recommendedWeightNeeded) z[j] = QuantityUIntVariable1(...)
        // 类型: UInteger（非负整数），上界: position.max_load_weight
        let mut z_idx = vec![0usize; position_count];
        for (j, position) in self.positions.iter().enumerate() {
            if position.status.recommended_weight_needed {
                let var = UIntegerVariableItem::auto_with_range(
                    &format!("z_{}", position.id),
                    VariableRange::bounded(0.0, position.max_load_weight),
                );
                z_idx[j] = model.register_variable(var)?;
            }
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
            let load_amount_linear =
                Linear::new(vec![LinearMonomial::new(1.0, load_amount_idx[j])], 0.0);
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
        // Kotlin: estimateLoadWeight[j] = sum(item.weight * stowage[i][j])
        //         + (if predicateWeightNeeded then y[j] else 0)
        //         + (if recommendedWeightNeeded then z[j] else 0)
        let wu = weight_unit();
        let mut estimate_load_weight_idx = vec![0usize; position_count];
        for (j, position) in self.positions.iter().enumerate() {
            let mut monomials: Vec<LinearMonomial<f64>> = Vec::new();
            for (i, item) in self.items.iter().enumerate() {
                let w = quantity_value_in_unit(&item.weight, &wu)?;
                monomials.push(LinearMonomial::new(w, stowage_vars.stowage[i][j]));
            }
            // 条件添加 y[j]：仅当 predicateWeightNeeded
            if position.status.predicate_weight_needed {
                monomials.push(LinearMonomial::new(1.0, y_idx[j]));
            }
            // 条件添加 z[j]：仅当 recommendedWeightNeeded
            if position.status.recommended_weight_needed {
                monomials.push(LinearMonomial::new(1.0, z_idx[j]));
            }
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
        // Kotlin 语义：
        //   if (!predicateWeightNeeded) → 常量 0
        //   else:
        //     val plw_min = position.predicateLoadWeightMin  // 必须有值
        //     if (position.maxLoadAmount == 1 && (stowageNeeded || adjustmentNeeded))
        //       slack = Slack(y[j], loadAmount[j] * plw_min)
        //     else
        //       slack = Slack(y[j], plw_min)
        let mut slack_idx = vec![0usize; position_count];
        for (j, position) in self.positions.iter().enumerate() {
            if position.status.predicate_weight_needed {
                let plw_min = position.status.predicate_load_weight_min.ok_or_else(|| {
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!(
                            "Position {} requires predicate_load_weight_min when predicate_weight_needed=true",
                            position.id
                        ),
                    )) as Box<dyn std::error::Error>
                })?;

                let left = Linear::new(vec![LinearMonomial::new(1.0, y_idx[j])], 0.0);

                let right = if position.max_load_amount == 1
                    && (position.status.stowage_needed || position.status.adjustment_needed)
                {
                    // Slack(y[j], loadAmount[j] * plw_min)
                    Linear::new(vec![LinearMonomial::new(plw_min, load_amount_idx[j])], 0.0)
                } else {
                    // Slack(y[j], plw_min)
                    Linear::new(Vec::new(), plw_min)
                };

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
            // else: slack_idx[j] remains 0 (constant 0)
        }

        // 8. 创建 y_same_as[j] = SameAsFunction(y, actualLoadWeight) 中间符号
        // 仅当 predicate_weight_needed 时创建
        let mut y_same_as_idx = vec![0usize; position_count];
        for (j, position) in self.positions.iter().enumerate() {
            if position.status.predicate_weight_needed {
                let first = Linear::new(vec![LinearMonomial::new(1.0, y_idx[j])], 0.0);
                let second = Linear::new(
                    vec![LinearMonomial::new(1.0, actual_load_weight_idx[j])],
                    0.0,
                );
                let same_as_fn = SameAsFunction::new(
                    next_id,
                    &format!("y_same_as_{}", position.id),
                    first,
                    second,
                    0.0,
                );
                let result_idx = same_as_fn.result_variable().index();
                model.add_symbol(Arc::new(same_as_fn))?;
                y_same_as_idx[j] = result_idx;
                next_id += 1;
            }
        }

        // 9. 创建 z_same_as[j] = SameAsFunction(z, estimateLoadWeight) 中间符号
        // 仅当 recommended_weight_needed 时创建
        let mut z_same_as_idx = vec![0usize; position_count];
        for (j, position) in self.positions.iter().enumerate() {
            if position.status.recommended_weight_needed {
                let first = Linear::new(vec![LinearMonomial::new(1.0, z_idx[j])], 0.0);
                let second = Linear::new(
                    vec![LinearMonomial::new(1.0, estimate_load_weight_idx[j])],
                    0.0,
                );
                let same_as_fn = SameAsFunction::new(
                    next_id,
                    &format!("z_same_as_{}", position.id),
                    first,
                    second,
                    0.0,
                );
                let result_idx = same_as_fn.result_variable().index();
                model.add_symbol(Arc::new(same_as_fn))?;
                z_same_as_idx[j] = result_idx;
                next_id += 1;
            }
        }

        // 10. 创建 y_if[j] = IfFunction(loadAmount >= 1, then_=y_same_as, else_=y) 中间符号
        // 仅当 predicate_weight_needed 时创建
        let mut y_if_idx = vec![0usize; position_count];
        for (j, position) in self.positions.iter().enumerate() {
            if position.status.predicate_weight_needed {
                let condition =
                    Linear::new(vec![LinearMonomial::new(1.0, load_amount_idx[j])], -1.0);
                let then_expr = Linear::new(vec![LinearMonomial::new(1.0, y_same_as_idx[j])], 0.0);
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
        }

        // 11. 创建 z_if[j] = IfFunction(loadAmount >= 1, then_=z_same_as, else_=z) 中间符号
        // 仅当 recommended_weight_needed 时创建
        let mut z_if_idx = vec![0usize; position_count];
        for (j, position) in self.positions.iter().enumerate() {
            if position.status.recommended_weight_needed {
                let condition =
                    Linear::new(vec![LinearMonomial::new(1.0, load_amount_idx[j])], -1.0);
                let then_expr = Linear::new(vec![LinearMonomial::new(1.0, z_same_as_idx[j])], 0.0);
                let else_expr = Linear::new(vec![LinearMonomial::new(1.0, z_idx[j])], 0.0);
                let if_fn = IfFunction::new(
                    next_id,
                    &format!("z_if_{}", position.id),
                    condition,
                    then_expr,
                    else_expr,
                );
                let result_idx = if_fn.result_variable().index();
                model.add_symbol(Arc::new(if_fn))?;
                z_if_idx[j] = result_idx;
                next_id += 1;
            }
        }

        // 12. 创建 estimateLoaded[j] = OrFunction(loadedItem, y_if, z_if)
        // Kotlin: estimateLoaded = Or(loadedItem, predicateWeightNeeded, recommendedWeightNeeded)
        // loadedItem = Binaryzation(loadAmount) (not full = Binaryzation(loadAmount >= mla))
        // predicateWeightNeeded = y_if (IfFunction, only when predicate_weight_needed)
        // recommendedWeightNeeded = z_if (IfFunction, only when recommended_weight_needed)
        let mut estimate_loaded_idx = vec![0usize; position_count];
        for (j, position) in self.positions.iter().enumerate() {
            // loadedItem = Binaryzation(loadAmount >= 1)
            // Kotlin: loadedItem = Binaryzation(loadAmount[j])
            // threshold = 1.0 means position is loaded when loadAmount >= 1
            let load_amount_linear =
                Linear::new(vec![LinearMonomial::new(1.0, load_amount_idx[j])], 0.0);
            let loaded_fn = BinaryzationFunction::with_threshold(
                next_id,
                &format!("loaded_item_{}", position.id),
                load_amount_linear,
                1.0,
            );
            let loaded_idx = loaded_fn.result_variable().index();
            model.add_symbol(Arc::new(loaded_fn))?;
            next_id += 1;

            let mut or_inputs: Vec<Linear<f64>> = Vec::new();
            or_inputs.push(Linear::new(vec![LinearMonomial::new(1.0, loaded_idx)], 0.0));

            // y_if: only when predicate_weight_needed
            if position.status.predicate_weight_needed {
                or_inputs.push(Linear::new(
                    vec![LinearMonomial::new(1.0, y_if_idx[j])],
                    0.0,
                ));
            }

            // z_if: only when recommended_weight_needed
            if position.status.recommended_weight_needed {
                or_inputs.push(Linear::new(
                    vec![LinearMonomial::new(1.0, z_if_idx[j])],
                    0.0,
                ));
            }

            let or_fn = OrFunction::new(
                next_id,
                &format!("estimate_loaded_{}", position.id),
                or_inputs,
            );
            let result_idx = or_fn.result_variable().index();
            model.add_symbol(Arc::new(or_fn))?;
            estimate_loaded_idx[j] = result_idx;
            next_id += 1;
        }

        // 13. 创建 actualLoaded[j] = Binaryzation(loadAmount)
        // Kotlin: actualLoaded = loadedItem = Binaryzation(loadAmount)
        let mut actual_loaded_idx = vec![0usize; position_count];
        for (j, position) in self.positions.iter().enumerate() {
            let load_amount_linear =
                Linear::new(vec![LinearMonomial::new(1.0, load_amount_idx[j])], 0.0);
            // actualLoaded = Binaryzation(loadAmount >= 1)
            // threshold = 1.0 means position is loaded when loadAmount >= 1
            let actual_fn = BinaryzationFunction::with_threshold(
                next_id,
                &format!("actual_loaded_{}", position.id),
                load_amount_linear,
                1.0,
            );
            let result_idx = actual_fn.result_variable().index();
            model.add_symbol(Arc::new(actual_fn))?;
            actual_loaded_idx[j] = result_idx;
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
            estimate_loaded: estimate_loaded_idx,
            actual_loaded: actual_loaded_idx,
        })
    }
}
