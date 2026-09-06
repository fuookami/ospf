impl<V, U> LayerGenerator<V, U> for HistoricalLayerGenerator
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn generate(
        &self,
        request: &LayerGenerationRequest<V, U>,
    ) -> Vec<LayerGenerationResult<V, U>> {
        let mut results = request
            .existing_layers
            .iter()
            .take(request.max_candidates.min(self.config.max_candidates.max(1)))
            .enumerate()
            .map(|(index, layer)| {
                let mut layer = layer.clone();
                layer.from = self.name.clone();
                let (score, numeric_score) = score_layer_coverage(request, &layer.demand_coverage);
                LayerGenerationResult {
                    layer,
                    reduced_cost: None,
                    score,
                    numeric_score,
                    block_traces: Vec::new(),
                    placement_traces: Vec::new(),
                    diagnostics: vec![format!(
                        "historical layer reused existing layer candidate: existing_layer_index={}, existing_layer_hint={}",
                        index,
                        self.config.use_existing_layer_hint,
                    )],
                    source: self.name.clone(),
                }
            })
            .collect::<Vec<_>>();
        rank_layer_results(&mut results);
        if results.is_empty() {
            vec![unsupported_layer_generation_result(
                request,
                &self.name,
                "historical layer generation requires existing layers".to_string(),
            )]
        } else {
            results
        }
    }
}

