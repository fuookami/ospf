//! 相对顺序模型 / Relative order model
//! 条件只提供已有的 loaded solver 索引，缺少可证明的有限范围，暂保留旧的非零语义 / Conditions only receive an existing loaded solver index, without a provable finite range; retain the legacy nonzero semantics for now.
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::IfFunction as LegacyIfFunction;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

/// 相对顺序 / Relative order (对齐 Kotlin RelativeOrder)
#[derive(Debug, Clone)]
pub struct RelativeOrder {
    /// 物品前置依赖映射 / Item precedence mapping (item_id -> items that must load before it)
    pub precedence: HashMap<String, Vec<String>>, // item_id -> [must_load_before items]
}

/// RelativeOrder IfFunction 注册结果 / RelativeOrder IfFunction registration result
#[derive(Debug, Clone)]
pub struct RelativeOrderVariables {
    /// order_if 的结果变量 solver index / Solver index of order_if result variable
    /// 当物品被装载时值为 1，否则为 0；用于相对顺序约束
    pub order_if: usize,
}

impl RelativeOrder {
    /// 注册 order[i] IfFunction 到模型
    ///
    /// 对齐 Kotlin RelativeOrder:
    /// ```kotlin
    /// val order = IfFunction(
    ///     condition = loaded[i],
    ///     then_ = 1.0,
    ///     else_ = 0.0
    /// )
    /// ```
    ///
    /// 创建条件指示变量，当物品被装载时值为 1，否则为 0。
    /// 用于相对顺序约束中判断物品是否参与排序。
    ///
    /// # Arguments
    /// * `model` - 模型实例
    /// * `item_idx` - 物品在 loaded_idx 中的索引
    /// * `loaded_idx` - loaded[i] 的 solver 索引（来自 StowageVariables.loaded）
    /// * `next_id` - 下一个可用的符号 ID
    pub fn register_order_if(
        model: &mut MetaModel<f64>,
        item_idx: usize,
        loaded_idx: &[usize],
        next_id: &mut u64,
    ) -> Result<RelativeOrderVariables, Box<dyn Error>> {
        // condition: loaded[item] (nonzero when item is loaded)
        let condition = Linear::new(vec![LinearMonomial::new(1.0, loaded_idx[item_idx])], 0.0);
        // then_expr: 1.0 (item participates in ordering)
        let then_expr = Linear::new(Vec::new(), 1.0);
        // else_expr: 0.0 (item does not participate)
        let else_expr = Linear::new(Vec::new(), 0.0);

        let if_fn = LegacyIfFunction::new(
            *next_id,
            &format!("relative_order_if_{}", item_idx),
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
                        "relative order result token {} was not registered",
                        result_id
                    ),
                )
            })?;
        *next_id += 1;

        Ok(RelativeOrderVariables {
            order_if: result_idx,
        })
    }
}
