//! 物品模型 / Item model
use super::super::super::shared::units;
use super::cargo::Cargo;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::Unit;

/// 物品位置标签 / Item location tag (对齐 Kotlin ItemLocationTag)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemLocationTag {
    /// 主舱 / Main deck
    Main,
    /// 下舱 / Lower deck
    Low,
    /// 散货舱 / Bulk compartment
    Bulk,
    /// 前舱 / Head compartment
    Head,
    /// 后舱 / Tail compartment
    Tail,
}

/// 物品位置 / Item location (对齐 Kotlin ItemLocation)
#[derive(Debug, Clone)]
pub struct ItemLocation {
    /// 位置标签列表 / Location tag list
    pub tags: Vec<ItemLocationTag>,
}

impl ItemLocation {
    /// 是否在主舱 / Whether on main deck
    pub fn main(&self) -> bool {
        self.tags.contains(&ItemLocationTag::Main)
    }
    /// 是否在下舱 / Whether on lower deck
    pub fn low(&self) -> bool {
        self.tags.contains(&ItemLocationTag::Low)
    }
    /// 是否在散货舱 / Whether in bulk compartment
    pub fn bulk(&self) -> bool {
        self.tags.contains(&ItemLocationTag::Bulk)
    }
    /// 是否在前舱 / Whether in head compartment
    pub fn head(&self) -> bool {
        self.tags.contains(&ItemLocationTag::Head)
    }
    /// 是否在后舱 / Whether in tail compartment
    pub fn tail(&self) -> bool {
        self.tags.contains(&ItemLocationTag::Tail)
    }
    /// 是否为普通主舱位置 / Whether in normal main deck position
    pub fn normal_main(&self) -> bool {
        self.main() && !self.head() && !self.tail()
    }
    /// 是否为特殊主舱位置 / Whether in special main deck position
    pub fn special_main(&self) -> bool {
        self.head() || self.tail()
    }
    /// 是否为下舱非散货位置 / Whether in lower deck non-bulk position
    pub fn low_not_bulk(&self) -> bool {
        self.low() && !self.bulk()
    }

    /// 前舱位置 / Head location
    pub fn head_location() -> Self {
        Self {
            tags: vec![ItemLocationTag::Main, ItemLocationTag::Head],
        }
    }
    /// 后舱位置 / Tail location
    pub fn tail_location() -> Self {
        Self {
            tags: vec![ItemLocationTag::Main, ItemLocationTag::Tail],
        }
    }
    /// 普通主舱位置 / Normal main deck location
    pub fn normal_main_location() -> Self {
        Self {
            tags: vec![ItemLocationTag::Main],
        }
    }
    /// 下舱散货位置 / Lower deck bulk location
    pub fn low_bulk_location() -> Self {
        Self {
            tags: vec![ItemLocationTag::Low, ItemLocationTag::Bulk],
        }
    }
    /// 下舱非散货位置 / Lower deck non-bulk location
    pub fn low_not_bulk_location() -> Self {
        Self {
            tags: vec![ItemLocationTag::Low],
        }
    }
}

/// 物品状态 / Item status (对齐 Kotlin ItemStatus)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemStatus {
    /// 已预留 / Reserved
    Reserved,
    /// 可选 / Optional
    Optional,
    /// 已预分配 / Preassigned
    Preassigned,
    /// 已装载 / Loaded
    Loaded,
    /// 需要调整 / Adjustment needed
    AdjustmentNeeded,
}

impl ItemStatus {
    /// 是否需要装载 / Whether stowage is needed
    pub fn stowage_needed(&self) -> bool {
        matches!(
            self,
            ItemStatus::Optional
                | ItemStatus::Preassigned
                | ItemStatus::Loaded
                | ItemStatus::AdjustmentNeeded
        )
    }

    /// 是否需要调整 / Whether adjustment is needed
    pub fn adjustment_needed(&self) -> bool {
        matches!(self, ItemStatus::AdjustmentNeeded)
    }
}

/// ULD / ULD (对齐 Kotlin ULD)
#[derive(Debug, Clone)]
pub struct Uld {
    /// ULD 代码 / ULD code
    pub code: String,
    /// ULD 名称 / ULD name
    pub name: String,
}

/// 物品 / Item (对齐 Kotlin Item)
#[derive(Debug, Clone)]
pub struct Item {
    /// 物品标识 / Item identifier
    pub id: String,
    /// 物品名称 / Item name
    pub name: String,
    /// 物品重量 / Item weight
    pub weight: Quantity<f64, Unit>,
    /// 货物分类 / Cargo classification
    pub cargo: Cargo,
    /// 位置信息 / Location information
    pub location: ItemLocation,
    /// 物品状态 / Item status
    pub status: ItemStatus,
    /// 目的站 / Destination station
    pub destination: String,
    /// 所属 ULD / Associated ULD
    pub uld: Option<Uld>,
}
