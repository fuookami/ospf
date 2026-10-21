//! 物料、设备与副产物 / Materials, machines and by-products
//!
//! 物料是**双模式**的核心：`height` 为空表示卷材（幅宽固定、长度待定，对应条带排样），否则为定尺板材。
//! The material is the core of the **dual mode**: a null `height` means a coil (fixed width, length to be
//! determined, i.e. strip packing), otherwise it is a fixed-size sheet.

use ospf_rust_base::error::Ret;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::infrastructure::error::validation_error;
use crate::infrastructure::region::{Region2, region2_of};

use super::ids::{CostarId2, MachineId2, MaterialId2};

// ============================================================================
// Material2Kind - 物料形态 / Material shape kind
// ============================================================================

/// 物料形态 / Material shape kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Material2Kind {
    /// 卷材：幅宽固定，长度待定 / Coil: fixed width, length to be determined
    Coil,
    /// 定尺板材：宽高固定 / Fixed-size sheet: both width and height fixed
    Sheet,
}

// ============================================================================
// Material2 - 物料 / Material
// ============================================================================

/// 物料 / Material
#[derive(Debug, Clone)]
pub struct Material2<V, U: UnitTrait> {
    /// 物料编号 / Material identifier
    pub id: MaterialId2,
    /// 物料名称 / Material name
    pub name: String,
    /// 幅宽（卷材）或板宽（定尺）/ Coil width or sheet width
    pub width: Quantity<V, U>,
    /// 板高；卷材为 `None` / Sheet height; `None` for a coil
    pub height: Option<Quantity<V, U>>,
    /// 卷长；`None` 表示待推导 / Coil length; `None` means to be derived
    pub length: Option<Quantity<V, U>>,
    /// 单位重量 / Unit weight
    pub unit_weight: Option<Quantity<V, U>>,
    /// 所属设备 / Owning machine
    pub machine_id: Option<MachineId2>,
    /// 可用卷数/张数 / Available coil or sheet count
    pub available_batches: u64,
    /// 最小可用余料面积，低于该值视为废料 / Minimum usable remnant area; below it counts as scrap
    pub min_remnant_area: Option<Quantity<V, ospf_rust_quantities::unit::Unit>>,
}

impl<V, U> Material2<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    /// 创建定尺板材 / Create a fixed-size sheet
    pub fn sheet(
        id: MaterialId2,
        name: impl Into<String>,
        width: Quantity<V, U>,
        height: Quantity<V, U>,
        available_batches: u64,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            width,
            height: Some(height),
            length: None,
            unit_weight: None,
            machine_id: None,
            available_batches,
            min_remnant_area: None,
        }
    }

    /// 创建卷材 / Create a coil
    pub fn coil(
        id: MaterialId2,
        name: impl Into<String>,
        width: Quantity<V, U>,
        available_batches: u64,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            width,
            height: None,
            length: None,
            unit_weight: None,
            machine_id: None,
            available_batches,
            min_remnant_area: None,
        }
    }

    /// 设置卷长 / Set the coil length
    pub fn with_length(mut self, length: Quantity<V, U>) -> Self {
        self.length = Some(length);
        self
    }

    /// 设置所属设备 / Set the owning machine
    pub fn with_machine(mut self, machine_id: MachineId2) -> Self {
        self.machine_id = Some(machine_id);
        self
    }

    /// 设置最小可用余料面积 / Set the minimum usable remnant area
    pub fn with_min_remnant_area(
        mut self,
        area: Quantity<V, ospf_rust_quantities::unit::Unit>,
    ) -> Self {
        self.min_remnant_area = Some(area);
        self
    }

    /// 物料形态 / Material kind
    pub fn kind(&self) -> Material2Kind {
        if self.height.is_none() {
            Material2Kind::Coil
        } else {
            Material2Kind::Sheet
        }
    }

    /// 是否为卷材 / Whether this is a coil
    pub fn is_coil(&self) -> bool {
        self.kind() == Material2Kind::Coil
    }

    /// 定尺板材的面积；卷材返回 `None` / The area of a fixed-size sheet, or `None` for a coil
    pub fn sheet_area(&self) -> Option<Quantity<V, ospf_rust_quantities::unit::Unit>> {
        let height = self.height.as_ref()?;
        Some(ospf_rust_framework_geometry2d::quantity_multiply(
            &self.width,
            height,
        ))
    }

    /// 构造一块料域 / Build a region of material
    ///
    /// 定尺模式用整块板材；卷材模式用「幅宽 × 给定段高」，段高就是这一段消耗的卷长。
    /// 卷材未给段高时返回 `None`，不构造非法区域。
    /// In sheet mode this is the whole sheet; in coil mode it is "width by the given segment height", where the
    /// segment height is the coil length consumed by that segment. A coil without a segment height returns
    /// `None` rather than an illegal region.
    pub fn region(&self, segment_height: Option<Quantity<V, U>>) -> Option<Region2<V, U>> {
        let actual_height = match (&self.height, segment_height) {
            (Some(height), _) => height.clone(),
            (None, Some(segment_height)) => segment_height,
            (None, None) => return None,
        };
        Some(region2_of(
            ospf_rust_framework_geometry2d::quantity_zero_of(&self.width),
            osp_rust_zero(&self.width),
            self.width.clone(),
            actual_height,
        ))
    }

    /// 校验物料：尺寸为正，且卷材不得声明板高 / Validate the material: positive sizes, and a coil must not
    /// declare a sheet height
    pub fn validate(&self) -> Ret<()> {
        if !ospf_rust_framework_geometry2d::quantity_is_positive(&self.width) {
            return Err(validation_error(format!(
                "物料 {} 的幅宽必须为正 / Material {} width must be positive",
                self.id, self.id
            )));
        }
        if let Some(height) = &self.height
            && !ospf_rust_framework_geometry2d::quantity_is_positive(height)
        {
            return Err(validation_error(format!(
                "物料 {} 的板高必须为正 / Material {} height must be positive",
                self.id, self.id
            )));
        }
        if self.available_batches == 0 {
            return Err(validation_error(format!(
                "物料 {} 的可用数量必须为正 / Material {} available batch count must be positive",
                self.id, self.id
            )));
        }
        Ok(())
    }
}

