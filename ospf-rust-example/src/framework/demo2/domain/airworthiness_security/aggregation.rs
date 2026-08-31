//! 适航性安全聚合 / Airworthiness security aggregation
use crate::framework::demo2::domain::airworthiness_security::context::AirworthinessContext;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::flatten::LinearMonomial;
use std::sync::Arc;

/// 适航性安全聚合 / Airworthiness security aggregation
///
/// 同时持有显式中间符号和向后兼容的裸系数。
/// 后续应逐步移除裸系数，完全依赖符号。
/// Holds both explicit intermediate symbols and backward-compatible raw coefficients.
/// Raw coefficients should be gradually removed in favor of symbols.
pub struct AirworthinessAggregation {
    /// 总载荷符号 / Total payload symbol
    pub total_payload_symbol: Option<Arc<LinearExpressionSymbol<f64>>>,
    /// 纵向力矩符号 / Envelope longitudinal moment symbol
    pub envelope_longitudinal_moment_symbol: Option<Arc<LinearExpressionSymbol<f64>>>,
    /// 横向力矩符号 / Lateral moment symbol
    pub lateral_moment_symbol: Option<Arc<LinearExpressionSymbol<f64>>>,
    /// 每位置重量符号 / Per-position weight symbols
    pub per_position_weight_symbols: Vec<Option<Arc<LinearExpressionSymbol<f64>>>>,
    /// 总载荷裸系数（向后兼容，后续移除）/ Total payload raw coefficients (backward-compatible, remove later)
    pub total_payload_coefficients: Vec<(usize, f64)>,
    /// 纵向力矩裸系数（向后兼容，后续移除）/ Envelope longitudinal moment raw coefficients (backward-compatible, remove later)
    pub envelope_longitudinal_moment_coefficients: Vec<(usize, f64)>,
    /// 横向力矩裸系数（向后兼容，后续移除）/ Lateral moment raw coefficients (backward-compatible, remove later)
    pub lateral_moment_coefficients: Vec<(usize, f64)>,
    /// 每位置重量裸系数（向后兼容，后续移除）/ Per-position weight raw coefficients (backward-compatible, remove later)
    pub per_position_weight_coefficients: Vec<Vec<(usize, f64)>>,
}

impl AirworthinessAggregation {
    /// 从适航性上下文构建聚合 / Build aggregation from airworthiness context
    ///
    /// 遍历所有位置和货物，计算总载荷、纵向力矩、横向力矩和每位置重量的系数。
    /// Iterates over all positions and cargos, computing coefficients for
    /// total payload, longitudinal moment, lateral moment, and per-position weight.
    pub fn from_context(context: &AirworthinessContext<'_>) -> Self {
        let pos_count = context.request.positions.len();
        let mut total_payload_monomials = Vec::new();
        let mut envelope_monomials = Vec::new();
        let mut lateral_monomials = Vec::new();
        let mut per_position_monomials: Vec<Vec<LinearMonomial<f64>>> = vec![Vec::new(); pos_count];

        // 向后兼容的裸系数 / Backward-compatible raw coefficients
        let mut total_payload_coefficients: Vec<(usize, f64)> = Vec::new();
        let mut envelope_longitudinal_moment_coefficients: Vec<(usize, f64)> = Vec::new();
        let mut lateral_moment_coefficients: Vec<(usize, f64)> = Vec::new();
        let mut per_position_weight_coefficients: Vec<Vec<(usize, f64)>> =
            vec![Vec::new(); pos_count];

        for p in 0..pos_count {
            for c in 0..context.request.cargos.len() {
                let weight = context.request.cargos[c].weight;
                let var_idx = context.x_idx[c][p];

                total_payload_monomials.push(LinearMonomial::new(weight, var_idx));
                envelope_monomials.push(LinearMonomial::new(
                    weight * context.request.positions[p].longitudinal_arm,
                    var_idx,
                ));
                lateral_monomials.push(LinearMonomial::new(
                    weight * context.request.positions[p].lateral_arm,
                    var_idx,
                ));
                per_position_monomials[p].push(LinearMonomial::new(weight, var_idx));

                // 向后兼容
                total_payload_coefficients.push((var_idx, weight));
                envelope_longitudinal_moment_coefficients.push((
                    var_idx,
                    weight * context.request.positions[p].longitudinal_arm,
                ));
                lateral_moment_coefficients
                    .push((var_idx, weight * context.request.positions[p].lateral_arm));
                per_position_weight_coefficients[p].push((var_idx, weight));
            }
        }

        Self {
            total_payload_symbol: None,
            envelope_longitudinal_moment_symbol: None,
            lateral_moment_symbol: None,
            per_position_weight_symbols: vec![None; pos_count],
            total_payload_coefficients,
            envelope_longitudinal_moment_coefficients,
            lateral_moment_coefficients,
            per_position_weight_coefficients,
        }
    }

    /// 注册符号到模型 / Register symbols to model
    pub fn register_symbols(
        &mut self,
        model: &mut MetaModel<f64>,
        next_id: &mut u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 总载荷符号
        let symbol = LinearExpressionSymbol::new(
            *next_id,
            "total_payload",
            self.total_payload_coefficients
                .iter()
                .map(|(idx, coeff)| LinearMonomial::new(*coeff, *idx))
                .collect(),
            0.0,
        );
        self.total_payload_symbol = Some(Arc::new(symbol));
        model.add_symbol(self.total_payload_symbol.as_ref().unwrap().clone())?;
        *next_id += 1;

        // 纵向力矩符号
        let symbol = LinearExpressionSymbol::new(
            *next_id,
            "envelope_longitudinal_moment",
            self.envelope_longitudinal_moment_coefficients
                .iter()
                .map(|(idx, coeff)| LinearMonomial::new(*coeff, *idx))
                .collect(),
            0.0,
        );
        self.envelope_longitudinal_moment_symbol = Some(Arc::new(symbol));
        model.add_symbol(
            self.envelope_longitudinal_moment_symbol
                .as_ref()
                .unwrap()
                .clone(),
        )?;
        *next_id += 1;

        // 横向力矩符号
        let symbol = LinearExpressionSymbol::new(
            *next_id,
            "lateral_moment",
            self.lateral_moment_coefficients
                .iter()
                .map(|(idx, coeff)| LinearMonomial::new(*coeff, *idx))
                .collect(),
            0.0,
        );
        self.lateral_moment_symbol = Some(Arc::new(symbol));
        model.add_symbol(self.lateral_moment_symbol.as_ref().unwrap().clone())?;
        *next_id += 1;

        // 每位置重量符号
        for (p, coeffs) in self.per_position_weight_coefficients.iter().enumerate() {
            let symbol = LinearExpressionSymbol::new(
                *next_id,
                &format!("per_position_weight_{}", p),
                coeffs
                    .iter()
                    .map(|(idx, coeff)| LinearMonomial::new(*coeff, *idx))
                    .collect(),
                0.0,
            );
            self.per_position_weight_symbols[p] = Some(Arc::new(symbol));
            model.add_symbol(
                self.per_position_weight_symbols[p]
                    .as_ref()
                    .unwrap()
                    .clone(),
            )?;
            *next_id += 1;
        }

        Ok(())
    }
}
