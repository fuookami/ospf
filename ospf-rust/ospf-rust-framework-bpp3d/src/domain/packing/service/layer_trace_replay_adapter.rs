// ============================================================================
// LayerTraceReplayAdapter - 层 trace 回放适配器 / Layer trace replay adapter
// ============================================================================

/// 层 trace 回放结果 / Layer trace replay result
#[derive(Debug, Clone, Default)]
pub struct LayerTraceReplayResult<V, U: UnitTrait> {
    /// 已装箱列表 / Packed bins
    pub packed_bins: Vec<PackedBin<V, U>>,
    /// 诊断信息 / Diagnostics
    pub diagnostics: Vec<String>,
}

/// 层 trace 回放适配器 / Layer trace replay adapter
///
/// 将 layer generation trace 和 coverage fallback 回放为最终已装箱结果。
/// Replays layer generation traces and coverage fallback into final packed bins.
#[derive(Debug, Clone, Default)]
pub struct LayerTraceReplayAdapter {
    placement_adapter: LayerPlacementAdapter,
}

impl LayerTraceReplayAdapter {
    /// 创建适配器 / Create adapter
    pub fn new() -> Self {
        Self {
            placement_adapter: LayerPlacementAdapter::new(),
        }
    }

    /// 回放选中层 / Replay selected layers
    pub fn replay_selected_layers<V, U>(
        &self,
        layers: &[BinLayer<V, U>],
        layer_indices: &[usize],
        items: &[ActualItem<V, U>],
        bins: &[BinType<V, U>],
        block_traces: &HashMap<usize, Vec<LayerBlockTrace<V, U>>>,
        placement_traces: &HashMap<usize, Vec<LayerPlacementTrace<V, U>>>,
    ) -> LayerTraceReplayResult<V, U>
    where
        V: num_traits::Float + Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + Into<f64>,
        U: CTUnit + Default + Clone + Debug + Send + Sync,
    {
        let mut packed_bins = Vec::new();
        let mut diagnostics = Vec::new();

        for (layer_idx, layer) in layers.iter().enumerate() {
            let original_layer_index = layer_indices.get(layer_idx).copied().unwrap_or(layer_idx);
            let Some(bin_type) = layer.bin.clone().or_else(|| bins.first().cloned()) else {
                diagnostics.push(format!("selected layer {} has no bin type", layer_idx));
                continue;
            };
            let placements = if let Some(traces) = block_traces.get(&original_layer_index) {
                placements_from_block_traces(traces, items, &mut diagnostics)
            } else if let Some(traces) = placement_traces.get(&original_layer_index) {
                placements_from_traces(traces, items, &bin_type, &mut diagnostics)
            } else {
                placements_from_coverage(layer, items, layer_idx, &mut diagnostics)
            };
            if placements.is_empty() {
                continue;
            }
            match self.placement_adapter.to_packed_bin(
                format!("selected-layer-{}", layer_idx),
                bin_type,
                None,
                placements,
            ) {
                Ok(packed_bin) => packed_bins.push(packed_bin),
                Err(errors) => diagnostics.extend(errors),
            }
        }

        LayerTraceReplayResult {
            packed_bins,
            diagnostics,
        }
    }
}

fn placements_from_block_traces<V, U>(
    traces: &[LayerBlockTrace<V, U>],
    items: &[ActualItem<V, U>],
    diagnostics: &mut Vec<String>,
) -> Vec<KnownCoordinatePlacement<V, U>>
where
    V: num_traits::Float + Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + Into<f64>,
    U: CTUnit + Default + Clone + Debug + Send + Sync,
{
    let mut placements = Vec::new();
    for trace in traces {
        let Some(item) = items.get(trace.item_index).cloned() else {
            diagnostics.push(format!(
                "block trace references missing item index {}",
                trace.item_index,
            ));
            continue;
        };
        placements.extend(expand_block_trace(trace, item));
    }
    placements
}

fn placements_from_traces<V, U>(
    traces: &[LayerPlacementTrace<V, U>],
    items: &[ActualItem<V, U>],
    bin_type: &BinType<V, U>,
    diagnostics: &mut Vec<String>,
) -> Vec<KnownCoordinatePlacement<V, U>>
where
    V: num_traits::Float + Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + Into<f64>,
    U: CTUnit + Default + Clone + Debug + Send + Sync,
{
    let mut placements = Vec::new();
    for trace in traces {
        let Some(item) = items.get(trace.item_index).cloned() else {
            diagnostics.push(format!(
                "placement trace references missing item index {}",
                trace.item_index,
            ));
            continue;
        };
        placements.extend(expand_placement_trace(trace, item, bin_type, diagnostics));
    }
    placements
}

