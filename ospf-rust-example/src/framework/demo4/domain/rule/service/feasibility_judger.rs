//! 可行性判断器 / Feasibility judger

use super::super::model::{Link, Restriction, RestrictionCheckingResult};

/// 可行性判断器 / Feasibility judger
/// 对齐 Kotlin FeasibilityJudger
pub struct FeasibilityJudger;

impl FeasibilityJudger {
    /// 判断任务组合是否可行 / Judge whether the task combination is feasible
    pub fn judge(
        &self,
        _task_ids: &[String],
        _links: &[Link],
        restrictions: &[Restriction],
    ) -> FeasibilityResult {
        let mut violations = Vec::new();

        for restriction in restrictions {
            match restriction.check() {
                RestrictionCheckingResult::Violate { reason } => {
                    violations.push(reason);
                }
                RestrictionCheckingResult::ViolableViolate { reason } => {
                    violations.push(format!("(可违反) {}", reason));
                }
                _ => {}
            }
        }

        FeasibilityResult {
            feasible: violations.is_empty(),
            violations,
        }
    }
}

/// 可行性结果 / Feasibility result
#[derive(Debug, Clone)]
pub struct FeasibilityResult {
    /// 是否可行 / Whether feasible
    pub feasible: bool,
    /// 违规原因列表 / Violation reason list
    pub violations: Vec<String>,
}
