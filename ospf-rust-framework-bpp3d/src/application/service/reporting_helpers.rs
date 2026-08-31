fn packed_bins_from_selected_layers(
    layers: &[BinLayer<f64, Meter>],
    layer_indices: &[usize],
    state: &ColumnGenerationApplicationState,
) -> (Vec<PackedBin<f64, Meter>>, Vec<String>) {
    let replay = LayerTraceReplayAdapter::new().replay_selected_layers(
        layers,
        layer_indices,
        &state.items,
        &state.bins,
        &state.layer_block_traces,
        &state.layer_placement_traces,
    );
    (replay.packed_bins, replay.diagnostics)
}

fn merge_noop_solve_result(
    mut solve: MetaModelExecutorSolveResult,
    fallback_objective: Option<f64>,
    fallback_primal_solution: Vec<f64>,
    fallback_dual_solution: Vec<f64>,
) -> MetaModelExecutorSolveResult {
    if solve.objective.is_none() {
        solve.objective = fallback_objective;
    }
    if solve.primal_solution.is_empty() {
        solve.primal_solution = fallback_primal_solution;
    }
    if solve.dual_solution.is_empty() {
        solve.dual_solution = fallback_dual_solution;
    }
    solve
}

fn item_demand_entries(items: &[ActualItem<f64, Meter>]) -> Vec<Bpp3dDemandEntry> {
    items
        .iter()
        .map(|item| Bpp3dDemandEntry {
            mode: Bpp3dDemandMode::Item,
            key: Bpp3dDemandKey::Item { id: item.id.clone() },
            demand: 1.0,
        })
        .collect()
}

fn bins_from_layers(layers: &[BinLayer<f64, Meter>]) -> Vec<BinType<f64, Meter>> {
    layers
        .iter()
        .filter_map(|layer| layer.bin.clone())
        .collect()
}

fn demand_key_name(entry: &Bpp3dDemandEntry) -> String {
    match &entry.key {
        Bpp3dDemandKey::Item { id } => format!("item:{}", id),
        Bpp3dDemandKey::Material { no } => format!("material:{}", no),
    }
}

#[cfg(feature = "serde")]
fn demand_mode_name(mode: Bpp3dDemandMode) -> &'static str {
    match mode {
        Bpp3dDemandMode::Item => "item",
        Bpp3dDemandMode::Material => "material",
        Bpp3dDemandMode::ItemAmount => "item_amount",
        Bpp3dDemandMode::ItemWeight => "item_weight",
        Bpp3dDemandMode::ItemMaterialAmount => "item_material_amount",
        Bpp3dDemandMode::ItemMaterialWeight => "item_material_weight",
    }
}

#[cfg(feature = "serde")]
fn demand_key_report_name(key: &Bpp3dDemandKey) -> String {
    match key {
        Bpp3dDemandKey::Item { id } => format!("item:{}", id),
        Bpp3dDemandKey::Material { no } => format!("material:{}", no),
    }
}

#[cfg(feature = "serde")]
fn selected_layer_reports(
    layers: &[BinLayer<f64, Meter>],
) -> Vec<super::report::Bpp3dSelectedLayerReport> {
    layers
        .iter()
        .enumerate()
        .map(|(index, layer)| super::report::Bpp3dSelectedLayerReport {
            index,
            iteration: layer.iteration,
            from: layer.from.clone(),
            bin_type: layer.bin.as_ref().map(|bin| bin.type_code.clone()),
            depth: layer.depth.value,
            coverage: layer
                .demand_coverage
                .iter()
                .map(|coverage| super::report::Bpp3dDemandCoverageReport {
                    mode: demand_mode_name(coverage.mode).to_string(),
                    key: demand_key_report_name(&coverage.key),
                    coefficient: coverage.coefficient,
                })
                .collect(),
        })
        .collect()
}

#[cfg(feature = "serde")]
fn packed_bin_reports(
    packed_bins: &[PackedBin<f64, Meter>],
) -> Vec<super::report::Bpp3dPackedBinReport> {
    packed_bins
        .iter()
        .map(|bin| {
            let mut loading_orders = bin
                .items
                .iter()
                .map(|item| item.loading_order)
                .collect::<Vec<_>>();
            loading_orders.sort_unstable();
            super::report::Bpp3dPackedBinReport {
                name: bin.name.clone(),
                bin_type: bin.bin_type.type_code.clone(),
                item_count: bin.items.len(),
                batch_no: bin.batch_no.clone(),
                loading_orders,
            }
        })
        .collect()
}

