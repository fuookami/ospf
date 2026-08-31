use super::item::{Item, ItemLocationTag, ItemStatus};
use super::super::super::shared::units;

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
}
