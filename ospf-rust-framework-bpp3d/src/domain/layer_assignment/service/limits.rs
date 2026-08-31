//! 层分配约束和目标 Pipeline / Layer assignment constraint and objective pipelines
//!
//! 实现层分配 RMP 和 Final MILP 阶段的约束族和目标族。
//! Implements constraint and objective families for layer assignment
//! RMP and Final MILP phases.

use std::fmt::Debug;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::object::SubObjective;
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::mechanism::constraint_group::ConstraintGroup;
use ospf_rust_framework::model::pipeline::Pipeline;
use ospf_rust_framework::model::shadow_price::{ShadowPrice, ShadowPriceKey, BasicShadowPriceMap, ShadowPriceMap};
use ospf_rust_framework::model::pipeline::CGPipeline;
use ospf_rust_core::error::Result;

use super::{
    Bpp3dDemandEntry, DemandShadowPriceKey,
    ImpreciseAssignment, PreciseAssignment,
    Bpp3dSolverValueAdapter, Bpp3dSolverValueAdapterKind,
};
use crate::domain::item::BinType;

// ============================================================================
// DemandConstraint - 需求约束 / Demand constraint
// ============================================================================

/// 需求约束 / Demand constraint
///
/// 对每个需求条目添加上下界约束：
/// - RMP: `demand <= sum(x[layer] * layer_demand[layer]) + overLoad`
/// - Final MILP: `demand <= sum(x[bin, layer] * layer_demand[layer]) + overLoad`
///
/// 同时支持从对偶解提取 shadow price。
///
/// Adds lower and upper bound constraints for each demand entry:
/// - RMP: `demand <= sum(x[layer] * layer_demand[layer]) + overLoad`
/// - Final MILP: `demand <= sum(x[bin, layer] * layer_demand[layer]) + overLoad`
///
/// Also supports shadow price extraction from dual solution.
#[derive(Debug)]
pub struct DemandConstraint<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    name: String,
    group: Option<ConstraintGroup>,
    /// 需求条目列表 / Demand entries
    pub demand_entries: Vec<Bpp3dDemandEntry>,
    /// 赋值变量引用 / Assignment variable references
    pub assignment_ref: DemandAssignmentRef<V, U>,
}

/// 需求赋值变量引用 / Demand assignment variable references
///
/// 持有赋值变量的索引信息，用于约束注册和 shadow price 提取。
/// Holds assignment variable index information for constraint registration
/// and shadow price extraction.
#[derive(Debug, Clone)]
pub enum DemandAssignmentRef<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// RMP 阶段：x[layer] 连续变量 / RMP phase: x[layer] continuous variables
    Imprecise {
        /// 赋值模型 / Assignment model
        assignment: ImpreciseAssignment<V, U>,
    },
    /// Final MILP 阶段：x[bin, layer] 二值变量 / Final MILP phase: x[bin, layer] binary variables
    Precise {
        /// 赋值模型 / Assignment model
        assignment: PreciseAssignment<V, U>,
    },
}

impl<V, U> DemandConstraint<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 创建 RMP 阶段需求约束 / Create RMP phase demand constraint
    pub fn imprecise(
        demand_entries: Vec<Bpp3dDemandEntry>,
        assignment: ImpreciseAssignment<V, U>,
    ) -> Self {
        Self {
            name: "demand_constraint".to_string(),
            group: None,
            demand_entries,
            assignment_ref: DemandAssignmentRef::Imprecise { assignment },
        }
    }

    /// 创建 Final MILP 阶段需求约束 / Create Final MILP phase demand constraint
    pub fn precise(
        demand_entries: Vec<Bpp3dDemandEntry>,
        assignment: PreciseAssignment<V, U>,
    ) -> Self {
        Self {
            name: "demand_constraint".to_string(),
            group: None,
            demand_entries,
            assignment_ref: DemandAssignmentRef::Precise { assignment },
        }
    }

    /// 获取需求 shadow price 键 / Get demand shadow price key
    pub fn shadow_price_key(entry: &Bpp3dDemandEntry) -> DemandShadowPriceKey {
        DemandShadowPriceKey {
            mode: entry.mode,
            key: entry.key.clone(),
        }
    }
}

impl<V, U> Pipeline<MetaModel<f64>> for DemandConstraint<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { self.group.as_ref() }

    fn register(&self, model: &mut MetaModel<f64>) {
        match &self.assignment_ref {
            DemandAssignmentRef::Imprecise { assignment } => {
                for (demand_idx, entry) in self.demand_entries.iter().enumerate() {
                    let terms = imprecise_layer_terms(assignment, entry);
                    if let Err(e) = model.add_ge_constraint(
                        &terms,
                        entry.demand,
                        &format!("{}_{}", self.name, demand_idx),
                    ) {
                        log::warn!("Failed to register {}_{}: {:?}", self.name, demand_idx, e);
                    }
                }
            }
            DemandAssignmentRef::Precise { assignment } => {
                for (demand_idx, entry) in self.demand_entries.iter().enumerate() {
                    let terms = precise_layer_terms(assignment, entry);
                    if let Err(e) = model.add_ge_constraint(
                        &terms,
                        entry.demand,
                        &format!("{}_{}", self.name, demand_idx),
                    ) {
                        log::warn!("Failed to register {}_{}: {:?}", self.name, demand_idx, e);
                    }
                }
            }
        }
    }

    fn invoke(&self, model: &MetaModel<f64>) -> Result<()> {
        let _ = model;
        Ok(())
    }
}

