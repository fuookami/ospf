//! 层分配服务 / Layer assignment services
//!
//! RMP/final MILP 赋值模型、约束和目标族。
//! RMP/final MILP assignment models, constraints, and objective families.

pub mod limits;

use std::fmt::Debug;
use ospf_rust_core::model::meta_model::MetaModel;
use ospf_rust_framework::model::pipeline::Pipeline;

use super::model::{Bpp3dModelComponent, ExpressionArray1, VariableArray1, VariableArray2};
use crate::domain::item::{Bpp3dDemandKey, Bpp3dDemandMode, BinLayer, BinType};

// ============================================================================
// Bpp3dSolverValueAdapter - 求解器值适配器 / Solver value adapter
// ============================================================================

/// 求解器值适配器 / Solver value adapter
///
/// 将业务类型转换为求解器数值类型。
/// Converts business types to solver numeric types.
pub trait Bpp3dSolverValueAdapter: Debug + Clone + Send + Sync {
    /// 数量转求解器值 / Amount to solver value
    fn amount_to_solver(&self, value: u64) -> f64;

    /// 长度转求解器值 / Length to solver value
    fn length_to_solver(&self, value: f64) -> f64;

    /// 重量转求解器值 / Weight to solver value
    fn weight_to_solver(&self, value: f64) -> f64;

    /// 体积转求解器值 / Volume to solver value
    fn volume_to_solver(&self, value: f64) -> f64;

    /// 深度转求解器值 / Depth to solver value
    fn depth_to_solver(&self, value: f64) -> f64;
}

/// 默认求解器值适配器 / Default solver value adapter
#[derive(Debug, Clone, Default)]
pub struct DefaultBpp3dSolverValueAdapter;

impl Bpp3dSolverValueAdapter for DefaultBpp3dSolverValueAdapter {
    fn amount_to_solver(&self, value: u64) -> f64 { value as f64 }
    fn length_to_solver(&self, value: f64) -> f64 { value }
    fn weight_to_solver(&self, value: f64) -> f64 { value }
    fn volume_to_solver(&self, value: f64) -> f64 { value }
    fn depth_to_solver(&self, value: f64) -> f64 { value }
}

/// 缩放求解器值适配器 / Scaled solver value adapter
///
/// 对每种物理量提供独立的缩放因子，用于求解器精度优化。
/// Provides independent scaling factors for each physical quantity,
/// used for solver precision optimization.
#[derive(Debug, Clone)]
pub struct ScaledBpp3dSolverValueAdapter {
    /// 数量缩放因子 / Amount scale factor
    pub amount_scale: f64,
    /// 长度缩放因子 / Length scale factor
    pub length_scale: f64,
    /// 重量缩放因子 / Weight scale factor
    pub weight_scale: f64,
    /// 体积缩放因子 / Volume scale factor
    pub volume_scale: f64,
    /// 深度缩放因子 / Depth scale factor
    pub depth_scale: f64,
}

impl ScaledBpp3dSolverValueAdapter {
    /// 创建缩放适配器 / Create a scaled adapter
    pub fn new(
        amount_scale: f64,
        length_scale: f64,
        weight_scale: f64,
        volume_scale: f64,
        depth_scale: f64,
    ) -> Self {
        Self {
            amount_scale,
            length_scale,
            weight_scale,
            volume_scale,
            depth_scale,
        }
    }

    /// 创建十倍缩放适配器 / Create a 10x scaled adapter
    pub fn ten_times() -> Self {
        Self {
            amount_scale: 10.0,
            length_scale: 10.0,
            weight_scale: 10.0,
            volume_scale: 10.0,
            depth_scale: 10.0,
        }
    }
}

impl Default for ScaledBpp3dSolverValueAdapter {
    fn default() -> Self {
        Self {
            amount_scale: 1.0,
            length_scale: 1.0,
            weight_scale: 1.0,
            volume_scale: 1.0,
            depth_scale: 1.0,
        }
    }
}

