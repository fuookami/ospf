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
    pub x: VariableArray1<usize, UContinuousVariableItem>,
    /// 列变量上界 / Column variable upper bounds
    pub upper_bounds: Vec<Option<f64>>,
}

impl<V: Debug + Clone + Send + Sync, U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync> Bpp3dModelComponent for ImpreciseAssignment<V, U> {
    fn name(&self) -> &str {
        "ImpreciseAssignment"
    }

    fn register(&mut self, model: &mut MetaModel<f64>) -> Result<(), String> {
        for layer_index in 0..self.layers.len() {
            if self.x.index(&layer_index).is_none() {
                self.x.register_unsigned_continuous_with_upper_bound(
                    layer_index,
                    model,
                    self.upper_bounds.get(layer_index).copied().flatten(),
                )?;
            }
        }
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
    pub x: VariableArray2<usize, usize, ospf_rust_core::variable::variable_item::BinaryVariableItem>,
    /// 箱使用标记 v[bin] / Bin usage markers v[bin]
    pub v: VariableArray1<usize, ospf_rust_core::variable::variable_item::BinaryVariableItem>,
}

impl<V: Debug + Clone + Send + Sync, U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync> Bpp3dModelComponent for PreciseAssignment<V, U> {
    fn name(&self) -> &str {
        "PreciseAssignment"
    }

    fn register(&mut self, model: &mut MetaModel<f64>) -> Result<(), String> {
        let bin_keys: Vec<usize> = (0..self.bins.len()).collect();
        let layer_keys: Vec<usize> = (0..self.layers.len()).collect();

        self.x.register_binary(&bin_keys, &layer_keys, model)?;
        self.v.register_binary(&bin_keys, model)?;
        Ok(())
    }
}