fn imprecise_layer_terms<V, U>(
    assignment: &ImpreciseAssignment<V, U>,
    entry: &Bpp3dDemandEntry,
) -> Vec<(usize, f64)>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    (0..assignment.layers.len())
        .filter_map(|layer_idx| {
            assignment
                .x
                .index(&layer_idx)
                .or(Some(layer_idx))
                .and_then(|model_idx| {
                    let coefficient = assignment.layers[layer_idx]
                        .demand_coverage_coefficient(entry.mode, &entry.key);
                    (coefficient != 0.0).then_some((model_idx, coefficient))
                })
        })
        .collect()
}

fn precise_layer_terms<V, U>(
    assignment: &PreciseAssignment<V, U>,
    entry: &Bpp3dDemandEntry,
) -> Vec<(usize, f64)>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    assignment.bins
        .iter()
        .enumerate()
        .flat_map(|(bin_idx, _)| {
            assignment.layers.iter().enumerate().filter_map(move |(layer_idx, _)| {
                assignment
                    .x
                    .index(&bin_idx, &layer_idx)
                    .or(Some(bin_idx * assignment.layers.len() + layer_idx))
                    .and_then(|model_idx| {
                        let coefficient = assignment.layers[layer_idx]
                            .demand_coverage_coefficient(entry.mode, &entry.key);
                        (coefficient != 0.0).then_some((model_idx, coefficient))
                    })
            })
        })
        .collect()
}

fn precise_bin_marker_index<V, U>(
    assignment: &PreciseAssignment<V, U>,
    bin_idx: usize,
) -> Option<usize>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    assignment
        .v
        .index(&bin_idx)
        .or(Some(assignment.bins.len() * assignment.layers.len() + bin_idx))
}

fn precise_assignment_index<V, U>(
    assignment: &PreciseAssignment<V, U>,
    bin_idx: usize,
    layer_idx: usize,
) -> Option<usize>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    assignment
        .x
        .index(&bin_idx, &layer_idx)
        .or(Some(bin_idx * assignment.layers.len() + layer_idx))
}

// ============================================================================
// PreciseAssignmentActivationConstraint - 精确赋值启用约束 / Precise assignment activation constraint
// ============================================================================

/// 精确赋值启用约束 / Precise assignment activation constraint
///
/// 将 final MILP 的层赋值变量绑定到箱启用变量：`x[bin, layer] <= v[bin]`。
/// Binds final MILP layer assignment variables to bin activation variables:
/// `x[bin, layer] <= v[bin]`.
#[derive(Debug)]
pub struct PreciseAssignmentActivationConstraint<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    name: String,
    group: Option<ConstraintGroup>,
    /// 精确赋值模型 / Precise assignment model
    pub assignment: PreciseAssignment<V, U>,
}

impl<V, U> PreciseAssignmentActivationConstraint<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 创建精确赋值启用约束 / Create precise assignment activation constraint
    pub fn new(assignment: PreciseAssignment<V, U>) -> Self {
        Self {
            name: "precise_assignment_activation_constraint".to_string(),
            group: None,
            assignment,
        }
    }
}

