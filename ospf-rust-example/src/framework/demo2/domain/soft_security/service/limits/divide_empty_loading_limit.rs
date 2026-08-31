use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::IfFunction;
use crate::framework::demo2::domain::soft_security::aggregation::SoftSecurityAggregation;
use crate::framework::demo2::domain::soft_security::context::SoftSecurityContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;

/// 空舱位标志变量索引 / Empty position flag variable indices
#[derive(Debug, Clone)]
pub struct EmptyFlagVariables {
    /// empty_flag[p] = IfFunction(condition=sum(x[c][p]), then=0, else=1) 的结果变量 solver index
    /// 当舱位为空时值为 1，当舱位有货物时值为 0
    pub empty_flag: Vec<usize>,
}

/// 分离空装载限制: requires_separation 货物应分散装载
/// 对齐 Kotlin DivideEmptyLoadingLimit
///
/// 创建 emptyFlag[i] IfFunction 符号，并使用分离约束限制每个舱位最多一个分离货物。
pub fn apply_divide_empty_loading_limits(
    model: &mut MetaModel<f64>,
    context: &SoftSecurityContext<'_>,
    _aggregation: &SoftSecurityAggregation,
) -> Result<(), Box<dyn Error>> {
    let mut next_id = 40000u64;

    // 1. 创建 emptyFlag[p] IfFunction 符号
    // 对齐 Kotlin: emptyFlag[p] = IfFunction(condition=sum(x[c][p]), then=0, else=1)
    // 当舱位有货物时 (condition != 0) 结果为 0，当舱位为空时 (condition = 0) 结果为 1
    let mut empty_flag_idx = vec![0usize; context.request.positions.len()];
    for p in 0..context.request.positions.len() {
        let monomials: Vec<LinearMonomial<f64>> = (0..context.request.cargos.len())
            .map(|c| LinearMonomial::new(1.0, context.x_idx[c][p]))
            .collect();
        // condition: sum(x[c][p]) (nonzero when position has any cargo)
        let condition = Linear::new(monomials, 0.0);
        // then_expr: 0.0 (not empty)
        let then_expr = Linear::new(Vec::new(), 0.0);
        // else_expr: 1.0 (empty)
        let else_expr = Linear::new(Vec::new(), 1.0);

        let if_fn = IfFunction::new(
            next_id,
            &format!(
                "soft_security_empty_flag_{}_{}",
                mode_name(context.mode),
                p
            ),
            condition,
            then_expr,
            else_expr,
        );
        empty_flag_idx[p] = if_fn.result_variable().index();
        model.add_symbol(Arc::new(if_fn))?;
        next_id += 1;
    }

    // 2. requires_separation 的货物每个舱位最多一个
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

    // empty_flag_idx is registered but not yet consumed by downstream constraints.
    // This follows the Phase K pattern of making IfFunction symbols available
    // for future constraint pipeline steps.
    let _ = empty_flag_idx;

    Ok(())
}