// ============================================================================
// Machine2 - 设备 / Machine
// ============================================================================

/// 设备 / Machine
#[derive(Debug, Clone)]
pub struct Machine2<V> {
    /// 设备编号 / Machine identifier
    pub id: MachineId2,
    /// 设备名称 / Machine name
    pub name: String,
    /// 产能：单位时间可加工的面积 / Capacity: area processable per unit time
    pub capacity: Option<Quantity<V, ospf_rust_quantities::unit::Unit>>,
    /// 裁切工艺 / Cutting process
    pub process: crate::infrastructure::cutting_process::CuttingProcess2,
}

impl<V> Machine2<V> {
    /// 创建设备 / Create a machine
    pub fn new(
        id: MachineId2,
        name: impl Into<String>,
        process: crate::infrastructure::cutting_process::CuttingProcess2,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            capacity: None,
            process,
        }
    }

    /// 设置产能 / Set the capacity
    pub fn with_capacity(
        mut self,
        capacity: Quantity<V, ospf_rust_quantities::unit::Unit>,
    ) -> Self {
        self.capacity = Some(capacity);
        self
    }
}

// ============================================================================
// Costar2 - 副产物 / By-product
// ============================================================================

/// 副产物 / By-product
#[derive(Debug, Clone)]
pub struct Costar2<V, U: UnitTrait> {
    /// 副产物编号 / By-product identifier
    pub id: CostarId2,
    /// 副产物名称 / By-product name
    pub name: String,
    /// 宽度 / Width
    pub width: Quantity<V, U>,
    /// 高度 / Height
    pub height: Quantity<V, U>,
    /// 单位重量 / Unit weight
    pub unit_weight: Option<Quantity<V, U>>,
}

impl<V, U> Costar2<V, U>
where
    U: UnitTrait + CTUnit + Clone,
    V: ospf_rust_math::algebra::concept::Field + Clone,
{
    /// 创建副产物 / Create a by-product
    pub fn new(
        id: CostarId2,
        name: impl Into<String>,
        width: Quantity<V, U>,
        height: Quantity<V, U>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            width,
            height,
            unit_weight: None,
        }
    }

    /// 单件面积 / Single-piece area
    pub fn area(&self) -> Quantity<V, ospf_rust_quantities::unit::Unit> {
        ospf_rust_framework_geometry2d::quantity_multiply(&self.width, &self.height)
    }
}

fn osp_rust_zero<V, U>(sample: &Quantity<V, U>) -> Quantity<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone,
    U: UnitTrait + Clone,
{
    Quantity::new(V::zero(), sample.unit.clone())
}
