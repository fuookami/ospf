use num_traits::One;
use ospf_rust_core::error::{ModelError, Result};
use ospf_rust_core::model::LinearModelValue;
use ospf_rust_framework_bpp::infrastructure::PackingValueAdapter;
use ospf_rust_quantities::unit::{Meter, UnitTrait};
use crate::domain::{PackingContext, PackingResult};

/// 渲染单件，数值只在此显式展示边界转为 f64。 / Rendered piece, converted to f64 only at this explicit display boundary.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct RenderItem1d {
    /// 物品身份。 / Item identity.
    pub item_id: String,
    /// 展示单位起点。 / Start in display units.
    pub start: f64,
    /// 展示单位长度。 / Length in display units.
    pub length: f64,
}

/// 渲染箱。 / Rendered bin.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct RenderBin1d {
    /// 实例身份。 / Instance identity.
    pub id: String,
    /// 箱型身份。 / Bin-type identity.
    pub bin_type_id: String,
    /// 展示单位箱长。 / Bin length in display units.
    pub length: f64,
    /// 实际放置。 / Actual placements.
    pub items: Vec<RenderItem1d>,
}

/// 一维渲染契约，不可用作源模型的精度或最优性证据。 / 1D rendering contract, never source-model precision or optimality evidence.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct RenderSchema1d {
    /// 显式展示长度单位。 / Explicit display length unit.
    pub length_unit: String,
    /// 箱体。 / Bins.
    pub bins: Vec<RenderBin1d>,
    /// 展示精度下的净长度利用率。 / Net-length utilization at display precision.
    pub utilization: f64,
}

/// 明确的领域到展示数值适配。 / Explicit domain-to-display numeric adapter.
pub struct PackingRendererAdapter;

impl PackingRendererAdapter {
    /// 先在原域复核完整放置，再显式换单位及转换数值；非有限或舍入后非法几何失败。
    /// Validate complete placements in the native domain before explicit unit/value conversion; nonfinite or invalid rounded geometry fails.
    pub fn to_schema<V, L, W, F>(
        context: &PackingContext<V, L, W>,
        result: &PackingResult<V, L>,
        unit: &L,
        adapter: &dyn PackingValueAdapter<V, L>,
        mut to_f64: F,
    ) -> Result<RenderSchema1d>
    where
        V: LinearModelValue + One,
        L: UnitTrait + Clone,
        W: UnitTrait + Clone,
        F: FnMut(&V) -> Result<f64>,
    {
        let invalid = || ModelError::InvalidConstraint("invalid BPP1D render geometry, units or finite display values".into());
        if unit.dimension_symbol() != Meter.dimension_symbol() { return Err(invalid().into()); }
        let result = context.verify_plans(result.plans.clone())?;
        let mut bins = Vec::new();
        let mut consumed = 0.0;
        let mut packed = 0.0;
        for plan in result.plans {
            let bin = context.problem().bin_types.iter().find(|bin| bin.id == plan.bin_type_id).ok_or_else(invalid)?;
            let length = to_f64(&adapter.quantity(&bin.length, unit)?)?;
            if !length.is_finite() || length <= 0.0 { return Err(invalid().into()); }
            let mut end = 0.0;
            let mut items = Vec::new();
            for placement in plan.placements {
                let start = to_f64(&adapter.quantity(&placement.start, unit)?)?;
                let size = to_f64(&adapter.quantity(&placement.length, unit)?)?;
                let next = start + size;
                if !start.is_finite() || !size.is_finite() || !next.is_finite() || start < end || size <= 0.0 || next <= start || next > length {
                    return Err(invalid().into());
                }
                packed += size;
                items.push(RenderItem1d { item_id: placement.item_id, start, length: size });
                end = next;
            }
            consumed += length;
            if !packed.is_finite() || !consumed.is_finite() { return Err(invalid().into()); }
            bins.push(RenderBin1d { id: plan.bin_id, bin_type_id: plan.bin_type_id, length, items });
        }
        Ok(RenderSchema1d { length_unit: unit.symbol().into(), bins, utilization: if consumed == 0.0 { 0.0 } else { packed / consumed } })
    }
}
