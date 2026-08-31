use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::flatten::LinearMonomial;
use crate::framework::demo2::domain::mac_optimization::context::MacOptimizationContext;
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;

/// MAC 优化聚合 / MAC optimization aggregation
///
/// 同时持有显式中间符号和向后兼容的裸系数。
/// Holds both explicit intermediate symbols and backward-compatible raw coefficients.
pub struct MacOptimizationAggregation {
    pub target_balance: f64,
    pub max_arm_abs: f64,
    /// 纵向力矩符号 / Longitudinal moment symbol
    pub long_moment_symbol: Option<Arc<LinearExpressionSymbol<f64>>>,
    /// 负纵向力矩符号 / Negative longitudinal moment symbol
    pub neg_long_moment_symbol: Option<Arc<LinearExpressionSymbol<f64>>>,
    /// 横向力矩符号 / Lateral moment symbol
    pub lat_moment_symbol: Option<Arc<LinearExpressionSymbol<f64>>>,
    /// 负横向力矩符号 / Negative lateral moment symbol
    pub neg_lat_moment_symbol: Option<Arc<LinearExpressionSymbol<f64>>>,
    // 向后兼容字段 / Backward-compatible fields
    pub long_moment: Vec<(usize, f64)>,
    pub neg_long_moment: Vec<(usize, f64)>,
    pub lat_moment: Vec<(usize, f64)>,
    pub neg_lat_moment: Vec<(usize, f64)>,
}

impl MacOptimizationAggregation {
    pub fn from_context(context: &MacOptimizationContext<'_>) -> Self {
        let total_capacity: f64 = context
            .request
            .positions
            .iter()
            .map(|pos| pos.max_weight)
            .sum();
        let total_weight: f64 = context
            .request
            .cargos
            .iter()
            .map(|cargo| cargo.weight)
            .sum();
        let target_balance = match context.mode {
            Demo2PipelineMode::Predistribution => {
                total_weight / context.request.positions.len() as f64
            }
            Demo2PipelineMode::WeightRecommendation => {
                total_capacity.min(total_weight) / context.request.positions.len() as f64
            }
            Demo2PipelineMode::FullLoad => {
                total_capacity.min(total_weight) / context.request.positions.len() as f64
            }
        };

        let max_arm_abs = context
            .request
            .positions
            .iter()
            .map(|position| position.longitudinal_arm.abs())
            .fold(0.0_f64, f64::max);

        let mut long_moment: Vec<(usize, f64)> = Vec::new();
        let mut neg_long_moment: Vec<(usize, f64)> = Vec::new();
        let mut lat_moment: Vec<(usize, f64)> = Vec::new();
        let mut neg_lat_moment: Vec<(usize, f64)> = Vec::new();

        for p in 0..context.request.positions.len() {
            for c in 0..context.request.cargos.len() {
                let long_coeff = context.request.cargos[c].weight
                    * context.request.positions[p].longitudinal_arm;
                let lat_coeff =
                    context.request.cargos[c].weight * context.request.positions[p].lateral_arm;
                long_moment.push((context.x_idx[c][p], long_coeff));
                neg_long_moment.push((context.x_idx[c][p], -long_coeff));
                lat_moment.push((context.x_idx[c][p], lat_coeff));
                neg_lat_moment.push((context.x_idx[c][p], -lat_coeff));
            }
        }

        Self {
            target_balance,
            max_arm_abs,
            long_moment_symbol: None,
            neg_long_moment_symbol: None,
            lat_moment_symbol: None,
            neg_lat_moment_symbol: None,
            long_moment,
            neg_long_moment,
            lat_moment,
            neg_lat_moment,
        }
    }

    /// 注册符号到模型 / Register symbols to model
    pub fn register_symbols(
        &mut self,
        model: &mut MetaModel<f64>,
        next_id: &mut u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let long_moment_sym = LinearExpressionSymbol::new(
            *next_id,
            "long_moment",
            self.long_moment.iter().map(|(idx, coeff)| LinearMonomial::new(*coeff, *idx)).collect(),
            0.0,
        );
        self.long_moment_symbol = Some(Arc::new(long_moment_sym));
        model.add_symbol(self.long_moment_symbol.as_ref().unwrap().clone())?;
        *next_id += 1;

        let neg_long_moment_sym = LinearExpressionSymbol::new(
            *next_id,
            "neg_long_moment",
            self.neg_long_moment.iter().map(|(idx, coeff)| LinearMonomial::new(*coeff, *idx)).collect(),
            0.0,
        );
        self.neg_long_moment_symbol = Some(Arc::new(neg_long_moment_sym));
        model.add_symbol(self.neg_long_moment_symbol.as_ref().unwrap().clone())?;
        *next_id += 1;

        let lat_moment_sym = LinearExpressionSymbol::new(
            *next_id,
            "lat_moment",
            self.lat_moment.iter().map(|(idx, coeff)| LinearMonomial::new(*coeff, *idx)).collect(),
            0.0,
        );
        self.lat_moment_symbol = Some(Arc::new(lat_moment_sym));
        model.add_symbol(self.lat_moment_symbol.as_ref().unwrap().clone())?;
        *next_id += 1;

        let neg_lat_moment_sym = LinearExpressionSymbol::new(
            *next_id,
            "neg_lat_moment",
            self.neg_lat_moment.iter().map(|(idx, coeff)| LinearMonomial::new(*coeff, *idx)).collect(),
            0.0,
        );
        self.neg_lat_moment_symbol = Some(Arc::new(neg_lat_moment_sym));
        model.add_symbol(self.neg_lat_moment_symbol.as_ref().unwrap().clone())?;
        *next_id += 1;

        Ok(())
    }
}
