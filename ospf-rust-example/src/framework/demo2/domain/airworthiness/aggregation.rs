use crate::framework::demo2::domain::airworthiness::context::AirworthinessContext;

pub struct AirworthinessAggregation {
    pub total_payload_coefficients: Vec<(usize, f64)>,
    pub envelope_longitudinal_moment_coefficients: Vec<(usize, f64)>,
}

impl AirworthinessAggregation {
    pub fn from_context(context: &AirworthinessContext<'_>) -> Self {
        let mut total_payload_coefficients: Vec<(usize, f64)> = Vec::new();
        let mut envelope_longitudinal_moment_coefficients: Vec<(usize, f64)> = Vec::new();

        for p in 0..context.request.positions.len() {
            for c in 0..context.request.cargos.len() {
                let weight = context.request.cargos[c].weight;
                total_payload_coefficients.push((context.x_idx[c][p], weight));
                envelope_longitudinal_moment_coefficients.push((
                    context.x_idx[c][p],
                    weight * context.request.positions[p].longitudinal_arm,
                ));
            }
        }

        Self {
            total_payload_coefficients,
            envelope_longitudinal_moment_coefficients,
        }
    }
}

