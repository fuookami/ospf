//! 产品与需求 / Products and demands
//!
//! 产品是二维套裁的需求载体：宽高固定、可旋转与否、单位重量可选。需求量值有两种口径：
//! 按件数与按面积，与 Kotlin `DemandMode2` 对齐。
//! A product carries the demand of 2D cutting stock: fixed width and height, an optional rotation allowance
//! and an optional unit weight. Demand has two conventions — piece count and area — matching Kotlin
//! `DemandMode2`.

use ospf_rust_base::error::Ret;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::infrastructure::error::validation_error;

use super::ids::ProductId2;

// ============================================================================
// DemandMode2 - 需求口径 / Demand mode
// ============================================================================

/// 需求口径 / Demand mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DemandMode2 {
    /// 按件数 / By piece count
    Amount,
    /// 按面积 / By area
    Area,
}

// ============================================================================
// DemandValue2 - 需求量值 / Demand value
// ============================================================================

/// 需求量值 / Demand value
#[derive(Debug, Clone, PartialEq)]
pub enum DemandValue2 {
    /// 件数 / Piece count
    Amount(u64),
    /// 面积 / Area
    Area(f64),
}

impl DemandValue2 {
    /// 需求口径 / Demand mode
    pub fn mode(&self) -> DemandMode2 {
        match self {
            Self::Amount(_) => DemandMode2::Amount,
            Self::Area(_) => DemandMode2::Area,
        }
    }

    /// 求解器数值 / The solver number
    ///
    /// 件数直接转 `f64`，面积已是求解器数值。
    /// A piece count converts directly to `f64`; an area already is a solver number.
    pub fn to_solver(&self) -> f64 {
        match self {
            Self::Amount(value) => *value as f64,
            Self::Area(value) => *value,
        }
    }

    /// 是否为正 / Whether strictly positive
    pub fn is_positive(&self) -> bool {
        self.to_solver() > 0.0
    }
}

// ============================================================================
// Product2 - 产品 / Product
// ============================================================================

/// 产品 / Product
#[derive(Debug, Clone)]
pub struct Product2<V, U: UnitTrait> {
    /// 产品编号 / Product identifier
    pub id: ProductId2,
    /// 产品名称 / Product name
    pub name: String,
    /// 宽度 / Width
    pub width: Quantity<V, U>,
    /// 高度 / Height
    pub height: Quantity<V, U>,
    /// 是否允许旋转 90° / Whether a 90-degree rotation is allowed
    pub allow_rotate: bool,
    /// 单位重量 / Unit weight
    pub unit_weight: Option<Quantity<V, U>>,
}

impl<V, U> Product2<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    /// 创建产品 / Create a product
    pub fn new(
        id: ProductId2,
        name: impl Into<String>,
        width: Quantity<V, U>,
        height: Quantity<V, U>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            width,
            height,
            allow_rotate: true,
            unit_weight: None,
        }
    }

    /// 设置旋转许可 / Set the rotation allowance
    pub fn with_allow_rotate(mut self, allow_rotate: bool) -> Self {
        self.allow_rotate = allow_rotate;
        self
    }

    /// 设置单位重量 / Set the unit weight
    pub fn with_unit_weight(mut self, unit_weight: Quantity<V, U>) -> Self {
        self.unit_weight = Some(unit_weight);
        self
    }

    /// 单件面积 / Single-piece area
    pub fn area(&self) -> Quantity<V, ospf_rust_quantities::unit::Unit> {
        ospf_rust_framework_geometry2d::quantity_multiply(&self.width, &self.height)
    }

    /// 校验产品几何 / Validate the product geometry
    pub fn validate(&self) -> Ret<()> {
        if !ospf_rust_framework_geometry2d::quantity_is_positive(&self.width)
            || !ospf_rust_framework_geometry2d::quantity_is_positive(&self.height)
        {
            return Err(validation_error(format!(
                "产品 {} 的宽高必须为正 / Product {} dimensions must be positive",
                self.id, self.id
            )));
        }
        Ok(())
    }

    /// 允许的摆放方向集合 / The set of allowed orientations
    ///
    /// 第一个元素恒为不旋转；允许旋转时追加旋转方向。
    /// The first element is always unrotated; a rotated orientation is appended when rotation is allowed.
    pub fn orientations(&self) -> Vec<bool> {
        if self.allow_rotate {
            vec![false, true]
        } else {
            vec![false]
        }
    }

    /// 按方向给出宽高 / Width and height under an orientation
    pub fn size_of(&self, rotated: bool) -> (Quantity<V, U>, Quantity<V, U>) {
        if rotated {
            (self.height.clone(), self.width.clone())
        } else {
            (self.width.clone(), self.height.clone())
        }
    }
}

// ============================================================================
// ProductDemand2 - 产品需求 / Product demand
// ============================================================================

/// 产品需求 / Product demand
#[derive(Debug, Clone)]
pub struct ProductDemand2<V, U: UnitTrait> {
    /// 产品 / Product
    pub product: Product2<V, U>,
    /// 需求量值 / Demanded quantity
    pub demand: DemandValue2,
}

impl<V, U> ProductDemand2<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    /// 创建产品需求 / Create a product demand
    pub fn new(product: Product2<V, U>, demand: DemandValue2) -> Self {
        Self { product, demand }
    }

    /// 按件数创建 / Create by piece count
    pub fn amount(product: Product2<V, U>, amount: u64) -> Self {
        Self::new(product, DemandValue2::Amount(amount))
    }

    /// 按面积创建 / Create by area
    pub fn area(product: Product2<V, U>, area: f64) -> Self {
        Self::new(product, DemandValue2::Area(area))
    }

    /// 需求口径 / Demand mode
    pub fn mode(&self) -> DemandMode2 {
        self.demand.mode()
    }
}

// ============================================================================
// ProductDemands2 - 需求集合 / Demand set
// ============================================================================

/// 需求集合 / Demand set
#[derive(Debug, Clone, Default)]
pub struct ProductDemands2<V, U: UnitTrait> {
    /// 需求条目 / Demand entries
    pub entries: Vec<ProductDemand2<V, U>>,
}

impl<V, U> ProductDemands2<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    /// 创建空集合 / Create an empty set
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// 追加需求 / Append a demand
    pub fn with(mut self, demand: ProductDemand2<V, U>) -> Self {
        self.entries.push(demand);
        self
    }

    /// 件数上限：所有按件数需求的件数与按面积需求的折算件数中的最大值
    /// / The piece-count cap: the maximum over amount demands and area demands converted to pieces
    pub fn piece_cap(&self) -> u64 {
        self.entries
            .iter()
            .map(|entry| match entry.demand {
                DemandValue2::Amount(amount) => amount,
                DemandValue2::Area(area) => {
                    let unit = entry.product.area().value.to_f64().unwrap_or(0.0);
                    if unit <= 0.0 {
                        0
                    } else {
                        (area / unit).ceil().max(0.0) as u64
                    }
                }
            })
            .max()
            .unwrap_or(0)
    }
}
