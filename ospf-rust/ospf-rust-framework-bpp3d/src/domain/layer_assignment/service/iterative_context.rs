// IterativeLayerAssignmentContext - 迭代层赋值上下文 / Iterative layer assignment context
// ============================================================================

/// 迭代层列 / Iterative layer column
///
/// 记录领域层列索引、所属迭代以及对应的
/// RMP `x[layer]` 模型变量索引。
/// Records the domain layer-column index, owning iteration, and corresponding
/// RMP `x[layer]` model variable index.
#[derive(Debug, Clone)]
pub struct IterativeLayerColumn<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 列索引 / Column index
    pub index: usize,
    /// 所属迭代 / Owning iteration
    pub iteration: usize,
    /// 层候选 / Layer candidate
    pub layer: BinLayer<V, U>,
    /// x 变量模型索引 / x variable model index
    pub x_model_index: Option<usize>,
    /// 列变量上界 / Column upper bound
    pub upper_bound: Option<f64>,
}

/// 迭代层赋值上下文 / Iterative layer assignment context
///
/// 将 BPP3D 的领域层列映射到 framework 公共动态生命周期，
/// 支持 add/remove/hide/fix/flush 和 solution extraction 的统一列状态。
/// Maps BPP3D domain layer columns to the shared framework dynamic lifecycle,
/// supporting unified add/remove/hide/fix/flush and solution extraction state.
#[derive(Debug, Clone)]
pub struct IterativeLayerAssignmentContext<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 层聚合 / Layer aggregation
    pub aggregation: LayerAggregation<V, U>,
    /// 列记录 / Column records
    pub columns: Vec<IterativeLayerColumn<V, U>>,
    /// 业务列索引到模型变量索引 / Domain column index to model variable index
    pub column_to_model_index: HashMap<usize, usize>,
    /// 移除列集合 / Removed column set
    pub removed_columns: HashSet<usize>,
    /// 列计数器 / Column counter
    column_counter: usize,
}

