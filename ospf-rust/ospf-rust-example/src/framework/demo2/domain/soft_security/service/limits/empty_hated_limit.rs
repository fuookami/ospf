//! 不宜空舱限制 / Empty hated limits
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use crate::framework::demo2::domain::soft_security::aggregation::SoftSecurityAggregation;
use crate::framework::demo2::domain::soft_security::context::SoftSecurityContext;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::flatten::LinearMonomial;
use ospf_rust_core::variable::UContinuousVariableItem;
use std::error::Error;
use std::sync::Arc;

/// 空舱厌恶限制: 目标函数中惩罚空舱位 / Empty hated limit: penalize empty positions in objective
/// 对齐 Kotlin EmptyHatedLimit
///
/// 创建辅助变量 y[p] 表示舱位是否为空，并在目标函数中添加惩罚项
/// Creates auxiliary variable y[p] indicating whether a position is empty, and adds penalty terms to the objective
pub fn apply_empty_hated_limits(
    model: &mut MetaModel<f64>,
    context: &SoftSecurityContext<'_>,
    _aggregation: &SoftSecurityAggregation,
) -> Result<(), Box<dyn Error>> {
    let mut next_id = 60000u64;

    for p in 0..context.request.positions.len() {
        // 创建辅助变量 y[p] 表示舱位 p 是否为空
        let var = UContinuousVariableItem::auto(&format!("empty_hated_y_{}", p));
        let y_idx = model.register_variable(var)?;

        // 空舱约束: sum(stowage[c][p]) + y[p] >= 1
        // 即: 如果没有货物装载 (sum=0)，则 y[p] >= 1
        let mut coefficients: Vec<(usize, f64)> = (0..context.request.cargos.len())
            .map(|c| (context.x_idx[c][p], 1.0))
            .collect();
        coefficients.push((y_idx, 1.0));

        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::GreaterEqual,
            1.0,
            &format!(
                "soft_security_empty_hated_{}_{}",
                mode_name(context.mode),
                p
            ),
        )?;

        // 目标函数中添加惩罚项: empty_hated_weight * y[p]
        // 使用 LinearExpressionSymbol 创建惩罚项
        let penalty_weight = 10.0; // 惩罚权重
        let penalty_symbol = LinearExpressionSymbol::new(
            next_id,
            &format!("empty_hated_penalty_{}", p),
            vec![LinearMonomial::new(penalty_weight, y_idx)],
            0.0,
        );
        model.add_symbol(Arc::new(penalty_symbol))?;
        next_id += 1;
    }
    Ok(())
}
