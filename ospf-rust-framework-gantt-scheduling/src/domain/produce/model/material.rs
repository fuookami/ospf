//! 物料类型标记 / Material type markers
//!
//! 定义产出/消耗的物料类型标记。
//! Defines material type markers for production/consumption.

/// 物料 trait / Material trait
///
/// 所有物料的基础接口。
/// Base interface for all materials.
pub trait MaterialTrait: Send + Sync + std::fmt::Debug + 'static {
    /// 物料 ID / Material ID
    fn id(&self) -> &str;
    /// 物料名称 / Material name
    fn name(&self) -> &str;
}

/// 产品 / Product
///
/// 产出侧的成品物料。
/// Finished product on the production side.
#[derive(Debug, Clone)]
pub struct Product {
    /// 产品 ID / Product ID
    pub id: String,
    /// 产品名称 / Product name
    pub name: String,
}

impl Product {
    /// 创建新产品 / Create new product
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self { id: id.into(), name: name.into() }
    }
}

impl MaterialTrait for Product {
    fn id(&self) -> &str { &self.id }
    fn name(&self) -> &str { &self.name }
}

/// 半成品 / Semi-product
///
/// 中间产品物料。
/// Intermediate product material.
#[derive(Debug, Clone)]
pub struct SemiProduct {
    /// 半成品 ID / Semi-product ID
    pub id: String,
    /// 半成品名称 / Semi-product name
    pub name: String,
}

impl SemiProduct {
    /// 创建新半成品 / Create new semi-product
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self { id: id.into(), name: name.into() }
    }
}

impl MaterialTrait for SemiProduct {
    fn id(&self) -> &str { &self.id }
    fn name(&self) -> &str { &self.name }
}

/// 原材料 / Raw material
///
/// 消耗侧的原料。
/// Raw material on the consumption side.
#[derive(Debug, Clone)]
pub struct RawMaterial {
    /// 原材料 ID / Raw material ID
    pub id: String,
    /// 原材料名称 / Raw material name
    pub name: String,
}

impl RawMaterial {
    /// 创建新原材料 / Create new raw material
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self { id: id.into(), name: name.into() }
    }
}

impl MaterialTrait for RawMaterial {
    fn id(&self) -> &str { &self.id }
    fn name(&self) -> &str { &self.name }
}
