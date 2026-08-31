// ============================================================================
// ImpreciseAssignment - 不精确赋值 / Imprecise assignment
// ============================================================================

/// 不精确赋值 / Imprecise assignment
///
/// 列生成 RMP 阶段的赋值模型，x[layer] 为列变量。
/// Assignment model for the column generation RMP phase,
/// where x[layer] is a column variable.
#[derive(Debug, Clone)]
pub struct ImpreciseAssignment<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 层列表 / Layers
    pub layers: Vec<BinLayer<V, U>>,
    /// 列变量 x[layer] / Column variables x[layer]
    pub x: Option<IndexedVariableCombination1<usize, UContinuous>>,
    /// 列变量上界 / Column variable upper bounds
    pub upper_bounds: Vec<Option<f64>>,
    /// 每层载重符号 load_weight[layer] = sum(x[layer] * weight[layer])
    /// Per-layer load weight symbols
    pub load_weight_symbols: Vec<Arc<ospf_rust_core::symbol::expression_symbol::LinearExpressionSymbol<f64>>>,
    /// 每层体积符号 load_volume[layer] = sum(x[layer] * volume[layer])
    /// Per-layer load volume symbols
    pub load_volume_symbols: Vec<Arc<ospf_rust_core::symbol::expression_symbol::LinearExpressionSymbol<f64>>>,
    /// 每层深度符号 load_depth[layer] = sum(x[layer] * depth[layer])
    /// Per-layer load depth symbols
    pub load_depth_symbols: Vec<Arc<ospf_rust_core::symbol::expression_symbol::LinearExpressionSymbol<f64>>>,
    /// 每层需求覆盖符号 load[layer] = sum(x[layer] * demand_coeff[layer])
    /// Per-layer demand coverage symbols
    pub load_symbols: Vec<Arc<ospf_rust_core::symbol::expression_symbol::LinearExpressionSymbol<f64>>>,
}

impl<V: Debug + Clone + Send + Sync, U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync> ImpreciseAssignment<V, U> {
    /// 创建不精确赋值 / Create imprecise assignment
    pub fn new(
        layers: Vec<BinLayer<V, U>>,
        upper_bounds: Vec<Option<f64>>,
    ) -> Self {
        Self {
            layers,
            x: None,
            upper_bounds,
            load_weight_symbols: Vec::new(),
            load_volume_symbols: Vec::new(),
            load_depth_symbols: Vec::new(),
            load_symbols: Vec::new(),
        }
    }

    /// 构建并注册中间表达式符号 / Build and register intermediate expression symbols
    ///
    /// 在变量注册后调用，为每个层创建载重、体积、深度和需求覆盖的中间符号。
    /// Called after variable registration to create intermediate symbols for
    /// load weight, volume, depth, and demand coverage per layer.
    pub fn build_symbols(
        &mut self,
        model: &mut MetaModel<f64>,
        layer_weights: &[f64],
        layer_volumes: &[f64],
        layer_depths: &[f64],
        demand_entries: &[Bpp3dDemandEntry],
    ) -> Result<(), String> {
        let x = match self.x.as_ref() {
            Some(x) => x,
            None => return Err("x variables must be registered before building symbols".to_string()),
        };

        // Build per-layer load weight symbols
        self.load_weight_symbols.clear();
        for layer_idx in 0..self.layers.len() {
            let weight = layer_weights.get(layer_idx).copied().unwrap_or(0.0);
            if let Some(model_idx) = x.model_index(&layer_idx) {
                let symbol = build_linear_expression_symbol(
                    &format!("loadWeight_{}", layer_idx),
                    &[(model_idx, weight)],
                    0.0,
                );
                model.add_symbol(symbol.clone())
                    .map_err(|e| format!("Failed to register loadWeight_{}: {:?}", layer_idx, e))?;
                self.load_weight_symbols.push(symbol);
            }
        }

        // Build per-layer load volume symbols
        self.load_volume_symbols.clear();
        for layer_idx in 0..self.layers.len() {
            let volume = layer_volumes.get(layer_idx).copied().unwrap_or(0.0);
            if let Some(model_idx) = x.model_index(&layer_idx) {
                let symbol = build_linear_expression_symbol(
                    &format!("loadVolume_{}", layer_idx),
                    &[(model_idx, volume)],
                    0.0,
                );
                model.add_symbol(symbol.clone())
                    .map_err(|e| format!("Failed to register loadVolume_{}: {:?}", layer_idx, e))?;
                self.load_volume_symbols.push(symbol);
            }
        }

        // Build per-layer load depth symbols
        self.load_depth_symbols.clear();
        for layer_idx in 0..self.layers.len() {
            let depth = layer_depths.get(layer_idx).copied().unwrap_or(0.0);
            if let Some(model_idx) = x.model_index(&layer_idx) {
                let symbol = build_linear_expression_symbol(
                    &format!("loadDepth_{}", layer_idx),
                    &[(model_idx, depth)],
                    0.0,
                );
                model.add_symbol(symbol.clone())
                    .map_err(|e| format!("Failed to register loadDepth_{}: {:?}", layer_idx, e))?;
                self.load_depth_symbols.push(symbol);
            }
        }

        // Build per-layer demand coverage symbols
        self.load_symbols.clear();
        for layer_idx in 0..self.layers.len() {
            if let Some(model_idx) = x.model_index(&layer_idx) {
                let mut terms = Vec::new();
                for entry in demand_entries {
                    let coefficient = self.layers[layer_idx]
                        .demand_coverage_coefficient(entry.mode, &entry.key);
                    if coefficient != 0.0 {
                        terms.push((model_idx, coefficient));
                    }
                }
                let symbol = build_linear_expression_symbol(
                    &format!("load_{}", layer_idx),
                    &terms,
                    0.0,
                );
                model.add_symbol(symbol.clone())
                    .map_err(|e| format!("Failed to register load_{}: {:?}", layer_idx, e))?;
                self.load_symbols.push(symbol);
            }
        }

        Ok(())
    }
}

