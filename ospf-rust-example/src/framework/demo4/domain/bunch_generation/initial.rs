//! 从原始束偏好及不可变锁定域自动生成初始恢复列，不启动嵌套求解。 / Generate initial recovery columns from original-bunch preferences and immutable locks without a nested solve.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use ospf_rust_framework::model::{ColumnGenerationBatch, ColumnGenerationCompletion, ColumnGenerationScope};
use ospf_rust_framework_gantt_network::NetworkBunchGenerationResult;
use ospf_rust_framework_gantt_scheduling::{GanttError, GanttResult};
use ospf_rust_framework_network_scheduling::domain::path::PathSearchBudget;
use crate::framework::demo4::domain::rule::service::{BunchRecoveryRules, RecoveryCostPolicy};
use crate::framework::demo4::domain::task::model::FlightTaskBunch;
use super::problem::RecoveryProblem;
use super::recovery::{RecoveryBunchCandidate, RecoveryBunchGenerationContext, RecoveryPathRequest};

/// 初始列业务偏好；锁定约束在 RecoveryProblem 上声明并持续作用于全部定价。 / Initial-column preferences; locks are declared on RecoveryProblem and remain effective in all pricing rounds.
#[derive(Debug, Clone, Default)]
pub struct RecoveryInitialConfiguration {
    /// 完整原始束；实际恢复记录仅从当前有限活动域取得，不根据任务 ID 猜时间。 / Complete original bunches; actual recoveries come only from the current finite activity domain, never inferred from task IDs.
    pub originals: Vec<RecoveryBunchCandidate>,
}

/// 自动初始列及未被截断的搜索诊断；不是初始主问题可行性或最优性证书。 / Automatic initial columns and untruncated search diagnostics, not a certificate of initial-master feasibility or optimality.
#[derive(Debug, Clone)]
pub struct RecoveryInitialGenerationResult {
    /// 每飞机至多一个偏好候选，包含实际完整费用。 / At most one preferred candidate per aircraft, with actual complete costs.
    pub batch: ColumnGenerationBatch<RecoveryBunchCandidate>,
    /// 每飞机的完整搜索状态、trace 和诊断。 / Complete per-aircraft search state, traces and diagnostics.
    pub searches: Vec<NetworkBunchGenerationResult<RecoveryBunchCandidate, RecoveryBunchCandidate>>,
}

/// 无状态初始列服务，复用与定价相同的共享路径及航空终止校验。 / Stateless initial-column service sharing the pricing path kernel and aviation terminal checks.
#[derive(Debug, Clone, Copy, Default)]
pub struct RecoveryInitialBunchGenerator;

impl RecoveryInitialBunchGenerator {
    /// 优先保留原始任务顺序，随后最少增加活动并按完整费用排序；无法恢复时保留完成度而不伪造可行初始解。 / Prefer original task order, then fewer added activities and complete cost; preserve completion without fabricating a feasible seed when recovery fails.
    pub fn generate<C: RecoveryCostPolicy>(
        &self,
        problem: &RecoveryProblem<C>,
        configuration: &RecoveryInitialConfiguration,
        budget: &PathSearchBudget<'_>,
    ) -> GanttResult<RecoveryInitialGenerationResult> {
        let mut executors = HashSet::new();
        let mut tasks = HashSet::new();
        for original in &configuration.originals {
            problem.entry(original)?;
            if !executors.insert(original.bunch.aircraft().reg_no.0.as_str()) || original.bunch.task_ids().any(|task| !tasks.insert(task)) {
                return Err(invalid("duplicate original aircraft or task"));
            }
        }
        let scope = ColumnGenerationScope { request: format!("demo4-initial-{}", NEXT_REQUEST.fetch_add(1, Ordering::Relaxed)),
            model: "unregistered-recovery-domain".into(), dual: "heuristic-initial".into(), branch: "root".into(),
            groups: problem.domains().iter().map(|domain| domain.aircraft.reg_no.0.clone()).collect(), executor_slots: Default::default(), global: false };
        let mut columns = Vec::new();
        let mut searches = Vec::new();
        let mut children = Vec::new();
        for (index, domain) in problem.domains().iter().enumerate() {
            let mut child = scope.clone();
            child.request = format!("{}:{index}", scope.request);
            child.groups = [domain.aircraft.reg_no.0.clone()].into();
            let result = RecoveryBunchGenerationContext.generate_with_policy(&RecoveryPathRequest {
                aircraft: &domain.aircraft, candidates: &domain.candidates,
                rules: BunchRecoveryRules { usability: &domain.usability, previous: domain.previous.as_ref(), links: problem.links() },
                cost: problem.cost(), scope: child.clone(), maximum_results: None,
            }, budget, |bunch| problem.accepts_bunch(bunch), problem.transition_policy()).map_err(invalid)?;
            let original = configuration.originals.iter().find(|original| original.bunch.aircraft() == &domain.aircraft)
                .map(|original| original.bunch.task_ids().collect::<Vec<_>>()).unwrap_or_default();
            let preferred = result.batch.columns.iter().min_by(|left, right| {
                let left_score = retained(&left.bunch, &original);
                let right_score = retained(&right.bunch, &original);
                right_score.cmp(&left_score)
                    .then_with(|| (left.bunch.activities().len() - left_score).cmp(&(right.bunch.activities().len() - right_score)))
                    .then_with(|| left.cost.total_cmp(&right.cost)).then_with(|| left.bunch.id().cmp(right.bunch.id()))
            }).cloned().into_iter().collect::<Vec<_>>();
            columns.extend(preferred.clone());
            children.push(ColumnGenerationBatch { columns: preferred, completion: result.batch.completion,
                scope: Some(child), no_improving_columns: None, children: vec![] });
            searches.push(result);
        }
        let completion = if children.iter().any(|batch| batch.completion == ColumnGenerationCompletion::Interrupted) { ColumnGenerationCompletion::Interrupted }
            else if children.iter().any(|batch| batch.completion == ColumnGenerationCompletion::BudgetLimit) { ColumnGenerationCompletion::BudgetLimit }
            else if children.iter().all(|batch| batch.completion == ColumnGenerationCompletion::Exhausted) { ColumnGenerationCompletion::Exhausted }
            else { ColumnGenerationCompletion::Heuristic };
        Ok(RecoveryInitialGenerationResult { batch: ColumnGenerationBatch { columns, completion, scope: Some(scope),
            no_improving_columns: None, children }, searches })
    }
}

// 比较原始顺序，不重建任何实际航线或时间。 / Compare original order without reconstructing actual routes or times.
fn retained(bunch: &FlightTaskBunch, original: &[&str]) -> usize {
    let mut lengths = vec![0; original.len() + 1];
    for task in bunch.task_ids() {
        let mut diagonal = 0;
        for (index, planned) in original.iter().enumerate() {
            let previous = lengths[index + 1];
            lengths[index + 1] = if &task == planned { diagonal + 1 } else { lengths[index].max(previous) };
            diagonal = previous;
        }
    }
    lengths.last().copied().unwrap_or(0)
}

static NEXT_REQUEST: AtomicU64 = AtomicU64::new(1);

fn invalid(error: impl std::fmt::Display) -> GanttError {
    GanttError::Calculation { message: format!("恢复初始列生成失败 / recovery initial column generation failed: {error}") }
}