fn expand_block_trace<V, U>(
    trace: &LayerBlockTrace<V, U>,
    item: ActualItem<V, U>,
) -> Vec<KnownCoordinatePlacement<V, U>>
where
    V: num_traits::Float + Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + Into<f64>,
    U: CTUnit + Default + Clone + Debug + Send + Sync,
{
    let size = oriented_item_size(&item, trace.orientation);
    let mut placements = Vec::new();
    let mut emitted = 0_u64;
    for z in 0..trace.nz {
        for y in 0..trace.ny {
            for x in 0..trace.nx {
                if emitted >= trace.item_count {
                    return placements;
                }
                placements.push(KnownCoordinatePlacement {
                    item_index: trace.item_index,
                    item: item.clone(),
                    position: MetricPoint3 {
                        x: quantity_from_f64(trace.origin.x.value.clone().into() + size.width.value.clone().into() * x as f64),
                        y: quantity_from_f64(trace.origin.y.value.clone().into() + size.height.value.clone().into() * y as f64),
                        z: quantity_from_f64(trace.origin.z.value.clone().into() + size.depth.value.clone().into() * z as f64),
                    },
                    orientation: trace.orientation,
                });
                emitted += 1;
            }
        }
    }
    placements
}

fn placements_from_coverage<V, U>(
    layer: &BinLayer<V, U>,
    items: &[ActualItem<V, U>],
    layer_idx: usize,
    diagnostics: &mut Vec<String>,
) -> Vec<KnownCoordinatePlacement<V, U>>
where
    V: num_traits::Float + Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + Into<f64>,
    U: CTUnit + Default + Clone + Debug + Send + Sync,
{
    let Some((item_index, item)) = first_covered_item(layer, items) else {
        diagnostics.push(format!("selected layer {} has no matching covered item", layer_idx));
        return Vec::new();
    };
    vec![KnownCoordinatePlacement {
        item_index,
        item,
        position: MetricPoint3 {
            x: quantity_from_f64(0.0),
            y: quantity_from_f64(0.0),
            z: quantity_from_f64(0.0),
        },
        orientation: Orientation::Upright,
    }]
}

fn expand_placement_trace<V, U>(
    trace: &LayerPlacementTrace<V, U>,
    item: ActualItem<V, U>,
    bin_type: &BinType<V, U>,
    diagnostics: &mut Vec<String>,
) -> Vec<KnownCoordinatePlacement<V, U>>
where
    V: num_traits::Float + Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + Into<f64>,
    U: CTUnit + Default + Clone + Debug + Send + Sync,
{
    let size = oriented_item_size(&item, trace.orientation);
    let max_x = grid_capacity(
        bin_type.width.value.clone().into() - trace.position.x.value.clone().into(),
        size.width.value.clone().into(),
    );
    let max_y = grid_capacity(
        bin_type.height.value.clone().into() - trace.position.y.value.clone().into(),
        size.height.value.clone().into(),
    );
    let max_z = grid_capacity(
        bin_type.depth.value.clone().into() - trace.position.z.value.clone().into(),
        size.depth.value.clone().into(),
    );
    let layer_capacity = max_x.saturating_mul(max_y).saturating_mul(max_z);
    if trace.amount > layer_capacity {
        diagnostics.push(format!(
            "placement trace for item {} truncated from {} to {} placements",
            trace.item_id,
            trace.amount,
            layer_capacity,
        ));
    }
    (0..trace.amount.min(layer_capacity))
        .map(|offset| {
            let x_index = offset % max_x;
            let y_index = (offset / max_x) % max_y;
            let z_index = offset / max_x / max_y;
            KnownCoordinatePlacement {
                item_index: trace.item_index,
                item: item.clone(),
                position: MetricPoint3 {
                    x: quantity_from_f64(
                        trace.position.x.value.clone().into()
                            + size.width.value.clone().into() * x_index as f64,
                    ),
                    y: quantity_from_f64(
                        trace.position.y.value.clone().into()
                            + size.height.value.clone().into() * y_index as f64,
                    ),
                    z: quantity_from_f64(
                        trace.position.z.value.clone().into()
                            + size.depth.value.clone().into() * z_index as f64,
                    ),
                },
                orientation: trace.orientation,
            }
        })
        .collect()
}

fn grid_capacity(available: f64, step: f64) -> u64 {
    if available <= 0.0 || step <= 0.0 {
        return 1;
    }
    (available / step).floor().max(1.0) as u64
}

fn oriented_item_size<V, U>(
    item: &ActualItem<V, U>,
    orientation: Orientation,
) -> MetricSize3<V, U>
where
    V: num_traits::Float + Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + Into<f64>,
    U: CTUnit + Default + Clone + Debug + Send + Sync,
{
    let shape = item.oriented_packing_shape(orientation);
    MetricSize3 {
        width: shape.bounding_width,
        height: shape.bounding_height,
        depth: shape.bounding_depth,
    }
}

fn first_covered_item<V, U>(
    layer: &BinLayer<V, U>,
    items: &[ActualItem<V, U>],
) -> Option<(usize, ActualItem<V, U>)>
where
    V: Clone + Debug + Send + Sync,
    U: UnitTrait + Debug + Clone + Send + Sync,
{
    layer
        .demand_coverage
        .iter()
        .find_map(|coverage| match &coverage.key {
            Bpp3dDemandKey::Item { id } if coverage.coefficient > 0.0 => {
                items
                    .iter()
                    .enumerate()
                    .find(|(_, item)| &item.id == id)
                    .map(|(index, item)| (index, item.clone()))
            }
            _ => None,
        })
}

fn quantity_from_f64<V, U>(value: f64) -> Quantity<V, U>
where
    V: num_traits::Float,
    U: CTUnit + Default,
{
    Quantity::new_ct(V::from(value).unwrap_or_else(V::zero))
}

