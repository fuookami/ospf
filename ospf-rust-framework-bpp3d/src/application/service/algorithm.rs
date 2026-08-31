// ColumnGenerationAlgorithm - 列生成算法 / Column generation algorithm
// ============================================================================

/// 列生成算法 / Column generation algorithm
///
/// 只编排层生成与列集合状态，RMP/final MILP 建模仍由 domain context 负责。
/// Orchestrates layer generation and column-set state only; RMP/final MILP
/// modeling remains owned by domain contexts.
#[derive(Debug)]
pub struct ColumnGenerationAlgorithm<V, U>
where
    V: Field + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
    U: UnitTrait + Debug + Clone + Send + Sync,
{
    /// 配置 / Config
    pub config: ColumnGenerationConfig,
    /// 层生成上下文 / Layer generation context
    pub layer_generation: LayerGenerationContext<V, U>,
    /// 层聚合 / Layer aggregation
    pub layer_aggregation: LayerAggregation<V, U>,
    /// 迭代层赋值上下文 / Iterative layer assignment context
    pub iterative_assignment: IterativeLayerAssignmentContext<V, U>,
    /// 动态模型生命周期 / Dynamic model lifecycle
    pub dynamic_lifecycle: DynamicModelLifecycle,
    /// 状态 / State
    pub state: ColumnGenerationState,
}

impl<V, U> ColumnGenerationAlgorithm<V, U>
where
    V: Field + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
    U: UnitTrait + Debug + Clone + Send + Sync,
{
    /// 创建算法 / Create algorithm
    pub fn new(config: ColumnGenerationConfig) -> Self {
        Self {
            config,
            layer_generation: LayerGenerationContext::new(),
            layer_aggregation: LayerAggregation::new(),
            iterative_assignment: IterativeLayerAssignmentContext::new(),
            dynamic_lifecycle: DynamicModelLifecycle::new(),
            state: ColumnGenerationState::new(),
        }
    }

    /// 使用层生成上下文创建算法 / Create algorithm with layer generation context
    pub fn with_layer_generation(
        config: ColumnGenerationConfig,
        layer_generation: LayerGenerationContext<V, U>,
    ) -> Self {
        Self {
            config,
            layer_generation,
            layer_aggregation: LayerAggregation::new(),
            iterative_assignment: IterativeLayerAssignmentContext::new(),
            dynamic_lifecycle: DynamicModelLifecycle::new(),
            state: ColumnGenerationState::new(),
        }
    }

    /// 添加初始层 / Add initial layers
    pub fn add_initial_layers(&mut self, layers: Vec<BinLayer<V, U>>) -> Vec<BinLayer<V, U>> {
        let added = self.add_columns_with_iteration(0, layers);
        added
    }

    /// 添加层列 / Add layer columns
    pub fn add_columns(
        &mut self,
        iteration: usize,
        layers: Vec<BinLayer<V, U>>,
    ) -> Vec<BinLayer<V, U>> {
        self.add_columns_with_iteration(iteration, layers)
    }

    /// 移除层列 / Remove layer columns
    pub fn remove_columns(&mut self, columns: impl IntoIterator<Item = usize>) {
        self.iterative_assignment.remove_columns(columns);
    }

    /// 活跃层集合 / Active layers
    pub fn active_layers(&self) -> Vec<BinLayer<V, U>> {
        self.iterative_assignment.active_layers()
    }

    /// 活跃列数量 / Active column count
    pub fn active_column_count(&self) -> usize {
        self.iterative_assignment.active_column_count()
    }

    /// 已移除列数量 / Removed column count
    pub fn removed_column_count(&self) -> usize {
        self.iterative_assignment.removed_column_count()
    }

    fn add_columns_with_iteration(
        &mut self,
        iteration: usize,
        layers: Vec<BinLayer<V, U>>,
    ) -> Vec<BinLayer<V, U>> {
        let added = self
            .iterative_assignment
            .add_columns(iteration, layers, Vec::new())
            .into_iter()
            .map(|column| column.layer)
            .collect::<Vec<_>>();
        self.layer_aggregation = self.iterative_assignment.aggregation.clone();
        self.state.register_columns(added.len());
        added
    }

    /// 执行一轮层生成 / Execute one layer-generation iteration
    pub fn generate_once(
        &mut self,
        bin: Option<BinType<V, U>>,
        items: Vec<ActualItem<V, U>>,
        demand_entries: Vec<LayerGenerationDemandEntry>,
    ) -> Vec<LayerGenerationResult<V, U>> {
        if self.state.status == ColumnGenerationStatus::NotStarted {
            self.state.start();
        }

        let mut request = LayerGenerationRequest::new(self.state.iteration as i64, items)
            .with_demand_entries(demand_entries)
            .with_max_candidates(self.config.max_candidates_per_iteration);
        request.existing_layers = self.active_layers();
        request.bin = bin;

        let results = self.layer_generation.generate(&request);
        let layers = results.iter().map(|result| result.layer.clone()).collect();
        self.add_columns(self.state.iteration, layers);

        self.state.advance_iteration();
        results
    }

    /// 判断是否继续 / Check whether to continue
    pub fn should_continue(&mut self) -> bool {
        self.state.should_continue(&self.config)
    }

    /// 构造当前结果 / Build current result
    pub fn result(&self) -> ColumnGenerationResult<V, U> {
        ColumnGenerationResult {
            state: self.state.clone(),
            layers: self.active_layers(),
            packing_result: None,
            render_loading_plans: Vec::new(),
            info: HashMap::from([
                (
                    "framework_lifecycle_active_column_count".to_string(),
                    self.active_column_count().to_string(),
                ),
                (
                    "framework_lifecycle_removed_column_count".to_string(),
                    self.removed_column_count().to_string(),
                ),
                (
                    "framework_lifecycle_flush_count".to_string(),
                    self.dynamic_lifecycle.flush_count().to_string(),
                ),
            ]),
        }
    }
}

// ============================================================================
