use std::error::Error;
use ospf_rust_core::solver::{

    LinearSolver, SolverCapability, SolverInfo, SolverOutput, SolverStatus,
};
use ospf_rust_core::symbol::BinaryzationMethod;

use super::common::resolve_binaryzation_method;

struct FallbackDemoSolver;

impl SolverInfo for FallbackDemoSolver {
    fn name(&self) -> &str {
        "fallback-demo-solver"
    }

    fn capabilities(&self) -> Vec<SolverCapability> {
        vec![SolverCapability::Linear, SolverCapability::Mip]
    }
}

impl LinearSolver for FallbackDemoSolver {
    fn solve_linear(
        &self,
        _model: &ospf_rust_core::model::intermediate::LinearTriadModel,
    ) -> ospf_rust_core::Result<SolverOutput> {
        Ok(SolverOutput::new(SolverStatus::Feasible).with_solution(vec![]))
    }
}

fn print_gate_result(solver: &dyn SolverInfo, requested: BinaryzationMethod) {
    let (resolved, fallback) = resolve_binaryzation_method(solver, requested);
    println!(
        "solver={} requested={:?} resolved={:?}",
        solver.name(),
        requested,
        resolved
    );
    if let Some(message) = fallback {
        println!("fallback: {}", message);
    }
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let native_solver = ospf_rust_core::solver::solvers::GurobiSolver::new();
    let fallback_solver = FallbackDemoSolver;

    println!("=== Capability Gate Demo ===");

    print_gate_result(&native_solver, BinaryzationMethod::Indicator);
    print_gate_result(&native_solver, BinaryzationMethod::SOS1);
    print_gate_result(&fallback_solver, BinaryzationMethod::Indicator);
    print_gate_result(&fallback_solver, BinaryzationMethod::SOS1);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_gate() {
        assert!(run().is_ok());
    }
}
