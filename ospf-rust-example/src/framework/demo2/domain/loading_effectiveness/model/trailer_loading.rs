//! 拖车装载模型 / Trailer loading model
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::IfFunction;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

use super::trailer::Trailer;
use crate::framework::demo2::infrastructure::dto::PositionPair;

/// 拖车装载 / Trailer loading (对齐 Kotlin TrailerLoading)
#[derive(Debug, Clone)]
pub struct TrailerLoading {
    /// 拖车信息 / Trailer information
    pub trailer: Trailer,
    /// 舱位标识 / Position identifier
    pub position_id: String,
}

/// TrailerLoading IfFunction 注册结果 / TrailerLoading IfFunction registration result
#[derive(Debug, Clone)]
pub struct TrailerLoadingVariables {
    /// loading_if 的结果变量 solver index / Solver index of loading_if result variable
    /// 当拖车上的任一物品被装载时值为 1，否则为 0
    pub loading_if: usize,
}

/// 拖车更换注册结果 / Trailer change registration result
/// 对齐 Kotlin TrailerLoading.trailerChange / trailerCircling
#[derive(Debug, Clone)]
pub struct TrailerChangeVariables {
    /// trailer_change 的结果变量 solver index 二维数组 / 2D array of trailer_change result variable solver indices
    /// 对齐 Kotlin trailerChange: LinearIntermediateSymbols2
    /// 维度: [ordered_trailers.len(), adjacent_positions.len()]
    pub trailer_change: Vec<Vec<usize>>,
}

impl TrailerLoading {
    /// 注册 loading IfFunction 到模型
    ///
    /// 对齐 Kotlin TrailerLoading:
    /// ```kotlin
    /// val loading = IfFunction(
    ///     condition = sum(loaded[item] for items in trailer),
    ///     then_ = 1.0,
    ///     else_ = 0.0
    /// )
    /// ```
    ///
    /// 创建条件指示变量，当拖车上的任一物品被装载时值为 1，否则为 0。
    ///
    /// # Arguments
    /// * `model` - 模型实例
    /// * `trailer_name` - 拖车名称（用于符号命名）
    /// * `item_indices` - 拖车上物品在 loaded_idx 中的索引列表
    /// * `loaded_idx` - loaded[i] 的 solver 索引（来自 StowageVariables.loaded）
    /// * `next_id` - 下一个可用的符号 ID
    pub fn register_loading_if(
        model: &mut MetaModel<f64>,
        trailer_name: &str,
        item_indices: &[usize],
        loaded_idx: &[usize],
        next_id: &mut u64,
    ) -> Result<TrailerLoadingVariables, Box<dyn Error>> {
        // condition: sum(loaded[item] for items in trailer)
        // nonzero when any item on the trailer is loaded
        let monomials: Vec<LinearMonomial<f64>> = item_indices
            .iter()
            .map(|&idx| LinearMonomial::new(1.0, loaded_idx[idx]))
            .collect();
        let condition = Linear::new(monomials, 0.0);
        // then_expr: 1.0 (trailer has loaded items)
        let then_expr = Linear::new(Vec::new(), 1.0);
        // else_expr: 0.0 (trailer has no loaded items)
        let else_expr = Linear::new(Vec::new(), 0.0);

        let safe_name = trailer_name.replace(' ', "_").to_lowercase();
        let if_fn = IfFunction::new(
            *next_id,
            &format!("trailer_loading_if_{}", safe_name),
            condition,
            then_expr,
            else_expr,
        );
        let result_idx = if_fn.result_variable().index();
        model.add_symbol(Arc::new(if_fn))?;
        *next_id += 1;

        Ok(TrailerLoadingVariables {
            loading_if: result_idx,
        })
    }

