use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::SlackFunction;

/// 冗余 / Redundancy (对齐 Kotlin Redundancy)
///
/// Kotlin-Rust 映射 / Kotlin-Rust Mapping:
/// - Kotlin `redundancySlack` -> Rust `SlackFunction` (松弛函数)
/// - `redundancySlack = |main_deck_capacity - actual_load|`
#[derive(Debug, Clone)]
pub struct Redundancy {
    pub main_deck_capacity: f64,
    pub actual_load: f64,
    pub slack: f64,
}

impl Redundancy {
    pub fn redundancy(&self) -> f64 {
        self.main_deck_capacity - self.actual_load
    }

    /// 注册冗余松弛中间符号到模型
    ///
    /// 对齐 Kotlin Redundancy.register:
    /// 创建 redundancySlack = SlackFunction(main_deck_capacity, actual_load)
    /// slack = |main_deck_capacity - actual_load|
    ///
    /// Kotlin-Rust 映射 / Kotlin-Rust Mapping:
    /// - `redundancySlack` -> `SlackFunction(capacity_linear, load_linear)`
    ///
    /// 参数 / Parameters:
    /// - `id`: 符号 ID
    /// - `model`: 元模型
    /// - `capacity_idx`: main_deck_capacity 对应的变量索引
    /// - `load_idx`: actual_load 对应的变量索引
    pub fn register_slack(
        &self,
        id: u64,
        model: &mut MetaModel<f64>,
        capacity_idx: usize,
        load_idx: usize,
    ) -> Result<usize, Box<dyn Error>> {
        // Kotlin: val redundancySlack = SlackFunction(mainDeckCapacity, actualLoad)
        // slack = |main_deck_capacity - actual_load|
        let left = Linear::new(vec![LinearMonomial::new(1.0, capacity_idx)], 0.0);
        let right = Linear::new(vec![LinearMonomial::new(1.0, load_idx)], 0.0);
        let slack_fn = SlackFunction::new(id, "redundancy_slack", left, right);
        let result_idx = slack_fn.result_variable().index();
        model.add_symbol(Arc::new(slack_fn))?;
        Ok(result_idx)
    }
}
