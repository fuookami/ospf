impl<V, U> LayerGenerator<V, U> for PileLayerGenerator
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
        let mut results = pile_layer_candidates(request, &self.name, &self.config);
        if results.is_empty() {
            let indexed_items = request
                .items
                .iter()
                .enumerate()
                .map(|(index, item)| (index, item.clone()))
                .collect::<Vec<_>>();
            results = block_loading_layer_candidates(
                request,
                &self.name,
                "pile",
                &indexed_items,
                false,
                &self.config,
            );
            for result in &mut results {
                result.diagnostics.push(
                    "pile layer used block-loading fallback because no vertical stack candidate was feasible"
                        .to_string(),
                );
            }
        }
        if results.is_empty() {
            vec![unsupported_layer_generation_result(
                request,
                &self.name,
                "pile layer generation found no stacking candidate".to_string(),
            )]
        } else {
            results
        }
    }
}

