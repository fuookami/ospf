
/// MetaModel final MILP executor / MetaModel final MILP executor
#[derive(Debug, Clone, Default)]
pub struct MetaModelFinalExecutor {
    /// 配置 / Config
    pub config: MetaModelFinalExecutorConfig,
}

impl MetaModelFinalExecutor {
    /// 使用配置创建 / Create with config
    pub fn new(config: MetaModelFinalExecutorConfig) -> Self {
        Self { config }
    }

    fn build_context(
        &self,
        state: &ColumnGenerationApplicationState,
    ) -> (LayerAssignmentContext<f64, Meter>, PreciseAssignment<f64, Meter>, Vec<Bpp3dDemandEntry>) {
        let demand_entries = item_demand_entries(&state.items);
        let bins = if state.bins.is_empty() {
            bins_from_layers(&state.layers)
        } else {
            state.bins.clone()
        };
        let assignment = PreciseAssignment {
            bins,
            layers: state.layers.clone(),
            x: None,
            v: None,
            load_weight_symbols: Vec::new(),
            load_volume_symbols: Vec::new(),
            load_depth_symbols: Vec::new(),
        };
        let aggregation = LayerAssignmentAggregation::final_milp(
            assignment.clone(),
            Load::new(demand_entries.clone()),
            Capacity::new(),
        );
        let context = LayerAssignmentContext::new(aggregation);
        // Limits are added after registration in execute_with_backend
        (context, assignment, demand_entries)
    }
}

impl ColumnGenerationFinalExecutor for MetaModelFinalExecutor {
    fn execute(&self, state: &ColumnGenerationApplicationState) -> ColumnGenerationFinalExecution {
        let backend = NoopMetaModelSolverBackend;
        self.execute_with_backend(state, &backend)
    }
}

/// 求解器驱动的 MetaModel final MILP executor / Solver-backed MetaModel final MILP executor
#[derive(Debug)]
pub struct SolverBackedMetaModelFinalExecutor<B>
where
    B: MetaModelSolverBackend,
{
    /// 配置 / Config
    pub config: MetaModelFinalExecutorConfig,
    /// 求解 backend / Solver backend
    pub backend: B,
}

impl<B> SolverBackedMetaModelFinalExecutor<B>
where
    B: MetaModelSolverBackend,
{
    /// 使用配置和 backend 创建 / Create with config and backend
    pub fn new(config: MetaModelFinalExecutorConfig, backend: B) -> Self {
        Self { config, backend }
    }
}

impl<B> ColumnGenerationFinalExecutor for SolverBackedMetaModelFinalExecutor<B>
where
    B: MetaModelSolverBackend,
{
    fn execute(&self, state: &ColumnGenerationApplicationState) -> ColumnGenerationFinalExecution {
        let executor = MetaModelFinalExecutor::new(self.config.clone());
        executor.execute_with_backend(state, &self.backend)
    }
}