impl Bpp3dSolverValueAdapter for ScaledBpp3dSolverValueAdapter {
    fn amount_to_solver(&self, value: u64) -> f64 { value as f64 * self.amount_scale }
    fn length_to_solver(&self, value: f64) -> f64 { value * self.length_scale }
    fn weight_to_solver(&self, value: f64) -> f64 { value * self.weight_scale }
    fn volume_to_solver(&self, value: f64) -> f64 { value * self.volume_scale }
    fn depth_to_solver(&self, value: f64) -> f64 { value * self.depth_scale }
}

// ============================================================================
// Bpp3dSolverValueAdapterKind - 求解器值适配器枚举 / Solver value adapter enum
// ============================================================================

/// 求解器值适配器枚举 / Solver value adapter enum
///
/// 枚举已知的适配器类型，避免 `dyn` trait 兼容性问题。
/// Enumerates known adapter types, avoiding `dyn` trait compatibility issues.
#[derive(Debug, Clone)]
pub enum Bpp3dSolverValueAdapterKind {
    /// 默认适配器 / Default adapter
    Default,
    /// 缩放适配器 / Scaled adapter
    Scaled(ScaledBpp3dSolverValueAdapter),
}

impl Default for Bpp3dSolverValueAdapterKind {
    fn default() -> Self {
        Self::Default
    }
}

impl Bpp3dSolverValueAdapter for Bpp3dSolverValueAdapterKind {
    fn amount_to_solver(&self, value: u64) -> f64 {
        match self {
            Self::Default => DefaultBpp3dSolverValueAdapter.amount_to_solver(value),
            Self::Scaled(s) => s.amount_to_solver(value),
        }
    }
    fn length_to_solver(&self, value: f64) -> f64 {
        match self {
            Self::Default => DefaultBpp3dSolverValueAdapter.length_to_solver(value),
            Self::Scaled(s) => s.length_to_solver(value),
        }
    }
    fn weight_to_solver(&self, value: f64) -> f64 {
        match self {
            Self::Default => DefaultBpp3dSolverValueAdapter.weight_to_solver(value),
            Self::Scaled(s) => s.weight_to_solver(value),
        }
    }
    fn volume_to_solver(&self, value: f64) -> f64 {
        match self {
            Self::Default => DefaultBpp3dSolverValueAdapter.volume_to_solver(value),
            Self::Scaled(s) => s.volume_to_solver(value),
        }
    }
    fn depth_to_solver(&self, value: f64) -> f64 {
        match self {
            Self::Default => DefaultBpp3dSolverValueAdapter.depth_to_solver(value),
            Self::Scaled(s) => s.depth_to_solver(value),
        }
    }
}

// ============================================================================
// ImpreciseAssignment - 不精确赋值 / Imprecise assignment
// ============================================================================

/// 不精确赋值 / Imprecise assignment
///
/// 列生成 RMP 阶段的赋值模型，x[layer] 为列变量。
/// Assignment model for the column generation RMP phase,
/// where x[layer] is a column variable.
#[derive(Debug, Clone)]
pub struct ImpreciseAssignment<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 层列表 / Layers
    pub layers: Vec<BinLayer<V, U>>,
    /// 列变量 x[layer] / Column variables x[layer]
    pub x: VariableArray1<usize, ospf_rust_core::variable::variable_item::ContinuousVariableItem>,
}

impl<V: Debug + Clone + Send + Sync, U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync> Bpp3dModelComponent for ImpreciseAssignment<V, U> {
    fn name(&self) -> &str {
        "ImpreciseAssignment"
    }

    fn register(&mut self, model: &mut MetaModel<f64>) -> Result<(), String> {
        let keys: Vec<usize> = (0..self.layers.len()).collect();
        self.x.register_continuous(&keys, model)
    }
}

// ============================================================================
// PreciseAssignment - 精确赋值 / Precise assignment
// ============================================================================

