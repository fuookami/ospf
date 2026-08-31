#![allow(dead_code)]

//! 示例建模公共 helper（Kotlin 对齐入口）
//! Example modeling helpers (Kotlin-aligned entry)

use ospf_rust_core::error::{CoreError, SolverError};
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::{
    FeasibleSolverOutput, SolveValueConversionPolicy, Solver, SolverExt, SolverOutput,
};
use std::error::Error;

/// 统一线性模型求解 helper
/// Unified linear model solve helper
pub fn solve_linear_meta_model<S>(
    model: MetaModel<f64>,
    solver: &S,
) -> Result<SolverOutput, Box<dyn Error>>
where
    S: Solver + ?Sized,
{
    Ok(model.solve(solver)?)
}

/// 统一线性模型 typed 求解 helper
/// Unified typed linear model solve helper
pub fn solve_linear_meta_model_typed<S>(
    model: MetaModel<f64>,
    solver: &S,
) -> Result<FeasibleSolverOutput<f64>, Box<dyn Error>>
where
    S: Solver + ?Sized,
{
    Ok(solver.solve_typed(&model)?)
}

/// 将求解输出转换为 typed 可行输出（不可行或无解返回 None）
/// Convert solver output to typed feasible output (None for infeasible or no-solution)
pub fn solver_output_to_feasible_typed_if_feasible(
    output: &SolverOutput,
) -> Result<Option<FeasibleSolverOutput<f64>>, Box<dyn Error>> {
    if !output.status.is_feasible() {
        return Ok(None);
    }
    match output
        .clone()
        .try_into_feasible_typed::<f64>(SolveValueConversionPolicy::Strict)
    {
        Ok(feasible_output) => Ok(Some(feasible_output)),
        Err(CoreError::Solver(SolverError::NoSolution)) => Ok(None),
        Err(err) => Err(err.into()),
    }
}

/// 统一线性模型 typed 求解 helper（仅返回可行解）
/// Unified typed linear model solve helper (only feasible output)
pub fn solve_linear_meta_model_typed_if_feasible<S>(
    model: MetaModel<f64>,
    solver: &S,
) -> Result<Option<FeasibleSolverOutput<f64>>, Box<dyn Error>>
where
    S: Solver + ?Sized,
{
    let output = model.solve(solver)?;
    solver_output_to_feasible_typed_if_feasible(&output)
}

/// 示例阈值松弛 helper（占位）
/// Example threshold slack helper (placeholder)
pub fn example_threshold_slack(value: f64, threshold: f64) -> f64 {
    (value - threshold).max(0.0)
}

/// 示例绝对值松弛 helper（占位）
/// Example absolute slack helper (placeholder)
pub fn example_absolute_slack(value: f64, target: f64) -> f64 {
    (value - target).abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_core::solver::SolverStatus;

    #[test]
    fn solver_output_to_feasible_typed_if_feasible_returns_none_for_infeasible_status() {
        let output = SolverOutput::new(SolverStatus::Infeasible);
        let feasible = solver_output_to_feasible_typed_if_feasible(&output)
            .expect("infeasible status should map to None");
        assert!(feasible.is_none());
    }

    #[test]
    fn solver_output_to_feasible_typed_if_feasible_maps_feasible_output() {
        let output = SolverOutput::optimal(2.0, vec![1.0, 3.0]);
        let feasible = solver_output_to_feasible_typed_if_feasible(&output)
            .expect("feasible output should map")
            .expect("feasible output should be Some");
        assert!(feasible.status.is_feasible());
        assert_eq!(feasible.objective_value, Some(2.0));
        assert_eq!(feasible.solution, vec![1.0, 3.0]);
    }

    #[test]
    fn solver_output_to_feasible_typed_if_feasible_maps_missing_solution_to_none() {
        let output = SolverOutput::new(SolverStatus::Feasible).with_objective(1.0);
        let feasible = solver_output_to_feasible_typed_if_feasible(&output)
            .expect("missing solution should map to None");
        assert!(feasible.is_none());
    }
}
