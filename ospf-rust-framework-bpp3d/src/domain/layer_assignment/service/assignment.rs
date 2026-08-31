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

