//! 求解器终态适配契约测试 / Solver terminal-state adaptation contract tests
//!
//! 覆盖真实后端接入时的三类边界：终态分类（不可行 / 无界 / 最优 / 其它）、只有最优允许定价、
//! 以及缺对偶时的显式说明。
//! Covers the three boundaries of real backend integration: terminal classification (infeasible, unbounded,
//! optimal, other), "only optimal allows pricing", and the explicit message for missing duals.

use ospf_rust_framework_csp1d::application::lp_solver::LpTerminalStatus;
use ospf_rust_framework_csp1d::application::solver_adapter::{
    allows_pricing, classify_terminal_status, missing_dual_message,
};

/// 不可行终态被正确识别 / An infeasible terminal state is recognised
#[test]
fn classifies_infeasible() {
    for raw in ["INFEASIBLE", "infeasible_or_unbounded", "No_Solution", "no solution"] {
        assert_eq!(
            classify_terminal_status(raw),
            LpTerminalStatus::Infeasible,
            "raw = {raw}"
        );
    }
}

/// 无界终态被正确识别 / An unbounded terminal state is recognised
#[test]
fn classifies_unbounded() {
    for raw in ["UNBOUNDED", "unbounded_or_infeasible"] {
        // "unbounded_or_infeasible" 同时含两个关键词，按不可行优先归类（更保守）。
        // "unbounded_or_infeasible" contains both keywords and is classified as infeasible first (more
        // conservative).
        let expected = if raw.contains("infeasible") {
            LpTerminalStatus::Infeasible
        } else {
            LpTerminalStatus::Unbounded
        };
        assert_eq!(classify_terminal_status(raw), expected, "raw = {raw}");
    }
}

/// 最优终态被正确识别 / An optimal terminal state is recognised
#[test]
fn classifies_optimal() {
    for raw in ["OPTIMAL", "optimal", " Optimal "] {
        assert_eq!(
            classify_terminal_status(raw),
            LpTerminalStatus::Optimal,
            "raw = {raw}"
        );
    }
}

/// 未识别与子最优终态归入其它 / Unrecognised and sub-optimal states fall into "other"
#[test]
fn classifies_other() {
    for raw in ["TIME_LIMIT", "SUBOPTIMAL", "NUMERICAL", ""] {
        assert_eq!(
            classify_terminal_status(raw),
            LpTerminalStatus::Other,
            "raw = {raw}"
        );
    }
}

/// 只有最优允许交付定价结果 / Only optimal may deliver a pricing result
#[test]
fn only_optimal_allows_pricing() {
    assert!(allows_pricing(LpTerminalStatus::Optimal));
    assert!(!allows_pricing(LpTerminalStatus::Infeasible));
    assert!(!allows_pricing(LpTerminalStatus::Unbounded));
    assert!(!allows_pricing(LpTerminalStatus::Other));
}

/// 缺对偶的说明包含后端名 / The missing-dual message names the backend
#[test]
fn missing_dual_message_names_backend() {
    let message = missing_dual_message("gurobi");
    assert!(message.contains("gurobi"), "{message}");
    assert!(
        message.contains("对偶") || message.contains("dual"),
        "{message}"
    );
}