impl MetaModelFinalExecutor {
    fn execute_with_backend(
        &self,
        state: &ColumnGenerationApplicationState,
        backend: &dyn MetaModelSolverBackend,
    ) -> ColumnGenerationFinalExecution {
        let (mut context, assignment, demand_entries) = self.build_context(state);
        let mut model = MetaModel::<f64>::new(&self.config.model_name);

        // Step 1: Register variables
        if let Err(error) = context.aggregation_mut().register(&mut model) {
            return ColumnGenerationFinalExecution {
                layers: Vec::new(),
                packed_bins: Vec::new(),
                objective: None,
                diagnostics: None,
                info: HashMap::from([
                    ("executor".to_string(), "meta_model_final".to_string()),
                    ("status".to_string(), "registration_failed".to_string()),
                    ("error".to_string(), error),
                ]),
            };
        }

        // Step 2: Get registered assignment with x/v indices
        let registered = context.aggregation().precise_assignment.clone().unwrap_or(assignment);

        // Step 3: Add limits using registered assignment
        context.add_limit(Box::new(DemandConstraint::precise(
            demand_entries.clone(),
            registered.clone(),
        )));
        context.add_limit(Box::new(PreciseAssignmentActivationConstraint::new(
            registered.clone(),
        )));
        context.add_limit(Box::new(BinCapacityConstraint::<f64, Meter>::from_bins(
            &registered.bins,
            final_assignment_indices_by_bin(&registered),
            final_layer_weights(&registered.layers, &state.items),
            final_layer_volumes(&registered.layers),
            &Default::default(),
        )));
        context.add_limit(Box::new(BinDepthConstraint::<f64, Meter>::from_bins(
            &registered.bins,
            final_assignment_indices_by_bin(&registered),
            layer_depths(&registered.layers),
            &Default::default(),
        )));
        context.add_objective(Box::new(BinAmountMinimization::new(
            final_bin_marker_indices(&registered),
            1.0,
        )));

        // Step 4: Register limits and invoke
        if let Err(error) = context.register_limits(&mut model).and_then(|_| context.invoke(&model)) {
            return ColumnGenerationFinalExecution {
                layers: Vec::new(),
                packed_bins: Vec::new(),
                objective: None,
                diagnostics: None,
                info: HashMap::from([
                    ("executor".to_string(), "meta_model_final".to_string()),
                    ("status".to_string(), "registration_failed".to_string()),
                    ("error".to_string(), error),
                ]),
            };
        }
        let continuous_radius_registration = match state.continuous_radius_component.as_ref() {
            Some(component) => match component.register_solver_model(&mut model) {
                Ok(registration) => registration,
                Err(error) => {
                    return ColumnGenerationFinalExecution {
                        layers: Vec::new(),
                        packed_bins: Vec::new(),
                        objective: None,
                        diagnostics: None,
                        info: HashMap::from([
                            ("executor".to_string(), "meta_model_final".to_string()),
                            ("status".to_string(), "registration_failed".to_string()),
                            ("error".to_string(), error),
                        ]),
                    };
                }
            },
            None => Default::default(),
        };

        let diagnostics = MetaModelExecutionDiagnostics::from_model(
            self.config.model_name.clone(),
            &model,
            demand_entries.len(),
            state.layers.len(),
            context
                .aggregation
                .precise_assignment
                .as_ref()
                .map(|assignment| assignment.bins.len())
                .unwrap_or(0),
        );
        let solve = match backend.solve_final(&model, &diagnostics) {
            Ok(solve) => merge_noop_solve_result(
                solve,
                self.config.objective,
                self.config.primal_solution.clone(),
                Vec::new(),
            ),
            Err(error) => {
                return ColumnGenerationFinalExecution {
                    layers: Vec::new(),
                    packed_bins: Vec::new(),
                    objective: None,
                    diagnostics: Some(diagnostics),
                    info: HashMap::from([
                        ("executor".to_string(), "meta_model_final".to_string()),
                        ("backend".to_string(), backend.name().to_string()),
                        ("status".to_string(), "solve_failed".to_string()),
                        ("error".to_string(), error),
                    ]),
                };
            }
        };
        let mut lifecycle = DynamicModelLifecycle::new();
        lifecycle.set_solution_to_model(&mut model, solve.primal_solution.clone());
        let (continuous_radius_solutions, mut continuous_radius_diagnostics) =
            match state.continuous_radius_component.as_ref() {
                Some(component) => component.selected_solutions_from_primal(
                    &continuous_radius_registration,
                    &solve.primal_solution,
                ),
                None => (Vec::new(), Vec::new()),
            };
        continuous_radius_diagnostics.extend(continuous_radius_registration.diagnostics.clone());
        let Some(assignment) = context.aggregation.precise_assignment.as_ref() else {
            return ColumnGenerationFinalExecution {
                layers: Vec::new(),
                packed_bins: Vec::new(),
                objective: None,
                diagnostics: None,
                info: HashMap::from([
                    ("executor".to_string(), "meta_model_final".to_string()),
                    ("status".to_string(), "registration_failed".to_string()),
                    ("error".to_string(), "missing precise assignment after registration".to_string()),
                ]),
            };
        };
        let selected_assignments = assignment.x.as_ref().map(|x| {
            SolutionExtractor::extract_indexed_binary_2(&solve.primal_solution, x)
        }).unwrap_or_default();
        let selected_layer_indices = state.layers
            .iter()
            .enumerate()
            .filter_map(|(layer_index, _)| {
                let selected = assignment.bins
                    .iter()
                    .enumerate()
                    .any(|(bin_index, _)| {
                        selected_assignments
                            .get(&(bin_index, layer_index))
                            .copied()
                            .unwrap_or(false)
                    });
                selected.then_some(layer_index)
            })
            .collect::<Vec<_>>();
        let layers = selected_layer_indices
            .iter()
            .filter_map(|index| state.layers.get(*index).cloned())
            .collect::<Vec<_>>();
        let output_layers = if solve.primal_solution.is_empty() {
            state.layers.clone()
        } else {
            layers
        };
        let output_layer_indices = if solve.primal_solution.is_empty() {
            (0..state.layers.len()).collect::<Vec<_>>()
        } else {
            selected_layer_indices
        };
        let trace_layer_count = output_layers
            .iter()
            .filter(|layer| !layer.demand_coverage.is_empty())
            .count();
        let (packed_bins, mut packing_diagnostics) = packed_bins_from_selected_layers(
            &output_layers,
            &output_layer_indices,
            state,
        );
        if state.bins.is_empty() && bins_from_layers(&state.layers).is_empty() {
            packing_diagnostics.push("no available bin type for final packing".to_string());
        }
        let mut info = HashMap::from([
            ("executor".to_string(), "meta_model_final".to_string()),
            ("backend".to_string(), backend.name().to_string()),
            ("status".to_string(), "registered".to_string()),
            ("selected_assignment_count".to_string(), selected_assignments.len().to_string()),
            ("selected_layer_count".to_string(), output_layers.len().to_string()),
            ("selected_layer_with_coverage_count".to_string(), trace_layer_count.to_string()),
            ("packed_bin_count".to_string(), packed_bins.len().to_string()),
            (
                "framework_lifecycle_solution_len".to_string(),
                lifecycle.solution().map(|solution| solution.len()).unwrap_or(0).to_string(),
            ),
            (
                "framework_lifecycle_flush_count".to_string(),
                lifecycle.flush_count().to_string(),
            ),
            (
                "continuous_radius_model_variable_count".to_string(),
                continuous_radius_registration.variable_count().to_string(),
            ),
            (
                "continuous_radius_model_constraint_count".to_string(),
                continuous_radius_registration.constraint_count.to_string(),
            ),
            (
                "continuous_radius_model_objective_term_count".to_string(),
                continuous_radius_registration.objective_term_count.to_string(),
            ),
            (
                "continuous_radius_selected_count".to_string(),
                continuous_radius_solutions.len().to_string(),
            ),
        ]);
        for solution in &continuous_radius_solutions {
            info.insert(
                ContinuousRadiusModelComponent::solution_info_key(&solution.variable_name),
                solution.radius.to_string(),
            );
            if let Some(radius_squared) = solution.radius_squared {
                info.insert(
                    ContinuousRadiusModelComponent::radius_squared_info_key(&solution.variable_name),
                    radius_squared.to_string(),
                );
            }
            if let Some(segment_index) = solution.segment_index {
                info.insert(
                    ContinuousRadiusModelComponent::segment_info_key(&solution.variable_name),
                    segment_index.to_string(),
                );
            }
        }
        if !continuous_radius_diagnostics.is_empty() {
            info.insert(
                "continuous_radius_diagnostics".to_string(),
                continuous_radius_diagnostics.join("; "),
            );
        }
        if !packing_diagnostics.is_empty() {
            info.insert(
                "packing_diagnostics".to_string(),
                packing_diagnostics.join("; "),
            );
        }
        if !packed_bins.is_empty() {
            info.insert(
                "final_diagnostics".to_string(),
                "selected_layers_renderable".to_string(),
            );
        } else if output_layers.is_empty() {
            info.insert(
                "final_diagnostics".to_string(),
                "no_selected_layer_assignment".to_string(),
            );
        } else if trace_layer_count == 0 {
            info.insert(
                "final_diagnostics".to_string(),
                "selected_layers_without_coverage_trace".to_string(),
            );
        } else {
            info.insert(
                "final_diagnostics".to_string(),
                "selected_layers_have_coverage_trace".to_string(),
            );
        }
        info.extend(solve.info);
        diagnostics.write_info(&mut info);
        ColumnGenerationFinalExecution {
            layers: output_layers,
            packed_bins,
            objective: solve.objective,
            diagnostics: Some(diagnostics),
            info,
        }
    }
}
