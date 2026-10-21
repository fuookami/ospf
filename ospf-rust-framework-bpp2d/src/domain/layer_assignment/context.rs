//! 层分配上下文 / Layer assignment context
//!
//! 作为列生成建模的领域入口，覆盖注册、加列、移除列、影子价格刷新与结果提取五个生命周期阶段
//! （见 `.rules/framework-architecture.md` 第 4 节）。
//! The domain entry point for column-generation modelling, covering the five lifecycle stages of register,
//! add columns, remove columns, shadow price refresh and solution extraction (see
//! `.rules/framework-architecture.md` section 4).

use std::collections::BTreeMap;

use ospf_rust_base::error::Ret;

use crate::domain::item::model::{Layer, SheetTypeId};
use crate::domain::layer_assignment::model::{
    Bpp2dAssignmentDemandEntry, Bpp2dDemandShadowPriceKey, Bpp2dDemandShadowPriceMap, Load,
    LoadPrecision, LayerAggregation, LayerAssignmentObjectiveConfig, layer_demand_coefficient,
};
use ospf_rust_quantities::unit::derived::Meter;
use crate::infrastructure::validation_error;

// ============================================================================
// RmpAssignmentArtifacts - RMP 赋值构件 / RMP assignment artifacts
// ============================================================================

/// RMP 赋值构件 / RMP assignment artifacts
#[derive(Debug, Clone)]
pub struct RmpAssignmentArtifacts {
    /// 层聚合 / Layer aggregation
    pub aggregation: LayerAggregation<f64, Meter>,
    /// 负载 / Load
    pub load: Load,
    /// 面积目标项 / Area objective terms
    pub area_terms: Vec<(usize, f64)>,
}

// ============================================================================
// FinalAssignmentArtifacts - 最终 MILP 赋值构件 / Final MILP assignment artifacts
// ============================================================================

/// 最终 MILP 赋值构件 / Final MILP assignment artifacts
#[derive(Debug, Clone)]
pub struct FinalAssignmentArtifacts {
    /// 候选层 / Candidate layers
    pub layers: Vec<Layer<f64, Meter>>,
    /// 每列份数上界 / Per-column copy upper bound
    pub max_copies: Vec<u64>,
    /// 负载 / Load
    pub load: Load,
}

// ============================================================================
// LayerAssignmentContext - 层分配上下文 / Layer assignment context
// ============================================================================

/// 层分配上下文 / Layer assignment context
#[derive(Debug, Clone, Default)]
pub struct LayerAssignmentContext {
    /// 目标配置 / Objective configuration
    pub objective_config: LayerAssignmentObjectiveConfig,
    /// 影子价格 / Shadow prices
    pub shadow_prices: Bpp2dDemandShadowPriceMap,
    /// 最近一次迭代编号 / Latest iteration index
    pub iteration: u64,
}

impl LayerAssignmentContext {
    /// 创建上下文 / Create the context
    pub fn new(objective_config: LayerAssignmentObjectiveConfig) -> Self {
        Self {
            objective_config,
            shadow_prices: Bpp2dDemandShadowPriceMap::new(),
            iteration: 0,
        }
    }

    /// 第一阶段：注册 RMP 阶段的赋值、负载与需求覆盖约束
    /// / Stage one: register the RMP assignment, load and demand coverage constraint
    pub fn register_rmp(
        &self,
        layers: Vec<Layer<f64, Meter>>,
        demand_entries: Vec<Bpp2dAssignmentDemandEntry>,
        over_enabled: bool,
    ) -> Ret<RmpAssignmentArtifacts> {
        if demand_entries.is_empty() {
            return Err(validation_error(
                "RMP 没有需求项 / The RMP has no demand entry",
            ));
        }
        let mut aggregation = LayerAggregation::new();
        aggregation.add_columns(layers);
        let load = Load::imprecise(demand_entries.clone()).with_over(over_enabled);
        let area_terms = crate::domain::layer_assignment::model::sheet_area_terms(
            aggregation.layers(),
            self.objective_config.rmp_area_coefficient,
        );
        Ok(RmpAssignmentArtifacts {
            aggregation,
            load,
            area_terms,
        })
    }

    /// 第二阶段：新增列并刷新覆盖量与目标项
    /// / Stage two: add columns and refresh coverage plus objective terms
    pub fn add_columns(
        &mut self,
        aggregation: &mut LayerAggregation<f64, Meter>,
        iteration: u64,
        new_layers: Vec<Layer<f64, Meter>>,
    ) -> Vec<Layer<f64, Meter>> {
        self.iteration = iteration;
        aggregation.add_columns(new_layers)
    }

