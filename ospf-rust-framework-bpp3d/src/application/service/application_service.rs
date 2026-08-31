// ColumnGenerationApplicationService - 列生成应用服务 / Application service
// ============================================================================

/// 列生成应用服务 / Column generation application service
///
/// 提供轻量应用入口，组合配置、层生成和装箱分析。
/// Provides a lightweight application entry point composing config, layer
/// generation, and packing analysis.
#[derive(Debug, Clone)]
pub struct ColumnGenerationApplicationService {
    /// 配置 / Config
    pub config: ColumnGenerationConfig,
    /// 装箱分析器 / Packing analyzer
    pub packing_analyzer: ColumnGenerationPackingAnalyzer,
}

impl Default for ColumnGenerationApplicationService {
    fn default() -> Self {
        Self::new(ColumnGenerationConfig::default())
    }
}

impl ColumnGenerationApplicationService {
    /// 创建应用服务 / Create application service
    pub fn new(config: ColumnGenerationConfig) -> Self {
        Self {
            config,
            packing_analyzer: ColumnGenerationPackingAnalyzer::new(),
        }
    }

    /// 创建列生成算法 / Create column generation algorithm
    pub fn create_algorithm<V, U>(&self) -> ColumnGenerationAlgorithm<V, U>
    where
        V: Field + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
        U: UnitTrait + Debug + Clone + Send + Sync,
    {
        ColumnGenerationAlgorithm::new(self.config.clone())
    }

    /// 分析装箱结果 / Analyze packing result
    pub fn analyze_packing<V, U>(
        &self,
        packed_bins: Vec<PackedBin<V, U>>,
    ) -> Result<ColumnGenerationPackingAnalysis<V, U>, Vec<String>>
    where
        V: num_traits::Float + Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + Into<f64>,
        U: CTUnit + Default + Clone,
    {
        self.packing_analyzer.analyze(packed_bins)
    }

    /// 运行 materialized request / Run materialized request
    ///
    /// 该入口只编排 executor，不直接注册 MetaModel。
    /// This entry only orchestrates executors and does not register MetaModel directly.
    pub fn run_materialized(
        &self,
        items: Vec<ActualItem<f64, Meter>>,
        initial_layers: Vec<BinLayer<f64, Meter>>,
        rmp_executor: &dyn ColumnGenerationRmpExecutor,
        final_executor: &dyn ColumnGenerationFinalExecutor,
    ) -> Result<ColumnGenerationApplicationFlowResult, Vec<String>> {
        self.run_materialized_with_bins(
            items,
            Vec::new(),
            initial_layers,
            rmp_executor,
            final_executor,
        )
    }

    /// 运行带箱型的 materialized request / Run materialized request with bins
    ///
    /// 该入口只传递完整应用状态给 executor，不直接注册 MetaModel。
    /// This entry only passes complete application state to executors and does not register MetaModel directly.
    pub fn run_materialized_with_bins(
        &self,
        items: Vec<ActualItem<f64, Meter>>,
        bins: Vec<BinType<f64, Meter>>,
        initial_layers: Vec<BinLayer<f64, Meter>>,
        rmp_executor: &dyn ColumnGenerationRmpExecutor,
        final_executor: &dyn ColumnGenerationFinalExecutor,
    ) -> Result<ColumnGenerationApplicationFlowResult, Vec<String>> {
        self.run_materialized_with_bins_and_continuous_radius(
            items,
            bins,
            initial_layers,
            None,
            rmp_executor,
            final_executor,
        )
    }

    fn run_materialized_with_bins_and_continuous_radius(
        &self,
        items: Vec<ActualItem<f64, Meter>>,
        bins: Vec<BinType<f64, Meter>>,
        initial_layers: Vec<BinLayer<f64, Meter>>,
        continuous_radius_component: Option<ContinuousRadiusModelComponent>,
        rmp_executor: &dyn ColumnGenerationRmpExecutor,
        final_executor: &dyn ColumnGenerationFinalExecutor,
    ) -> Result<ColumnGenerationApplicationFlowResult, Vec<String>> {
        let mut algorithm = self.create_algorithm::<f64, Meter>();
        let initial_layers = ensure_layer_demand_coverage(initial_layers, &items);
        algorithm.add_initial_layers(initial_layers.clone());
        let state = application_state_from_algorithm_with_continuous_radius(
            &algorithm,
            items,
            bins,
            initial_layers,
            HashMap::new(),
            HashMap::new(),
            continuous_radius_component,
        );
        let rmp = rmp_executor.execute(&state);
        if let Some(objective) = rmp.objective {
            algorithm.state.observe_objective(
                objective,
                self.config.objective_sense,
                self.config.reduced_cost_tolerance,
            );
        }
        let final_execution = final_executor.execute(&state);
        let packing_analysis = if final_execution.packed_bins.is_empty() {
            None
        } else {
            Some(self.analyze_packing(final_execution.packed_bins.clone())?)
        };
        let mut info = HashMap::new();
        info.extend(rmp.info.iter().map(|(k, v)| (format!("rmp_{}", k), v.clone())));
        info.extend(final_execution.info.iter().map(|(k, v)| (format!("final_{}", k), v.clone())));
        let result = ColumnGenerationResult {
            state: algorithm.state,
            layers: final_execution.layers.clone(),
            packing_result: packing_analysis.as_ref().map(|analysis| analysis.packing_result.clone()),
            render_loading_plans: packing_analysis
                .map(|analysis| analysis.render_loading_plans)
                .unwrap_or_default(),
            info,
        };
        Ok(ColumnGenerationApplicationFlowResult {
            result,
            rmp,
            final_execution,
            selected_radius_solutions: Vec::new(),
        })
    }

