// ============================================================================

/// 更优层最大化 / Better layer maximization
///
/// 最大化高价值层的赋值量，常用于 RMP 阶段。
/// Maximizes assignment of high-value layers, commonly used in the RMP phase.
#[derive(Debug)]
pub struct BetterLayerMaximization {
    name: String,
    /// 层价值项：(x_model_index, coefficient) 列表 / Layer value terms
    pub value_terms: Vec<(usize, f64)>,
    /// 系数 / Coefficient
    pub coefficient: f64,
}

impl BetterLayerMaximization {
    /// 创建更优层最大化 / Create better layer maximization
    pub fn new(value_terms: Vec<(usize, f64)>, coefficient: f64) -> Self {
        Self {
            name: "better_layer_maximization".to_string(),
            value_terms,
            coefficient,
        }
    }
}

impl Pipeline<MetaModel<f64>> for BetterLayerMaximization {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { None }

    fn register(&self, model: &mut MetaModel<f64>) {
        if self.value_terms.is_empty() {
            return;
        }
        let monomials: Vec<LinearMonomial<f64>> = self.value_terms.iter()
            .map(|&(idx, coeff)| LinearMonomial::new(coeff * self.coefficient, idx))
            .collect();
        let polynomial = Linear::new(monomials, 0.0);
        let sub_obj = SubObjective::maximize(polynomial, &self.name);
        model.add_sub_objective(sub_obj);
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
