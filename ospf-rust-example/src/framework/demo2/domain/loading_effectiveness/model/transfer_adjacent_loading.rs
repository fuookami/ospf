//! 转移邻接装载模型 / Transfer adjacent loading model
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::IfFunction;
use std::error::Error;
use std::sync::Arc;

/// 转运邻接装载 / Transfer adjacent loading (对齐 Kotlin TransferAdjacentLoading)
#[derive(Debug, Clone)]
pub struct TransferAdjacentLoading {
    /// 物品标识 / Item identifier
    pub item_id: String,
    /// 邻接舱位标识 / Adjacent position identifier
    pub adjacent_position_id: String,
}

/// TransferAdjacentLoading IfFunction 注册结果 / TransferAdjacentLoading IfFunction registration result
#[derive(Debug, Clone)]
pub struct TransferAdjacentLoadingVariables {
    /// loading_if 的结果变量 solver index / Solver index of loading_if result variable
    /// 当物品被装载到邻接舱位时值为 1，否则为 0
    pub loading_if: usize,
}

impl TransferAdjacentLoading {
    /// 注册 loading IfFunction 到模型
    ///
    /// 对齐 Kotlin TransferAdjacentLoading:
    /// ```kotlin
    /// val loading = IfFunction(
    ///     condition = x[item][adjacentPosition],
    ///     then_ = 1.0,
    ///     else_ = 0.0
    /// )
    /// ```
    ///
    /// 创建条件指示变量，当物品被装载到邻接舱位时值为 1，否则为 0。
    ///
    /// # Arguments
    /// * `model` - 模型实例
    /// * `item_idx` - 物品在 x_idx 中的索引
    /// * `adjacent_position_idx` - 邻接舱位在 x_idx[item] 中的索引
    /// * `x_idx` - x[i][j] 的 solver 索引（来自 StowageVariables.x 或 RegistrationResult.x_idx）
    /// * `next_id` - 下一个可用的符号 ID
    pub fn register_loading_if(
        model: &mut MetaModel<f64>,
        item_idx: usize,
        adjacent_position_idx: usize,
        x_idx: &[Vec<usize>],
        next_id: &mut u64,
    ) -> Result<TransferAdjacentLoadingVariables, Box<dyn Error>> {
        // condition: x[item][adjacent_position] (nonzero when item is loaded at adjacent position)
        let condition = Linear::new(
            vec![LinearMonomial::new(
                1.0,
                x_idx[item_idx][adjacent_position_idx],
            )],
            0.0,
        );
        // then_expr: 1.0 (item loaded at adjacent position)
        let then_expr = Linear::new(Vec::new(), 1.0);
        // else_expr: 0.0 (item not loaded at adjacent position)
        let else_expr = Linear::new(Vec::new(), 0.0);

        let if_fn = IfFunction::new(
            *next_id,
            &format!(
                "transfer_adjacent_loading_if_{}_{}",
                item_idx, adjacent_position_idx
            ),
            condition,
            then_expr,
            else_expr,
        );
        let result_idx = if_fn.result_variable().index();
        model.add_symbol(Arc::new(if_fn))?;
        *next_id += 1;

        Ok(TransferAdjacentLoadingVariables {
            loading_if: result_idx,
        })
    }
}