impl<V, U> Pipeline<MetaModel<f64>> for PreciseAssignmentActivationConstraint<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { self.group.as_ref() }

    fn register(&self, model: &mut MetaModel<f64>) {
        for bin_idx in 0..self.assignment.bins.len() {
            let Some(v_idx) = precise_bin_marker_index(&self.assignment, bin_idx) else {
                continue;
            };
            for layer_idx in 0..self.assignment.layers.len() {
                let Some(x_idx) = precise_assignment_index(&self.assignment, bin_idx, layer_idx) else {
                    continue;
                };
                if let Err(e) = model.add_le_constraint(
                    &[(x_idx, 1.0), (v_idx, -1.0)],
                    0.0,
                    &format!("{}_{}_{}", self.name, bin_idx, layer_idx),
                ) {
                    log::warn!(
                        "Failed to register {}_{}_{}: {:?}",
                        self.name,
                        bin_idx,
                        layer_idx,
                        e
                    );
                }
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

impl<V, U> CGPipeline<DemandShadowPriceKey, MetaModel<f64>, BasicShadowPriceMap<DemandShadowPriceKey>>
    for DemandConstraint<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    type Extractor = fn(&BasicShadowPriceMap<DemandShadowPriceKey>, &DemandShadowPriceKey) -> f64;

    fn extractor(&self) -> Option<Self::Extractor> {
        Some(|map, key| map.get(&ShadowPriceKey::named::<DemandShadowPriceKey>(format!("{:?}", key)))
            .map(|sp| sp.price)
            .unwrap_or(0.0))
    }

    fn refresh(
        &self,
        shadow_price_map: &mut BasicShadowPriceMap<DemandShadowPriceKey>,
        _model: &MetaModel<f64>,
        shadow_prices: &[f64],
    ) -> Result<()> {
        for (idx, entry) in self.demand_entries.iter().enumerate() {
            let key = DemandShadowPriceKey {
                mode: entry.mode,
                key: entry.key.clone(),
            };
            let price = shadow_prices.get(idx).copied().unwrap_or(0.0);
            let sp_key = ShadowPriceKey::named::<DemandShadowPriceKey>(format!("{:?}", key));
            shadow_price_map.put(ShadowPrice::new(sp_key, price));
        }
        Ok(())
    }
}

// ============================================================================
// BinCapacityConstraint - 箱容量约束 / Bin capacity constraint
// ============================================================================

/// 箱容量约束 / Bin capacity constraint
///
/// 对每个箱添加载重和体积上界约束：
/// - `loadWeight[bin] <= weight_capacity`
/// - `loadVolume[bin] <= volume_capacity`
///
/// Adds weight and volume upper bound constraints per bin:
/// - `loadWeight[bin] <= weight_capacity`
/// - `loadVolume[bin] <= volume_capacity`
#[derive(Debug)]
pub struct BinCapacityConstraint {
    name: String,
    group: Option<ConstraintGroup>,
    /// 箱载重上界 / Weight capacity per bin
    pub weight_capacities: Vec<f64>,
    /// 箱体积上界 / Volume capacity per bin
    pub volume_capacities: Vec<f64>,
    /// 赋值变量索引 (bin_idx, layer_idx) -> model_index / Assignment variable indices
    pub x_indices: Vec<Vec<(usize, usize)>>,
    /// 层载重系数 / Layer weight coefficients
    pub layer_weights: Vec<f64>,
    /// 层体积系数 / Layer volume coefficients
    pub layer_volumes: Vec<f64>,
}

impl BinCapacityConstraint {
    /// 从箱型和适配器创建箱容量约束 / Create bin capacity constraint from bin types and adapter
    pub fn from_bins<U>(
        bins: &[BinType<f64, U>],
        x_indices: Vec<Vec<(usize, usize)>>,
        layer_weights: Vec<f64>,
        layer_volumes: Vec<f64>,
        adapter: &Bpp3dSolverValueAdapterKind,
    ) -> Self
    where
        U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
    {
        let weight_capacities: Vec<f64> = bins.iter()
            .map(|b| adapter.weight_to_solver(b.capacity.value))
            .collect();
        let volume_capacities: Vec<f64> = bins.iter()
            .map(|b| adapter.volume_to_solver(b.width.value * b.height.value * b.depth.value))
            .collect();

        Self {
            name: "bin_capacity_constraint".to_string(),
            group: None,
            weight_capacities,
            volume_capacities,
            x_indices,
            layer_weights,
            layer_volumes,
        }
    }

    /// 直接从系数创建箱容量约束 / Create bin capacity constraint directly from coefficients
    pub fn new(
        weight_capacities: Vec<f64>,
        volume_capacities: Vec<f64>,
        x_indices: Vec<Vec<(usize, usize)>>,
        layer_weights: Vec<f64>,
        layer_volumes: Vec<f64>,
    ) -> Self {
        Self {
            name: "bin_capacity_constraint".to_string(),
            group: None,
            weight_capacities,
            volume_capacities,
            x_indices,
            layer_weights,
            layer_volumes,
        }
    }
}

impl Pipeline<MetaModel<f64>> for BinCapacityConstraint {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { self.group.as_ref() }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (bin_idx, layer_indices) in self.x_indices.iter().enumerate() {
            let weight_cap = self.weight_capacities.get(bin_idx).copied().unwrap_or(0.0);
            let volume_cap = self.volume_capacities.get(bin_idx).copied().unwrap_or(0.0);

            // 载重约束: sum(x[bin, layer] * weight[layer]) <= weight_capacity
            let weight_terms: Vec<(usize, f64)> = layer_indices.iter()
                .filter_map(|&(layer_idx, model_idx)| {
                    self.layer_weights.get(layer_idx).map(|&w| (model_idx, w))
                })
                .collect();

            if !weight_terms.is_empty() {
                if let Err(e) = model.add_le_constraint(
                    &weight_terms,
                    weight_cap,
                    &format!("{}_weight_{}", self.name, bin_idx),
                ) {
                    log::warn!("Failed to register {}_weight_{}: {:?}", self.name, bin_idx, e);
                }
            }

            // 体积约束: sum(x[bin, layer] * volume[layer]) <= volume_capacity
            let volume_terms: Vec<(usize, f64)> = layer_indices.iter()
                .filter_map(|&(layer_idx, model_idx)| {
                    self.layer_volumes.get(layer_idx).map(|&v| (model_idx, v))
                })
                .collect();

            if !volume_terms.is_empty() {
                if let Err(e) = model.add_le_constraint(
                    &volume_terms,
                    volume_cap,
                    &format!("{}_volume_{}", self.name, bin_idx),
                ) {
                    log::warn!("Failed to register {}_volume_{}: {:?}", self.name, bin_idx, e);
                }
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
// BinDepthConstraint - 箱深度约束 / Bin depth constraint
// ============================================================================

/// 箱深度约束 / Bin depth constraint
///
/// 对每个箱添加深度上界约束：
/// - `loadDepth[bin] <= depth_capacity`
///
/// Adds depth upper bound constraint per bin:
/// - `loadDepth[bin] <= depth_capacity`
#[derive(Debug)]
pub struct BinDepthConstraint {
    name: String,
    group: Option<ConstraintGroup>,
    /// 箱深度上界 / Depth capacity per bin
    pub depth_capacities: Vec<f64>,
    /// 赋值变量索引 (bin_idx, layer_idx) -> model_index / Assignment variable indices
    pub x_indices: Vec<Vec<(usize, usize)>>,
    /// 层深度系数 / Layer depth coefficients
    pub layer_depths: Vec<f64>,
}

impl BinDepthConstraint {
    /// 从箱型和适配器创建箱深度约束 / Create bin depth constraint from bin types and adapter
    pub fn from_bins<U>(
        bins: &[BinType<f64, U>],
        x_indices: Vec<Vec<(usize, usize)>>,
        layer_depths: Vec<f64>,
        adapter: &Bpp3dSolverValueAdapterKind,
    ) -> Self
    where
        U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
    {
        let depth_capacities: Vec<f64> = bins.iter()
            .map(|b| adapter.depth_to_solver(b.depth.value))
            .collect();

        Self {
            name: "bin_depth_constraint".to_string(),
            group: None,
            depth_capacities,
            x_indices,
            layer_depths,
        }
    }

    /// 直接从系数创建箱深度约束 / Create bin depth constraint directly from coefficients
    pub fn new(
        depth_capacities: Vec<f64>,
        x_indices: Vec<Vec<(usize, usize)>>,
        layer_depths: Vec<f64>,
    ) -> Self {
        Self {
            name: "bin_depth_constraint".to_string(),
            group: None,
            depth_capacities,
            x_indices,
            layer_depths,
        }
    }
}

impl Pipeline<MetaModel<f64>> for BinDepthConstraint {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { self.group.as_ref() }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (bin_idx, layer_indices) in self.x_indices.iter().enumerate() {
            let depth_cap = self.depth_capacities.get(bin_idx).copied().unwrap_or(0.0);

            // 深度约束: sum(x[bin, layer] * depth[layer]) <= depth_capacity
            let depth_terms: Vec<(usize, f64)> = layer_indices.iter()
                .filter_map(|&(layer_idx, model_idx)| {
                    self.layer_depths.get(layer_idx).map(|&d| (model_idx, d))
                })
                .collect();

            if !depth_terms.is_empty() {
                if let Err(e) = model.add_le_constraint(
                    &depth_terms,
                    depth_cap,
                    &format!("{}_depth_{}", self.name, bin_idx),
                ) {
                    log::warn!("Failed to register {}_depth_{}: {:?}", self.name, bin_idx, e);
                }
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
// BinAmountMinimization - 箱数量最小化 / Bin amount minimization
// ============================================================================

/// 箱数量最小化 / Bin amount minimization
///
/// 最小化使用的箱数量：`min sum(v[bin])`。
/// Minimizes the number of used bins: `min sum(v[bin])`.
#[derive(Debug)]
pub struct BinAmountMinimization {
    name: String,
    /// 箱使用标记变量索引 / Bin usage marker variable indices
    pub v_indices: Vec<usize>,
    /// 系数 / Coefficient
    pub coefficient: f64,
}

impl BinAmountMinimization {
    /// 创建箱数量最小化 / Create bin amount minimization
    pub fn new(v_indices: Vec<usize>, coefficient: f64) -> Self {
        Self {
            name: "bin_amount_minimization".to_string(),
            v_indices,
            coefficient,
        }
    }
}

impl Pipeline<MetaModel<f64>> for BinAmountMinimization {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { None }

    fn register(&self, model: &mut MetaModel<f64>) {
        if self.v_indices.is_empty() {
            return;
        }
        let monomials: Vec<LinearMonomial<f64>> = self.v_indices.iter()
            .map(|&idx| LinearMonomial::new(self.coefficient, idx))
            .collect();
        let polynomial = Linear::new(monomials, 0.0);
        let sub_obj = SubObjective::minimize(polynomial, &self.name);
        model.add_sub_objective(sub_obj);
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
// VolumeMinimization - 体积最小化 / Volume minimization
// ============================================================================

/// 体积最小化 / Volume minimization
///
/// 最小化使用的总体积。
/// Minimizes the total volume used.
#[derive(Debug)]
pub struct VolumeMinimization {
    name: String,
    /// 体积项：(x_model_index, coefficient) 列表 / Volume terms: (x model index, coefficient) list
    pub volume_terms: Vec<(usize, f64)>,
    /// 系数 / Coefficient
    pub coefficient: f64,
}

impl VolumeMinimization {
    /// 创建体积最小化 / Create volume minimization
    pub fn new(volume_terms: Vec<(usize, f64)>, coefficient: f64) -> Self {
        Self {
            name: "volume_minimization".to_string(),
            volume_terms,
            coefficient,
        }
    }
}

impl Pipeline<MetaModel<f64>> for VolumeMinimization {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { None }

    fn register(&self, model: &mut MetaModel<f64>) {
        if self.volume_terms.is_empty() {
            return;
        }
        let monomials: Vec<LinearMonomial<f64>> = self.volume_terms.iter()
            .map(|&(idx, coeff)| LinearMonomial::new(coeff * self.coefficient, idx))
            .collect();
        let polynomial = Linear::new(monomials, 0.0);
        let sub_obj = SubObjective::minimize(polynomial, &self.name);
        model.add_sub_objective(sub_obj);
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
// BetterLayerMaximization - 更优层最大化 / Better layer maximization
// ============================================================================

/// 更优层最大化 / Better layer maximization
///
/// 最大化高价值层的赋值量，常用于 RMP 阶段。
/// Maximizes assignment of high-value layers, commonly used in the RMP phase.
#[derive(Debug)]
pub struct BetterLayerMaximization {
    name: String,
    /// 层价值项：(x_model_index, coefficient) 列表 / Layer value terms
    pub value_terms: Vec<(usize, f64)>,
    /// 系数 / Coefficient
    pub coefficient: f64,
}

impl BetterLayerMaximization {
    /// 创建更优层最大化 / Create better layer maximization
    pub fn new(value_terms: Vec<(usize, f64)>, coefficient: f64) -> Self {
        Self {
            name: "better_layer_maximization".to_string(),
            value_terms,
            coefficient,
        }
    }
}

impl Pipeline<MetaModel<f64>> for BetterLayerMaximization {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { None }

    fn register(&self, model: &mut MetaModel<f64>) {
        if self.value_terms.is_empty() {
            return;
        }
        let monomials: Vec<LinearMonomial<f64>> = self.value_terms.iter()
            .map(|&(idx, coeff)| LinearMonomial::new(coeff * self.coefficient, idx))
            .collect();
        let polynomial = Linear::new(monomials, 0.0);
        let sub_obj = SubObjective::maximize(polynomial, &self.name);
        model.add_sub_objective(sub_obj);
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
// TailBinAssignmentConstraint - 尾箱赋值约束 / Tail bin assignment constraint
// ============================================================================

/// 尾箱赋值约束 / Tail bin assignment constraint
///
/// 确保尾箱（最后一个使用的箱）在箱序列中排在最后。
/// 对于已使用的箱 b_i 和 b_j (i < j)，如果 b_j 未使用，则 b_i 必须未使用：
/// - `v[b_j] <= v[b_i]` for all i < j
///
/// Ensures the tail bin (last used bin) appears last in the bin sequence.
/// For used bins b_i and b_j (i < j), if b_j is unused, b_i must be unused:
/// - `v[b_j] <= v[b_i]` for all i < j
#[derive(Debug)]
pub struct TailBinAssignmentConstraint {
    name: String,
    group: Option<ConstraintGroup>,
    /// 箱使用标记变量索引 / Bin usage marker variable indices
    pub v_indices: Vec<usize>,
}

impl TailBinAssignmentConstraint {
    /// 创建尾箱赋值约束 / Create tail bin assignment constraint
    pub fn new(v_indices: Vec<usize>) -> Self {
        Self {
            name: "tail_bin_assignment_constraint".to_string(),
            group: None,
            v_indices,
        }
    }
}

impl Pipeline<MetaModel<f64>> for TailBinAssignmentConstraint {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { self.group.as_ref() }

    fn register(&self, model: &mut MetaModel<f64>) {
        // v[b_j] <= v[b_i] => v[b_j] - v[b_i] <= 0
        for i in 0..self.v_indices.len() {
            for j in (i + 1)..self.v_indices.len() {
                let v_i = self.v_indices[i];
                let v_j = self.v_indices[j];
                if let Err(e) = model.add_le_constraint(
                    &[(v_j, 1.0), (v_i, -1.0)],
                    0.0,
                    &format!("{}_{}_{}", self.name, i, j),
                ) {
                    log::warn!("Failed to register {}_{}_{}: {:?}", self.name, i, j, e);
                }
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
// Deferred objectives and limits - 延后目标和约束 / Deferred objectives and limits
// ============================================================================

/// 延后注册计划 / Deferred registration plan
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DeferredRegistrationPlan {
    /// 名称 / Name
    pub name: String,
    /// 目标族 / Objective family
    pub objective_family: Option<String>,
    /// 约束族 / Constraint family
    pub constraint_family: Option<String>,
    /// 变量索引 / Variable indices
    pub variable_indices: Vec<usize>,
    /// 诊断信息 / Diagnostics
    pub diagnostics: Vec<String>,
}

macro_rules! deferred_limit {
    ($type_name:ident, $name:literal, $message:literal) => {
        /// 延后约束或目标 skeleton / Deferred constraint or objective skeleton
        #[derive(Debug, Clone, Default)]
        pub struct $type_name {
            /// 名称 / Name
            pub name: String,
        }

        impl $type_name {
            /// 创建 skeleton / Create skeleton
            pub fn new() -> Self {
                Self {
                    name: $name.to_string(),
                }
            }

            /// 诊断信息 / Diagnostics
            pub fn diagnostics(&self) -> Vec<String> {
                vec![$message.to_string()]
            }

            /// 创建结构化注册计划 / Create structured registration plan
            pub fn registration_plan(&self, variable_indices: Vec<usize>) -> DeferredRegistrationPlan {
                DeferredRegistrationPlan {
                    name: self.name.clone(),
                    objective_family: Some(self.name.clone()),
                    constraint_family: None,
                    variable_indices,
                    diagnostics: self.diagnostics(),
                }
            }

            /// 注册最小目标 / Register minimal objective
            pub fn register_minimal_objective(
                &self,
                model: &mut MetaModel<f64>,
                variable_indices: Vec<usize>,
                coefficient: f64,
            ) -> DeferredRegistrationPlan {
                let plan = self.registration_plan(variable_indices.clone());
                if !variable_indices.is_empty() {
                    let polynomial = Linear::new(
                        variable_indices
                            .iter()
                            .map(|index| LinearMonomial::new(coefficient, *index))
                            .collect(),
                        0.0,
                    );
                    model.add_sub_objective(SubObjective::minimize(polynomial, &self.name));
                }
                plan
            }
        }
    };
}

deferred_limit!(
    RestAmountMinimization,
    "rest_amount_minimization",
    "rest amount minimization is not implemented"
);
deferred_limit!(
    TailBinLoadingRateMinimization,
    "tail_bin_loading_rate_minimization",
    "tail bin loading rate minimization is not implemented"
);
deferred_limit!(
    BinLoadingOrderConstraint,
    "bin_loading_order_constraint",
    "bin loading order constraint is not implemented"
);

impl BinLoadingOrderConstraint {
    /// 创建结构化约束注册计划 / Create structured constraint registration plan
    pub fn constraint_registration_plan(&self, variable_indices: Vec<usize>) -> DeferredRegistrationPlan {
        DeferredRegistrationPlan {
            name: self.name.clone(),
            objective_family: None,
            constraint_family: Some(self.name.clone()),
            variable_indices,
            diagnostics: self.diagnostics(),
        }
    }

    /// 注册最小顺序约束 / Register minimal ordering constraint
    pub fn register_minimal_constraint(
        &self,
        model: &mut MetaModel<f64>,
        variable_indices: Vec<usize>,
    ) -> DeferredRegistrationPlan {
        let plan = self.constraint_registration_plan(variable_indices.clone());
        for pair in variable_indices.windows(2) {
            let _ = model.add_le_constraint(
                &[(pair[1], 1.0), (pair[0], -1.0)],
                0.0,
                &format!("{}_{}_{}", self.name, pair[0], pair[1]),
            );
        }
        plan
    }
}

impl RestAmountMinimization {
    /// 注册余量最小化目标 / Register rest-amount minimization objective
    pub fn register_objective(
        &self,
        model: &mut MetaModel<f64>,
        rest_variable_indices: Vec<usize>,
        coefficient: f64,
    ) -> DeferredRegistrationPlan {
        self.register_minimal_objective(model, rest_variable_indices, coefficient)
    }
}

impl TailBinLoadingRateMinimization {
    /// 注册尾箱装载率最小化目标 / Register tail-bin loading-rate minimization objective
    pub fn register_objective(
        &self,
        model: &mut MetaModel<f64>,
        tail_bin_variable_indices: Vec<usize>,
        coefficient: f64,
    ) -> DeferredRegistrationPlan {
        self.register_minimal_objective(model, tail_bin_variable_indices, coefficient)
    }
}

impl BinLoadingOrderConstraint {
    /// 注册装箱顺序约束 / Register bin loading order constraint
    pub fn register_constraint(
        &self,
        model: &mut MetaModel<f64>,
        ordered_variable_indices: Vec<usize>,
    ) -> DeferredRegistrationPlan {
        self.register_minimal_constraint(model, ordered_variable_indices)
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::item::{
        BinLayer, BinType, Bpp3dDemandKey, Bpp3dDemandMode, Bpp3dLayerDemandCoverage,
    };
    use crate::domain::layer_assignment::model::{
        Bpp3dModelComponent, VariableArray1, VariableArray2,
    };
    use crate::domain::layer_assignment::service::ScaledBpp3dSolverValueAdapter;
    use ospf_rust_quantities::unit::derived::Meter;
    use ospf_rust_quantities::quantity::Quantity;

    fn meters(v: f64) -> Quantity<f64, Meter> {
        Quantity::new_ct(v)
    }

    fn make_bin_type() -> BinType<f64, Meter> {
        BinType {
            width: meters(10.0),
            height: meters(10.0),
            depth: meters(10.0),
            capacity: meters(1000.0),
            type_code: "BIN-10".to_string(),
            is_main: true,
        }
    }

    fn make_layer(depth: f64) -> BinLayer<f64, Meter> {
        BinLayer {
            iteration: 0,
            from: "test".to_string(),
            bin: None,
            depth: meters(depth),
            demand_coverage: Vec::new(),
        }
    }

    #[test]
    fn demand_constraint_shadow_price_key() {
        let entry = Bpp3dDemandEntry {
            mode: crate::domain::item::Bpp3dDemandMode::Item,
            key: crate::domain::item::Bpp3dDemandKey::Item { id: "item1".to_string() },
            demand: 10.0,
        };
        let key = DemandConstraint::<f64, Meter>::shadow_price_key(&entry);
        assert_eq!(key.mode, crate::domain::item::Bpp3dDemandMode::Item);
    }

    #[test]
    fn demand_constraint_registers_linear_cover_rows() {
        let key = Bpp3dDemandKey::Item { id: "item1".to_string() };
        let mut assignment = ImpreciseAssignment {
            layers: vec![
                make_layer(1.0).with_demand_coverage(vec![Bpp3dLayerDemandCoverage::new(
                    Bpp3dDemandMode::Item,
                    key.clone(),
                    1.0,
                )]),
                make_layer(2.0),
            ],
            x: VariableArray1::new("x"),
        };
        let mut model = MetaModel::<f64>::new("demand_cover");
        assignment.register(&mut model).unwrap();
        let constraint: DemandConstraint<f64, Meter> = DemandConstraint::imprecise(
            vec![Bpp3dDemandEntry {
                mode: Bpp3dDemandMode::Item,
                key,
                demand: 1.0,
            }],
            assignment,
        );

        constraint.register(&mut model);

        assert_eq!(model.num_constraints(), 1);
    }

    #[test]
    fn precise_activation_constraint_registers_x_to_v_rows() {
        let bin = make_bin_type();
        let mut assignment = PreciseAssignment {
            bins: vec![bin],
            layers: vec![make_layer(1.0), make_layer(2.0)],
            x: VariableArray2::new("x"),
            v: VariableArray1::new("v"),
        };
        let mut model = MetaModel::<f64>::new("activation");
        assignment.register(&mut model).unwrap();
        let constraint = PreciseAssignmentActivationConstraint::new(assignment);

        constraint.register(&mut model);

        assert_eq!(model.num_constraints(), 2);
    }

    #[test]
    fn demand_constraint_cgpipeline_refresh() {
        let assignment = ImpreciseAssignment {
            layers: vec![make_layer(1.0)],
            x: VariableArray1::new("x"),
        };
        let entries = vec![
            Bpp3dDemandEntry {
                mode: crate::domain::item::Bpp3dDemandMode::Item,
                key: crate::domain::item::Bpp3dDemandKey::Item { id: "item1".to_string() },
                demand: 10.0,
            },
        ];
        let constraint: DemandConstraint<f64, Meter> = DemandConstraint::imprecise(entries, assignment);

        let mut map: BasicShadowPriceMap<DemandShadowPriceKey> = BasicShadowPriceMap::new();
        let model = MetaModel::<f64>::new("test");
        let shadow_prices = vec![2.5];

        constraint.refresh(&mut map, &model, &shadow_prices).unwrap();

        // 验证 shadow price 已写入 map
        let sp_key = DemandShadowPriceKey {
            mode: crate::domain::item::Bpp3dDemandMode::Item,
            key: crate::domain::item::Bpp3dDemandKey::Item { id: "item1".to_string() },
        };
        let lookup_key = ShadowPriceKey::named::<DemandShadowPriceKey>(format!("{:?}", sp_key));
        let sp = map.get(&lookup_key);
        assert!(sp.is_some());
        assert_eq!(sp.unwrap().price, 2.5);
    }

    #[test]
    fn demand_constraint_cgpipeline_extractor() {
        let assignment = ImpreciseAssignment {
            layers: vec![make_layer(1.0)],
            x: VariableArray1::new("x"),
        };
        let entries = vec![
            Bpp3dDemandEntry {
                mode: crate::domain::item::Bpp3dDemandMode::Item,
                key: crate::domain::item::Bpp3dDemandKey::Item { id: "item1".to_string() },
                demand: 10.0,
            },
        ];
        let constraint: DemandConstraint<f64, Meter> = DemandConstraint::imprecise(entries, assignment);

        let extractor = constraint.extractor();
        assert!(extractor.is_some());

        let mut map: BasicShadowPriceMap<DemandShadowPriceKey> = BasicShadowPriceMap::new();
        let sp_key = DemandShadowPriceKey {
            mode: crate::domain::item::Bpp3dDemandMode::Item,
            key: crate::domain::item::Bpp3dDemandKey::Item { id: "item1".to_string() },
        };
        let lookup_key = ShadowPriceKey::named::<DemandShadowPriceKey>(format!("{:?}", sp_key));
        map.put(ShadowPrice::new(lookup_key, 3.0));

        let extract_fn = extractor.unwrap();
        let price = extract_fn(&map, &sp_key);
        assert_eq!(price, 3.0);
    }

    #[test]
    fn bin_capacity_constraint_register() {
        let mut model = MetaModel::<f64>::new("test_bin_cap");

        // 注册赋值变量
        let bin_keys = vec![0usize];
        let layer_keys = vec![0usize, 1usize];
        let mut x: VariableArray2<usize, usize, ospf_rust_core::variable::variable_item::BinaryVariableItem> = VariableArray2::new("x");
        x.register_binary(&bin_keys, &layer_keys, &mut model).unwrap();

        let x_indices: Vec<Vec<(usize, usize)>> = vec![
            vec![
                (0, x.index(&0, &0).unwrap()),
                (1, x.index(&0, &1).unwrap()),
            ],
        ];

        let constraint = BinCapacityConstraint::from_bins(
            &[make_bin_type()],
            x_indices,
            vec![5.0, 3.0],    // layer weights
            vec![20.0, 15.0],  // layer volumes
            &Bpp3dSolverValueAdapterKind::Default,
        );

        constraint.register(&mut model);
        constraint.invoke(&model).unwrap();
    }

    #[test]
    fn bin_depth_constraint_register() {
        let mut model = MetaModel::<f64>::new("test_bin_depth");

        let bin_keys = vec![0usize];
        let layer_keys = vec![0usize, 1usize];
        let mut x: VariableArray2<usize, usize, ospf_rust_core::variable::variable_item::BinaryVariableItem> = VariableArray2::new("x");
        x.register_binary(&bin_keys, &layer_keys, &mut model).unwrap();

        let x_indices: Vec<Vec<(usize, usize)>> = vec![
            vec![
                (0, x.index(&0, &0).unwrap()),
                (1, x.index(&0, &1).unwrap()),
            ],
        ];

        let constraint = BinDepthConstraint::from_bins(
            &[make_bin_type()],
            x_indices,
            vec![2.0, 3.0],    // layer depths
            &Bpp3dSolverValueAdapterKind::Default,
        );

        constraint.register(&mut model);
        constraint.invoke(&model).unwrap();
    }

    #[test]
    fn bin_amount_minimization_register() {
        let mut model = MetaModel::<f64>::new("test_bin_amount_obj");

        // 注册 v 变量
        let mut v: VariableArray1<usize, ospf_rust_core::variable::variable_item::BinaryVariableItem> = VariableArray1::new("v");
        v.register_binary(&[0usize, 1usize], &mut model).unwrap();

        let obj = BinAmountMinimization::new(
            vec![v.index(&0).unwrap(), v.index(&1).unwrap()],
            1.0,
        );

        obj.register(&mut model);
        obj.invoke(&model).unwrap();
    }

    #[test]
    fn volume_minimization_register() {
        let mut model = MetaModel::<f64>::new("test_vol_min");

        let mut x: VariableArray1<usize, ospf_rust_core::variable::variable_item::ContinuousVariableItem> = VariableArray1::new("x");
        x.register_continuous(&[0usize, 1usize], &mut model).unwrap();

        let obj = VolumeMinimization::new(
            vec![
                (x.index(&0).unwrap(), 10.0),
                (x.index(&1).unwrap(), 20.0),
            ],
            1.0,
        );

        obj.register(&mut model);
        obj.invoke(&model).unwrap();
    }

    #[test]
    fn better_layer_maximization_register() {
        let mut model = MetaModel::<f64>::new("test_better_layer");

        let mut x: VariableArray1<usize, ospf_rust_core::variable::variable_item::ContinuousVariableItem> = VariableArray1::new("x");
        x.register_continuous(&[0usize, 1usize], &mut model).unwrap();

        let obj = BetterLayerMaximization::new(
            vec![
                (x.index(&0).unwrap(), 5.0),
                (x.index(&1).unwrap(), 3.0),
            ],
            1.0,
        );

        obj.register(&mut model);
        obj.invoke(&model).unwrap();
    }

    #[test]
    fn tail_bin_assignment_constraint_register() {
        let mut model = MetaModel::<f64>::new("test_tail_bin");

        let mut v: VariableArray1<usize, ospf_rust_core::variable::variable_item::BinaryVariableItem> = VariableArray1::new("v");
        v.register_binary(&[0usize, 1usize, 2usize], &mut model).unwrap();

        let constraint = TailBinAssignmentConstraint::new(
            vec![v.index(&0).unwrap(), v.index(&1).unwrap(), v.index(&2).unwrap()],
        );

        constraint.register(&mut model);
        constraint.invoke(&model).unwrap();

        // 对于 3 个箱，应有 C(3,2)=3 个约束
        // v[1] <= v[0], v[2] <= v[0], v[2] <= v[1]
    }

    #[test]
    fn deferred_limits_report_diagnostics() {
        assert!(RestAmountMinimization::new()
            .diagnostics()[0]
            .contains("not implemented"));
        assert!(TailBinLoadingRateMinimization::new()
            .diagnostics()[0]
            .contains("not implemented"));
        assert!(BinLoadingOrderConstraint::new()
            .diagnostics()[0]
            .contains("not implemented"));
        let objective_plan = RestAmountMinimization::new().registration_plan(vec![1, 2]);
        let constraint_plan = BinLoadingOrderConstraint::new()
            .constraint_registration_plan(vec![3, 4]);

        assert_eq!(objective_plan.objective_family.as_deref(), Some("rest_amount_minimization"));
        assert_eq!(objective_plan.variable_indices, vec![1, 2]);
        assert_eq!(constraint_plan.constraint_family.as_deref(), Some("bin_loading_order_constraint"));
        assert_eq!(constraint_plan.variable_indices, vec![3, 4]);

        let mut model = MetaModel::<f64>::new("deferred_register");
        let mut v: VariableArray1<usize, ospf_rust_core::variable::variable_item::ContinuousVariableItem> =
            VariableArray1::new("v");
        v.register_continuous(&[0usize, 1usize], &mut model).unwrap();
        let indices = vec![v.index(&0).unwrap(), v.index(&1).unwrap()];
        let _ = RestAmountMinimization::new()
            .register_objective(&mut model, indices.clone(), 1.0);
        let _ = TailBinLoadingRateMinimization::new()
            .register_objective(&mut model, indices.clone(), 0.5);
        assert_eq!(model.objective().sub_objectives.len(), 2);
        assert_eq!(model.objective().sub_objectives[0].name, "rest_amount_minimization");
        assert_eq!(model.objective().sub_objectives[1].name, "tail_bin_loading_rate_minimization");
        let before_constraints = model.num_constraints();
        let _ = BinLoadingOrderConstraint::new()
            .register_constraint(&mut model, indices);

        assert!(model.num_constraints() > before_constraints);
    }

    #[test]
    fn deferred_limits_register_production_semantic_rows() {
        let mut model = MetaModel::<f64>::new("production_semantic_limits");
        let mut v: VariableArray1<usize, ospf_rust_core::variable::variable_item::ContinuousVariableItem> =
            VariableArray1::new("v");
        v.register_continuous(&[0usize, 1usize, 2usize], &mut model).unwrap();
        let indices = vec![
            v.index(&0).unwrap(),
            v.index(&1).unwrap(),
            v.index(&2).unwrap(),
        ];

        let rest_plan = RestAmountMinimization::new()
            .register_objective(&mut model, indices.clone(), 2.0);
        let tail_plan = TailBinLoadingRateMinimization::new()
            .register_objective(&mut model, indices.clone(), 0.25);
        let before_constraints = model.num_constraints();
        let order_plan = BinLoadingOrderConstraint::new()
            .register_constraint(&mut model, indices.clone());

        assert_eq!(rest_plan.objective_family.as_deref(), Some("rest_amount_minimization"));
        assert_eq!(tail_plan.objective_family.as_deref(), Some("tail_bin_loading_rate_minimization"));
        assert_eq!(order_plan.constraint_family.as_deref(), Some("bin_loading_order_constraint"));
        assert_eq!(rest_plan.variable_indices, indices);
        assert_eq!(model.objective().sub_objectives.len(), 2);
        assert_eq!(model.num_constraints(), before_constraints + 2);
    }

    #[test]
    fn scaled_adapter_with_bin_capacity() {
        let mut model = MetaModel::<f64>::new("test_scaled_cap");

        let bin_keys = vec![0usize];
        let layer_keys = vec![0usize];
        let mut x: VariableArray2<usize, usize, ospf_rust_core::variable::variable_item::BinaryVariableItem> = VariableArray2::new("x");
        x.register_binary(&bin_keys, &layer_keys, &mut model).unwrap();

        let x_indices: Vec<Vec<(usize, usize)>> = vec![
            vec![(0, x.index(&0, &0).unwrap())],
        ];

        let constraint = BinCapacityConstraint::from_bins(
            &[make_bin_type()],
            x_indices,
            vec![5.0],
            vec![20.0],
            &Bpp3dSolverValueAdapterKind::Scaled(ScaledBpp3dSolverValueAdapter::ten_times()),
        );

        constraint.register(&mut model);
        constraint.invoke(&model).unwrap();
    }

    #[test]
    fn bin_capacity_constraint_direct() {
        let mut model = MetaModel::<f64>::new("test_bin_cap_direct");

        let bin_keys = vec![0usize];
        let layer_keys = vec![0usize];
        let mut x: VariableArray2<usize, usize, ospf_rust_core::variable::variable_item::BinaryVariableItem> = VariableArray2::new("x");
        x.register_binary(&bin_keys, &layer_keys, &mut model).unwrap();

        let x_indices: Vec<Vec<(usize, usize)>> = vec![
            vec![(0, x.index(&0, &0).unwrap())],
        ];

        let constraint = BinCapacityConstraint::new(
            vec![100.0],    // weight capacity
            vec![500.0],    // volume capacity
            x_indices,
            vec![5.0],      // layer weights
            vec![20.0],     // layer volumes
        );

        constraint.register(&mut model);
        constraint.invoke(&model).unwrap();
    }

    #[test]
    fn bin_depth_constraint_direct() {
        let mut model = MetaModel::<f64>::new("test_bin_depth_direct");

        let bin_keys = vec![0usize];
        let layer_keys = vec![0usize];
        let mut x: VariableArray2<usize, usize, ospf_rust_core::variable::variable_item::BinaryVariableItem> = VariableArray2::new("x");
        x.register_binary(&bin_keys, &layer_keys, &mut model).unwrap();

        let x_indices: Vec<Vec<(usize, usize)>> = vec![
            vec![(0, x.index(&0, &0).unwrap())],
        ];

        let constraint = BinDepthConstraint::new(
            vec![10.0],      // depth capacity
            x_indices,
            vec![2.0],       // layer depths
        );

        constraint.register(&mut model);
        constraint.invoke(&model).unwrap();
    }
}
