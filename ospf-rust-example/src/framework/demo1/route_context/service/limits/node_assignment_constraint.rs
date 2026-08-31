use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use crate::framework::demo1::route_context::model::Assignment;

/// 节点分配约束：每个 normal node 最多分配一个 service
/// sum(x[node, s] for all s) <= 1
///
/// 从 node_assignment[node] 符号组合取多项式：
/// node_assignment[node] = sum(x[node, s] for all s)
pub fn apply_node_assignment_constraints(
    model: &mut MetaModel<f64>,
    assignment: &Assignment,
    _node_count: usize,
) -> Result<(), Box<dyn Error>> {
    for (row, _) in assignment.normal_node_indices.iter().enumerate() {
        // node_assignment[node] 的多项式 = sum(x[node, s])
        let poly = assignment.node_assignment[row].to_linear_polynomial();
        let coefficients: Vec<(usize, f64)> = poly
            .monomials()
            .iter()
            .map(|m| (m.var_index(), *m.coefficient()))
            .collect();
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::LessEqual,
            1.0,
            &format!("node_assignment_{}", row),
        )?;
    }
    Ok(())
}