impl<V: Debug + Clone + Send + Sync, U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync> Bpp3dModelComponent for ImpreciseAssignment<V, U> {
    fn name(&self) -> &str {
        "ImpreciseAssignment"
    }

    fn register(&mut self, model: &mut MetaModel<f64>) -> Result<(), String> {
        if self.x.is_some() {
            return Ok(());
        }
        let keys: Vec<usize> = (0..self.layers.len()).collect();
        let upper_bounds = &self.upper_bounds;
        self.x = Some(
            IndexedVariableCombination1::new(
                "x",
                &keys,
                model,
                |key| format!("x_{:?}", key),
                |key| {
                    let ub = upper_bounds.get(*key).copied().flatten();
                    VariableRange::new(Some(0.0), ub)
                },
            )
            .map_err(|e| format!("Failed to register ImpreciseAssignment variables: {:?}", e))?,
        );
        Ok(())
    }
}

// ============================================================================
// PreciseAssignment - 精确赋值 / Precise assignment
// ============================================================================

/// 精确赋值 / Precise assignment
///
/// Final MILP 阶段的赋值模型，x[bin, layer] 为二值赋值变量。
/// Assignment model for the final MILP phase,
/// where x[bin, layer] is a binary assignment variable.
#[derive(Debug, Clone)]
pub struct PreciseAssignment<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 箱列表 / Bins
    pub bins: Vec<BinType<V, U>>,
    /// 层列表 / Layers
    pub layers: Vec<BinLayer<V, U>>,
    /// 赋值变量 x[bin, layer] / Assignment variables x[bin, layer]
    pub x: Option<IndexedVariableCombination2<usize, usize, Binary>>,
    /// 箱使用标记 v[bin] / Bin usage markers v[bin]
    pub v: Option<IndexedVariableCombination1<usize, Binary>>,
    /// 每箱载重符号 load_weight[bin] = sum(x[bin, layer] * weight[layer])
    /// Per-bin load weight symbols
    pub load_weight_symbols: Vec<Arc<ospf_rust_core::symbol::expression_symbol::LinearExpressionSymbol<f64>>>,
    /// 每箱体积符号 load_volume[bin] = sum(x[bin, layer] * volume[layer])
    /// Per-bin load volume symbols
    pub load_volume_symbols: Vec<Arc<ospf_rust_core::symbol::expression_symbol::LinearExpressionSymbol<f64>>>,
    /// 每箱深度符号 load_depth[bin] = sum(x[bin, layer] * depth[layer])
    /// Per-bin load depth symbols
    pub load_depth_symbols: Vec<Arc<ospf_rust_core::symbol::expression_symbol::LinearExpressionSymbol<f64>>>,
}

