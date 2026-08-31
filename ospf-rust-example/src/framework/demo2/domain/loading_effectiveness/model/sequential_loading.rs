//! 顺序装载模型 / Sequential loading model
//! 条件只提供已有的 loaded solver 索引，缺少可证明的有限范围，暂保留旧的非零语义 / Conditions only receive an existing loaded solver index, without a provable finite range; retain the legacy nonzero semantics for now.
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::IfFunction as LegacyIfFunction;
use std::error::Error;
use std::sync::Arc;

/// 顺序装载 / Sequential loading (对齐 Kotlin SequentialLoading)
#[derive(Debug, Clone)]
pub struct SequentialLoading {
    /// 物品标识 / Item identifier
    pub item_id: String,
    /// 舱位标识 / Position identifier
    pub position_id: String,
    /// 装载顺序 / Loading order
    pub order: u32,
}

/// SequentialLoading IfFunction 注册结果 / SequentialLoading IfFunction registration result
#[derive(Debug, Clone)]
pub struct SequentialLoadingVariables {
    /// loading_if 的结果变量 solver index / Solver index of loading_if result variable
    /// 当物品被装载时值为 1，否则为 0
    pub loading_if: usize,
}

impl SequentialLoading {
    /// 注册 loading[i] IfFunction 到模型
    ///
    /// 对齐 Kotlin SequentialLoading:
    /// ```kotlin
    /// val loading = IfFunction(
    ///     condition = loaded[i],
    ///     then_ = 1.0,
    ///     else_ = 0.0
    /// )
    /// ```
    ///
    /// 创建条件指示变量，当物品被装载时值为 1，否则为 0。
    ///
    /// # Arguments
    /// * `model` - 模型实例
    /// * `item_idx` - 物品在 loaded_idx 中的索引
    /// * `loaded_idx` - loaded[i] 的 solver 索引（来自 StowageVariables.loaded）
    /// * `next_id` - 下一个可用的符号 ID
    pub fn register_loading_if(
        model: &mut MetaModel<f64>,
        item_idx: usize,
        loaded_idx: &[usize],
        next_id: &mut u64,
    ) -> Result<SequentialLoadingVariables, Box<dyn Error>> {
        // condition: loaded[item] (nonzero when item is loaded at any position)
        let condition = Linear::new(vec![LinearMonomial::new(1.0, loaded_idx[item_idx])], 0.0);
        // then_expr: 1.0 (loaded)
        let then_expr = Linear::new(Vec::new(), 1.0);
        // else_expr: 0.0 (not loaded)
        let else_expr = Linear::new(Vec::new(), 0.0);

        let if_fn = LegacyIfFunction::new(
            *next_id,
            &format!("sequential_loading_if_{}", item_idx),
            condition,
            then_expr,
            else_expr,
        );
        let result_id = if_fn.result_variable().id();
        model.add_symbol(Arc::new(if_fn))?;
        let result_idx = model
            .find_token(result_id)
            .map(|token| token.solver_index)
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "sequential loading result token {} was not registered",
                        result_id
                    ),
                )
            })?;
        *next_id += 1;

        Ok(SequentialLoadingVariables {
            loading_if: result_idx,
        })
    }
}
