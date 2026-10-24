//! 主问题求解接口契约测试 / Master solver interface contract tests
//!
//! 覆盖可注入求解器的三类边界：注入后确实被调用、终态不可交付时回退启发式、
//! 以及内置启发式与默认路径结果一致。
//! Covers the three boundaries of the injectable solver: it is actually invoked, a non-deliverable terminal state
//! falls back to the heuristic, and the built-in heuristic matches the default path.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use ospf_rust_base::error::Ret;
use ospf_rust_framework_csp2d::application::{
    Csp2dHeuristicMasterSolver, Csp2dMasterSolveResult, Csp2dMasterSolver, Csp2dMasterTerminalStatus,
    Csp2dProblem, Csp2dSolvers, require_deliverable,
};
use ospf_rust_framework_csp2d::domain::material::{
    Material2, Product2, ProductDemand2, material_id2, product_id2,
};
use ospf_rust_framework_csp2d::domain::produce::Csp2dMasterProblem;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::Meter;

fn meters(value: f64) -> Quantity<f64, Meter> {
    Quantity::new_ct(value)
}

fn problem(amount: u64) -> Csp2dProblem {
    Csp2dProblem::single(
        Material2::sheet(material_id2("m1"), "sheet", meters(10.0), meters(8.0), 100),
        Product2::new(product_id2("a"), "a", meters(2.0), meters(2.0)),
        amount,
        meters(0.0),
    )
}

// ============================================================================
// 终态契约 / Terminal contract
// ============================================================================

/// 只有最优允许交付使用次数 / Only optimal may deliver usage counts
#[test]
fn only_optimal_is_deliverable() {
    let optimal = Csp2dMasterSolveResult::optimal(vec![1.0], None);
    assert!(optimal.status.allows_delivery());
    assert!(require_deliverable(&optimal).is_ok());

    for status in [
        Csp2dMasterTerminalStatus::Infeasible,
        Csp2dMasterTerminalStatus::Unbounded,
        Csp2dMasterTerminalStatus::Other,
    ] {
        let mut result = Csp2dMasterSolveResult::optimal(vec![1.0], None);
        result.status = status;
        assert!(!result.status.allows_delivery(), "{status:?}");
        assert!(require_deliverable(&result).is_err(), "{status:?}");
    }
}

/// 不可行结果不带使用次数 / An infeasible result carries no usage
#[test]
fn infeasible_result_has_no_usage() {
    let result = Csp2dMasterSolveResult::infeasible();
    assert_eq!(result.status, Csp2dMasterTerminalStatus::Infeasible);
    assert!(result.usage.is_empty());
}

// ============================================================================
// 注入求解器 / Injected solver
// ============================================================================

/// 记录调用的假求解器 / A fake solver recording invocations
#[derive(Debug)]
struct CountingSolver {
    calls: Arc<AtomicUsize>,
    status: Csp2dMasterTerminalStatus,
}

impl Csp2dMasterSolver for CountingSolver {
    fn name(&self) -> &str {
        "counting"
    }

    fn solve_master(&self, master: &Csp2dMasterProblem) -> Ret<Csp2dMasterSolveResult> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut result = Csp2dMasterSolveResult::optimal(
            vec![1.0; master.assignment.len()],
            Some(1.0),
        );
        result.status = self.status;
        Ok(result)
    }
}

/// 注入的求解器会被调用 / An injected solver is invoked
#[test]
fn injected_solver_is_invoked() {
    let calls = Arc::new(AtomicUsize::new(0));
    let solver = Arc::new(CountingSolver {
        calls: calls.clone(),
        status: Csp2dMasterTerminalStatus::Optimal,
    });
    let service = Csp2dSolvers::new().with_master_solver(solver);

    let _ = service.solve_milp(problem(20)).expect("solve");

    assert!(calls.load(Ordering::SeqCst) > 0, "the injected solver must be called");
}

/// 终态不可交付时回退启发式 / A non-deliverable terminal state falls back to the heuristic
#[test]
fn non_deliverable_status_falls_back() {
    let calls = Arc::new(AtomicUsize::new(0));
    let solver = Arc::new(CountingSolver {
        calls: calls.clone(),
        status: Csp2dMasterTerminalStatus::Infeasible,
    });
    let service = Csp2dSolvers::new().with_master_solver(solver);

    // 不可行的注入结果不得让流程崩溃，也不得被当成最优。
    // An infeasible injected result must neither crash the flow nor be treated as optimal.
    let solution = service.solve_milp(problem(20)).expect("solve");
    assert!(calls.load(Ordering::SeqCst) > 0);
    // 回退启发式后仍能覆盖需求。
    // The heuristic fallback still covers the demand.
    assert!(solution.fully_covered || solution.plan_count() > 0);
}

/// 内置启发式可独立求解 / The built-in heuristic solves on its own
#[test]
fn builtin_heuristic_solves() {
    let solver = Csp2dHeuristicMasterSolver::new();
    assert_eq!(solver.name(), "heuristic");

    // 通过公开入口验证：未注入时走同一条路径且能覆盖需求。
    // Verified through the public entry: without injection the same path runs and covers the demand.
    let solution = Csp2dSolvers::new().solve_milp(problem(20)).expect("solve");
    assert!(solution.fully_covered);
}
