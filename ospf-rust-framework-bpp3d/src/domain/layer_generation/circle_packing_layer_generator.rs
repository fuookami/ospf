// ============================================================================
// CirclePackingLayerGenerator - 圆填充层生成器 / Circle packing layer generator
// ============================================================================

/// 圆填充层生成器 / Circle packing layer generator
///
/// 为圆柱物品（Axis3.Y 竖直或 Axis3.X/Z 横向固定/离散半径）生成层候选。
/// 横向圆柱必须有支撑覆盖验证。
///
/// Generates layer candidates for cylindrical items (Axis3.Y vertical
/// or Axis3.X/Z horizontal fixed/discrete radius).
/// Horizontal cylinders must pass support coverage verification.
#[derive(Debug)]
pub struct CirclePackingLayerGenerator {
    /// 生成器名称 / Generator name
    name: String,
}

impl CirclePackingLayerGenerator {
    /// 创建圆填充层生成器 / Create circle packing layer generator
    pub fn new() -> Self {
        Self {
            name: "circle_packing_layer_generator".to_string(),
        }
    }
}

impl Default for CirclePackingLayerGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl<V, U> LayerGenerator<V, U> for CirclePackingLayerGenerator
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync + CTUnit + Default,
{
    fn name(&self) -> &str { &self.name }

    fn generate(&self, request: &LayerGenerationRequest<V, U>) -> Vec<LayerGenerationResult<V, U>> {
        let Some(bin) = request.bin.clone() else {
            return Vec::new();
        };
        request
            .items
            .iter()
            .enumerate()
            .filter_map(|(item_index, item)| {
                let Some(PackageShapeSpec::Cylinder {
                    axis,
                    radius,
                    radius_candidates,
                    ..
                }) = &item.shape_spec_override else {
                    return None;
                };
                let attribute = request.package_attributes.get(&item.id);
                if !item.enabled_orientations_at_bin(attribute, &bin).contains(&Orientation::Upright)
                    || !package_rule_policy_allows_orientation(
                        request,
                        item,
                        attribute,
                        Orientation::Upright,
                        &bin,
                    )
                {
                    return None;
                }
                let radius = radius_candidates
                    .as_ref()
                    .and_then(|candidates| candidates.first())
                    .cloned()
                    .unwrap_or_else(|| radius.clone());
                let demand_key = Bpp3dDemandKey::Item {
                    id: item.id.clone(),
                };
                let amount = cylinder_grid_amount(&bin, item, *axis, &radius);
                if amount == 0 {
                    return Some(unsupported_layer_generation_result(
                        request,
                        &self.name,
                        format!(
                            "circle packing generation found no feasible grid position for item {} on axis {:?}",
                            item.id, axis,
                        ),
                    ));
                }
                let y = match axis {
                    Axis3::Y => Quantity::new_ct(V::zero()),
                    Axis3::X | Axis3::Z => Quantity::new_ct(V::zero()),
                };
                if let Err(message) = HorizontalCylinderGuard::validate_candidate(*axis, &y) {
                    return Some(unsupported_layer_generation_result(
                        request,
                        &self.name,
                        message,
                    ));
                }
                let depth = item.packing_shape().bounding_depth;
                let shadow_key = DemandShadowPriceKey {
                    mode: Bpp3dDemandMode::Item,
                    key: demand_key.clone(),
                };
                let shadow_price = request.shadow_prices.get(&shadow_key).cloned();
                let amount_f64 = amount.to_f64().unwrap_or(0.0);
                let numeric_score = amount_f64 + if shadow_price.is_some() { 1.0 } else { 0.0 };
                Some(LayerGenerationResult {
                    layer: BinLayer {
                        iteration: request.iteration,
                        from: self.name.clone(),
                        bin: Some(bin.clone()),
                        depth,
                        demand_coverage: vec![Bpp3dLayerDemandCoverage::new(
                            Bpp3dDemandMode::Item,
                            demand_key,
                            amount_f64,
                        )],
                    },
                    reduced_cost: None,
                    score: shadow_price.clone(),
                    numeric_score: Some(numeric_score),
                    block_traces: Vec::new(),
                    placement_traces: vec![LayerPlacementTrace {
                        item_index,
                        item_id: item.id.clone(),
                        position: MetricPoint3 {
                            x: Quantity::new_ct(V::zero()),
                            y,
                            z: Quantity::new_ct(V::zero()),
                        },
                        orientation: Orientation::Upright,
                        amount,
                    }],
                    diagnostics: vec![format!(
                        "circle packing generated conservative grid candidate: item_id={}, axis={:?}, radius={:?}, amount={}, shadow_price={:?}",
                        item.id,
                        axis,
                        radius.value,
                        amount,
                        shadow_price,
                    )],
                    source: self.name.clone(),
                })
            })
            .take(request.max_candidates)
            .collect()
    }
}

fn cylinder_grid_amount<V, U>(
    bin: &BinType<V, U>,
    item: &ActualItem<V, U>,
    axis: Axis3,
    radius: &Quantity<V, U>,
) -> u64
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialOrd,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    let diameter = radius.value + radius.value;
    if diameter <= V::zero() {
        return 0;
    }
    let (axis_len, lane_a, lane_b) = match axis {
        Axis3::X => (bin.width.value, bin.height.value, bin.depth.value),
        Axis3::Y => (bin.height.value, bin.width.value, bin.depth.value),
        Axis3::Z => (bin.depth.value, bin.width.value, bin.height.value),
    };
    let item_axis_len = match axis {
        Axis3::X => item.width.value,
        Axis3::Y => item.height.value,
        Axis3::Z => item.depth.value,
    };
    if item_axis_len > axis_len {
        return 0;
    }
    let count_a = (lane_a / diameter).floor().to_u64().unwrap_or(0);
    let count_b = (lane_b / diameter).floor().to_u64().unwrap_or(0);
    count_a.saturating_mul(count_b).max(1)
}

