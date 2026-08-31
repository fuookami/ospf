//! 浪费最小化领域 / Waste minimization domain

use std::collections::BTreeMap;

use ospf_rust_core::solver::SolveValue;

pub mod model;

#[allow(unused_imports)]
pub use model::*;

/// 余宽面积衡量 / Over-production area measure
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverProductionAreaMeasure {
    /// 产品最大幅宽代理 / Product max-width proxy
    ProductMaxWidthProxy,
}

/// 余料衡量 / Rest material measure
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestMaterialMeasure {
    /// 余宽乘物料长度代理 / Rest width by material length proxy
    RestWidthByMaterialLengthProxy,
}

/// Waste 分析 / Waste analysis
#[derive(Debug, Clone, Default)]
pub struct WasteAnalysis<V: SolveValue> {
    pub metrics: BTreeMap<String, V>,
}

/// Waste 配置 / Waste minimization config
#[derive(Debug, Clone)]
pub struct WasteMinimizationConfig<V: SolveValue> {
    /// 余宽惩罚权重 / Trim width penalty
    pub trim_width_penalty: Option<V>,
    /// 物料成本惩罚 / Material cost penalty
    pub material_cost_penalty: BTreeMap<String, V>,
    /// 超产面积惩罚 / Over-production area penalty
    pub over_production_area_penalty: Option<V>,
    /// 余料惩罚 / Rest material penalty
    pub rest_material_penalty: Option<V>,
    /// 超产面积度量 / Over-production area measure
    pub over_production_area_measure: OverProductionAreaMeasure,
    /// 余料度量 / Rest material measure
    pub rest_material_measure: RestMaterialMeasure,
}

impl<V: SolveValue> Default for WasteMinimizationConfig<V> {
    fn default() -> Self {
        Self {
            trim_width_penalty: None,
            material_cost_penalty: BTreeMap::new(),
            over_production_area_penalty: None,
            rest_material_penalty: None,
            over_production_area_measure: OverProductionAreaMeasure::ProductMaxWidthProxy,
            rest_material_measure: RestMaterialMeasure::RestWidthByMaterialLengthProxy,
        }
    }
}

/// Waste 结果 / Waste minimization result
#[derive(Debug, Clone)]
pub struct WasteMinimizationResult<V: SolveValue> {
    /// 总余宽 / Total trim width
    pub total_trim_width: Option<V>,
    /// 物料成本 / Material costs
    pub material_costs: Vec<ModeledMaterialCost<V>>,
    /// 超产面积代理 / Over-production area proxy
    pub over_production_area: Option<V>,
    /// 总余料代理 / Total rest material proxy
    pub total_rest_material: Option<V>,
    /// 超产面积度量 / Over-production area measure
    pub over_production_area_measure: OverProductionAreaMeasure,
    /// 余料度量 / Rest material measure
    pub rest_material_measure: RestMaterialMeasure,
    /// 分析指标 / Analysis metrics
    pub analysis: WasteAnalysis<V>,
}

/// Waste 聚合 / Waste aggregation
#[derive(Debug, Clone, Default)]
pub struct WasteAggregation<V: SolveValue> {
    pub analysis: Option<WasteAnalysis<V>>,
}

/// 余宽浪费 / Rest width waste
#[derive(Debug, Clone)]
pub struct ModeledMaterialCost<V: SolveValue> {
    pub material_id: String,
    pub cost: V,
}