fn framework_shadow_price_summary(
    demand_constraint: &DemandConstraint<f64, Meter>,
    demand_entries: &[Bpp3dDemandEntry],
    model: &MetaModel<f64>,
    dual_solution: &[f64],
) -> HashMap<String, f64> {
    let mut shadow_price_map = BasicShadowPriceMap::<DemandShadowPriceKey>::new();
    if extract_shadow_price::<
        DemandShadowPriceKey,
        MetaModel<f64>,
        BasicShadowPriceMap<DemandShadowPriceKey>,
        DemandConstraint<f64, Meter>,
    >(
        &mut shadow_price_map,
        std::slice::from_ref(demand_constraint),
        model,
        dual_solution,
    )
    .is_err()
    {
        return demand_entries
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                (
                    demand_key_name(entry),
                    dual_solution.get(index).copied().unwrap_or(0.0),
                )
            })
            .collect();
    }
    demand_entries
        .iter()
        .map(|entry| {
            let key = DemandConstraint::<f64, Meter>::shadow_price_key(entry);
            let price_key = ShadowPriceKey::named::<DemandShadowPriceKey>(format!("{:?}", key));
            (
                demand_key_name(entry),
                shadow_price_map
                    .get(&price_key)
                    .map(|price| price.price)
                    .unwrap_or(0.0),
            )
        })
        .collect()
}

fn rmp_column_upper_bounds(
    layers: &[BinLayer<f64, Meter>],
    demand_entries: &[Bpp3dDemandEntry],
) -> Vec<Option<f64>> {
    layers
        .iter()
        .map(|layer| {
            let mut upper_bound: Option<f64> = None;
            for entry in demand_entries {
                let coefficient = layer.demand_coverage_coefficient(entry.mode, &entry.key);
                if coefficient > 0.0 {
                    let candidate = entry.demand / coefficient;
                    upper_bound = Some(match upper_bound {
                        Some(current) => current.min(candidate),
                        None => candidate,
                    });
                }
            }
            upper_bound.map(|bound| bound.max(0.0))
        })
        .collect()
}

fn rmp_layer_volume_terms(
    assignment: &ImpreciseAssignment<f64, Meter>,
) -> Vec<(usize, f64)> {
    assignment
        .layers
        .iter()
        .enumerate()
        .map(|(layer_idx, layer)| {
            let model_idx = assignment.x.index(&layer_idx).unwrap_or(layer_idx);
            let volume = layer
                .bin
                .as_ref()
                .map(|bin| bin.width.value * bin.height.value * layer.depth.value)
                .unwrap_or(layer.depth.value);
            (model_idx, volume)
        })
        .collect()
}

fn final_assignment_indices_by_bin(
    assignment: &PreciseAssignment<f64, Meter>,
) -> Vec<Vec<(usize, usize)>> {
    assignment.bins
        .iter()
        .enumerate()
        .map(|(bin_idx, _)| {
            assignment.layers
                .iter()
                .enumerate()
                .filter_map(|(layer_idx, _)| {
                    assignment
                        .x
                        .index(&bin_idx, &layer_idx)
                        .or(Some(bin_idx * assignment.layers.len() + layer_idx))
                        .map(|model_idx| (layer_idx, model_idx))
                })
                .collect()
        })
        .collect()
}

fn final_bin_marker_indices(
    assignment: &PreciseAssignment<f64, Meter>,
) -> Vec<usize> {
    (0..assignment.bins.len())
        .filter_map(|bin_idx| {
            assignment
                .v
                .index(&bin_idx)
                .or(Some(assignment.bins.len() * assignment.layers.len() + bin_idx))
        })
        .collect()
}

fn final_layer_weights(
    layers: &[BinLayer<f64, Meter>],
    items: &[ActualItem<f64, Meter>],
) -> Vec<f64> {
    let item_weights = items
        .iter()
        .map(|item| (item.id.as_str(), item.weight.value))
        .collect::<HashMap<_, _>>();
    layers
        .iter()
        .map(|layer| {
            layer
                .demand_coverage
                .iter()
                .filter_map(|coverage| match (&coverage.mode, &coverage.key) {
                    (
                        Bpp3dDemandMode::Item | Bpp3dDemandMode::ItemAmount,
                        Bpp3dDemandKey::Item { id },
                    ) => item_weights
                        .get(id.as_str())
                        .map(|weight| coverage.coefficient * *weight),
                    (
                        Bpp3dDemandMode::ItemWeight,
                        Bpp3dDemandKey::Item { .. },
                    ) => Some(coverage.coefficient),
                    _ => None,
                })
                .sum()
        })
        .collect()
}

