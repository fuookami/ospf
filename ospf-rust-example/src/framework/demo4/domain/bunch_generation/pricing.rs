//! Gantt 定价适配；调用共享路径搜索，不拥有主模型或启动第二次求解。 / Gantt pricing adapter using shared path search without master ownership or a second solve.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex};
use ospf_rust_framework::model::{ColumnGenerationBatch, ColumnGenerationCompletion};
use ospf_rust_framework_gantt_network::NetworkBunchGenerationResult;
use ospf_rust_framework_gantt_scheduling::{GanttError, GanttResult};
use ospf_rust_framework_gantt_scheduling::application::algorithm::bunch_column_generation::{BunchCGPolicy, BunchPricingRequest};
use ospf_rust_framework_gantt_scheduling::domain::bunch_compilation::BunchEntry;
use ospf_rust_framework_gantt_scheduling::domain::common::ExecutorId;
use ospf_rust_framework_network_scheduling::domain::path::PathSearchBudget;
use crate::framework::demo4::domain::rule::service::{BunchRecoveryRules, RecoveryCostPolicy};
use super::problem::RecoveryProblem;
use super::recovery::{RecoveryBunchCandidate, RecoveryBunchGenerationContext, RecoveryPathRequest};

/// 定价结果包含原始业务状态、trace、完成度及完整 reduced cost。 / Pricing result retaining original business state, traces, completion and full reduced costs.
#[derive(Debug, Clone)]
pub struct RecoveryPricingResult {
    /// 向 Gantt 提交的真实批次，不凭空声明无改进证明。 / Actual batch submitted to Gantt, never inventing no-improvement proof.
    pub batch: ColumnGenerationBatch<BunchEntry>,
    /// 每个飞机域的完整搜索结果。 / Complete search results for each aircraft domain.
    pub searches: Vec<NetworkBunchGenerationResult<RecoveryBunchCandidate, RecoveryBunchCandidate>>,
    /// 所有已返回完整候选的 reduced cost，按稳定业务身份排列。 / Reduced costs of all returned complete candidates by stable business identity.
    pub reduced_costs: BTreeMap<String, f64>,
}

/// 只持有不可变业务域；注册态由每个 Gantt context 独占。 / Own only immutable business domains; each Gantt context exclusively owns registration state.
pub struct RecoveryPricingPolicy<C> {
    problem: Arc<RecoveryProblem<C>>,
    maximum_labels: Option<usize>,
    history: Mutex<Vec<RecoveryPricingResult>>,
}

impl<C: RecoveryCostPolicy> RecoveryPricingPolicy<C> {
    /// 配置单次每飞机标签预算，None 表示不截断搜索。 / Configure a per-aircraft label budget, with None leaving search untruncated.
    pub fn new(problem: Arc<RecoveryProblem<C>>, maximum_labels: Option<usize>) -> Self {
        Self { problem, maximum_labels, history: Mutex::new(vec![]) }
    }

    /// 保留本应用每轮作用域、完整状态和诊断，不包含模型注册态。 / Retain per-round scopes, complete states and diagnostics for this application, without model registration state.
    pub fn history(&self) -> GanttResult<Vec<RecoveryPricingResult>> {
        self.history.lock().map(|history| history.clone()).map_err(|_| invalid("poisoned recovery pricing history"))
    }

