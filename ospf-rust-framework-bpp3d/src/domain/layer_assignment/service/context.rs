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

