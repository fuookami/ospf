//! 服务分配约束 / Service assignment constraint

use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo1::route_context::model::Assignment;

/// 服务分配约束：每个服务最多被一个普通节点使用 / Service assignment constraint: each service is used by at most one normal node
///
/// sum(x[node, s] for all nodes) <= 1
///
/// 从 service_assignment[s] 符号组合取多项式：
/// service_assignment[s] = sum(x[node, s] for all normal nodes)
pub fn apply_service_assignment_constraints(
    model: &mut MetaModel<f64>,
    assignment: &Assignment,
    service_count: usize,
) -> Result<(), Box<dyn Error>> {
    for s in 0..service_count {
        // service_assignment[s] 的多项式 = sum(x[node, s])
        let poly = assignment.service_assignment.symbol_polynomial(s);
        let coefficients: Vec<(usize, f64)> = poly
            .monomials()
            .iter()
            .map(|m| (m.var_index(), *m.coefficient()))
            .collect();
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::LessEqual,
            1.0,
            &format!("service_assignment_{}", s),
        )?;
    }
    Ok(())
}
