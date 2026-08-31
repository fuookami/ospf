use super::cargo::Cargo;
use super::super::super::shared::units;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::Unit;

/// 物品位置标签 / Item location tag (对齐 Kotlin ItemLocationTag)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemLocationTag {
    Main, Low, Bulk, Head, Tail,
}

/// 物品位置 / Item location (对齐 Kotlin ItemLocation)
#[derive(Debug, Clone)]
pub struct ItemLocation {
    pub tags: Vec<ItemLocationTag>,
}

impl ItemLocation {
    pub fn main(&self) -> bool { self.tags.contains(&ItemLocationTag::Main) }
    pub fn low(&self) -> bool { self.tags.contains(&ItemLocationTag::Low) }
    pub fn bulk(&self) -> bool { self.tags.contains(&ItemLocationTag::Bulk) }
    pub fn head(&self) -> bool { self.tags.contains(&ItemLocationTag::Head) }
    pub fn tail(&self) -> bool { self.tags.contains(&ItemLocationTag::Tail) }
    pub fn normal_main(&self) -> bool { self.main() && !self.head() && !self.tail() }
    pub fn special_main(&self) -> bool { self.head() || self.tail() }
    pub fn low_not_bulk(&self) -> bool { self.low() && !self.bulk() }

    pub fn head_location() -> Self {
        Self { tags: vec![ItemLocationTag::Main, ItemLocationTag::Head] }
    }
    pub fn tail_location() -> Self {
        Self { tags: vec![ItemLocationTag::Main, ItemLocationTag::Tail] }
    }
    pub fn normal_main_location() -> Self {
        Self { tags: vec![ItemLocationTag::Main] }
    }
    pub fn low_bulk_location() -> Self {
        Self { tags: vec![ItemLocationTag::Low, ItemLocationTag::Bulk] }
    }
    pub fn low_not_bulk_location() -> Self {
        Self { tags: vec![ItemLocationTag::Low] }
    }
}

/// 物品状态 / Item status (对齐 Kotlin ItemStatus)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemStatus {
    Reserved,
    Optional,
    Preassigned,
    Loaded,
    AdjustmentNeeded,
}

impl ItemStatus {
    pub fn stowage_needed(&self) -> bool {
        matches!(self, ItemStatus::Optional | ItemStatus::Preassigned | ItemStatus::Loaded | ItemStatus::AdjustmentNeeded)
    }

    pub fn adjustment_needed(&self) -> bool {
        matches!(self, ItemStatus::AdjustmentNeeded)
    }
}

/// ULD / ULD (对齐 Kotlin ULD)
#[derive(Debug, Clone)]
pub struct Uld {
    pub code: String,
    pub name: String,
}

/// 物品 / Item (对齐 Kotlin Item)
#[derive(Debug, Clone)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub weight: Quantity<f64, Unit>,
    pub cargo: Cargo,
    pub location: ItemLocation,
    pub status: ItemStatus,
    pub destination: String,
    pub uld: Option<Uld>,
}