fn final_layer_volumes(layers: &[BinLayer<f64, Meter>]) -> Vec<f64> {
    layers
        .iter()
        .map(|layer| {
            layer
                .bin
                .as_ref()
                .map(|bin| bin.width.value * bin.height.value * layer.depth.value)
                .unwrap_or(layer.depth.value)
        })
        .collect()
}

fn layer_depths(layers: &[BinLayer<f64, Meter>]) -> Vec<f64> {
    layers
        .iter()
        .map(|layer| layer.depth.value)
        .collect()
}

fn layer_trace_key(layer: &BinLayer<f64, Meter>) -> String {
    format!("{}:{}", layer.from, layer.depth.value)
}

fn item_shadow_price_key(item_id: impl Into<String>) -> String {
    format!("item:{}", item_id.into())
}

fn application_state_from_algorithm(
    algorithm: &ColumnGenerationAlgorithm<f64, Meter>,
    items: Vec<ActualItem<f64, Meter>>,
    bins: Vec<BinType<f64, Meter>>,
    initial_layers: Vec<BinLayer<f64, Meter>>,
    layer_block_traces: HashMap<usize, Vec<LayerBlockTrace<f64, Meter>>>,
    layer_placement_traces: HashMap<usize, Vec<LayerPlacementTrace<f64, Meter>>>,
) -> ColumnGenerationApplicationState {
    application_state_from_algorithm_with_continuous_radius(
        algorithm,
        items,
        bins,
        initial_layers,
        layer_block_traces,
        layer_placement_traces,
        None,
    )
}

fn application_state_from_algorithm_with_continuous_radius(
    algorithm: &ColumnGenerationAlgorithm<f64, Meter>,
    items: Vec<ActualItem<f64, Meter>>,
    bins: Vec<BinType<f64, Meter>>,
    initial_layers: Vec<BinLayer<f64, Meter>>,
    layer_block_traces: HashMap<usize, Vec<LayerBlockTrace<f64, Meter>>>,
    layer_placement_traces: HashMap<usize, Vec<LayerPlacementTrace<f64, Meter>>>,
    continuous_radius_component: Option<ContinuousRadiusModelComponent>,
) -> ColumnGenerationApplicationState {
    ColumnGenerationApplicationState {
        items,
        bins,
        initial_layers,
        layers: algorithm.active_layers(),
        iteration: algorithm.state.iteration,
        layer_block_traces,
        layer_placement_traces,
        continuous_radius_component,
        info: HashMap::new(),
    }
}

fn default_item_coverage(items: &[ActualItem<f64, Meter>]) -> Vec<Bpp3dLayerDemandCoverage> {
    items
        .iter()
        .map(|item| {
            Bpp3dLayerDemandCoverage::new(
                Bpp3dDemandMode::Item,
                Bpp3dDemandKey::Item { id: item.id.clone() },
                1.0,
            )
        })
        .collect()
}

fn ensure_layer_demand_coverage(
    layers: Vec<BinLayer<f64, Meter>>,
    items: &[ActualItem<f64, Meter>],
) -> Vec<BinLayer<f64, Meter>> {
    let fallback = default_item_coverage(items);
    layers
        .into_iter()
        .map(|layer| {
            if layer.demand_coverage.is_empty() {
                layer.with_demand_coverage(fallback.clone())
            } else {
                layer
            }
        })
        .collect()
}

fn layer_generation_demand_entries(
    items: &[ActualItem<f64, Meter>],
    shadow_price_summary: &HashMap<String, f64>,
) -> Vec<LayerGenerationDemandEntry> {
    items
        .iter()
        .map(|item| LayerGenerationDemandEntry {
            mode: Bpp3dDemandMode::Item,
            key: Bpp3dDemandKey::Item { id: item.id.clone() },
            demand: 1.0,
            satisfied: if shadow_price_summary.contains_key(&item_shadow_price_key(&item.id)) {
                0.0
            } else {
                1.0
            },
        })
        .collect()
}

fn layer_generation_shadow_prices(
    shadow_price_summary: &HashMap<String, f64>,
) -> HashMap<DemandShadowPriceKey, f64> {
    shadow_price_summary
        .iter()
        .filter_map(|(key, value)| {
            key.strip_prefix("item:").map(|id| {
                (
                    DemandShadowPriceKey {
                        mode: Bpp3dDemandMode::Item,
                        key: Bpp3dDemandKey::Item { id: id.to_string() },
                    },
                    *value,
                )
            })
        })
        .collect()
}