/// 精确赋值 / Precise assignment
///
/// Final MILP 阶段的赋值模型，x[bin, layer] 为二值赋值变量。
/// Assignment model for the final MILP phase,
/// where x[bin, layer] is a binary assignment variable.
#[derive(Debug, Clone)]
pub struct PreciseAssignment<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 箱列表 / Bins
    pub bins: Vec<BinType<V, U>>,
    /// 层列表 / Layers
    pub layers: Vec<BinLayer<V, U>>,
    /// 赋值变量 x[bin, layer] / Assignment variables x[bin, layer]
    pub x: VariableArray2<usize, usize, ospf_rust_core::variable::variable_item::BinaryVariableItem>,
    /// 箱使用标记 v[bin] / Bin usage markers v[bin]
    pub v: VariableArray1<usize, ospf_rust_core::variable::variable_item::BinaryVariableItem>,
}

impl<V: Debug + Clone + Send + Sync, U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync> Bpp3dModelComponent for PreciseAssignment<V, U> {
    fn name(&self) -> &str {
        "PreciseAssignment"
    }

    fn register(&mut self, model: &mut MetaModel<f64>) -> Result<(), String> {
        let bin_keys: Vec<usize> = (0..self.bins.len()).collect();
        let layer_keys: Vec<usize> = (0..self.layers.len()).collect();

        self.x.register_binary(&bin_keys, &layer_keys, model)?;
        self.v.register_binary(&bin_keys, model)?;
        Ok(())
    }
}

// ============================================================================
// Load - 负载模型 / Load model
// ============================================================================

/// 需求条目 / Demand entry
#[derive(Debug, Clone)]
pub struct Bpp3dDemandEntry {
    /// 需求模式 / Demand mode
    pub mode: Bpp3dDemandMode,
    /// 需求键 / Demand key
    pub key: Bpp3dDemandKey,
    /// 需求值 / Demand value
    pub demand: f64,
}

/// 负载模型 / Load model
///
/// 管理需求约束的中间表达式。
/// Manages intermediate expressions for demand constraints.
#[derive(Debug)]
pub struct Load {
    /// 需求条目 / Demand entries
    pub demand_entries: Vec<Bpp3dDemandEntry>,
    /// 负载表达式 load[layer] / Load expressions load[layer]
    pub load: ExpressionArray1<usize>,
    /// 过载表达式 overLoad[layer] / Overload expressions
    pub over_load: ExpressionArray1<usize>,
    /// 欠载表达式 lessLoad[layer] / Less-load expressions
    pub less_load: ExpressionArray1<usize>,
}

impl Load {
    /// 创建负载模型 / Create a load model
    pub fn new(demand_entries: Vec<Bpp3dDemandEntry>) -> Self {
        Self {
            demand_entries,
            load: ExpressionArray1::new("load"),
            over_load: ExpressionArray1::new("overLoad"),
            less_load: ExpressionArray1::new("lessLoad"),
        }
    }
}

// ============================================================================
// Capacity - 容量模型 / Capacity model
// ============================================================================

/// 容量模型 / Capacity model
///
/// 管理载重、体积、深度和装载率表达式。
/// Manages load weight, volume, depth, and loading rate expressions.
#[derive(Debug)]
pub struct Capacity {
    /// 载重表达式 / Load weight expressions
    pub load_weight: ExpressionArray1<usize>,
    /// 载体积表达式 / Load volume expressions
    pub load_volume: ExpressionArray1<usize>,
    /// 载深表达式 / Load depth expressions
    pub load_depth: ExpressionArray1<usize>,
}

impl Capacity {
    /// 创建容量模型 / Create a capacity model
    pub fn new() -> Self {
        Self {
            load_weight: ExpressionArray1::new("loadWeight"),
            load_volume: ExpressionArray1::new("loadVolume"),
            load_depth: ExpressionArray1::new("loadDepth"),
        }
    }
}

impl Default for Capacity {
    fn default() -> Self {
        Self::new()
    }
}

/// 精确负载容量 / Precise load capacity
///
/// Final MILP 阶段的容量约束数据，包含每个箱的载重、体积和深度上界。
/// Capacity constraint data for the final MILP phase, containing per-bin
/// load weight, volume, and depth upper bounds.
#[derive(Debug, Clone)]
pub struct PreciseLoadCapacity {
    /// 载重上界 / Weight upper bound
    pub weight_capacity: f64,
    /// 体积上界 / Volume upper bound
    pub volume_capacity: f64,
    /// 深度上界 / Depth upper bound
    pub depth_capacity: f64,
}

