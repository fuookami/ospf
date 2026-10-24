//! 列选择诊断 / Column selection diagnostics
//!
//! 从最终 MILP 的份数解回填列选择、面积与余料统计。份数只保留非零项，避免产出零份数列。
//! Fills column selection, area and waste statistics back from the final MILP copy solution. Only
//! non-zero copies are kept so no zero-copy column is reported.

use crate::domain::item::model::Layer;
use crate::domain::layer_assignment::model::layer_waste_area;

/// 列选择 / Column selection
#[derive(Debug, Clone)]
pub struct ColumnSelection<V, U: ospf_rust_quantities::unit::concept::UnitTrait> {
    /// 被选中的层 / Selected layers
    pub layers: Vec<Layer<V, U>>,
    /// 对应的份数 / Corresponding copy counts
    pub copies: Vec<u64>,
}

impl<V, U: ospf_rust_quantities::unit::concept::UnitTrait> ColumnSelection<V, U> {
    /// 是否为空 / Whether empty
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    /// 总板材数 / Total sheet count
    pub fn sheet_count(&self) -> u64 {
        self.copies.iter().sum()
    }
}

/// 由份数解选出非零列 / Select the non-zero columns from a copy solution
///
/// 份数按四舍五入取整；取整后仍为零的列被丢弃，因此结果不会包含零份数列。
/// Copies are rounded to the nearest integer; columns that round to zero are dropped, so the result never
/// contains a zero-copy column.
pub fn select_columns_by_copies<V, U>(
    layers: &[Layer<V, U>],
    copies: &[f64],
) -> ColumnSelection<V, U>
where
    V: Clone,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Clone,
{
    let mut selected = Vec::new();
    let mut amounts = Vec::new();
    for (layer, copies) in layers.iter().zip(copies.iter()) {
        if *copies <= 0.0 {
            continue;
        }
        let amount = copies.round() as u64;
        if amount == 0 {
            continue;
        }
        selected.push(layer.clone());
        amounts.push(amount);
    }
    ColumnSelection {
        layers: selected,
        copies: amounts,
    }
}

/// 累计板材面积使用 / Accumulate the used sheet area
///
/// 板材面积按「板材类型面积 × 份数」累计；列未绑定板材类型时跳过。
/// Sheet area accumulates as "sheet type area × copies"; columns without a sheet type are skipped.
pub fn total_area_used<V, U>(selection: &ColumnSelection<V, U>) -> f64
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: ospf_rust_quantities::unit::concept::UnitTrait
        + ospf_rust_quantities::unit::physical_unit::CTUnit
        + Clone,
{
    selection
        .layers
        .iter()
        .zip(selection.copies.iter())
        .filter_map(|(layer, copies)| {
            let sheet = layer.sheet.as_ref()?;
            Some(sheet.area().value.to_f64()? * *copies as f64)
        })
        .sum()
}

/// 累计余料面积 / Accumulate the waste area
///
/// 余料按「单列余料 × 份数」累计，与 KPI 的余料口径一致。
/// Waste accumulates as "per-column waste × copies", matching the KPI waste convention.
pub fn total_waste_area<V, U>(selection: &ColumnSelection<V, U>) -> f64
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: ospf_rust_quantities::unit::concept::UnitTrait
        + ospf_rust_quantities::unit::physical_unit::CTUnit
        + Clone,
{
    selection
        .layers
        .iter()
        .zip(selection.copies.iter())
        .map(|(layer, copies)| layer_waste_area(layer) * *copies as f64)
        .sum()
}
