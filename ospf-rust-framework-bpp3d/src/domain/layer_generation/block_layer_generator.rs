// ============================================================================
// BlockLayerGenerator - 块层生成器 / Block layer generator
// ============================================================================

/// 块层生成器 / Block layer generator
///
/// 使用 SimpleBlockGenerator 生成块候选，每个块对应一个层候选。
/// Uses SimpleBlockGenerator to generate block candidates, each block
/// corresponds to a layer candidate.
#[derive(Debug)]
pub struct BlockLayerGenerator {
    /// 生成器名称 / Generator name
    name: String,
}

impl BlockLayerGenerator {
    /// 创建块层生成器 / Create block layer generator
    pub fn new() -> Self {
        Self {
            name: "block_layer_generator".to_string(),
        }
    }
}

impl Default for BlockLayerGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl<V, U> LayerGenerator<V, U> for BlockLayerGenerator
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + PartialOrd + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync + CTUnit + Default,
{
    fn name(&self) -> &str { &self.name }

    fn generate(&self, request: &LayerGenerationRequest<V, U>) -> Vec<LayerGenerationResult<V, U>> {
        let Some(bin) = request.bin.as_ref() else {
            return Vec::new();
        };
        let container_size = MetricSize3 {
            width: bin.width.clone(),
            height: bin.height.clone(),
            depth: bin.depth.clone(),
        };
        let amounts = request
            .items
            .iter()
            .map(|item| {
                request
                    .demand_entries
                    .iter()
                    .find_map(|entry| match &entry.key {
                        Bpp3dDemandKey::Item { id } if id == &item.id => {
                            Some(entry.remaining().ceil().to_u64().unwrap_or(1).max(1))
                        }
                        _ => None,
                    })
                    .unwrap_or(1)
            })
            .collect::<Vec<_>>();
        let package_attributes = request
            .items
            .iter()
            .map(|item| request.package_attributes.get(&item.id))
            .collect::<Vec<_>>();
        let indexed_items = request
            .items
            .iter()
            .cloned()
            .enumerate()
            .collect::<Vec<_>>();
        let blocks = SimpleBlockGenerator::default_generator()
            .generate_with_package_attributes(
                &request.items,
                &amounts,
                &container_size,
                &package_attributes,
            );
        blocks
            .into_iter()
            .filter(|block| block_allows_package_rule_policy(
                request,
                &indexed_items,
                block,
                bin,
            ))
            .enumerate()
            .filter_map(|(block_index, block)| {
                let Block::Simple(simple) = block else {
                    return None;
                };
                let item = request.items.get(simple.item_view.item_index)?;
                let coverage = vec![Bpp3dLayerDemandCoverage::new(
                    Bpp3dDemandMode::Item,
                    Bpp3dDemandKey::Item { id: item.id.clone() },
                    simple.item_count.to_f64().unwrap_or(0.0),
                )];
                let origin = MetricPoint3 {
                    x: Quantity::new_ct(V::zero()),
                    y: Quantity::new_ct(V::zero()),
                    z: Quantity::new_ct(V::zero()),
                };
                let size = MetricSize3 {
                    width: simple.width.clone(),
                    height: simple.height.clone(),
                    depth: simple.depth.clone(),
                };
                let block_trace = LayerBlockTrace {
                    block_index,
                    item_index: simple.item_view.item_index,
                    item_id: item.id.clone(),
                    orientation: simple.item_view.orientation,
                    nx: simple.nx,
                    ny: simple.ny,
                    nz: simple.nz,
                    item_count: simple.item_count,
                    size,
                    origin: origin.clone(),
                };
                let placement_trace = LayerPlacementTrace {
                    item_index: simple.item_view.item_index,
                    item_id: item.id.clone(),
                    position: origin,
                    orientation: simple.item_view.orientation,
                    amount: simple.item_count,
                };
                Some(LayerGenerationResult {
                    layer: BinLayer {
                        iteration: request.iteration,
                        from: self.name.clone(),
                        bin: request.bin.clone(),
                        depth: simple.depth.clone(),
                        demand_coverage: coverage,
                    },
                    reduced_cost: None,
                    score: None,
                    numeric_score: Some(simple.item_count.to_f64().unwrap_or(0.0)),
                    block_traces: vec![block_trace],
                    placement_traces: vec![placement_trace],
                    diagnostics: Vec::new(),
                    source: self.name.clone(),
                })
            })
            .collect()
    }
}

