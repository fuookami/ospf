//! 舱位模型 / Position model (stowage)
use super::super::super::shared::units;
use super::item::{Item, ItemLocationTag, ItemStatus};
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::flatten::LinearMonomial;
use ospf_rust_core::symbol::function::{Point2, UnivariateLinearPiecewiseFunction};
use std::error::Error;
use std::sync::Arc;

/// 舱位状态代码 / Position status code (对齐 Kotlin PositionStatusCode)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PositionStatusCode {
    /// 已装载 / Loaded
    Loaded,
    /// 未装载 / Unloaded
    Unloaded,
    /// 已预分配 / Preassigned
    Preassigned,
    /// 已预留 / Reserved
    Reserved,
}

/// 舱位状态 / Position status (对齐 Kotlin PositionStatus)
#[derive(Debug, Clone)]
pub struct PositionStatus {
    /// 状态代码 / Status code
    pub code: PositionStatusCode,
    /// 是否可用 / Whether available
    pub available: bool,
    /// 是否需要装载 / Whether stowage is needed
    pub stowage_needed: bool,
    /// 是否需要调整 / Whether adjustment is needed
    pub adjustment_needed: bool,
    /// 是否需要预测装载重量 / Whether predicate load weight is needed
    pub predicate_weight_needed: bool,
    /// 是否需要推荐装载重量 / Whether recommended load weight is needed
    pub recommended_weight_needed: bool,
    /// 最小预测装载重量 / Minimum predicate load weight (对齐 Kotlin plw.min)
    /// 当 predicate_weight_needed == true 时必须有值。
    /// Must be present when predicate_weight_needed == true.
    pub predicate_load_weight_min: Option<f64>,
}

/// 舱位 / Position (stowage domain, 对齐 Kotlin stowage Position)
#[derive(Debug, Clone)]
pub struct Position {
    /// 舱位标识 / Position identifier
    pub id: String,
    /// 空间名称 / Space name
    pub space_name: String,
    /// 最大装载数量 / Maximum load amount
    pub max_load_amount: u64,
    /// 最大装载重量 / Maximum load weight
    pub max_load_weight: f64, // 保留 f64 用于约束注册边界
    /// 舱位状态 / Position status
    pub status: PositionStatus,
    /// 已装载物品列表 / Loaded item identifiers
    pub loaded_items: Vec<String>,
    /// 是否为主舱 / Whether on main deck
    pub is_main_deck: bool,
    /// 是否为下舱 / Whether on lower deck
    pub is_low_deck: bool,
    /// 是否为散货舱 / Whether bulk compartment
    pub is_bulk: bool,
    /// 是否为前舱 / Whether head compartment
    pub is_head: bool,
    /// 是否为后舱 / Whether tail compartment
    pub is_tail: bool,
    /// 允许的 ULD 代码列表 / Enabled ULD codes
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
