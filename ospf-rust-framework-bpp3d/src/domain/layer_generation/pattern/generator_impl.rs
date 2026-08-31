
impl<V, U> LayerGenerator<V, U> for PatternLayerGenerator
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
        if request.items.is_empty() {
            return vec![unsupported_layer_generation_result(
                request,
                &self.name,
                "pattern layer generation requires at least one item".to_string(),
            )];
        }
        let has_cylinder = CylinderShapeContract::has_cylinder(&request.items);
        let mut groups = Vec::<(String, Vec<(usize, ActualItem<V, U>)>)>::new();
        for (index, item) in request.items.iter().enumerate() {
            let key = item
                .package_code
                .clone()
                .unwrap_or_else(|| item.id.clone());
            if let Some((_, group_items)) = groups.iter_mut().find(|(group_key, _)| group_key == &key) {
                group_items.push((index, item.clone()));
            } else {
                groups.push((key, vec![(index, item.clone())]));
            }
        }
        for (_, group_items) in &mut groups {
            group_items.sort_by(|(lhs_index, lhs), (rhs_index, rhs)| {
                pattern_item_priority_score(request, *lhs_index, lhs)
                    .partial_cmp(&pattern_item_priority_score(request, *rhs_index, rhs))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        let mut results = groups
            .iter()
            .flat_map(|(group_key, items)| {
                let step_generated = if has_cylinder {
                    Vec::new()
                } else {
                    pattern_step_layer_candidates(
                        request,
                        &self.name,
                        group_key,
                        items,
                        &self.config,
                    )
                };
                let mut generated = block_loading_layer_candidates(
                    request,
                    &self.name,
                    "pattern",
                    items,
                    false,
                    &self.config,
                );
                for result in &mut generated {
                    result.diagnostics.push(format!(
                        "pattern layer used item pattern group '{}'",
                        group_key,
                    ));
                    if has_cylinder {
                        result.diagnostics.push(
                            "pattern layer used block-loading fallback for cylinder item".to_string(),
                        );
                    }
                    result.diagnostics.push(format!(
                        "pattern config: with_piling={:?}, with_remainder={}, two_sum={}, three_sum={}",
                        self.config.pattern.with_piling,
                        self.config.pattern.with_remainder,
                        self.config.pattern.enables_two_sum(),
                        self.config.pattern.enables_three_sum(),
                    ));
                }
                if step_generated.is_empty() {
                    generated
                } else {
                    step_generated
                }
            })
            .collect::<Vec<_>>();
        rank_layer_results(&mut results);
        results.truncate(request.max_candidates.min(self.config.max_candidates.max(1)));
        if results.is_empty() {
            vec![unsupported_layer_generation_result(
                request,
                &self.name,
                "pattern layer generation found no block-loading candidate".to_string(),
            )]
        } else {
            results
        }
    }
}