impl<V: Debug + Clone + Send + Sync, U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync> PreciseAssignment<V, U> {
    /// 构建并注册中间表达式符号 / Build and register intermediate expression symbols
    ///
    /// 在变量注册后调用，为每个箱创建载重、体积和深度的中间符号。
    /// Called after variable registration to create intermediate symbols for
    /// load weight, volume, and depth per bin.
    pub fn build_symbols(
        &mut self,
        model: &mut MetaModel<f64>,
        layer_weights: &[f64],
        layer_volumes: &[f64],
        layer_depths: &[f64],
    ) -> Result<(), String> {
        let x = match self.x.as_ref() {
            Some(x) => x,
            None => return Err("x variables must be registered before building symbols".to_string()),
        };

        // Build per-bin load weight symbols
        self.load_weight_symbols.clear();
        for bin_idx in 0..self.bins.len() {
            let mut terms = Vec::new();
            for layer_idx in 0..self.layers.len() {
                if let Some(model_idx) = x.model_index(&bin_idx, &layer_idx) {
                    let w = layer_weights.get(layer_idx).copied().unwrap_or(0.0);
                    if w != 0.0 {
                        terms.push((model_idx, w));
                    }
                }
            }
            let symbol = build_linear_expression_symbol(
                &format!("loadWeight_{}", bin_idx),
                &terms,
                0.0,
            );
            model.add_symbol(symbol.clone())
                .map_err(|e| format!("Failed to register loadWeight_{}: {:?}", bin_idx, e))?;
            self.load_weight_symbols.push(symbol);
        }

        // Build per-bin load volume symbols
        self.load_volume_symbols.clear();
        for bin_idx in 0..self.bins.len() {
            let mut terms = Vec::new();
            for layer_idx in 0..self.layers.len() {
                if let Some(model_idx) = x.model_index(&bin_idx, &layer_idx) {
                    let v = layer_volumes.get(layer_idx).copied().unwrap_or(0.0);
                    if v != 0.0 {
                        terms.push((model_idx, v));
                    }
                }
            }
            let symbol = build_linear_expression_symbol(
                &format!("loadVolume_{}", bin_idx),
                &terms,
                0.0,
            );
            model.add_symbol(symbol.clone())
                .map_err(|e| format!("Failed to register loadVolume_{}: {:?}", bin_idx, e))?;
            self.load_volume_symbols.push(symbol);
        }

        // Build per-bin load depth symbols
        self.load_depth_symbols.clear();
        for bin_idx in 0..self.bins.len() {
            let mut terms = Vec::new();
            for layer_idx in 0..self.layers.len() {
                if let Some(model_idx) = x.model_index(&bin_idx, &layer_idx) {
                    let d = layer_depths.get(layer_idx).copied().unwrap_or(0.0);
                    if d != 0.0 {
                        terms.push((model_idx, d));
                    }
                }
            }
            let symbol = build_linear_expression_symbol(
                &format!("loadDepth_{}", bin_idx),
                &terms,
                0.0,
            );
            model.add_symbol(symbol.clone())
                .map_err(|e| format!("Failed to register loadDepth_{}: {:?}", bin_idx, e))?;
            self.load_depth_symbols.push(symbol);
        }

        Ok(())
    }
}

impl<V: Debug + Clone + Send + Sync, U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync> Bpp3dModelComponent for PreciseAssignment<V, U> {
    fn name(&self) -> &str {
        "PreciseAssignment"
    }

    fn register(&mut self, model: &mut MetaModel<f64>) -> Result<(), String> {
        if self.x.is_some() && self.v.is_some() {
            return Ok(());
        }
        let bin_keys: Vec<usize> = (0..self.bins.len()).collect();
        let layer_keys: Vec<usize> = (0..self.layers.len()).collect();

        if self.x.is_none() {
            self.x = Some(
                IndexedVariableCombination2::new(
                    "x",
                    &bin_keys,
                    &layer_keys,
                    model,
                    |k1, k2| format!("x_{:?}_{:?}", k1, k2),
                    |_k1, _k2| VariableRange::new(Some(0.0), Some(1.0)),
                )
                .map_err(|e| format!("Failed to register PreciseAssignment x variables: {:?}", e))?,
            );
        }
        if self.v.is_none() {
            self.v = Some(
                IndexedVariableCombination1::new(
                    "v",
                    &bin_keys,
                    model,
                    |key| format!("v_{:?}", key),
                    |_key| VariableRange::new(Some(0.0), Some(1.0)),
                )
                .map_err(|e| format!("Failed to register PreciseAssignment v variables: {:?}", e))?,
            );
        }
        Ok(())
    }
}