    /// 第三阶段：退役列 / Stage three: retire columns
    ///
    /// 以「从聚合中移除并记录」表达退役；真实模型侧需要把变量范围固定为 0，
    /// 这里返回被退役的层，由应用层决定模型操作。
    /// Retirement is expressed as "remove from the aggregation and report"; the real model side must fix the
    /// variable range to 0, so the retired layers are returned for the application layer to act on.
    pub fn retire_columns(
        &mut self,
        aggregation: &mut LayerAggregation<f64, Meter>,
        retired: &[Layer<f64, Meter>],
    ) -> Vec<Layer<f64, Meter>> {
        let retired_keys: std::collections::HashSet<String> =
            retired.iter().map(|layer| layer.identity_key()).collect();
        let kept: Vec<_> = aggregation
            .layers()
            .iter()
            .filter(|layer| !retired_keys.contains(&layer.identity_key()))
            .cloned()
            .collect();
        let removed: Vec<_> = aggregation
            .layers()
            .iter()
            .filter(|layer| retired_keys.contains(&layer.identity_key()))
            .cloned()
            .collect();
        *aggregation = LayerAggregation::new();
        aggregation.add_columns(kept);
        removed
    }

    /// 第四阶段：从覆盖量刷新影子价格 / Stage four: refresh shadow prices from coverage duals
    ///
    /// 对偶值必须来自真实 LP 求解；缺失对偶时返回空映射而不是用零值伪造。
    /// Dual values must come from a real LP solve; when duals are missing an empty map is returned rather than
    /// fabricating zeros.
    pub fn refresh_shadow_prices(
        &mut self,
        demand_entries: &[Bpp2dAssignmentDemandEntry],
        duals: &[f64],
    ) -> Bpp2dDemandShadowPriceMap {
        let mut map = Bpp2dDemandShadowPriceMap::new();
        for (index, entry) in demand_entries.iter().enumerate() {
            match duals.get(index) {
                Some(dual) => {
                    map.insert(
                        Bpp2dDemandShadowPriceKey::new(entry.mode, entry.item_id.as_str()),
                        *dual,
                    );
                }
                // 对偶缺失时保留空值：定价阶段会跳过该需求，而不是把 0 当成真实对偶。
                // A missing dual stays absent: pricing skips that demand instead of treating 0 as a real dual.
                None => {}
            }
        }
        self.shadow_prices = map.clone();
        map
    }

    /// 第四阶段：提取影子价格 / Stage four: extract the shadow prices
    pub fn shadow_prices(&self) -> &Bpp2dDemandShadowPriceMap {
        &self.shadow_prices
    }

    /// 第五阶段：注册最终 MILP 的赋值与负载 / Stage five: register the final MILP assignment and load
    pub fn register_final(
        &self,
        layers: Vec<Layer<f64, Meter>>,
        demand_entries: Vec<Bpp2dAssignmentDemandEntry>,
        enable_capacity_constraint: bool,
    ) -> Ret<FinalAssignmentArtifacts> {
        if layers.is_empty() {
            return Err(validation_error(
                "最终 MILP 没有候选列 / The final MILP has no candidate column",
            ));
        }
        let cap = crate::domain::layer_assignment::model::demand_cap(&demand_entries);
        let max_copies = crate::domain::layer_assignment::model::max_copies_per_column(&layers, cap);
        let mut load = Load::precise(demand_entries);
        if enable_capacity_constraint {
            // 容量约束关闭时不启用任何松弛，容量本身由 SheetCapacityConstraint 表达。
            // With the capacity constraint disabled no slack is enabled; capacity itself is expressed by
            // SheetCapacityConstraint.
            load.less_enabled = false;
        }
        Ok(FinalAssignmentArtifacts {
            layers,
            max_copies,
            load,
        })
    }

    /// 第五阶段：交付最终目标项 / Stage five: deliver the final objective terms
    pub fn final_objective_terms(
        &self,
        layers: &[Layer<f64, Meter>],
    ) -> Vec<(usize, f64)> {
        self.objective_config.all_terms(layers)
    }

    /// 检验数定价：只保留改进列 / Reduced-cost pricing: keep only improving columns
    pub fn filter_improving_columns(
        &self,
        layers: Vec<Layer<f64, Meter>>,
        demand_entries: &[Bpp2dAssignmentDemandEntry],
        tolerance: f64,
    ) -> Vec<Layer<f64, Meter>> {
        crate::domain::layer_assignment::model::filter_improving_columns(
            layers,
            demand_entries,
            &self.shadow_prices,
            self.objective_config.rmp_area_coefficient,
            tolerance,
        )
    }

    /// 按板材类型统计列数 / Count columns per sheet type
    pub fn columns_by_sheet_type(
        &self,
        layers: &[Layer<f64, Meter>],
    ) -> BTreeMap<SheetTypeId, usize> {
        let mut counts: BTreeMap<SheetTypeId, usize> = BTreeMap::new();
        for layer in layers {
            if let Some(sheet) = layer.sheet.as_ref() {
                *counts.entry(sheet.type_code.clone()).or_insert(0) += 1;
            }
        }
        counts
    }

    /// 列对某需求的覆盖量（供诊断） / Coverage of one demand across columns (for diagnostics)
    pub fn coverage(
        &self,
        layers: &[Layer<f64, Meter>],
        copies: &[f64],
        entry: &Bpp2dAssignmentDemandEntry,
    ) -> f64 {
        layers
            .iter()
            .zip(copies.iter())
            .map(|(layer, copies)| layer_demand_coefficient(layer, entry) * copies)
            .sum()
    }

    /// 负载精度 / Load precision of a load
    pub fn precision_of(load: &Load) -> LoadPrecision {
        load.precision
    }
}
