// ============================================================================
// PreciseAssignmentActivationConstraint - 精确赋值启用约束 / Precise assignment activation constraint
// ============================================================================

/// 精确赋值启用约束 / Precise assignment activation constraint
///
/// 将 final MILP 的层赋值变量绑定到箱启用变量：`x[bin, layer] <= v[bin]`。
/// Binds final MILP layer assignment variables to bin activation variables:
/// `x[bin, layer] <= v[bin]`.
#[derive(Debug)]
pub struct PreciseAssignmentActivationConstraint<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    name: String,
    group: Option<ConstraintGroup>,
    /// 精确赋值模型 / Precise assignment model
    pub assignment: PreciseAssignment<V, U>,
}

impl<V, U> PreciseAssignmentActivationConstraint<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 创建精确赋值启用约束 / Create precise assignment activation constraint
    pub fn new(assignment: PreciseAssignment<V, U>) -> Self {
        Self {
            name: "precise_assignment_activation_constraint".to_string(),
            group: None,
            assignment,
        }
    }
}

impl<V, U> Pipeline<MetaModel<f64>> for PreciseAssignmentActivationConstraint<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { self.group.as_ref() }

    fn register(&self, model: &mut MetaModel<f64>) {
        for bin_idx in 0..self.assignment.bins.len() {
            let Some(v_idx) = precise_bin_marker_index(&self.assignment, bin_idx) else {
                continue;
            };
            for layer_idx in 0..self.assignment.layers.len() {
                let Some(x_idx) = precise_assignment_index(&self.assignment, bin_idx, layer_idx) else {
                    continue;
                };
                if let Err(e) = model.add_le_constraint(
                    &[(x_idx, 1.0), (v_idx, -1.0)],
                    0.0,
                    &format!("{}_{}_{}", self.name, bin_idx, layer_idx),
                ) {
                    log::warn!(
                        "Failed to register {}_{}_{}: {:?}",
                        self.name,
                        bin_idx,
                        layer_idx,
                        e
                    );
                }
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

impl<V, U> CGPipeline<DemandShadowPriceKey, MetaModel<f64>, BasicShadowPriceMap<DemandShadowPriceKey>>
    for DemandConstraint<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    type Extractor = fn(&BasicShadowPriceMap<DemandShadowPriceKey>, &DemandShadowPriceKey) -> f64;

    fn extractor(&self) -> Option<Self::Extractor> {
        Some(|map, key| map.get(&ShadowPriceKey::named::<DemandShadowPriceKey>(format!("{:?}", key)))
            .map(|sp| sp.price)
            .unwrap_or(0.0))
    }

    fn refresh(
        &self,
        shadow_price_map: &mut BasicShadowPriceMap<DemandShadowPriceKey>,
        _model: &MetaModel<f64>,
        shadow_prices: &[f64],
    ) -> Result<()> {
        for (idx, entry) in self.demand_entries.iter().enumerate() {
            let key = DemandShadowPriceKey {
                mode: entry.mode,
                key: entry.key.clone(),
            };
            let price = shadow_prices.get(idx).copied().unwrap_or(0.0);
            let sp_key = ShadowPriceKey::named::<DemandShadowPriceKey>(format!("{:?}", key));
            shadow_price_map.put(ShadowPrice::new(sp_key, price));
        }
        Ok(())
    }
}

