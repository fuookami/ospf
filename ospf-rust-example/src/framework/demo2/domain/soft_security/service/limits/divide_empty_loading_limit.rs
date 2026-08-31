use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo2::domain::soft_security::aggregation::SoftSecurityAggregation;
use crate::framework::demo2::domain::soft_security::context::SoftSecurityContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 分离空装载限制: requires_separation 货物应分散装载
/// 对齐 Kotlin DivideEmptyLoadingLimit
pub fn apply_divide_empty_loading_limits(
    model: &mut MetaModel<f64>,
    context: &SoftSecurityContext<'_>,
    _aggregation: &SoftSecurityAggregation,
) -> Result<(), Box<dyn Error>> {
    // requires_separation 的货物每个舱位最多一个
    let separation_cargos: Vec<usize> = (0..context.request.cargos.len())
        .filter(|c| context.request.cargos[*c].requires_separation)
        .collect();

    for p in 0..context.request.positions.len() {
        let coefficients: Vec<(usize, f64)> = separation_cargos
            .iter()
            .map(|&c| (context.x_idx[c][p], 1.0))
            .collect();
        if !coefficients.is_empty() {
            model.add_linear_constraint(
                &coefficients,
                ConstraintRelation::LessEqual,
                1.0,
                &format!(
                    "soft_security_divide_empty_{}_{}",
                    mode_name(context.mode),
                    p
                ),
            )?;
        }
    }
    Ok(())
}
