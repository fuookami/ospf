//! 裁切图案与裁切方案 / Cutting patterns and cutting plans
//!
//! 图案是「一块料上的排布」，方案是「某个图案的使用记录」。二维方案按**图案使用次数**累加，
//! 覆盖量来自图案里的件数，消耗量来自图案料域的面积或段高——这是与一维「切片列表」的关键差异。
//! A pattern is "a layout on one piece of material" and a plan records its usage. The 2D master problem
//! accumulates **pattern usage counts**: coverage comes from the piece counts inside the pattern and
//! consumption from the region area or segment height. This is the key difference from the 1D "slice list".

use std::collections::BTreeMap;

use ospf_rust_base::error::Ret;
use ospf_rust_framework_geometry2d::{
    Placement2, quantity_greater, quantity_is_positive, quantity_sum, quantity_zero_of,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::infrastructure::cut_tree::CutTree2;
use crate::infrastructure::cutting_process::CuttingProcess2;
use crate::infrastructure::error::validation_error;
use crate::infrastructure::region::Region2;

use super::ids::{PatternId2, PlanId2, ProductId2};
use super::material::Material2;

// ============================================================================
// Pattern2 - 裁切图案 / Cutting pattern
// ============================================================================

/// 裁切图案 / Cutting pattern
#[derive(Debug, Clone)]
pub struct Pattern2<V, U: UnitTrait> {
    /// 图案编号 / Pattern identifier
    pub id: PatternId2,
    /// 所属物料 / Owning material
    pub material: Material2<V, U>,
    /// 该图案占用的料域 / The region occupied by this pattern
    pub region: Region2<V, U>,
    /// 成品件放置 / Finished part placements
    pub placements: Vec<Placement2<V, U>>,
    /// 副产物放置 / By-product placements
    pub costar_placements: Vec<Placement2<V, U>>,
    /// 断头台切分树；任意排样为 `None` / Guillotine cut tree; `None` for arbitrary nesting
    pub cut_tree: Option<CutTree2<V, U>>,
    /// 裁切工艺 / Cutting process
    pub process: CuttingProcess2,
}

impl<V, U> Pattern2<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    /// 创建图案 / Create a pattern
    pub fn new(
        id: PatternId2,
        material: Material2<V, U>,
        region: Region2<V, U>,
        placements: Vec<Placement2<V, U>>,
        process: CuttingProcess2,
    ) -> Self {
        Self {
            id,
            material,
            region,
            placements,
            costar_placements: Vec::new(),
            cut_tree: None,
            process,
        }
    }

    /// 设置副产物放置 / Set the by-product placements
    pub fn with_costars(mut self, costars: Vec<Placement2<V, U>>) -> Self {
        self.costar_placements = costars;
        self
    }

    /// 设置切分树 / Set the cut tree
    pub fn with_cut_tree(mut self, cut_tree: CutTree2<V, U>) -> Self {
        self.cut_tree = Some(cut_tree);
        self
    }

    /// 成品件占用面积 / Area occupied by finished parts
    pub fn used_area(&self) -> Quantity<V, ospf_rust_quantities::unit::Unit> {
        let areas: Vec<_> = self.placements.iter().map(|placement| placement.area()).collect();
        quantity_sum(areas).unwrap_or_else(|| quantity_zero_of(&self.region.width).to_runtime())
    }

    /// 副产物占用面积 / Area occupied by by-products
    pub fn costar_area(&self) -> Quantity<V, ospf_rust_quantities::unit::Unit> {
        let areas: Vec<_> = self
            .costar_placements
            .iter()
            .map(|placement| placement.area())
            .collect();
        quantity_sum(areas).unwrap_or_else(|| quantity_zero_of(&self.region.width).to_runtime())
    }

    /// 余料面积 = 料域面积 − 成品面积 − 副产物面积，下限为零
    /// / Remnant area: region area minus finished area minus by-product area, floored at zero
    pub fn waste_area(&self) -> Quantity<V, ospf_rust_quantities::unit::Unit> {
        let region_area = self.region.area();
        let occupied = self.used_area().value + self.costar_area().value;
        if quantity_greater(&region_area, &Quantity::new(occupied, region_area.unit.clone())) {
            Quantity::new(region_area.value - occupied, region_area.unit)
        } else {
            quantity_zero_of(&self.region.width).to_runtime()
        }
    }

    /// 成品件件数映射，按产品编号 / Finished part counts indexed by product identifier
    pub fn product_amounts(&self) -> BTreeMap<ProductId2, u64> {
        let mut result: BTreeMap<ProductId2, u64> = BTreeMap::new();
        for placement in &self.placements {
            let Some(product_id) = &placement.item_id else {
                continue;
            };
            *result.entry(ProductId2::new(product_id.clone())).or_insert(0) += 1;
        }
        result
    }

    /// 利用率：成品面积与料域面积之比 / Utilisation: finished area over region area
    ///
    /// 副产物不计入利用率——副产物不是需求驱动的产出。
    /// By-products are excluded from utilisation because they are not demand-driven output.
    pub fn utilization(&self) -> f64 {
        let region_area = self.region.area().value.to_f64().unwrap_or(0.0);
        if region_area <= 0.0 {
            return 0.0;
        }
        self.used_area().value.to_f64().unwrap_or(0.0) / region_area
    }

    /// 校验图案：料域非退化、物料合法、工艺与切分树一致、存在放置
    /// / Validate the pattern: non-degenerate region, legal material, process consistent with the cut tree, and
    /// at least one placement
    pub fn validate(&self, _kerf: &Quantity<V, U>) -> Ret<()> {
        if self.region.is_degenerate() {
            return Err(validation_error(format!(
                "图案 {} 的料域退化 / Pattern {} region is degenerate",
                self.id, self.id
            )));
        }
        self.material.validate()?;
        if self.process.requires_cut_tree() && self.cut_tree.is_none() {
            return Err(validation_error(format!(
                "图案 {} 使用断头台工艺 {:?} 但未提供切分树 / Pattern {} uses a guillotine process without a cut tree",
                self.id, self.process, self.id
            )));
        }
        if self.placements.is_empty() && self.costar_placements.is_empty() {
            return Err(validation_error(format!(
                "图案 {} 没有任何放置 / Pattern {} has no placement",
                self.id, self.id
            )));
        }
        if let Some(cut_tree) = self.cut_tree.as_ref() {
            cut_tree.validate(self.process)?;
        }
        Ok(())
    }

    /// 图案身份键 / Pattern identity key
    ///
    /// 物料、料域尺寸、工艺与放置集合共同参与，避免不同排布被误判为同一图案。
    /// The material, region size, process and placement set all take part so different layouts are never
    /// mistaken for the same pattern.
    pub fn identity_key(&self) -> String {
        let mut placements: Vec<String> = self
            .placements
            .iter()
            .map(|placement| {
                format!(
                    "{:?}|{:?}|{}|{:?}",
                    placement.position.x.value,
                    placement.position.y.value,
                    placement.rotated,
                    placement.item_id
                )
            })
            .collect();
        placements.sort();
        format!(
            "{}|{:?}x{:?}|{:?}|{}",
            self.material.id,
            self.region.width.value,
            self.region.height.value,
            self.process,
            placements.join(",")
        )
    }
}

