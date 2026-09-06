// ============================================================================
// BLLocalLayerGenerator - BL 局部层生成器 / BL local layer generator
// ============================================================================

/// BL 局部层生成器 / BL local layer generator
///
/// 使用 BLA 算法在局部空间中生成层候选。
/// Uses BLA algorithm to generate layer candidates in local space.
#[derive(Debug)]
pub struct BLLocalLayerGenerator {
    /// 生成器名称 / Generator name
    name: String,
}

impl BLLocalLayerGenerator {
    /// 创建 BL 局部层生成器 / Create BL local layer generator
    pub fn new() -> Self {
        Self {
            name: "bl_local_layer_generator".to_string(),
        }
    }
}

impl Default for BLLocalLayerGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl<V, U> LayerGenerator<V, U> for BLLocalLayerGenerator
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync + CTUnit + Default,
{
    fn name(&self) -> &str { &self.name }

    fn generate(&self, _request: &LayerGenerationRequest<V, U>) -> Vec<LayerGenerationResult<V, U>> {
        bl_layer_candidates(_request, &self.name, 1)
    }
}

// ============================================================================
// BLGlobalLayerGenerator - BL 全局层生成器 / BL global layer generator
// ============================================================================

/// BL 全局层生成器 / BL global layer generator
///
/// 使用 BLA 算法在整个容器空间中生成层候选。
/// Uses BLA algorithm to generate layer candidates in the full container space.
#[derive(Debug)]
pub struct BLGlobalLayerGenerator {
    /// 生成器名称 / Generator name
    name: String,
}

impl BLGlobalLayerGenerator {
    /// 创建 BL 全局层生成器 / Create BL global layer generator
    pub fn new() -> Self {
        Self {
            name: "bl_global_layer_generator".to_string(),
        }
    }
}

impl Default for BLGlobalLayerGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl<V, U> LayerGenerator<V, U> for BLGlobalLayerGenerator
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync + CTUnit + Default,
{
    fn name(&self) -> &str { &self.name }

    fn generate(&self, _request: &LayerGenerationRequest<V, U>) -> Vec<LayerGenerationResult<V, U>> {
        bl_layer_candidates(_request, &self.name, _request.max_candidates)
    }
}

fn bl_layer_candidates<V, U>(
    request: &LayerGenerationRequest<V, U>,
    generator_name: &str,
    max_items: usize,
) -> Vec<LayerGenerationResult<V, U>>
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync + CTUnit + Default,
{
    let Some(bin) = request.bin.clone() else {
        return Vec::new();
    };
    let selected_items = request
        .items
        .iter()
        .enumerate()
        .take(max_items.max(1))
        .collect::<Vec<_>>();
    if selected_items.is_empty() {
        return Vec::new();
    }
    let projections = selected_items
        .iter()
        .map(|(item_index, item)| {
            let shape = item.packing_shape();
            BlaProjection {
                footprint: shape.footprint(),
                item_index: *item_index,
                bottom_only: request
                    .package_attributes
                    .get(&item.id)
                    .is_some_and(|attribute| attribute.bottom_only),
                height: item.height.clone(),
                weight: item.weight.clone(),
                allow_rotation: item.enabled_orientations.iter().any(|orientation| {
                    matches!(
                        orientation,
                        Orientation::UprightRotated | Orientation::SideRotated | Orientation::LieRotated
                    )
                }),
            }
        })
        .collect::<Vec<_>>();
    let bla = BottomUpLeftJustifiedAlgorithm::new(
        bin.width.clone(),
        bin.depth.clone(),
        BlaConfig::default(),
    );
    let placements = bla.invoke(&projections);
    let placement_traces = placements
        .iter()
        .enumerate()
        .filter_map(|(projection_index, placement)| {
            let placement = placement.as_ref()?;
            let (item_index, item) = selected_items.get(projection_index)?;
            let orientation = if placement.rotated {
                Orientation::UprightRotated
            } else {
                Orientation::Upright
            };
            let attribute = request.package_attributes.get(&item.id);
            if !item.enabled_orientations_at_bin(attribute, &bin).contains(&orientation)
                || !package_rule_policy_allows_orientation(
                    request,
                    item,
                    attribute,
                    orientation,
                    &bin,
                )
            {
                return None;
            }
            Some(LayerPlacementTrace {
                item_index: *item_index,
                item_id: item.id.clone(),
                position: MetricPoint3 {
                    x: placement.position.x.clone(),
                    y: Quantity::new_ct(V::zero()),
                    z: placement.position.y.clone(),
                },
                orientation,
                amount: 1,
            })
        })
        .collect::<Vec<_>>();
    if placement_traces.is_empty() {
        return Vec::new();
    }
    let demand_coverage = placement_traces
        .iter()
        .map(|trace| {
            Bpp3dLayerDemandCoverage::new(
                Bpp3dDemandMode::Item,
                Bpp3dDemandKey::Item {
                    id: trace.item_id.clone(),
                },
                1.0,
            )
        })
        .collect::<Vec<_>>();
    let max_depth = placement_traces
        .iter()
        .filter_map(|trace| {
            request
                .items
                .get(trace.item_index)
                .map(|item| trace.position.z.value + item.depth.value)
        })
        .fold(V::zero(), |acc, value| if value > acc { value } else { acc });
    vec![LayerGenerationResult {
        layer: BinLayer {
            iteration: request.iteration,
            from: generator_name.to_string(),
            bin: Some(bin),
            depth: Quantity::new_ct(max_depth),
            demand_coverage,
        },
        reduced_cost: None,
        score: None,
        numeric_score: Some(placement_traces.len().to_f64().unwrap_or(0.0)),
        block_traces: Vec::new(),
        placement_traces,
        diagnostics: vec![format!(
            "{} generated BLA placement candidate: projected_items={}, placed_items={}",
            generator_name,
            projections.len(),
            placements.iter().filter(|placement| placement.is_some()).count(),
        )],
        source: generator_name.to_string(),
    }]
}