impl<V, U> IterativeLayerAssignmentContext<V, U>
where
    V: Debug + Clone + Send + Sync + PartialEq,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 创建上下文 / Create context
    pub fn new() -> Self {
        Self {
            aggregation: LayerAggregation::new(),
            columns: Vec::new(),
            column_to_model_index: HashMap::new(),
            removed_columns: HashSet::new(),
            column_counter: 0,
        }
    }

    /// 从聚合创建上下文 / Create context from aggregation
    pub fn from_aggregation(aggregation: LayerAggregation<V, U>) -> Self {
        let mut context = Self::new();
        for (index, layer) in aggregation.layers.iter().cloned().enumerate() {
            context.columns.push(IterativeLayerColumn {
                index,
                iteration: layer.iteration.max(0) as usize,
                layer,
                x_model_index: None,
                upper_bound: None,
            });
            context.column_counter = context.column_counter.max(index + 1);
        }
        context.aggregation = aggregation;
        context
    }

    /// 添加列 / Add columns
    pub fn add_columns(
        &mut self,
        iteration: usize,
        new_layers: Vec<BinLayer<V, U>>,
        upper_bounds: Vec<Option<f64>>,
    ) -> Vec<IterativeLayerColumn<V, U>> {
        let added_layers = self.aggregation.add_columns(new_layers);
        let mut added = Vec::new();
        for (offset, layer) in added_layers.into_iter().enumerate() {
            let index = self.column_counter;
            self.column_counter += 1;
            let column = IterativeLayerColumn {
                index,
                iteration,
                layer,
                x_model_index: None,
                upper_bound: upper_bounds.get(offset).copied().flatten(),
            };
            self.columns.push(column.clone());
            added.push(column);
        }
        added
    }

    /// 添加列并注册到现有模型 / Add columns and register them to an existing model
    ///
    /// 注意：变量注册延迟到 `ImpreciseAssignment::register()` 中执行。
    /// 此方法仅跟踪层和上界。
    /// Note: Variable registration is deferred to `ImpreciseAssignment::register()`.
    /// This method only tracks layers and upper bounds.
    pub fn add_columns_to_model(
        &mut self,
        iteration: usize,
        new_layers: Vec<BinLayer<V, U>>,
        upper_bounds: Vec<Option<f64>>,
        assignment: &mut ImpreciseAssignment<V, U>,
        _model: &mut MetaModel<f64>,
    ) -> Result<Vec<IterativeLayerColumn<V, U>>, String> {
        let mut added = self.add_columns(iteration, new_layers, upper_bounds);
        for column in &mut added {
            if assignment.layers.len() != column.index {
                return Err(format!(
                    "layer column index {} does not match assignment length {}",
                    column.index,
                    assignment.layers.len()
                ));
            }

            assignment.layers.push(column.layer.clone());
            assignment.upper_bounds.push(column.upper_bound);
        }
        Ok(added)
    }

    /// 绑定 RMP 赋值变量索引 / Bind RMP assignment variable indices
    pub fn bind_assignment(&mut self, assignment: &ImpreciseAssignment<V, U>) {
        self.column_to_model_index.clear();
        if let Some(ref x) = assignment.x {
            for layer_index in 0..assignment.layers.len() {
                if let Some(model_index) = x.model_index(&layer_index) {
                    self.column_to_model_index.insert(layer_index, model_index);
                    if let Some(column) = self.columns.get_mut(layer_index) {
                        column.x_model_index = Some(model_index);
                        column.upper_bound = assignment.upper_bounds.get(layer_index).copied().flatten();
                    }
                }
            }
        }
    }

    /// 移除列 / Remove columns
    pub fn remove_columns(&mut self, columns: impl IntoIterator<Item = usize>) {
        for column in columns {
            self.removed_columns.insert(column);
        }
    }

    /// 隐藏列并同步模型 / Hide columns and sync model
    pub fn hide_columns_in_model(
        &self,
        lifecycle: &mut DynamicModelLifecycle,
        model: &mut MetaModel<f64>,
        columns: impl IntoIterator<Item = usize>,
    ) -> Result<(), String> {
        let columns = columns.into_iter().collect::<Vec<_>>();
        DynamicColumnContext::hide_dynamic_columns_in_model(self, lifecycle, model, &columns)
            .map_err(|e| format!("failed to hide BPP3D layer columns: {:?}", e))
    }

    /// 固定列并同步模型 / Fix columns and sync model
    pub fn fix_columns_in_model(
        &self,
        lifecycle: &mut DynamicModelLifecycle,
        model: &mut MetaModel<f64>,
        columns: impl IntoIterator<Item = usize>,
    ) -> Result<(), String> {
        let columns = columns.into_iter().collect::<Vec<_>>();
        DynamicColumnContext::fix_dynamic_columns_in_model(self, lifecycle, model, &columns)
            .map_err(|e| format!("failed to fix BPP3D layer columns: {:?}", e))
    }

    /// 移除列并同步模型 / Remove columns and sync model
    pub fn remove_columns_in_model(
        &mut self,
        lifecycle: &mut DynamicModelLifecycle,
        model: &mut MetaModel<f64>,
        columns: impl IntoIterator<Item = usize>,
    ) -> Result<(), String> {
        let columns = columns.into_iter().collect::<Vec<_>>();
        DynamicColumnContext::remove_dynamic_columns_in_model(self, lifecycle, model, &columns)
            .map_err(|e| format!("failed to remove BPP3D layer columns: {:?}", e))
    }

    /// 刷新动态范围 / Flush dynamic ranges
    pub fn flush_model(
        &mut self,
        lifecycle: &mut DynamicModelLifecycle,
        model: &mut MetaModel<f64>,
        force: bool,
    ) -> Result<(), String> {
        DynamicColumnContext::flush_dynamic_model(self, lifecycle, model, force)
            .map_err(|e| format!("failed to flush BPP3D layer lifecycle: {:?}", e))
    }

    /// 提取可选列解 / Extract selectable column solution
    pub fn extract_selectable_values(
        &self,
        solution: &[f64],
        lifecycle: &DynamicModelLifecycle,
    ) -> HashMap<usize, f64> {
        let columns = self
            .column_to_model_index
            .keys()
            .copied()
            .collect::<Vec<_>>();
        DynamicColumnContext::extract_selectable_column_values(
            self,
            &columns,
            solution,
            lifecycle,
        )
    }

    /// 活跃列数量 / Active column count
    pub fn active_column_count(&self) -> usize {
        self.columns
            .iter()
            .filter(|column| !self.removed_columns.contains(&column.index))
            .count()
    }

    /// 已移除列数量 / Removed column count
    pub fn removed_column_count(&self) -> usize {
        self.removed_columns.len()
    }

    /// 活跃层集合 / Active layers
    pub fn active_layers(&self) -> Vec<BinLayer<V, U>> {
        self.columns
            .iter()
            .filter(|column| !self.removed_columns.contains(&column.index))
            .map(|column| column.layer.clone())
            .collect()
    }

    fn model_indices_for(&self, columns: impl IntoIterator<Item = usize>) -> Vec<usize> {
        columns
            .into_iter()
            .filter_map(|column| self.column_to_model_index.get(&column).copied())
            .collect()
    }
}

impl<V, U> Default for IterativeLayerAssignmentContext<V, U>
where
    V: Debug + Clone + Send + Sync + PartialEq,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<V, U> DynamicColumnContext for IterativeLayerAssignmentContext<V, U>
where
    V: Debug + Clone + Send + Sync + PartialEq,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    fn column_model_indices(&self, columns: &[usize]) -> Vec<(usize, usize)> {
        columns
            .iter()
            .filter_map(|column| {
                self.column_to_model_index
                    .get(column)
                    .copied()
                    .map(|model_index| (*column, model_index))
            })
            .collect()
    }

    fn mark_columns_removed(&mut self, columns: &[usize]) {
        self.remove_columns(columns.iter().copied());
    }

    fn active_column_count(&self) -> usize {
        self.columns
            .iter()
            .filter(|column| !self.removed_columns.contains(&column.index))
            .count()
    }

    fn removed_column_count(&self) -> usize {
        self.removed_columns.len()
    }

    fn flush_dynamic_model(
        &mut self,
        lifecycle: &mut DynamicModelLifecycle,
        model: &mut MetaModel<f64>,
        force: bool,
    ) -> ospf_rust_core::error::Result<()> {
        lifecycle.flush_model(model, force)?;
        let model_indices = self.model_indices_for(self.removed_columns.iter().copied());
        lifecycle.remove_columns_in_model(model, model_indices)
    }
}