// ============================================================================
// CuttingPlan2 - 裁切方案 / Cutting plan
// ============================================================================

/// 裁切方案 / Cutting plan
///
/// 一维方案是「切片列表」，二维方案是「一块料上的图案」。方案携带图案的完整信息，
/// 使主问题只需按使用次数累加。
/// A 1D plan is a slice list while a 2D plan is a pattern on one piece of material. The plan carries the whole
/// pattern so the master problem only has to accumulate usage counts.
#[derive(Debug, Clone)]
pub struct CuttingPlan2<V, U: UnitTrait> {
    /// 方案编号 / Plan identifier
    pub id: PlanId2,
    /// 图案 / Pattern
    pub pattern: Pattern2<V, U>,
}

impl<V, U> CuttingPlan2<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    /// 创建方案 / Create a plan
    pub fn new(id: PlanId2, pattern: Pattern2<V, U>) -> Self {
        Self { id, pattern }
    }

    /// 由图案自动生成方案编号 / Derive a plan identifier from the pattern
    pub fn from_pattern(pattern: Pattern2<V, U>) -> Self {
        let id = PlanId2::new(format!("plan-{}", pattern.id));
        Self { id, pattern }
    }

    /// 消耗料域面积 / Consumed region area
    pub fn consumed_area(&self) -> Quantity<V, ospf_rust_quantities::unit::Unit> {
        self.pattern.region.area()
    }

    /// 卷材模式下消耗的段高 / The consumed segment height in coil mode
    pub fn consumed_height(&self) -> Option<Quantity<V, U>> {
        if self.pattern.material.is_coil() {
            Some(self.pattern.region.height.clone())
        } else {
            None
        }
    }

    /// 余料面积 / Remnant area
    pub fn waste_area(&self) -> Quantity<V, ospf_rust_quantities::unit::Unit> {
        self.pattern.waste_area()
    }

    /// 成品件件数映射 / Finished part counts
    pub fn product_amounts(&self) -> BTreeMap<ProductId2, u64> {
        self.pattern.product_amounts()
    }

    /// 方案身份键 / Plan identity key
    pub fn identity_key(&self) -> String {
        self.pattern.identity_key()
    }

    /// 校验方案 / Validate the plan
    pub fn validate(&self, kerf: &Quantity<V, U>) -> Ret<()> {
        self.pattern.validate(kerf)
    }
}

// ============================================================================
// 余料阈值 / Remnant threshold
// ============================================================================

/// 判断余料是否达到可用阈值 / Whether a remnant reaches the usable threshold
///
/// 阈值为 `None` 时任何正面积余料都可用。
/// With no threshold, any positive remnant counts as usable.
pub fn remnant_reaches<V, U>(
    area: &Quantity<V, ospf_rust_quantities::unit::Unit>,
    threshold: Option<&Quantity<V, ospf_rust_quantities::unit::Unit>>,
) -> bool
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + Clone,
{
    let value = area.value.to_f64().unwrap_or(0.0);
    match threshold {
        Some(threshold) => {
            let limit = threshold.value.to_f64().unwrap_or(0.0);
            value > 0.0 && value >= limit
        }
        None => value > 0.0,
    }
}

/// 判断图案是否放得进料域 / Whether a pattern fits inside a region
pub fn fits_in<V, U>(pattern: &Pattern2<V, U>, region: &Region2<V, U>) -> bool
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    region.contains(&pattern.region) && quantity_is_positive(&pattern.region.width)
}
