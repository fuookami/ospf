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
pub struct BinDepthConstraint<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    name: String,
    group: Option<ConstraintGroup>,
    /// 箱深度上界 / Depth capacity per bin
    pub depth_capacities: Vec<f64>,
    /// 赋值变量索引 (bin_idx, layer_idx) -> model_index / Assignment variable indices
    pub x_indices: Vec<Vec<(usize, usize)>>,
    /// 层深度系数 / Layer depth coefficients
    pub layer_depths: Vec<f64>,
    /// 精确赋值模型 / Precise assignment model (optional, for direct field access)
    pub assignment: Option<PreciseAssignment<V, U>>,
}

impl<V, U> BinDepthConstraint<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 从箱型和适配器创建箱深度约束 / Create bin depth constraint from bin types and adapter
    pub fn from_bins(
        bins: &[BinType<f64, U>],
        x_indices: Vec<Vec<(usize, usize)>>,
        layer_depths: Vec<f64>,
        adapter: &Bpp3dSolverValueAdapterKind,
    ) -> Self {
        let depth_capacities: Vec<f64> = bins.iter()
            .map(|b| adapter.depth_to_solver(b.depth.value))
            .collect();

        Self {
            name: "bin_depth_constraint".to_string(),
            group: None,
            depth_capacities,
            x_indices,
            layer_depths,
            assignment: None,
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
            assignment: None,
        }
    }

    /// 从赋值模型和容量创建箱深度约束 / Create bin depth constraint from assignment and capacity
    ///
    /// 使用 `assignment.x.model_index()` 直接获取模型索引，无需预计算。
    /// Uses `assignment.x.model_index()` to get model indices directly,
    /// without pre-computation.
    pub fn new_with_assignment(
        assignment: PreciseAssignment<V, U>,
        depth_capacities: Vec<f64>,
        layer_depths: Vec<f64>,
    ) -> Self {
        Self {
            name: "bin_depth_constraint".to_string(),
            group: None,
            depth_capacities,
            x_indices: Vec::new(),
            layer_depths,
            assignment: Some(assignment),
        }
    }
}

impl<V, U> Pipeline<MetaModel<f64>> for BinDepthConstraint<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { self.group.as_ref() }

    fn register(&self, model: &mut MetaModel<f64>) {
        if let Some(ref assignment) = self.assignment {
            // Use registered symbols when available (Phase J)
            if !assignment.load_depth_symbols.is_empty() {
                // Build constraints from registered symbols
                for bin_idx in 0..assignment.bins.len() {
                    let depth_cap = self.depth_capacities.get(bin_idx).copied().unwrap_or(0.0);

                    // Depth constraint from registered load_depth symbol
                    if let Some(symbol) = assignment.load_depth_symbols.get(bin_idx) {
                        let poly = symbol.to_linear_polynomial();
                        let depth_terms: Vec<(usize, f64)> = poly.monomials().iter()
                            .map(|m| (m.var_index(), *m.coefficient()))
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
            } else {
                // Symbols not available: skip constraint registration and warn
                log::warn!(
                    "{}: load_depth_symbols not populated; \
                     call build_symbols() before registering constraints",
                    self.name
                );
            }
        } else {
            // Pre-computed x_indices path
            for (bin_idx, layer_indices) in self.x_indices.iter().enumerate() {
                let depth_cap = self.depth_capacities.get(bin_idx).copied().unwrap_or(0.0);

                // Depth constraint: sum(x[bin, layer] * depth[layer]) <= depth_capacity
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
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
// BinAmountMinimization - 箱数量最小化 / Bin amount minimization