// ============================================================================
// LayerAggregation - 层聚合 / Layer aggregation
// ============================================================================

/// 层聚合 / Layer aggregation
///
/// 管理列生成过程中的层集合。
/// Manages the layer set during column generation.
#[derive(Debug, Clone)]
pub struct LayerAggregation<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 所有层 / All layers
    pub layers: Vec<BinLayer<V, U>>,
    /// 每次迭代新增的层 / Layers added per iteration
    pub iterations: Vec<Vec<BinLayer<V, U>>>,
}

impl<V, U> LayerAggregation<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 创建层聚合 / Create a layer aggregation
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            iterations: Vec::new(),
        }
    }

    /// 添加新层（带去重）/ Add new layers (with deduplication)
    pub fn add_columns(&mut self, new_layers: Vec<BinLayer<V, U>>) -> Vec<BinLayer<V, U>>
    where
        V: Clone + PartialEq,
    {
        let mut added = Vec::new();
        for layer in new_layers {
            // 简化去重：基于深度和来源
            let is_dup = self.layers.iter().any(|existing| {
                existing.depth.value == layer.depth.value && existing.from == layer.from
            });
            if !is_dup {
                self.layers.push(layer.clone());
                added.push(layer);
            }
        }
        if !added.is_empty() {
            self.iterations.push(added.clone());
        }
        added
    }

    /// 获取上一次迭代新增的层 / Get layers added in the last iteration
    pub fn last_iteration_layers(&self) -> &[BinLayer<V, U>] {
        self.iterations.last().map(|v| v.as_slice()).unwrap_or(&[])
    }
}

impl<V, U> Default for LayerAggregation<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 需求影子价格键 / Demand shadow price key
// ============================================================================

/// 需求影子价格键 / Demand shadow price key
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DemandShadowPriceKey {
    /// 需求模式 / Demand mode
    pub mode: Bpp3dDemandMode,
    /// 需求键 / Demand key
    pub key: Bpp3dDemandKey,
}

// ============================================================================
// SolutionAnalyzer - 解分析器 / Solution analyzer
// ============================================================================

/// 解分析器 / Solution analyzer
///
/// 分析层分配求解结果。
/// Analyzes layer assignment solving results.
#[derive(Debug, Clone, Default)]
pub struct SolutionAnalyzer;

// ============================================================================
// LayerAssignmentAggregation - 层分配聚合 / Layer assignment aggregation
// ============================================================================

/// 层分配聚合 / Layer assignment aggregation
///
/// 编排赋值、负载、容量组件注册到 MetaModel。
/// Orchestrates registration of assignment, load, and capacity components to MetaModel.
#[derive(Debug)]
pub struct LayerAssignmentAggregation<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 不精确赋值（RMP 阶段）/ Imprecise assignment (RMP phase)
    pub imprecise_assignment: Option<ImpreciseAssignment<V, U>>,
    /// 精确赋值（Final MILP 阶段）/ Precise assignment (Final MILP phase)
    pub precise_assignment: Option<PreciseAssignment<V, U>>,
    /// 负载模型 / Load model
    pub load: Option<Load>,
    /// 容量模型 / Capacity model
    pub capacity: Option<Capacity>,
}

impl<V, U> LayerAssignmentAggregation<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 创建空聚合 / Create empty aggregation
    pub fn new() -> Self {
        Self {
            imprecise_assignment: None,
            precise_assignment: None,
            load: None,
            capacity: None,
        }
    }

    /// 创建 RMP 阶段聚合 / Create RMP phase aggregation
    pub fn rmp(assignment: ImpreciseAssignment<V, U>, load: Load, capacity: Capacity) -> Self {
        Self {
            imprecise_assignment: Some(assignment),
            precise_assignment: None,
            load: Some(load),
            capacity: Some(capacity),
        }
    }

    /// 创建 Final MILP 阶段聚合 / Create Final MILP phase aggregation
    pub fn final_milp(assignment: PreciseAssignment<V, U>, load: Load, capacity: Capacity) -> Self {
        Self {
            imprecise_assignment: None,
            precise_assignment: Some(assignment),
            load: Some(load),
            capacity: Some(capacity),
        }
    }

    /// 注册所有组件到模型 / Register all components to model
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> Result<(), String> {
        if let Some(ref mut assignment) = self.imprecise_assignment {
            assignment.register(model)?;
        }
        if let Some(ref mut assignment) = self.precise_assignment {
            assignment.register(model)?;
        }
        // Load and Capacity hold intermediate expressions,
        // not model variables, so they don't register to MetaModel directly.
        // They are populated by limits during invoke().
        Ok(())
    }
}

