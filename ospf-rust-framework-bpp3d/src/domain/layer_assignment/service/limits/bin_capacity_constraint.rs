// ============================================================================
// BinCapacityConstraint - 箱容量约束 / Bin capacity constraint
// ============================================================================

/// 箱容量约束 / Bin capacity constraint
///
/// 对每个箱添加载重和体积上界约束：
/// - `loadWeight[bin] <= weight_capacity`
/// - `loadVolume[bin] <= volume_capacity`
///
/// Adds weight and volume upper bound constraints per bin:
/// - `loadWeight[bin] <= weight_capacity`
/// - `loadVolume[bin] <= volume_capacity`
#[derive(Debug)]
pub struct BinCapacityConstraint {
    name: String,
    group: Option<ConstraintGroup>,
    /// 箱载重上界 / Weight capacity per bin
    pub weight_capacities: Vec<f64>,
    /// 箱体积上界 / Volume capacity per bin
    pub volume_capacities: Vec<f64>,
    /// 赋值变量索引 (bin_idx, layer_idx) -> model_index / Assignment variable indices
    pub x_indices: Vec<Vec<(usize, usize)>>,
    /// 层载重系数 / Layer weight coefficients
    pub layer_weights: Vec<f64>,
    /// 层体积系数 / Layer volume coefficients
    pub layer_volumes: Vec<f64>,
}

impl BinCapacityConstraint {
    /// 从箱型和适配器创建箱容量约束 / Create bin capacity constraint from bin types and adapter
    pub fn from_bins<U>(
        bins: &[BinType<f64, U>],
        x_indices: Vec<Vec<(usize, usize)>>,
        layer_weights: Vec<f64>,
        layer_volumes: Vec<f64>,
        adapter: &Bpp3dSolverValueAdapterKind,
    ) -> Self
    where
        U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
    {
        let weight_capacities: Vec<f64> = bins.iter()
            .map(|b| adapter.weight_to_solver(b.capacity.value))
            .collect();
        let volume_capacities: Vec<f64> = bins.iter()
            .map(|b| adapter.volume_to_solver(b.width.value * b.height.value * b.depth.value))
            .collect();

        Self {
            name: "bin_capacity_constraint".to_string(),
            group: None,
            weight_capacities,
            volume_capacities,
            x_indices,
            layer_weights,
            layer_volumes,
        }
    }

    /// 直接从系数创建箱容量约束 / Create bin capacity constraint directly from coefficients
    pub fn new(
        weight_capacities: Vec<f64>,
        volume_capacities: Vec<f64>,
        x_indices: Vec<Vec<(usize, usize)>>,
        layer_weights: Vec<f64>,
        layer_volumes: Vec<f64>,
    ) -> Self {
        Self {
            name: "bin_capacity_constraint".to_string(),
            group: None,
            weight_capacities,
            volume_capacities,
            x_indices,
            layer_weights,
            layer_volumes,
        }
    }
}

impl Pipeline<MetaModel<f64>> for BinCapacityConstraint {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { self.group.as_ref() }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (bin_idx, layer_indices) in self.x_indices.iter().enumerate() {
            let weight_cap = self.weight_capacities.get(bin_idx).copied().unwrap_or(0.0);
            let volume_cap = self.volume_capacities.get(bin_idx).copied().unwrap_or(0.0);

            // 载重约束: sum(x[bin, layer] * weight[layer]) <= weight_capacity
            let weight_terms: Vec<(usize, f64)> = layer_indices.iter()
                .filter_map(|&(layer_idx, model_idx)| {
                    self.layer_weights.get(layer_idx).map(|&w| (model_idx, w))
                })
                .collect();

            if !weight_terms.is_empty() {
                if let Err(e) = model.add_le_constraint(
                    &weight_terms,
                    weight_cap,
                    &format!("{}_weight_{}", self.name, bin_idx),
                ) {
                    log::warn!("Failed to register {}_weight_{}: {:?}", self.name, bin_idx, e);
                }
            }

            // 体积约束: sum(x[bin, layer] * volume[layer]) <= volume_capacity
            let volume_terms: Vec<(usize, f64)> = layer_indices.iter()
                .filter_map(|&(layer_idx, model_idx)| {
                    self.layer_volumes.get(layer_idx).map(|&v| (model_idx, v))
                })
                .collect();

            if !volume_terms.is_empty() {
                if let Err(e) = model.add_le_constraint(
                    &volume_terms,
                    volume_cap,
                    &format!("{}_volume_{}", self.name, bin_idx),
                ) {
                    log::warn!("Failed to register {}_volume_{}: {:?}", self.name, bin_idx, e);
                }
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
