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

