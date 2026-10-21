//! 层分配目标 / Layer assignment objectives
//!
//! 二维排样以**余料面积最少**为主目标，板数与板材类型成本作为次目标。三者都是 `Σ 系数_j · y_j`
//! 形式的线性项，通过显式权重相加成一个目标——core 的多目标机制会把同类目（`Minimum`）的系数逐项
//! 相加，因此权重必须由调用方自己给。
//! 2D packing targets **minimum waste area** as the primary objective, with sheet count and per-type cost as
//! secondary terms. All three are linear terms of the form `Σ coefficient_j · y_j` combined through explicit
//! weights: the core multi-objective mechanism sums the coefficients of sub-objectives with the same category,
//! so weights must be supplied by the caller.
//!
//! # 为什么主目标是板材面积消耗而不是逐列余料 / Why the primary objective is sheet area, not per-column waste
//!
//! 需求占用的面积由需求本身固定，因此
//! `总余料 = Σ 板材面积 − 需求占用面积 = Σ 板材面积 − 常数`，最小化板材面积就是最小化余料。
//! 不能直接对「`sheet_area_j − layer_area_j`」求和：列代表整板排布，铺满的整板余料记为 0，
//! 会让「一块铺满的大板」显得比「多块小板」更优，与真实裁剪后的余料相反。
//! The area occupied by demand is fixed by the demand itself, so
//! `total waste = Σ sheet area − demanded area = Σ sheet area − constant`, hence minimising sheet area
//! minimises waste. Summing `sheet_area_j − layer_area_j` instead would be wrong: a column is a whole-sheet
//! layout and a fully packed sheet would report zero waste, making "one full large sheet" look better than
//! "several small sheets", the opposite of the post-trim reality.

use std::collections::BTreeMap;

use crate::domain::item::model::{Layer, SheetTypeId};

// ============================================================================
// 余料统计 / Waste statistics
// ============================================================================

/// 计算单列的余料面积 / Compute the waste area of one column
///
/// 余料 = 板材面积 − 该列实际占用面积；列未绑定板材类型时退化为 0，表示无余料可言。
/// Waste is the sheet area minus the actually occupied area of the column. A column without a sheet type
/// yields zero, meaning no waste can be attributed.
pub fn layer_waste_area<V, U>(layer: &Layer<V, U>) -> f64
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: ospf_rust_quantities::unit::concept::UnitTrait
        + ospf_rust_quantities::unit::physical_unit::CTUnit
        + Clone,
{
    let Some(sheet) = layer.sheet.as_ref() else {
        return 0.0;
    };
    let sheet_area = sheet.area().value.to_f64().unwrap_or(0.0);
    let occupied = layer.area().value.to_f64().unwrap_or(0.0);
    if sheet_area > occupied {
        sheet_area - occupied
    } else {
        0.0
    }
}

/// 计算单列的归一化余料率 / Compute the normalised waste ratio of one column
///
/// 余料率 = 余料面积 / 板材面积，落在 `[0, 1]`，与板材尺寸无关，便于跨类型比较。
/// The waste ratio is the waste area over the sheet area, within `[0, 1]`, independent of sheet size, which
/// makes it comparable across sheet types.
pub fn layer_waste_ratio<V, U>(layer: &Layer<V, U>) -> f64
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: ospf_rust_quantities::unit::concept::UnitTrait
        + ospf_rust_quantities::unit::physical_unit::CTUnit
        + Clone,
{
    let Some(sheet) = layer.sheet.as_ref() else {
        return 0.0;
    };
    let sheet_area = sheet.area().value.to_f64().unwrap_or(0.0);
    if sheet_area <= 0.0 {
        return 0.0;
    }
    layer_waste_area(layer) / sheet_area
}

// ============================================================================
// 目标项 / Objective terms
// ============================================================================

/// 目标项：变量索引与系数 / Objective term: variable index and coefficient
pub type ObjectiveTerm = (usize, f64);

/// 板材面积消耗目标项（主目标，等价于余料最少）/ Sheet area consumption terms (primary, minimum waste)
///
/// `min weight · Σ_j sheet_area_j · y_j`。列未绑定板材类型时该列不计入。
/// `min weight · Σ_j sheet_area_j · y_j`. Columns without a sheet type are not counted.
pub fn sheet_area_terms<V, U>(layers: &[Layer<V, U>], weight: f64) -> Vec<ObjectiveTerm>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: ospf_rust_quantities::unit::concept::UnitTrait
        + ospf_rust_quantities::unit::physical_unit::CTUnit
        + Clone,
{
    layers
        .iter()
        .enumerate()
        .filter_map(|(index, layer)| {
            let sheet = layer.sheet.as_ref()?;
            let coefficient = weight * sheet.area().value.to_f64()?;
            if coefficient == 0.0 {
                None
            } else {
                Some((index, coefficient))
            }
        })
        .collect()
}

