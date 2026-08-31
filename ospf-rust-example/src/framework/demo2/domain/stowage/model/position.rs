use super::item::{Item, ItemLocationTag, ItemStatus};
use super::super::super::shared::units;
use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::{Point2, UnivariateLinearPiecewiseFunction};
use ospf_rust_core::symbol::flatten::LinearMonomial;

/// 舱位状态代码 / Position status code (对齐 Kotlin PositionStatusCode)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PositionStatusCode {
    Loaded,
    Unloaded,
    Preassigned,
    Reserved,
}

/// 舱位状态 / Position status (对齐 Kotlin PositionStatus)
#[derive(Debug, Clone)]
pub struct PositionStatus {
    pub code: PositionStatusCode,
    pub available: bool,
    pub stowage_needed: bool,
    pub adjustment_needed: bool,
    pub predicate_weight_needed: bool,
    pub recommended_weight_needed: bool,
}

/// 舱位 / Position (stowage domain, 对齐 Kotlin stowage Position)
#[derive(Debug, Clone)]
pub struct Position {
    pub id: String,
    pub space_name: String,
    pub max_load_amount: u64,
    pub max_load_weight: f64, // 保留 f64 用于约束注册边界
    pub status: PositionStatus,
    pub loaded_items: Vec<String>,
    pub is_main_deck: bool,
    pub is_low_deck: bool,
    pub is_bulk: bool,
    pub is_head: bool,
    pub is_tail: bool,
    pub enabled_uld_codes: Vec<String>,
}

impl Position {
    /// 检查物品是否可以装载到此舱位
    /// 对齐 Kotlin Position.enabled - 完整的位置兼容性检查
    pub fn enabled(&self, item: &Item) -> bool {
        // 1. 舱位位置限制
        if self.is_main_deck && item.location.low() {
            return false;
        }
        if self.is_low_deck && (item.location.head() || item.location.tail()) {
            return false;
        }
        if self.is_bulk && !item.location.bulk() {
            return false;
        }
        if !self.is_bulk && item.location.bulk() {
            return false;
        }
        if self.is_head && !item.location.head() {
            return false;
        }
        if !self.is_head && item.location.head() {
            return false;
        }
        if self.is_tail && !item.location.tail() {
            return false;
        }
        if !self.is_tail && item.location.tail() {
            return false;
        }

        // 2. 舱位类型限制 (简化实现)
        // 完整实现需要 PositionType 和 PositionStowageTaboo 数据

        true
    }

    /// 注册 capacityUsage 中间符号到模型
    ///
    /// 对齐 Kotlin Position.capacityUsage:
    /// 使用 UnivariateLinearPiecewiseFunction 将装载重量映射到容量使用率。
    ///
    /// Kotlin-Rust 映射 / Kotlin-Rust Mapping:
    /// - `capacityUsage[j]` -> `UnivariateLinearPiecewiseFunction(loadWeight)`
    ///
    /// 默认断点 / Default breakpoints:
    /// - (0, 0.0)                 — 空载时 0% 使用率
    /// - (max_load_weight, 1.0)   — 满载时 100% 使用率
    pub fn register_capacity_usage(
        &self,
        id: u64,
        model: &mut MetaModel<f64>,
        load_weight_idx: usize,
    ) -> Result<usize, Box<dyn Error>> {
        let points = vec![
            Point2::new(0.0, 0.0),
            Point2::new(self.max_load_weight, 1.0),
        ];
        let input = Linear::new(vec![LinearMonomial::new(1.0, load_weight_idx)], 0.0);
        let ulp_fn = UnivariateLinearPiecewiseFunction::new(
            id,
            &format!("capacity_usage_{}", self.id),
            input,
            points,
        );
        let result_idx = ulp_fn.result_variable().index();
        model.add_symbol(Arc::new(ulp_fn))?;
        Ok(result_idx)
    }
}