    /// 运行一轮 RMP -> 生成列 -> RMP refresh -> final / Run one RMP -> generate columns -> RMP refresh -> final round
    ///
    /// 该入口验证 shadow price 能穿过 application 编排进入 layer generation。
    /// This entry verifies shadow prices pass through application orchestration into layer generation.
    pub fn run_materialized_one_generation_round(
        &self,
        items: Vec<ActualItem<f64, Meter>>,
        bins: Vec<BinType<f64, Meter>>,
        initial_layers: Vec<BinLayer<f64, Meter>>,
        layer_generation: LayerGenerationContext<f64, Meter>,
        rmp_executor: &dyn ColumnGenerationRmpExecutor,
        final_executor: &dyn ColumnGenerationFinalExecutor,
    ) -> Result<ColumnGenerationApplicationFlowResult, Vec<String>> {
        self.run_materialized_one_generation_round_with_package_attributes(
            items,
            bins,
            initial_layers,
            None,
            HashMap::new(),
            layer_generation,
            rmp_executor,
            final_executor,
        )
    }

    fn run_materialized_one_generation_round_with_package_attributes(
        &self,
        items: Vec<ActualItem<f64, Meter>>,
        bins: Vec<BinType<f64, Meter>>,
        initial_layers: Vec<BinLayer<f64, Meter>>,
        continuous_radius_component: Option<ContinuousRadiusModelComponent>,
        package_attributes: HashMap<String, PackageAttribute>,
        layer_generation: LayerGenerationContext<f64, Meter>,
        rmp_executor: &dyn ColumnGenerationRmpExecutor,
        final_executor: &dyn ColumnGenerationFinalExecutor,
    ) -> Result<ColumnGenerationApplicationFlowResult, Vec<String>> {
        let mut algorithm = ColumnGenerationAlgorithm::with_layer_generation(
            self.config.clone(),
            layer_generation,
        );
        let initial_layers = ensure_layer_demand_coverage(initial_layers, &items);
        algorithm.add_initial_layers(initial_layers.clone());
        let initial_state = application_state_from_algorithm(
            &algorithm,
            items.clone(),
            bins.clone(),
            initial_layers.clone(),
            HashMap::new(),
            HashMap::new(),
        );
        let first_rmp = rmp_executor.execute(&initial_state);
        if let Some(objective) = first_rmp.objective {
            algorithm.state.observe_objective(
                objective,
                self.config.objective_sense,
                self.config.reduced_cost_tolerance,
            );
        }

        let demand_entries = layer_generation_demand_entries(&items, &first_rmp.shadow_price_summary);
        let shadow_prices = layer_generation_shadow_prices(&first_rmp.shadow_price_summary);
        let mut request = LayerGenerationRequest::new(algorithm.state.iteration as i64, items.clone())
            .with_demand_entries(demand_entries)
            .with_shadow_prices(shadow_prices)
            .with_package_attributes(package_attributes)
            .with_max_candidates(self.config.max_candidates_per_iteration);
        request.bin = bins.first().cloned();
        request.existing_layers = algorithm.active_layers();
        let generated = algorithm.layer_generation.generate(&request);
        let generated_block_trace_by_key = generated
            .iter()
            .filter(|result| !result.block_traces.is_empty())
            .map(|result| {
                (
                    layer_trace_key(&result.layer),
                    result.block_traces.clone(),
                )
            })
            .collect::<HashMap<_, _>>();
        let generated_trace_by_key = generated
            .iter()
            .filter(|result| !result.placement_traces.is_empty())
            .map(|result| {
                (
                    layer_trace_key(&result.layer),
                    result.placement_traces.clone(),
                )
            })
            .collect::<HashMap<_, _>>();
        let generated_layers = generated
            .into_iter()
            .map(|result| result.layer)
            .collect::<Vec<_>>();
        let generated_layers = ensure_layer_demand_coverage(generated_layers, &items);
        algorithm.add_columns(algorithm.state.iteration, generated_layers);
        let layer_block_traces = algorithm
            .active_layers()
            .iter()
            .enumerate()
            .filter_map(|(index, layer)| {
                generated_block_trace_by_key
                    .get(&layer_trace_key(layer))
                    .cloned()
                    .map(|traces| (index, traces))
            })
            .collect::<HashMap<_, _>>();
        let layer_placement_traces = algorithm
            .active_layers()
            .iter()
            .enumerate()
            .filter_map(|(index, layer)| {
                generated_trace_by_key
                    .get(&layer_trace_key(layer))
                    .cloned()
                    .map(|traces| (index, traces))
            })
            .collect::<HashMap<_, _>>();
        algorithm.state.advance_iteration();

        let refreshed_state = application_state_from_algorithm_with_continuous_radius(
            &algorithm,
            items,
            bins,
            initial_layers,
            layer_block_traces,
            layer_placement_traces,
            continuous_radius_component,
        );
        let rmp = rmp_executor.execute(&refreshed_state);
        if let Some(objective) = rmp.objective {
            algorithm.state.observe_objective(
                objective,
                self.config.objective_sense,
                self.config.reduced_cost_tolerance,
            );
        }
        let final_execution = final_executor.execute(&refreshed_state);
        let packing_analysis = if final_execution.packed_bins.is_empty() {
            None
        } else {
            Some(self.analyze_packing(final_execution.packed_bins.clone())?)
        };
        let mut info = HashMap::from([
            ("first_rmp_objective".to_string(), format!("{:?}", first_rmp.objective)),
            ("generated_layer_count".to_string(), (algorithm.state.total_columns.saturating_sub(refreshed_state.initial_layers.len())).to_string()),
        ]);
        info.extend(rmp.info.iter().map(|(k, v)| (format!("rmp_{}", k), v.clone())));
        info.extend(final_execution.info.iter().map(|(k, v)| (format!("final_{}", k), v.clone())));
        let result = ColumnGenerationResult {
            state: algorithm.state,
            layers: final_execution.layers.clone(),
            packing_result: packing_analysis.as_ref().map(|analysis| analysis.packing_result.clone()),
            render_loading_plans: packing_analysis
                .map(|analysis| analysis.render_loading_plans)
                .unwrap_or_default(),
            info,
        };
        Ok(ColumnGenerationApplicationFlowResult {
            result,
            rmp,
            final_execution,
            selected_radius_solutions: Vec::new(),
        })
    }

