//! CSP1D LP 主问题求解接口 / CSP1D LP master solving interface
//!
//! 列生成的 RMP 必须是真实 LP：只有拿到**最优解与真实对偶**才能定价。本模块把该需求抽象成一个
//! 可注入的接口，使 `Csp1dColumnGeneration` 不必绑定具体求解器后端，同时保证：
//! 1. 不可行终态返回显式错误，而不是让调用方读到未初始化的对偶；
//! 2. 非最优终态返回显式错误，避免把非最优对偶当成定价依据；
//! 3. 对偶缺失返回显式错误，绝不用零值伪造。
//!
//! The RMP of column generation must be a real LP: only an **optimal solution with real duals** supports
//! pricing. This module abstracts that requirement behind an injectable interface so
//! `Csp1dColumnGeneration` stays independent of a concrete solver backend while guaranteeing: infeasible
//! terminal states return an explicit error instead of uninitialised duals; non-optimal terminal states return
//! an explicit error; and missing duals return an explicit error, never fabricated zeros.

use ospf_rust_base::error::Ret;

use crate::domain::produce::Csp1dProduceContext;
use crate::Csp1dResult;

/// 构造显式的求解失败 / Build an explicit solving failure
///
/// 错误信息保留原始语义，便于定位「不可行」与「无对偶」的区别。
/// The message keeps the original semantics so "infeasible" and "no dual" stay distinguishable.
fn solving_failure(message: &str) -> crate::Csp1dError {
    crate::Csp1dError::Calculation {
        message: message.to_string(),
    }
}

// ============================================================================
// LpTerminalStatus - LP 终态 / LP terminal status
// ============================================================================

/// LP 终态 / LP terminal status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LpTerminalStatus {
    /// 取得最优解 / Optimal solution obtained
    Optimal,
    /// 模型不可行 / The model is infeasible
    Infeasible,
    /// 模型无界 / The model is unbounded
    Unbounded,
    /// 其它非最优终态（超时、数值问题等）/ Any other non-optimal terminal state
    Other,
}

impl LpTerminalStatus {
    /// 是否为最优 / Whether optimal
    pub fn is_optimal(&self) -> bool {
        matches!(self, Self::Optimal)
    }
}

// ============================================================================
// LpMasterSolution - LP 主问题解 / LP master solution
// ============================================================================

/// LP 主问题解 / LP master solution
#[derive(Debug, Clone)]
pub struct LpMasterSolution {
    /// 终态 / Terminal status
    pub status: LpTerminalStatus,
    /// 目标值 / Objective value
    pub objective: f64,
    /// 约束对偶值，顺序与模型约束一致 / Constraint duals, in model constraint order
    pub duals: Vec<f64>,
    /// 原始解，顺序与模型变量一致 / Primal solution, in model variable order
    pub primal: Vec<f64>,
    /// 诊断信息 / Diagnostics
    pub info: std::collections::BTreeMap<String, String>,
}

impl LpMasterSolution {
    /// 构造最优解 / Build an optimal solution
    pub fn optimal(objective: f64, duals: Vec<f64>, primal: Vec<f64>) -> Self {
        Self {
            status: LpTerminalStatus::Optimal,
            objective,
            duals,
            primal,
            info: std::collections::BTreeMap::new(),
        }
    }

    /// 构造不可行终态 / Build an infeasible terminal state
    pub fn infeasible() -> Self {
        Self {
            status: LpTerminalStatus::Infeasible,
            objective: 0.0,
            duals: Vec::new(),
            primal: Vec::new(),
            info: std::collections::BTreeMap::new(),
        }
    }

    /// 追加诊断信息 / Append a diagnostic entry
    pub fn with_info(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.info.insert(key.into(), value.into());
        self
    }
}

// ============================================================================
// Csp1dLpSolver - LP 求解器接口 / LP solver interface
// ============================================================================

/// CSP1D LP 主问题求解器 / CSP1D LP master solver
///
/// 实现方负责把已注册的 `MetaModel` 交给真实求解器，并如实回报终态、目标与对偶。
/// An implementation hands the registered `MetaModel` to a real solver and reports the terminal state, the
/// objective and the duals faithfully.
pub trait Csp1dLpSolver<V: ospf_rust_core::solver::SolveValue>: std::fmt::Debug + Send + Sync {
    /// 求解器名称 / Solver name
    fn name(&self) -> &str;

    /// 求解已注册的 LP 主问题 / Solve the registered LP master
    ///
    /// 返回 `Err` 表示求解过程本身失败（环境、许可证、数值异常等），返回 `Ok` 且终态非最优表示
    /// 模型层面没有最优解；两者都不应被静默忽略。
    /// An `Err` means the solve itself failed (environment, licence, numerics); an `Ok` with a non-optimal
    /// terminal state means the model has no optimal solution. Neither may be silently ignored.
    fn solve_lp(
        &self,
        name: &str,
        model: &ospf_rust_core::model::MetaModel<f64>,
    ) -> Ret<LpMasterSolution>;
}

// ============================================================================
// 结果校验 / Result validation
// ============================================================================

/// 校验 LP 结果可用于定价 / Validate that an LP result supports pricing
///
/// 三类失败都返回显式错误，且错误信息保留原始终态描述。
/// All three failure modes return an explicit error whose message keeps the original terminal description.
pub fn require_pricing_ready(solution: &LpMasterSolution) -> Csp1dResult<()> {
    match solution.status {
        LpTerminalStatus::Optimal => {}
        LpTerminalStatus::Infeasible => {
            return Err(solving_failure(
                "LP 没有可用于定价的最优解：模型不可行 / LP has no optimal pricing solution: the model is infeasible",
            ));
        }
        LpTerminalStatus::Unbounded => {
            return Err(solving_failure(
                "LP 没有可用于定价的最优解：模型无界 / LP has no optimal pricing solution: the model is unbounded",
            ));
        }
        LpTerminalStatus::Other => {
            return Err(solving_failure(
                "LP 没有可用于定价的最优解：终态非最优 / LP has no optimal pricing solution: non-optimal terminal state",
            ));
        }
    }
    if solution.duals.is_empty() {
        return Err(solving_failure(
            "LP 未取得对偶解 / LP dual solution is unavailable",
        ));
    }
    Ok(())
}

/// 构造可行性阶段的目标系数 / Build the objective coefficients of the feasibility phase
///
/// 人工短缺变量的系数为 1，真实方案列为 0：这正是「只最小化人工短缺」的口径。
/// Artificial deficit variables carry coefficient 1 while real plan columns carry 0, which is exactly the
/// "minimise artificial deficits only" convention.
pub fn feasibility_penalty<V: ospf_rust_core::solver::SolveValue>(
    context: &Csp1dProduceContext<V>,
) -> Vec<(usize, f64)> {
    let Some(r#yield) = context.r#yield.as_ref() else {
        return Vec::new();
    };
    let mut terms = Vec::new();
    for demand_index in 0..r#yield.demands.len() {
        if let Some(index) = r#yield.variables.under_index(demand_index) {
            terms.push((index, 1.0));
        }
        // 超产不参与可行性目标：短缺是唯一需要被消除的不可行来源。
        // Over-production never enters the feasibility objective: a deficit is the only source of infeasibility
        // that has to be removed.
    }
    terms
}
