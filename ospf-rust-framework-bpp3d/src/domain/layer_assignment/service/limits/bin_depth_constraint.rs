// BinDepthConstraint - 箱深度约束 / Bin depth constraint
// ============================================================================

/// 箱深度约束 / Bin depth constraint
///
/// 对每个箱添加深度上界约束：
/// - `loadDepth[bin] <= depth_capacity`
///
/// Adds depth upper bound constraint per bin:
/// - `loadDepth[bin] <= depth_capacity`
#[derive(Debug)]
pub struct BinDepthConstraint {
    name: String,
    group: Option<ConstraintGroup>,
    /// 箱深度上界 / Depth capacity per bin
    pub depth_capacities: Vec<f64>,
    /// 赋值变量索引 (bin_idx, layer_idx) -> model_index / Assignment variable indices
    pub x_indices: Vec<Vec<(usize, usize)>>,
    /// 层深度系数 / Layer depth coefficients
    pub layer_depths: Vec<f64>,
}

impl BinDepthConstraint {
    /// 从箱型和适配器创建箱深度约束 / Create bin depth constraint from bin types and adapter
    pub fn from_bins<U>(
        bins: &[BinType<f64, U>],
        x_indices: Vec<Vec<(usize, usize)>>,
        layer_depths: Vec<f64>,
        adapter: &Bpp3dSolverValueAdapterKind,
    ) -> Self
    where
        U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
    {
        let depth_capacities: Vec<f64> = bins.iter()
            .map(|b| adapter.depth_to_solver(b.depth.value))
            .collect();

        Self {
            name: "bin_depth_constraint".to_string(),
            group: None,
            depth_capacities,
            x_indices,
            layer_depths,
        }
    }

    /// 直接从系数创建箱深度约束 / Create bin depth constraint directly from coefficients
    pub fn new(
        depth_capacities: Vec<f64>,
        x_indices: Vec<Vec<(usize, usize)>>,
        layer_depths: Vec<f64>,
    ) -> Self {
        Self {
            name: "bin_depth_constraint".to_string(),
            group: None,
            depth_capacities,
            x_indices,
            layer_depths,
        }
    }
}

impl Pipeline<MetaModel<f64>> for BinDepthConstraint {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { self.group.as_ref() }

    fn register(&self, model: &mut MetaModel<f64>) {
        for (bin_idx, layer_indices) in self.x_indices.iter().enumerate() {
            let depth_cap = self.depth_capacities.get(bin_idx).copied().unwrap_or(0.0);

            // 深度约束: sum(x[bin, layer] * depth[layer]) <= depth_capacity
            let depth_terms: Vec<(usize, f64)> = layer_indices.iter()
                .filter_map(|&(layer_idx, model_idx)| {
                    self.layer_depths.get(layer_idx).map(|&d| (model_idx, d))
                })
                .collect();

            if !depth_terms.is_empty() {
                if let Err(e) = model.add_le_constraint(
                    &depth_terms,
                    depth_cap,
                    &format!("{}_depth_{}", self.name, bin_idx),
                ) {
                    log::warn!("Failed to register {}_depth_{}: {:?}", self.name, bin_idx, e);
                }
            }
        }
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
// BinAmountMinimization - 箱数量最小化 / Bin amount minimization
