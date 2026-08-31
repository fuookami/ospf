use std::error::Error;
use std::sync::Arc;

use ospf_rust_core::model::{ConstraintGroup, ConstraintRelation, MetaModel};
use ospf_rust_core::solver::{SolverCapability, SolverInfo, SolverOutput, solvers::GurobiSolver};
use ospf_rust_core::symbol::BinaryzationMethod;
use ospf_rust_core::variable::BinaryVariableItem;

/// 推荐入口：求解 MetaModel / Recommended entry: solve MetaModel
pub fn solve(meta_model: MetaModel<f64>) -> Result<SolverOutput, Box<dyn Error>> {
    let solver = GurobiSolver::new();
    let output = meta_model.solve(&solver)?;
    Ok(output)
}

pub fn read_solution_value(solution: &[f64], idx: usize) -> f64 {
    solution.get(idx).copied().unwrap_or(0.0)
}

pub fn register_binary_matrix(
    model: &mut MetaModel<f64>,
    rows: usize,
    cols: usize,
    prefix: &str,
) -> Result<Vec<Vec<usize>>, Box<dyn Error>> {
    let mut matrix = vec![vec![0usize; cols]; rows];
    for r in 0..rows {
        for c in 0..cols {
            let var = BinaryVariableItem::auto(&format!("{}_{}_{}", prefix, r, c));
            matrix[r][c] = model.register_variable(var)?;
        }
    }
    Ok(matrix)
}

pub fn linear_expr_from_indices(indices: &[usize], coefficient: f64) -> Vec<(usize, f64)> {
    indices
        .iter()
        .copied()
        .map(|idx| (idx, coefficient))
        .collect()
}

pub fn linear_expr_from_sparse_terms(terms: &[(usize, f64)]) -> Vec<(usize, f64)> {
    terms.to_vec()
}

pub fn add_constraint_with_metadata(
    model: &mut MetaModel<f64>,
    coefficients: &[(usize, f64)],
    relation: ConstraintRelation,
    rhs: f64,
    name: &str,
    group: Option<Arc<ConstraintGroup>>,
    lazy: bool,
    priority: u32,
    args: Option<String>,
) -> Result<(), Box<dyn Error>> {
    model.add_linear_constraint_with_metadata(
        coefficients,
        relation,
        rhs,
        name,
        group,
        lazy,
        priority,
        args,
    )?;
    Ok(())
}

pub fn resolve_binaryzation_method(
    solver: &dyn SolverInfo,
    requested: BinaryzationMethod,
) -> (BinaryzationMethod, Option<String>) {
    match requested {
        BinaryzationMethod::Indicator => {
            if solver.supports(SolverCapability::NativeIndicator) {
                (BinaryzationMethod::Indicator, None)
            } else {
                (
                    BinaryzationMethod::BigM,
                    Some(format!(
                        "{} does not support NativeIndicator, fallback to Big-M",
                        solver.name()
                    )),
                )
            }
        }
        BinaryzationMethod::SOS1 => {
            if solver.supports(SolverCapability::NativeSOS1) {
                (BinaryzationMethod::SOS1, None)
            } else {
                (
                    BinaryzationMethod::BigM,
                    Some(format!(
                        "{} does not support NativeSOS1, fallback to Big-M",
                        solver.name()
                    )),
                )
            }
        }
        _ => (requested, None),
    }
}
