//! 物料类型标记 / Material type markers
//!
//! 定义产出/消耗的物料类型标记。
//! Defines material type markers for production/consumption.

use crate::domain::common::{ProductionMaterialId, ProductionMaterialIdTrait};

/// 物料 trait / Material trait
///
/// 所有物料的基础接口。
/// Base interface for all materials.
pub trait MaterialTrait: Send + Sync + std::fmt::Debug + 'static {
    /// 物料 ID 类型 / Material id type
    type Id: ProductionMaterialIdTrait;

    /// 物料 ID / Material ID
    fn id(&self) -> &Self::Id;
    /// 物料名称 / Material name
    fn name(&self) -> &str;
}

/// 产品 / Product
///
/// 产出侧的成品物料。
/// Finished product on the production side.
#[derive(Debug, Clone)]
pub struct Product<I = ProductionMaterialId>
where
    I: ProductionMaterialIdTrait,
{
    /// 产品 ID / Product ID
    pub id: I,
    /// 产品名称 / Product name
    pub name: String,
}

impl<I> Product<I>
where
    I: ProductionMaterialIdTrait,
{
    /// 创建新产品 / Create new product
    pub fn new(id: impl Into<I>, name: impl Into<String>) -> Self {
        Self { id: id.into(), name: name.into() }
    }
}

impl<I> MaterialTrait for Product<I>
where
    I: ProductionMaterialIdTrait,
{
    type Id = I;

    fn id(&self) -> &Self::Id { &self.id }
    fn name(&self) -> &str { &self.name }
}

/// 半成品 / Semi-product
///
/// 中间产品物料。
/// Intermediate product material.
#[derive(Debug, Clone)]
pub struct SemiProduct<I = ProductionMaterialId>
where
    I: ProductionMaterialIdTrait,
{
    /// 半成品 ID / Semi-product ID
    pub id: I,
    /// 半成品名称 / Semi-product name
    pub name: String,
}

impl<I> SemiProduct<I>
where
    I: ProductionMaterialIdTrait,
{
    /// 创建新半成品 / Create new semi-product
    pub fn new(id: impl Into<I>, name: impl Into<String>) -> Self {
        Self { id: id.into(), name: name.into() }
    }
}

impl<I> MaterialTrait for SemiProduct<I>
where
    I: ProductionMaterialIdTrait,
{
    type Id = I;

    fn id(&self) -> &Self::Id { &self.id }
    fn name(&self) -> &str { &self.name }
}

/// 原材料 / Raw material
///
/// 消耗侧的原料。
/// Raw material on the consumption side.
#[derive(Debug, Clone)]
pub struct RawMaterial<I = ProductionMaterialId>
where
    I: ProductionMaterialIdTrait,
{
    /// 原材料 ID / Raw material ID
    pub id: I,
    /// 原材料名称 / Raw material name
    pub name: String,
}

impl<I> RawMaterial<I>
where
    I: ProductionMaterialIdTrait,
{
    /// 创建新原材料 / Create new raw material
    pub fn new(id: impl Into<I>, name: impl Into<String>) -> Self {
        Self { id: id.into(), name: name.into() }
    }
}

impl<I> MaterialTrait for RawMaterial<I>
where
    I: ProductionMaterialIdTrait,
{
    type Id = I;

    fn id(&self) -> &Self::Id { &self.id }
    fn name(&self) -> &str { &self.name }
}
