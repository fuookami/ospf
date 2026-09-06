// ============================================================================

/// 箱数量最小化 / Bin amount minimization
///
/// 最小化使用的箱数量：`min sum(v[bin])`。
/// Minimizes the number of used bins: `min sum(v[bin])`.
#[derive(Debug)]
pub struct BinAmountMinimization {
    name: String,
    /// 箱使用标记变量索引 / Bin usage marker variable indices
    pub v_indices: Vec<usize>,
    /// 系数 / Coefficient
    pub coefficient: f64,
}

impl BinAmountMinimization {
    /// 创建箱数量最小化 / Create bin amount minimization
    pub fn new(v_indices: Vec<usize>, coefficient: f64) -> Self {
        Self {
            name: "bin_amount_minimization".to_string(),
            v_indices,
            coefficient,
        }
    }
}

impl Pipeline<MetaModel<f64>> for BinAmountMinimization {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { None }

    fn register(&self, model: &mut MetaModel<f64>) {
        if self.v_indices.is_empty() {
            return;
        }
        let monomials: Vec<LinearMonomial<f64>> = self.v_indices.iter()
            .map(|&idx| LinearMonomial::new(self.coefficient, idx))
            .collect();
        let polynomial = Linear::new(monomials, 0.0);
        let sub_obj = SubObjective::minimize(polynomial, &self.name);
        model.add_sub_objective(sub_obj);
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
