use crate::framework_demo::demo2::domain::mac_optimization::context::MacOptimizationContext;
use crate::framework_demo::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;

pub struct MacOptimizationAggregation {
    pub target_balance: f64,
    pub max_arm_abs: f64,
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
            long_moment,
            neg_long_moment,
            lat_moment,
            neg_lat_moment,
        }
    }
}