impl<V, U> Default for LayerAssignmentAggregation<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// LayerAssignmentContext - 层分配上下文 / Layer assignment context
// ============================================================================

/// 层分配上下文 / Layer assignment context
///
/// 作为应用层入口，组装聚合和限制 Pipeline。
/// Entry point for the application layer, assembling aggregation and limit pipelines.
pub struct LayerAssignmentContext<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 层分配聚合 / Layer assignment aggregation
    pub aggregation: LayerAssignmentAggregation<V, U>,
    /// 限制 Pipeline 列表 / Limit pipeline list
    pub limits: Vec<Box<dyn Pipeline<MetaModel<f64>>>>,
    /// 目标 Pipeline 列表 / Objective pipeline list
    pub objectives: Vec<Box<dyn Pipeline<MetaModel<f64>>>>,
}

impl<V, U> LayerAssignmentContext<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 创建新的层分配上下文 / Create new layer assignment context
    pub fn new(aggregation: LayerAssignmentAggregation<V, U>) -> Self {
        Self {
            aggregation,
            limits: Vec::new(),
            objectives: Vec::new(),
        }
    }

    /// 添加限制 Pipeline / Add limit pipeline
    pub fn add_limit(&mut self, limit: Box<dyn Pipeline<MetaModel<f64>>>) {
        self.limits.push(limit);
    }

    /// 添加目标 Pipeline / Add objective pipeline
    pub fn add_objective(&mut self, objective: Box<dyn Pipeline<MetaModel<f64>>>) {
        self.objectives.push(objective);
    }

    /// 注册到模型 / Register to model
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> Result<(), String> {
        self.aggregation.register(model)?;
        for limit in &self.limits {
            limit.register(model);
        }
        for obj in &self.objectives {
            obj.register(model);
        }
        Ok(())
    }

    /// 调用验证 / Invoke validation
    pub fn invoke(&self, model: &MetaModel<f64>) -> Result<(), String> {
        for limit in &self.limits {
            limit.invoke(model)
                .map_err(|e| format!("Limit invoke failed: {:?}", e))?;
        }
        for obj in &self.objectives {
            obj.invoke(model)
                .map_err(|e| format!("Objective invoke failed: {:?}", e))?;
        }
        Ok(())
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_quantities::unit::derived::Meter;
    use ospf_rust_quantities::quantity::Quantity;

    fn meters(v: f64) -> Quantity<f64, Meter> {
        Quantity::new_ct(v)
    }

    #[test]
    fn default_solver_value_adapter() {
        let adapter = DefaultBpp3dSolverValueAdapter;
        assert_eq!(adapter.amount_to_solver(10), 10.0);
        assert_eq!(adapter.length_to_solver(1.5), 1.5);
    }

    #[test]
    fn scaled_solver_value_adapter() {
        let adapter = ScaledBpp3dSolverValueAdapter::ten_times();
        assert_eq!(adapter.amount_to_solver(5), 50.0);
        assert_eq!(adapter.length_to_solver(1.5), 15.0);
        assert_eq!(adapter.weight_to_solver(2.0), 20.0);
        assert_eq!(adapter.volume_to_solver(3.0), 30.0);
        assert_eq!(adapter.depth_to_solver(1.0), 10.0);
    }

    #[test]
    fn scaled_solver_value_adapter_default() {
        let adapter = ScaledBpp3dSolverValueAdapter::default();
        assert_eq!(adapter.amount_to_solver(5), 5.0);
        assert_eq!(adapter.length_to_solver(1.5), 1.5);
    }

    #[test]
    fn layer_aggregation_add_columns() {
        let mut agg: LayerAggregation<f64, Meter> = LayerAggregation::new();
        assert!(agg.layers.is_empty());

        let layer1 = BinLayer {
            iteration: 0,
            from: "test".to_string(),
            bin: None,
            depth: meters(1.0),
            demand_coverage: Vec::new(),
        };
        let layer2 = BinLayer {
            iteration: 0,
            from: "test".to_string(),
            bin: None,
            depth: meters(2.0),
            demand_coverage: Vec::new(),
        };

        let added = agg.add_columns(vec![layer1.clone(), layer2.clone()]);
        assert_eq!(added.len(), 2);
        assert_eq!(agg.layers.len(), 2);
        assert_eq!(agg.last_iteration_layers().len(), 2);
    }

    #[test]
    fn layer_aggregation_deduplication() {
        let mut agg: LayerAggregation<f64, Meter> = LayerAggregation::new();

        let layer = BinLayer {
            iteration: 0,
            from: "test".to_string(),
            bin: None,
            depth: meters(1.0),
            demand_coverage: Vec::new(),
        };

        agg.add_columns(vec![layer.clone()]);
        assert_eq!(agg.layers.len(), 1);

        // 重复添加应被去重
        agg.add_columns(vec![layer.clone()]);
        assert_eq!(agg.layers.len(), 1);
    }

    #[test]
    fn load_model_construction() {
        let entries = vec![
            Bpp3dDemandEntry {
                mode: Bpp3dDemandMode::Item,
                key: Bpp3dDemandKey::Item { id: "item1".to_string() },
                demand: 10.0,
            },
        ];
        let load = Load::new(entries);
        assert_eq!(load.demand_entries.len(), 1);
    }

    #[test]
    fn capacity_model_construction() {
        let capacity = Capacity::new();
        assert!(capacity.load_weight.is_empty());
        assert!(capacity.load_volume.is_empty());
    }

    #[test]
    fn precise_load_capacity() {
        let plc = PreciseLoadCapacity {
            weight_capacity: 100.0,
            volume_capacity: 50.0,
            depth_capacity: 10.0,
        };
        assert_eq!(plc.weight_capacity, 100.0);
        assert_eq!(plc.volume_capacity, 50.0);
        assert_eq!(plc.depth_capacity, 10.0);
    }

    #[test]
    fn demand_shadow_price_key() {
        let key = DemandShadowPriceKey {
            mode: Bpp3dDemandMode::Item,
            key: Bpp3dDemandKey::Item { id: "item1".to_string() },
        };
        assert_eq!(key.mode, Bpp3dDemandMode::Item);
    }

    #[test]
    fn layer_assignment_aggregation_rmp() {
        let assignment = ImpreciseAssignment {
            layers: vec![BinLayer {
                iteration: 0,
                from: "test".to_string(),
                bin: None,
                depth: meters(1.0),
                demand_coverage: Vec::new(),
            }],
            x: VariableArray1::new("x"),
        };
        let load = Load::new(vec![]);
        let capacity = Capacity::new();
        let agg = LayerAssignmentAggregation::rmp(assignment, load, capacity);
        assert!(agg.imprecise_assignment.is_some());
        assert!(agg.precise_assignment.is_none());
    }

    #[test]
    fn layer_assignment_context_register() {
        let mut model = MetaModel::<f64>::new("test_bpp3d_context");

        let assignment = ImpreciseAssignment {
            layers: vec![BinLayer {
                iteration: 0,
                from: "test".to_string(),
                bin: None,
                depth: meters(1.0),
                demand_coverage: Vec::new(),
            }],
            x: VariableArray1::new("x"),
        };
        let load = Load::new(vec![]);
        let capacity = Capacity::new();
        let agg = LayerAssignmentAggregation::rmp(assignment, load, capacity);
        let mut ctx = LayerAssignmentContext::new(agg);

        ctx.register(&mut model).unwrap();
        ctx.invoke(&model).unwrap();
    }
}
