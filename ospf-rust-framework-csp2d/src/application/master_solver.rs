//! 主问题求解接口 / Master problem solving interface
//!
//! 把「给定主问题、求各图案使用次数」抽象成可注入的接口，使应用层不必绑定具体求解器：
//! 未注入时使用内置的确定性启发式，注入后即可接真实 LP / 整数求解。
//! Abstracts "given a master problem, find the pattern usage counts" into an injectable interface so the
//! application layer stays independent of a concrete solver: without injection the built-in deterministic
//! heuristic is used, and with injection a real LP / integer solver can be plugged in.
//!
//! 终态语义与 csp1d / bpp2d 一致：只有最优才允许交付使用次数，其余状态都必须走显式失败路径。
//! The terminal semantics match csp1d and bpp2d: only optimal may deliver usage counts; every other state must
//! take the explicit failure path.

use ospf_rust_base::error::Ret;

use crate::domain::produce::Csp2dMasterProblem;
use crate::infrastructure::error::validation_error;

// ============================================================================
// Csp2dMasterTerminalStatus - 主问题终态 / Master problem terminal status
// ============================================================================

/// 主问题终态 / Master problem terminal status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Csp2dMasterTerminalStatus {
    /// 取得最优解 / Optimal solution obtained
    #[default]
    Optimal,
    /// 模型不可行 / The model is infeasible
    Infeasible,
    /// 模型无界 / The model is unbounded
    Unbounded,
    /// 其它非最优终态（超时、数值问题等）/ Any other non-optimal terminal state
    Other,
}

impl Csp2dMasterTerminalStatus {
    /// 是否为最优 / Whether optimal
    pub fn is_optimal(&self) -> bool {
        matches!(self, Self::Optimal)
    }

    /// 是否允许交付使用次数 / Whether usage counts may be delivered
    pub fn allows_delivery(&self) -> bool {
        self.is_optimal()
    }
}

// ============================================================================
// Csp2dMasterSolveResult - 主问题求解结果 / Master problem solve result
// ============================================================================

/// 主问题求解结果 / Master problem solve result
#[derive(Debug, Clone, Default)]
pub struct Csp2dMasterSolveResult {
    /// 终态 / Terminal status
    pub status: Csp2dMasterTerminalStatus,
    /// 各方案的使用次数 / Usage count per plan
    pub usage: Vec<f64>,
    /// 目标值 / Objective value
    pub objective: Option<f64>,
}

impl Csp2dMasterSolveResult {
    /// 构造最优结果 / Build an optimal result
    pub fn optimal(usage: Vec<f64>, objective: Option<f64>) -> Self {
        Self {
            status: Csp2dMasterTerminalStatus::Optimal,
            usage,
            objective,
        }
    }

    /// 构造不可行结果 / Build an infeasible result
    pub fn infeasible() -> Self {
        Self {
            status: Csp2dMasterTerminalStatus::Infeasible,
            usage: Vec::new(),
            objective: None,
        }
    }
}

/// 校验结果可用于交付 / Validate that a result may be delivered
pub fn require_deliverable(result: &Csp2dMasterSolveResult) -> Ret<()> {
    if result.status.allows_delivery() {
        return Ok(());
    }
    Err(validation_error(format!(
        "主问题终态 {:?} 没有可交付的使用次数 / Master terminal state {:?} has no deliverable usage counts",
        result.status, result.status
    )))
}

// ============================================================================
// Csp2dMasterSolver - 主问题求解器接口 / Master problem solver interface
// ============================================================================

/// 主问题求解器 / Master problem solver
///
/// 实现方接收完整主问题（图案使用量、需求、物料可用量），返回各方案的使用次数与终态。
/// An implementation receives the whole master problem (pattern usage, demands, material availability) and
/// returns the usage count per plan together with the terminal state.
pub trait Csp2dMasterSolver: std::fmt::Debug + Send + Sync {
    /// 求解器名称 / Solver name
    fn name(&self) -> &str;

    /// 求解主问题 / Solve the master problem
    fn solve_master(&self, master: &Csp2dMasterProblem) -> Ret<Csp2dMasterSolveResult>;
}

/// 内置启发式求解器 / Built-in heuristic solver
///
/// 与 `Csp2dSolvers` 的默认口径一致：先按「覆盖缺口 / 单位余料」挑方案逐次加一，直到覆盖完整或触到
/// 使用次数上界。它**不是**最优求解器，只保证给出可行且覆盖优先的使用次数。
/// Matches the default convention of `Csp2dSolvers`: repeatedly raise the count of the plan with the best
/// "deficit per unit remnant" until coverage is complete or the usage bound is reached. It is **not** an
/// optimal solver and only guarantees feasible, coverage-first usage counts.
#[derive(Debug, Clone, Default)]
pub struct Csp2dHeuristicMasterSolver;

impl Csp2dHeuristicMasterSolver {
    /// 创建求解器 / Create the solver
    pub fn new() -> Self {
        Self
    }
}

impl Csp2dMasterSolver for Csp2dHeuristicMasterSolver {
    fn name(&self) -> &str {
        "heuristic"
    }

    fn solve_master(&self, master: &Csp2dMasterProblem) -> Ret<Csp2dMasterSolveResult> {
        let plan_count = master.assignment.len();
        let mut usage = vec![0.0f64; plan_count];
        let mut guard = 0usize;
        while guard < 4096 {
            guard += 1;
            let coverage = master.assignment.coverage(&usage, &master.demands);
            let mut best: Option<(usize, f64)> = None;
            for (demand_index, demand) in master.demands.iter().enumerate() {
                let required = demand.demand_value();
                if coverage[demand_index] + 1e-6 >= required {
                    continue;
                }
                for plan_index in 0..plan_count {
                    let coefficient = master.assignment.coverage_coefficient(plan_index, demand);
                    if coefficient <= 0.0 {
                        continue;
                    }
                    // 优先补齐缺口、且单位消耗余料最少的方案。
                    // Prefer the plan closing the largest deficit per unit of remnant.
                    let waste = master.assignment.waste_area(plan_index).max(1e-9);
                    let score = coefficient / waste;
                    best = match best {
                        Some((_, best_score)) if best_score >= score => best,
                        _ => Some((plan_index, score)),
                    };
                }
            }
            let Some((plan_index, _)) = best else {
                break;
            };
            let upper = master
                .assignment
                .usage_upper_bounds
                .get(plan_index)
                .copied()
                .unwrap_or(u64::MAX) as f64;
            if usage[plan_index] + 1.0 > upper {
                break;
            }
            usage[plan_index] += 1.0;
        }
        Ok(Csp2dMasterSolveResult::optimal(usage, None))
    }
}
