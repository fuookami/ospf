//! 物料模型 / Material model
//!
//! 与 BPP3D 的 `Material` 对齐：物料以编号、类型、名称与单位重量描述，作为面积或重量需求的载体。
//! Mirrors BPP3D `Material`: a material is described by its number, type, name and unit weight, and carries
//! area or weight demands.

use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;

use super::id::{MaterialNo, material_no};

// ============================================================================
// MaterialType - 物料类型 / Material type
// ============================================================================

/// 物料类型 / Material type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MaterialType {
    /// 原材料 / Raw material
    RawMaterial,
    /// 半成品 / Semi-finished product
    SemiFinishedProduct,
    /// 成品 / Finished product
    FinishedProduct,
}

// ============================================================================
// MaterialKey - 物料键 / Material key
// ============================================================================

/// 物料键：按编号、类型与来源唯一识别物料 / Material key uniquely identifying a material
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MaterialKey {
    /// 物料编号 / Material number
    pub no: MaterialNo,
    /// 物料类型 / Material type
    pub material_type: MaterialType,
    /// 制造商 / Manufacturer
    pub manufacturer: Option<String>,
    /// 供应商 / Supplier
    pub supplier: Option<String>,
}

impl MaterialKey {
    /// 创建物料键 / Create a material key
    pub fn new(no: MaterialNo, material_type: MaterialType) -> Self {
        Self {
            no,
            material_type,
            manufacturer: None,
            supplier: None,
        }
    }

    /// 设置制造商 / Set the manufacturer
    pub fn with_manufacturer(mut self, manufacturer: impl Into<String>) -> Self {
        self.manufacturer = Some(manufacturer.into());
        self
    }

    /// 设置供应商 / Set the supplier
    pub fn with_supplier(mut self, supplier: impl Into<String>) -> Self {
        self.supplier = Some(supplier.into());
        self
    }
}

impl std::fmt::Display for MaterialKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{:?}", self.no, self.material_type)
    }
}

// ============================================================================
// Material - 物料 / Material
// ============================================================================

/// 物料 / Material
#[derive(Debug, Clone)]
pub struct Material<V, U: UnitTrait> {
    /// 物料编号 / Material number
    pub no: MaterialNo,
    /// 物料类型 / Material type
    pub material_type: MaterialType,
    /// 物料名称 / Material name
    pub name: String,
    /// 单件重量 / Per-piece weight
    pub weight: Quantity<V, U>,
    /// 制造商 / Manufacturer
    pub manufacturer: Option<String>,
    /// 供应商 / Supplier
    pub supplier: Option<String>,
    /// 仓库 / Warehouse
    pub warehouse: Option<String>,
}

impl<V, U: UnitTrait> Material<V, U> {
    /// 创建物料 / Create a material
    pub fn new(
        no: MaterialNo,
        material_type: MaterialType,
        name: impl Into<String>,
        weight: Quantity<V, U>,
    ) -> Self {
        Self {
            no,
            material_type,
            name: name.into(),
            weight,
            manufacturer: None,
            supplier: None,
            warehouse: None,
        }
    }

    /// 物料键 / Material key
    pub fn key(&self) -> MaterialKey {
        MaterialKey {
            no: self.no.clone(),
            material_type: self.material_type,
            manufacturer: self.manufacturer.clone(),
            supplier: self.supplier.clone(),
        }
    }
}

// ============================================================================
// 构造器 / Constructors
// ============================================================================

/// 创建原材料 / Create a raw material
pub fn raw_material<V, U: UnitTrait>(
    no: impl Into<String>,
    name: impl Into<String>,
    weight: Quantity<V, U>,
) -> Material<V, U> {
    Material::new(
        material_no(no),
        MaterialType::RawMaterial,
        name,
        weight,
    )
}

/// 创建半成品 / Create a semi-finished product
pub fn semi_finished_material<V, U: UnitTrait>(
    no: impl Into<String>,
    name: impl Into<String>,
    weight: Quantity<V, U>,
) -> Material<V, U> {
    Material::new(
        material_no(no),
        MaterialType::SemiFinishedProduct,
        name,
        weight,
    )
}

/// 创建成品 / Create a finished product
pub fn finished_material<V, U: UnitTrait>(
    no: impl Into<String>,
    name: impl Into<String>,
    weight: Quantity<V, U>,
) -> Material<V, U> {
    Material::new(
        material_no(no),
        MaterialType::FinishedProduct,
        name,
        weight,
    )
}
