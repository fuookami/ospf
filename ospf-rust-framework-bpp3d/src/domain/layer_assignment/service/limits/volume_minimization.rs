// VolumeMinimization - 体积最小化 / Volume minimization
// ============================================================================

/// 体积最小化 / Volume minimization
///
/// 最小化使用的总体积。
/// Minimizes the total volume used.
#[derive(Debug)]
pub struct VolumeMinimization {
    name: String,
    /// 体积项：(x_model_index, coefficient) 列表 / Volume terms: (x model index, coefficient) list
    pub volume_terms: Vec<(usize, f64)>,
    /// 系数 / Coefficient
    pub coefficient: f64,
}

impl VolumeMinimization {
    /// 创建体积最小化 / Create volume minimization
    pub fn new(volume_terms: Vec<(usize, f64)>, coefficient: f64) -> Self {
        Self {
            name: "volume_minimization".to_string(),
            volume_terms,
            coefficient,
        }
    }
}

impl Pipeline<MetaModel<f64>> for VolumeMinimization {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { None }

    fn register(&self, model: &mut MetaModel<f64>) {
        if self.volume_terms.is_empty() {
            return;
        }
        let monomials: Vec<LinearMonomial<f64>> = self.volume_terms.iter()
            .map(|&(idx, coeff)| LinearMonomial::new(coeff * self.coefficient, idx))
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
// BetterLayerMaximization - 更优层最大化 / Better layer maximization