    /// 注册 trailerChange IfFunction 符号到模型
    ///
    /// 对齐 Kotlin TrailerLoading.trailerChange:
    /// ```kotlin
    /// trailerChange = LinearIntermediateSymbols2("trailer_change",
    ///     Shape2(orderedTrailers.size, adjacentPositions.size)) { _, v ->
    ///     val (trailer1, trailer2) = orderedTrailers[v[0]]
    ///     val (position1, position2) = adjacentPositions[v[1]]
    ///     val loadAmount1 = load.loadAmountOf(position1) { item -> item in trailer2.items }
    ///     val loadAmount2 = load.loadAmountOf(position2) { item -> item in trailer1.items }
    ///     IfFunction(condition = loadAmount1 + loadAmount2 - 2)
    /// }
    /// ```
    ///
    /// trailerChange[t][p] = 1 当且仅当:
    ///   - trailer2 的物品在 position1 上有装载 (loadAmount1 >= 1)
    ///   - trailer1 的物品在 position2 上有装载 (loadAmount2 >= 1)
    /// 即两个拖车的物品在相邻位置发生了交叉装载（拖车更换）。
    ///
    /// # Arguments
    /// * `model` - 模型实例
    /// * `x_idx` - x_idx[cargo][position] 的 solver 索引
    /// * `ordered_trailers` - 有序拖车对列表
    /// * `adjacent_positions` - 相邻位置对列表
    /// * `item_name_to_idx` - 物品名称到 cargo 索引的映射
    /// * `next_id` - 下一个可用的符号 ID
    pub fn register_trailer_change(
        model: &mut MetaModel<f64>,
        x_idx: &[Vec<usize>],
        ordered_trailers: &[(Trailer, Trailer)],
        adjacent_positions: &[PositionPair],
        item_name_to_idx: &HashMap<String, usize>,
        next_id: &mut u64,
    ) -> Result<TrailerChangeVariables, Box<dyn Error>> {
        let mut trailer_change = Vec::with_capacity(ordered_trailers.len());

        for (_t_idx, (trailer1, trailer2)) in ordered_trailers.iter().enumerate() {
            let mut change_row = Vec::with_capacity(adjacent_positions.len());

            for (_p_idx, adj) in adjacent_positions.iter().enumerate() {
                let j1 = adj.first;
                let j2 = adj.second;

                // loadAmount1 = sum(x[c][j1]) for c in trailer2.items
                let load_amount1_monomials: Vec<LinearMonomial<f64>> = trailer2
                    .items
                    .iter()
                    .filter_map(|item_name| {
                        item_name_to_idx.get(item_name).and_then(|&c| {
                            if c < x_idx.len() && j1 < x_idx[c].len() {
                                Some(LinearMonomial::new(1.0, x_idx[c][j1]))
                            } else {
                                None
                            }
                        })
                    })
                    .collect();
                let load_amount1 = Linear::new(load_amount1_monomials, 0.0);

                // loadAmount2 = sum(x[c][j2]) for c in trailer1.items
                let load_amount2_monomials: Vec<LinearMonomial<f64>> = trailer1
                    .items
                    .iter()
                    .filter_map(|item_name| {
                        item_name_to_idx.get(item_name).and_then(|&c| {
                            if c < x_idx.len() && j2 < x_idx[c].len() {
                                Some(LinearMonomial::new(1.0, x_idx[c][j2]))
                            } else {
                                None
                            }
                        })
                    })
                    .collect();
                let load_amount2 = Linear::new(load_amount2_monomials, 0.0);

                // condition = loadAmount1 + loadAmount2 - 2
                // IfFunction fires when both load amounts >= 1 (sum >= 2)
                let condition_monomials: Vec<LinearMonomial<f64>> = load_amount1
                    .monomials()
                    .iter()
                    .chain(load_amount2.monomials().iter())
                    .cloned()
                    .collect();
                let condition = Linear::new(
                    condition_monomials,
                    *load_amount1.constant_term() + *load_amount2.constant_term() - 2.0,
                );
                let then_expr = Linear::new(Vec::new(), 1.0);
                let else_expr = Linear::new(Vec::new(), 0.0);

                let safe_t1 = trailer1.name.replace(' ', "_").to_lowercase();
                let safe_t2 = trailer2.name.replace(' ', "_").to_lowercase();
                let if_fn = IfFunction::new(
                    *next_id,
                    &format!("trailer_change_{}_{}_{}_{}", safe_t1, safe_t2, j1, j2),
                    condition,
                    then_expr,
                    else_expr,
                );
                let result_idx = if_fn.result_variable().index();
                model.add_symbol(Arc::new(if_fn))?;
                *next_id += 1;

                change_row.push(result_idx);
            }

            trailer_change.push(change_row);
        }

        Ok(TrailerChangeVariables { trailer_change })
    }
}
