/// MetaModel RMP executor / MetaModel RMP executor
#[derive(Debug, Clone, Default)]
pub struct MetaModelRmpExecutor {
    /// 配置 / Config
    pub config: MetaModelRmpExecutorConfig,
}

impl MetaModelRmpExecutor {
    /// 使用配置创建 / Create with config
    pub fn new(config: MetaModelRmpExecutorConfig) -> Self {
        Self { config }
    }

    fn build_context(
        &self,
        state: &ColumnGenerationApplicationState,
        model: &mut MetaModel<f64>,
    ) -> Result<(
        LayerAssignmentContext<f64, Meter>,
        ImpreciseAssignment<f64, Meter>,
        Vec<Bpp3dDemandEntry>,
        IterativeLayerAssignmentContext<f64, Meter>,
        DemandConstraint<f64, Meter>,
    ), String> {
        let demand_entries = item_demand_entries(&state.items);
        let upper_bounds = rmp_column_upper_bounds(&state.layers, &demand_entries);
        let mut assignment = ImpreciseAssignment {
            layers: Vec::new(),
            x: VariableArray1::new("x"),
            upper_bounds: Vec::new(),
        };
        let mut iterative_context = IterativeLayerAssignmentContext::new();
        iterative_context.add_columns_to_model(
            0,
            state.layers.clone(),
            upper_bounds,
            &mut assignment,
            model,
        )?;
        let aggregation = LayerAssignmentAggregation::rmp(
            assignment.clone(),
            Load::new(demand_entries.clone()),
            Capacity::new(),
        );
        let mut context = LayerAssignmentContext::new(aggregation);
        let demand_constraint = DemandConstraint::imprecise(
            demand_entries.clone(),
            assignment.clone(),
        );
        context.add_limit(Box::new(demand_constraint.clone()));
        context.add_objective(Box::new(VolumeMinimization::new(
            rmp_layer_volume_terms(&assignment),
            1.0,
        )));
        Ok((
            context,
            assignment,
            demand_entries,
            iterative_context,
            demand_constraint,
        ))
    }
}

impl ColumnGenerationRmpExecutor for MetaModelRmpExecutor {
    fn execute(&self, state: &ColumnGenerationApplicationState) -> ColumnGenerationRmpExecution {
        let backend = NoopMetaModelSolverBackend;
        self.execute_with_backend(state, &backend)
    }
}

/// 求解器驱动的 MetaModel RMP executor / Solver-backed MetaModel RMP executor
#[derive(Debug)]
pub struct SolverBackedMetaModelRmpExecutor<B>
where
    B: MetaModelSolverBackend,
{
    /// 配置 / Config
    pub config: MetaModelRmpExecutorConfig,
    /// 求解 backend / Solver backend
    pub backend: B,
}

impl<B> SolverBackedMetaModelRmpExecutor<B>
where
    B: MetaModelSolverBackend,
{
    /// 使用配置和 backend 创建 / Create with config and backend
    pub fn new(config: MetaModelRmpExecutorConfig, backend: B) -> Self {
        Self { config, backend }
    }
}

impl<B> ColumnGenerationRmpExecutor for SolverBackedMetaModelRmpExecutor<B>
where
    B: MetaModelSolverBackend,
{
    fn execute(&self, state: &ColumnGenerationApplicationState) -> ColumnGenerationRmpExecution {
        let executor = MetaModelRmpExecutor::new(self.config.clone());
        executor.execute_with_backend(state, &self.backend)
    }
}

impl MetaModelRmpExecutor {
    fn execute_with_backend(
        &self,
        state: &ColumnGenerationApplicationState,
        backend: &dyn MetaModelSolverBackend,
    ) -> ColumnGenerationRmpExecution {
        let mut model = MetaModel::<f64>::new(&self.config.model_name);
        let (
            mut context,
            _assignment,
            demand_entries,
            mut iterative_context,
            demand_constraint,
        ) = match self.build_context(state, &mut model) {
            Ok(context) => context,
            Err(error) => {
                return ColumnGenerationRmpExecution {
                    objective: None,
                    shadow_price_summary: HashMap::new(),
                    diagnostics: None,
                    info: HashMap::from([
                        ("executor".to_string(), "meta_model_rmp".to_string()),
                        ("status".to_string(), "registration_failed".to_string()),
                        ("error".to_string(), error),
                    ]),
                };
            }
        };
        if let Err(error) = context.register(&mut model).and_then(|_| context.invoke(&model)) {
            return ColumnGenerationRmpExecution {
                objective: None,
                shadow_price_summary: HashMap::new(),
                diagnostics: None,
                info: HashMap::from([
                    ("executor".to_string(), "meta_model_rmp".to_string()),
                    ("status".to_string(), "registration_failed".to_string()),
                    ("error".to_string(), error),
                ]),
            };
        }
        if let Some(assignment) = context.aggregation.imprecise_assignment.as_ref() {
            iterative_context.bind_assignment(assignment);
        }
        let mut lifecycle = DynamicModelLifecycle::new();

        let diagnostics = MetaModelExecutionDiagnostics::from_model(
            self.config.model_name.clone(),
            &model,
            demand_entries.len(),
            state.layers.len(),
            state.bins.len(),
        );
        let solve = match backend.solve_rmp(&model, &diagnostics) {
            Ok(solve) => merge_noop_solve_result(
                solve,
                self.config.objective,
                self.config.primal_solution.clone(),
                self.config.shadow_prices.clone(),
            ),
            Err(error) => {
                return ColumnGenerationRmpExecution {
                    objective: None,
                    shadow_price_summary: HashMap::new(),
                    diagnostics: Some(diagnostics),
                    info: HashMap::from([
                        ("executor".to_string(), "meta_model_rmp".to_string()),
                        ("backend".to_string(), backend.name().to_string()),
                        ("status".to_string(), "solve_failed".to_string()),
                        ("error".to_string(), error),
                    ]),
                };
            }
        };
        lifecycle.set_solution_to_model(&mut model, solve.primal_solution.clone());
        let layer_columns = iterative_context
            .columns
            .iter()
            .map(|column| column.index)
            .collect::<Vec<_>>();
        let layer_values = iterative_context.extract_selectable_column_values(
            &layer_columns,
            &solve.primal_solution,
            &lifecycle,
        );
        let shadow_price_summary = framework_shadow_price_summary(
            &demand_constraint,
            &demand_entries,
            &model,
            &solve.dual_solution,
        );
        let mut info = HashMap::from([
            ("executor".to_string(), "meta_model_rmp".to_string()),
            ("backend".to_string(), backend.name().to_string()),
            ("status".to_string(), "registered".to_string()),
            ("selected_layer_value_count".to_string(), layer_values.len().to_string()),
            (
                "framework_lifecycle_column_count".to_string(),
                DynamicColumnContext::active_column_count(&iterative_context).to_string(),
            ),
            (
                "framework_lifecycle_removed_column_count".to_string(),
                DynamicColumnContext::removed_column_count(&iterative_context).to_string(),
            ),
            (
                "framework_dynamic_column_context".to_string(),
                "iterative_layer_assignment".to_string(),
            ),
            (
                "framework_lifecycle_flush_count".to_string(),
                lifecycle.flush_count().to_string(),
            ),
        ]);
        info.extend(solve.info);
        diagnostics.write_info(&mut info);
        ColumnGenerationRmpExecution {
            objective: solve.objective,
            shadow_price_summary,
            diagnostics: Some(diagnostics),
            info,
        }
    }
}
