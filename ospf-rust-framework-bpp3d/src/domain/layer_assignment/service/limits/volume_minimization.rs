// VolumeMinimization - 体积最小化 / Volume minimization
// ============================================================================

/// 体积最小化 / Volume minimization
///
/// 最小化使用的总体积。支持从符号多项式或预计算项构建。
/// Minimizes the total volume used. Supports construction from
/// symbol polynomial or pre-computed terms.
#[derive(Debug)]
pub struct VolumeMinimization {
    name: String,
    /// 体积项：(x_model_index, coefficient) 列表 / Volume terms: (x model index, coefficient) list
    pub volume_terms: Vec<(usize, f64)>,
    /// 体积符号（可选，优先于 volume_terms） / Volume symbol (optional, takes precedence over volume_terms)
    pub volume_symbol: Option<Arc<LinearExpressionSymbol<f64>>>,
    /// 系数 / Coefficient
    pub coefficient: f64,
}

impl VolumeMinimization {
    /// 从预计算项创建体积最小化 / Create volume minimization from pre-computed terms
    pub fn new(volume_terms: Vec<(usize, f64)>, coefficient: f64) -> Self {
        Self {
            name: "volume_minimization".to_string(),
            volume_terms,
            volume_symbol: None,
            coefficient,
        }
    }

    /// 从符号创建体积最小化 / Create volume minimization from a linear expression symbol
    ///
    /// 注册时从符号多项式提取项。
    /// Extracts terms from the symbol polynomial at registration time.
    pub fn new_from_symbol(volume_symbol: Arc<LinearExpressionSymbol<f64>>, coefficient: f64) -> Self {
        Self {
            name: "volume_minimization".to_string(),
            volume_terms: Vec::new(),
            volume_symbol: Some(volume_symbol),
            coefficient,
        }
    }
}

impl Pipeline<MetaModel<f64>> for VolumeMinimization {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { None }

    fn register(&self, model: &mut MetaModel<f64>) {
        // Prefer symbol path; extract terms from polynomial at registration time
        let effective_terms: Vec<(usize, f64)>;
        let terms = if let Some(ref symbol) = self.volume_symbol {
            let poly = symbol.to_linear_polynomial();
            effective_terms = poly.monomials().iter()
                .map(|m| (m.var_index(), *m.coefficient()))
                .collect();
            &effective_terms
        } else {
            &self.volume_terms
        };

        if terms.is_empty() {
            return;
        }
        let monomials: Vec<LinearMonomial<f64>> = terms.iter()
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
