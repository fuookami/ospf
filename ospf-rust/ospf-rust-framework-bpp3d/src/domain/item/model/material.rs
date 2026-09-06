// ============================================================================
// Material - 物料 / Material
// ============================================================================

/// 物料类型 / Material type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MaterialType {
    /// 原材料 / Raw material
    RawMaterial,
    /// 半成品 / Semi-finished product
    SemiFinishedProduct,
    /// 成品 / Finished product
    FinishedProduct,
}

/// 物料标识 / Material key
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MaterialKey {
    /// 物料编号 / Material number
    pub no: String,
    /// 物料类型 / Material type
    pub material_type: MaterialType,
    /// 制造商 / Manufacturer
    pub manufacturer: Option<String>,
    /// 供应商 / Supplier
    pub supplier: Option<String>,
}

/// 物料 / Material
#[derive(Debug, Clone)]
pub struct Material<V, U: UnitTrait> {
    /// 编号 / Number
    pub no: String,
    /// 类型 / Type
    pub material_type: MaterialType,
    /// 名称 / Name
    pub name: String,
    /// 制造商 / Manufacturer
    pub manufacturer: Option<String>,
    /// 供应商 / Supplier
    pub supplier: Option<String>,
    /// 仓库 / Warehouse
    pub warehouse: Option<String>,
    /// 重量 / Weight
    pub weight: Quantity<V, U>,
    /// 货物属性键 / Cargo attribute key
    pub cargo: Option<CargoAttributeKey>,
}

impl<V, U: UnitTrait> Material<V, U> {
    /// 获取物料标识 / Get material key
    pub fn key(&self) -> MaterialKey {
        MaterialKey {
            no: self.no.clone(),
            material_type: self.material_type,
            manufacturer: self.manufacturer.clone(),
            supplier: self.supplier.clone(),
        }
    }
}