/// 板材数量目标项（次目标）/ Sheet amount terms (secondary)
///
/// `min weight · Σ_j y_j`。默认权重应极小，只在面积相同的解之间打破平局。
/// `min weight · Σ_j y_j`. The default weight should be tiny so it only breaks ties between equal-area
/// solutions.
pub fn sheet_amount_terms<V, U>(
    layers: &[Layer<V, U>],
    weight: f64,
) -> Vec<ObjectiveTerm>
where
    V: Clone,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Clone,
{
    if weight == 0.0 {
        return Vec::new();
    }
    layers
        .iter()
        .enumerate()
        .map(|(index, _)| (index, weight))
        .collect()
}

/// 板材类型成本目标项（次目标）/ Per-sheet-type cost terms (secondary)
///
/// `min weight · Σ_j type_cost(sheet_j) · y_j`。未配置成本的类型按 0 处理且零系数项被跳过。
/// `min weight · Σ_j type_cost(sheet_j) · y_j`. Types without a configured cost count as zero and zero
/// coefficients are skipped.
pub fn sheet_type_cost_terms<V, U>(
    layers: &[Layer<V, U>],
    type_cost: &BTreeMap<SheetTypeId, f64>,
    weight: f64,
) -> Vec<ObjectiveTerm>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: ospf_rust_quantities::unit::concept::UnitTrait
        + ospf_rust_quantities::unit::physical_unit::CTUnit
        + Clone,
{
    if weight == 0.0 || type_cost.is_empty() {
        return Vec::new();
    }
    layers
        .iter()
        .enumerate()
        .filter_map(|(index, layer)| {
            let sheet = layer.sheet.as_ref()?;
            let cost = type_cost.get(&sheet.type_code).copied().unwrap_or(0.0);
            let coefficient = weight * cost;
            if coefficient == 0.0 {
                None
            } else {
                Some((index, coefficient))
            }
        })
        .collect()
}

/// 目标配置 / Objective configuration
///
/// 默认值与「余料为主、板数与类型成本为次」一致。
/// Defaults match "waste primary, sheet count and type cost secondary".
#[derive(Debug, Clone)]
pub struct LayerAssignmentObjectiveConfig {
    /// 板材面积消耗权重（主目标）/ Sheet area consumption weight (primary)
    pub sheet_area_weight: f64,
    /// 板材数量权重 / Sheet amount weight
    pub sheet_amount_weight: f64,
    /// 板材类型成本权重 / Sheet type cost weight
    pub sheet_type_cost_weight: f64,
    /// 板材类型到成本的映射 / Sheet type to cost mapping
    pub sheet_type_costs: BTreeMap<SheetTypeId, f64>,
    /// RMP 面积系数，必须与检验数计算一致 / RMP area coefficient, must match the reduced-cost computation
    pub rmp_area_coefficient: f64,
}

impl Default for LayerAssignmentObjectiveConfig {
    fn default() -> Self {
        Self {
            sheet_area_weight: 1.0,
            // 余料为主目标；板数权重保持极小，仅用于在「余料相同」的解之间打破平局，
            // 避免多选零余料的整板。
            // Waste is primary; the sheet-count weight stays tiny and only breaks ties between equal-waste
            // solutions, so the model does not add perfectly filled sheets for free.
            sheet_amount_weight: 1e-6,
            sheet_type_cost_weight: 0.0,
            sheet_type_costs: BTreeMap::new(),
            rmp_area_coefficient: 1.0,
        }
    }
}

impl LayerAssignmentObjectiveConfig {
    /// 汇总全部目标项 / Aggregate every objective term
    ///
    /// 同类别（`Minimum`）子目标会被 core 逐项相加，因此这里直接返回一个合并后的项列表。
    /// Sub-objectives of the same category are summed term by term by the core, so one merged term list is
    /// returned.
    pub fn all_terms<V, U>(&self, layers: &[Layer<V, U>]) -> Vec<ObjectiveTerm>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: ospf_rust_quantities::unit::concept::UnitTrait
            + ospf_rust_quantities::unit::physical_unit::CTUnit
            + Clone,
    {
        let mut terms = sheet_area_terms(layers, self.sheet_area_weight);
        terms.extend(sheet_amount_terms(layers, self.sheet_amount_weight));
        terms.extend(sheet_type_cost_terms(
            layers,
            &self.sheet_type_costs,
            self.sheet_type_cost_weight,
        ));
        terms
    }
}
