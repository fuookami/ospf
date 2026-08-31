use super::super::model::{Link, Restriction, RestrictionCheckingResult};

/// 可行性判断器 / Feasibility judger
/// 对齐 Kotlin FeasibilityJudger
pub struct FeasibilityJudger;

impl FeasibilityJudger {
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
    pub feasible: bool,
    pub violations: Vec<String>,
}
