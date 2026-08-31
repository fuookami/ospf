use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::IfFunction;

use super::trailer::Trailer;

/// 拖车装载 / Trailer loading (对齐 Kotlin TrailerLoading)
#[derive(Debug, Clone)]
pub struct TrailerLoading {
    pub trailer: Trailer,
    pub position_id: String,
}

/// TrailerLoading IfFunction 注册结果 / TrailerLoading IfFunction registration result
#[derive(Debug, Clone)]
pub struct TrailerLoadingVariables {
    /// loading_if = IfFunction(condition=sum(loaded[item]), then=1, else=0) 的结果变量 solver index
    /// 当拖车上的任一物品被装载时值为 1，否则为 0
    pub loading_if: usize,
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
}
