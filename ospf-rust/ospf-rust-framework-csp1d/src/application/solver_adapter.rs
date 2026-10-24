//! 求解器终态适配 / Solver terminal-state adaptation
//!
//! 把求解后端回报的终态映射成 LP 列生成需要的四态，并明确区分「求解失败」与「模型无最优解」。
//! 这是接入真实求解器时唯一需要随后端调整的地方，定价与主问题逻辑都不受影响。
//! Maps a backend's terminal state onto the four states LP column generation needs and keeps "the solve failed"
//! distinct from "the model has no optimum". This is the only place that changes when a real backend is wired in;
//! pricing and master-problem logic stay untouched.

use crate::application::lp_solver::LpTerminalStatus;

/// 后端终态分类 / Backend terminal-state classification
///
/// 后端的字符串终态千差万别，这里只按语义归入四类，未识别的终态一律归入 `Other`，
/// 由上层显式拒绝而不是乐观当成最优。
/// Backends name terminal states very differently, so they are grouped into four semantic classes; anything
/// unrecognised falls into `Other` and is explicitly rejected upstream instead of being optimistically treated as
/// optimal.
pub fn classify_terminal_status(raw: &str) -> LpTerminalStatus {
    let normalized = raw.trim().to_ascii_lowercase();
    // 不可行：明确无解，与「无界」和「超时」都不同。
    // Infeasible: definitively no solution, distinct from both "unbounded" and "timed out".
    // 归一化前先把空格与连字符折叠成下划线，使 "no solution" / "no-solution" 与
    // "no_solution" 得到同一判定。
    // Spaces and hyphens are folded to underscores first so "no solution", "no-solution" and "no_solution" all
    // reach the same decision.
    if normalized.contains("infeasible")
        || normalized.contains("no_solution")
        || normalized.contains("no solution")
        || normalized.contains("no-solution")
    {
        return LpTerminalStatus::Infeasible;
    }
    if normalized.contains("unbounded") {
        return LpTerminalStatus::Unbounded;
    }
    if normalized == "optimal" || normalized.contains("optimal") && !normalized.contains("sub") {
        return LpTerminalStatus::Optimal;
    }
    LpTerminalStatus::Other
}

/// 终态是否允许交付定价结果 / Whether a terminal state may deliver a pricing result
///
/// 只有最优允许；其余状态都必须走显式失败路径。
/// Only optimal is allowed; every other state must take the explicit failure path.
pub fn allows_pricing(status: LpTerminalStatus) -> bool {
    status.is_optimal()
}

/// 缺少对偶时的显式说明 / Explicit explanation for missing duals
///
/// 真实后端可能只回原始解而不回对偶（例如某些 MIP 后端或未开启对偶保存）。
/// 这种情况下不能把零值当成对偶继续定价，必须显式说明。
/// A real backend may return only the primal solution without duals (some MIP backends, or when dual saving is
/// off). Zeros must not be passed off as duals in that case; it has to be stated explicitly.
pub fn missing_dual_message(backend: &str) -> String {
    format!(
        "求解后端 {backend} 未返回对偶解，无法定价 / The backend {backend} returned no dual solution, so pricing is impossible"
    )
}
