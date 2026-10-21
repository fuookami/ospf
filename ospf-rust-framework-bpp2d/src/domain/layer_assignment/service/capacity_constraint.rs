//! 板材容量约束 / Sheet capacity constraint
//!
//! 每一列的份数受板材面积与承重上限约束。二维列代表整板排布，份数即该排布使用的板材数，
//! 因此容量检查按「份数 × 单列占用」对板材口径累计。
//! The copies of a column are bounded by the sheet area and load limits. A 2D column is a whole-sheet
//! layout, so its copies equal the number of sheets that layout uses; capacity is therefore accumulated as
//! "copies × per-column usage" against the sheet limit.

use crate::domain::item::model::Layer;

/// 板材容量约束 / Sheet capacity constraint
#[derive(Debug, Clone)]
pub struct SheetCapacityConstraint {
    /// 板材面积上限 / Sheet area limit
    pub sheet_area: f64,
    /// 板材承重上限 / Sheet load limit
    pub max_load: f64,
    /// 每列实际占用面积 / Per-column occupied area
    pub layer_areas: Vec<f64>,
    /// 每列载重 / Per-column load
    pub layer_loads: Vec<f64>,
}

impl SheetCapacityConstraint {
    /// 由层列表与板材上限创建 / Create from layers and sheet limits
    pub fn from_layers<V, U>(
        layers: &[Layer<V, U>],
        sheet_area: f64,
        max_load: f64,
    ) -> Self
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: ospf_rust_quantities::unit::concept::UnitTrait
            + ospf_rust_quantities::unit::physical_unit::CTUnit
            + Clone,
    {
        Self {
            sheet_area,
            max_load,
            layer_areas: layers
                .iter()
                .map(|layer| layer.area().value.to_f64().unwrap_or(0.0))
                .collect(),
            layer_loads: vec![0.0; layers.len()],
        }
    }

    /// 设置每列载重 / Set the per-column load
    pub fn with_layer_loads(mut self, loads: Vec<f64>) -> Self {
        self.layer_loads = loads;
        self
    }

    /// 校验单列是否满足容量 / Validate that one column satisfies the capacity
    ///
    /// 面积超限或载重超限都返回错误描述；面积上限非正时视为无上限（不约束）。
    /// Returns a description when the area or the load exceeds the limit. A non-positive area limit is treated
    /// as unbounded (no constraint).
    pub fn validate_column(&self, index: usize) -> Result<(), String> {
        let area = self.layer_areas.get(index).copied().unwrap_or(0.0);
        if self.sheet_area > 0.0 && area > self.sheet_area + 1e-9 {
            return Err(format!(
                "列的占用面积 {area} 超过板材面积 {} / Column area {area} exceeds the sheet area {}",
                self.sheet_area, self.sheet_area
            ));
        }
        let load = self.layer_loads.get(index).copied().unwrap_or(0.0);
        if self.max_load > 0.0 && load > self.max_load + 1e-9 {
            return Err(format!(
                "列的载重 {load} 超过板材承重 {} / Column load {load} exceeds the sheet load limit {}",
                self.max_load, self.max_load
            ));
        }
        Ok(())
    }

    /// 校验全部列 / Validate every column
    pub fn validate(&self) -> Result<(), String> {
        for index in 0..self.layer_areas.len() {
            self.validate_column(index)?;
        }
        Ok(())
    }
}

/// 由份数推导实际使用的板材数 / Derive the number of sheets actually used from the copies
///
/// 份数向上取整后求和；负份数按 0 处理。
/// Copies are rounded up and summed; negative copies count as zero.
pub fn sheets_used_by_copies(copies: &[f64]) -> u64 {
    copies
        .iter()
        .map(|value| value.max(0.0).ceil() as u64)
        .sum()
}