    /// 运行 CSV materialized request / Run CSV materialized request
    #[cfg(feature = "serde")]
    pub fn run_csv_materialized(
        &self,
        request: CsvMaterializedApplicationRequest,
        rmp_executor: &dyn ColumnGenerationRmpExecutor,
        final_executor: &dyn ColumnGenerationFinalExecutor,
    ) -> Result<ColumnGenerationApplicationFlowResult, Vec<String>> {
        let continuous_radius_component = request.continuous_radius_component.clone();
        let mut flow = self.run_materialized_with_bins_and_continuous_radius(
            request.items,
            Vec::new(),
            request.initial_layers,
            continuous_radius_component.clone(),
            rmp_executor,
            final_executor,
        )?;
        apply_continuous_radius_solutions(self, &mut flow, continuous_radius_component.as_ref())?;
        append_continuous_radius_info(&mut flow.result.info, continuous_radius_component.as_ref());
        Ok(flow)
    }

    /// 运行带箱型的 CSV materialized request / Run CSV materialized request with bins
    #[cfg(feature = "serde")]
    pub fn run_csv_materialized_with_bins(
        &self,
        request: CsvMaterializedApplicationRequest,
        rmp_executor: &dyn ColumnGenerationRmpExecutor,
        final_executor: &dyn ColumnGenerationFinalExecutor,
    ) -> Result<ColumnGenerationApplicationFlowResult, Vec<String>> {
        let continuous_radius_component = request.continuous_radius_component.clone();
        let mut flow = self.run_materialized_with_bins_and_continuous_radius(
            request.items,
            request.bins,
            request.initial_layers,
            continuous_radius_component.clone(),
            rmp_executor,
            final_executor,
        )?;
        apply_continuous_radius_solutions(self, &mut flow, continuous_radius_component.as_ref())?;
        append_continuous_radius_info(&mut flow.result.info, continuous_radius_component.as_ref());
        Ok(flow)
    }

    /// 运行一轮 CSV materialized request / Run one-round CSV materialized request
    #[cfg(feature = "serde")]
    pub fn run_csv_materialized_one_generation_round(
        &self,
        request: CsvMaterializedApplicationRequest,
        layer_generation: LayerGenerationContext<f64, Meter>,
        rmp_executor: &dyn ColumnGenerationRmpExecutor,
        final_executor: &dyn ColumnGenerationFinalExecutor,
    ) -> Result<ColumnGenerationApplicationFlowResult, Vec<String>> {
        let continuous_radius_component = request.continuous_radius_component.clone();
        let package_attributes = request
            .package_attributes
            .into_iter()
            .collect::<HashMap<_, _>>();
        let mut flow = self.run_materialized_one_generation_round_with_package_attributes(
            request.items,
            request.bins,
            request.initial_layers,
            continuous_radius_component.clone(),
            package_attributes,
            layer_generation,
            rmp_executor,
            final_executor,
        )?;
        apply_continuous_radius_solutions(self, &mut flow, continuous_radius_component.as_ref())?;
        append_continuous_radius_info(&mut flow.result.info, continuous_radius_component.as_ref());
        Ok(flow)
    }
}
