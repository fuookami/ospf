// ============================================================================

/// 更优层最大化 / Better layer maximization
///
/// 最大化高价值层的赋值量，常用于 RMP 阶段。支持从符号多项式或预计算项构建。
/// Maximizes assignment of high-value layers, commonly used in the RMP phase.
/// Supports construction from symbol polynomial or pre-computed terms.
#[derive(Debug)]
pub struct BetterLayerMaximization {
    name: String,
    /// 层价值项：(x_model_index, coefficient) 列表 / Layer value terms
    pub value_terms: Vec<(usize, f64)>,
    /// 层价值符号（可选，优先于 value_terms） / Layer value symbol (optional, takes precedence over value_terms)
    pub value_symbol: Option<Arc<LinearExpressionSymbol<f64>>>,
    /// 系数 / Coefficient
    pub coefficient: f64,
}

impl BetterLayerMaximization {
    /// 从预计算项创建更优层最大化 / Create better layer maximization from pre-computed terms
    pub fn new(value_terms: Vec<(usize, f64)>, coefficient: f64) -> Self {
        Self {
            name: "better_layer_maximization".to_string(),
            value_terms,
            value_symbol: None,
            coefficient,
        }
    }

    /// 从符号创建更优层最大化 / Create better layer maximization from a linear expression symbol
    ///
    /// 注册时从符号多项式提取项。
    /// Extracts terms from the symbol polynomial at registration time.
    pub fn new_from_symbol(value_symbol: Arc<LinearExpressionSymbol<f64>>, coefficient: f64) -> Self {
        Self {
            name: "better_layer_maximization".to_string(),
            value_terms: Vec::new(),
            value_symbol: Some(value_symbol),
            coefficient,
        }
    }
}

impl Pipeline<MetaModel<f64>> for BetterLayerMaximization {
    fn name(&self) -> &str { &self.name }
    fn constraint_group(&self) -> Option<&ConstraintGroup> { None }

    fn register(&self, model: &mut MetaModel<f64>) {
        // Prefer symbol path; extract terms from polynomial at registration time
        let effective_terms: Vec<(usize, f64)>;
        let terms = if let Some(ref symbol) = self.value_symbol {
            let poly = symbol.to_linear_polynomial();
            effective_terms = poly.monomials().iter()
                .map(|m| (m.var_index(), *m.coefficient()))
                .collect();
            &effective_terms
        } else {
            &self.value_terms
        };

        if terms.is_empty() {
            return;
        }
        let monomials: Vec<LinearMonomial<f64>> = terms.iter()
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