    /// 使用全部声明行的有符号对偶计算完整成本，不把真实运行费用改写成 reduced cost。 / Compute full costs using signed duals for every declared row without overwriting operating cost with reduced cost.
    pub fn price(
        &self,
        request: &BunchPricingRequest,
        budget: &PathSearchBudget<'_>,
    ) -> GanttResult<RecoveryPricingResult> {
        let groups = request.executor_ids.iter().map(ToString::to_string).collect::<std::collections::BTreeSet<_>>();
        if groups.len() != request.executor_ids.len() || groups != request.scope.groups || !request.scope.executor_slots.is_empty()
            || !request.executor_slot_shadow_prices.is_empty()
        { return Err(invalid("invalid or slotted recovery pricing scope")); }
        let mut rows = request.row_duals.clone();
        let known = self.problem.domains().iter().map(|domain| domain.aircraft.reg_no.0.as_str()).collect::<HashSet<_>>();
        if groups.iter().any(|group| !known.contains(group.as_str())) { return Err(invalid("unknown aircraft pricing domain")); }
        // 裸价格路径只允许启发式；证明请求的完整原行由 Gantt 从当前模型证书归一化。 / Raw prices are heuristic only; Gantt normalizes certified original rows from the current model certificate.
        if rows.is_empty() && request.verified_duals.is_none() {
            for (task, _) in self.problem.configuration().tasks.iter().enumerate() {
                rows.insert(format!("task_compilation_{task}"), request.shadow_prices.get(&task).copied().unwrap_or(0.0));
            }
            for aircraft in &self.problem.configuration().aircraft {
                rows.insert(format!("executor_compilation_{}", aircraft.reg_no.0), 0.0);
            }
            for name in self.problem.row_names() { rows.entry(name).or_insert(0.0); }
        }
        let expected = self.problem.row_names();
        if rows.keys().cloned().collect::<std::collections::BTreeSet<_>>() != expected || rows.values().any(|price| !price.is_finite()) {
            return Err(invalid("missing, unmapped or non-finite recovery row duals"));
        }
        let mut searches = Vec::new();
        let mut children = Vec::new();
        let mut priced = Vec::new();
        let mut reduced_costs = BTreeMap::new();
        for (position, executor) in request.executor_ids.iter().enumerate() {
            if request.hidden_executors.contains(executor) || request.fixed_groups.iter().any(|group| &group.executor_id == executor) {
                continue;
            }
            let domain = self.problem.domains().iter().find(|domain| domain.aircraft.reg_no.0 == executor.as_str())
                .ok_or_else(|| invalid("missing aircraft domain"))?;
            let mut scope = request.scope.clone();
            scope.request = format!("{}:{position}", scope.request);
            scope.groups = [executor.to_string()].into();
            let result = RecoveryBunchGenerationContext.generate_with_policy(&RecoveryPathRequest {
                aircraft: &domain.aircraft, candidates: &domain.candidates,
                rules: BunchRecoveryRules { usability: &domain.usability, previous: domain.previous.as_ref(), links: self.problem.links() },
                cost: self.problem.cost(), scope: scope.clone(), maximum_results: None,
            }, budget, |bunch| self.problem.accepts_bunch(bunch), self.problem.transition_policy()).map_err(invalid)?;
            let mut child_columns = Vec::new();
            for candidate in &result.batch.columns {
                let mut entry = self.problem.entry(candidate)?;
                entry.iteration = request.iteration;
                let coefficients = self.problem.column_coefficients(candidate)?;
                if coefficients.len() != rows.len() || coefficients.keys().ne(rows.keys()) {
                    return Err(invalid("missing or unmapped recovery row dual"));
                }
                let reduced = coefficients.iter().try_fold(candidate.cost, |value, (key, coefficient)| {
                    let next = value - coefficient * rows[key];
                    if next.is_finite() { Ok(next) } else { Err(invalid("non-finite full reduced cost")) }
                })?;
                reduced_costs.insert(entry.id.to_string(), reduced);
                if reduced < -1e-7 {
                    child_columns.push(entry.clone());
                    priced.push((reduced, entry));
                }
            }
            children.push(ColumnGenerationBatch { columns: child_columns, completion: result.batch.completion,
                scope: Some(scope), no_improving_columns: None, children: vec![] });
            searches.push(result);
        }
        priced.sort_by(|left, right| left.0.total_cmp(&right.0).then_with(|| left.1.id.as_str().cmp(right.1.id.as_str())));
        let completion = if children.len() != request.executor_ids.len() { ColumnGenerationCompletion::Heuristic }
            else if children.iter().any(|child| child.completion == ColumnGenerationCompletion::Interrupted) { ColumnGenerationCompletion::Interrupted }
            else if children.iter().any(|child| child.completion == ColumnGenerationCompletion::BudgetLimit) { ColumnGenerationCompletion::BudgetLimit }
            else if children.iter().all(|child| child.completion == ColumnGenerationCompletion::Exhausted) { ColumnGenerationCompletion::Exhausted }
            else { ColumnGenerationCompletion::Heuristic };
        Ok(RecoveryPricingResult {
            batch: ColumnGenerationBatch { columns: priced.into_iter().map(|(_, entry)| entry).collect(), completion,
                scope: Some(request.scope.clone()), no_improving_columns: None, children }, searches, reduced_costs,
        })
    }
}

impl<C: RecoveryCostPolicy + Send + Sync> BunchCGPolicy for RecoveryPricingPolicy<C> {
    fn build_shadow_price_map(&self) -> HashMap<usize, f64> { HashMap::new() }

    fn reduced_cost(&self, _: &HashMap<usize, f64>, _: &BunchEntry) -> f64 {
        // 旧回调缺少执行器和扩展行对偶，不能安全退休列。 / The legacy callback lacks executor and extension-row duals and cannot safely retire columns.
        f64::MIN
    }

    fn generate_bunches(&self, _: usize, _: &[ExecutorId], _: &HashMap<usize, f64>) -> Vec<BunchEntry> { vec![] }

    fn generate_bunch_batch(&self, request: &BunchPricingRequest) -> GanttResult<ColumnGenerationBatch<BunchEntry>> {
        let result = self.price(request, &PathSearchBudget { maximum_labels: self.maximum_labels, ..Default::default() })?;
        self.history.lock().map_err(|_| invalid("poisoned recovery pricing history"))?.push(result.clone());
        Ok(result.batch)
    }
}

fn invalid(error: impl std::fmt::Display) -> GanttError {
    GanttError::Calculation { message: format!("非法恢复定价 / invalid recovery pricing: {error}") }
}
