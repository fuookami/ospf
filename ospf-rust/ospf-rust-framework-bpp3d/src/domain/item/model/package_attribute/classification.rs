/// 包装分类 / Package classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackageClassification {
    /// 外包装 / Outer package
    Outer,
    /// 内包装 / Inner package
    Inner,
}

/// 包装类别 / Package category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackageCategory {
    /// 硬箱 / Hard box
    HardBox,
    /// 托盘 / Pallet
    Pallet,
    /// 软箱 / Soft box
    SoftBox,
    /// 填充物 / Filler
    Filler,
}

/// 包装类型 / Package type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackageType {
    /// 重型瓦楞纸托 / Duty corrugated board pedal
    DutyCorrugatedBoardPedal,
    /// 木箱 / Wooden container
    WoodenContainer,
    /// 蜂窝箱 / Honeycomb box
    HoneycombBox,
    /// 托盘 / Pallet
    Pallet,
    /// 纸箱托盘 / Carton pallet
    CartonPallet,
    /// 纸箱 / Carton container
    CartonContainer,
    /// 包装泡棉 / Packing foam
    PackingFoam,
}

impl PackageType {
    /// 所有包装类型 / All package types
    pub const ALL: [Self; 7] = [
        Self::DutyCorrugatedBoardPedal,
        Self::WoodenContainer,
        Self::HoneycombBox,
        Self::Pallet,
        Self::CartonPallet,
        Self::CartonContainer,
        Self::PackingFoam,
    ];

    /// 包装类别 / Package category
    pub fn category(self) -> PackageCategory {
        match self {
            Self::DutyCorrugatedBoardPedal
            | Self::WoodenContainer
            | Self::HoneycombBox => PackageCategory::HardBox,
            Self::Pallet | Self::CartonPallet => PackageCategory::Pallet,
            Self::CartonContainer => PackageCategory::SoftBox,
            Self::PackingFoam => PackageCategory::Filler,
        }
    }
}

impl Default for PackageType {
    fn default() -> Self {
        Self::CartonContainer
    }
}

