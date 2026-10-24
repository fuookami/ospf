//! 任务列定价请求和模型绑定证据。 / Task-column pricing requests and model-bound evidence.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::{SolveReport, linear_model_fingerprint};
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_framework::model::{ColumnGenerationBatch, ColumnGenerationCompletion, ColumnGenerationScope, VerifiedPricingDuals};
use ospf_rust_framework::solver::column_generation_solver::LPResult;
use crate::{GanttError, GanttResult};
use crate::domain::common::PricingEvidence;
use crate::domain::task::Cost;
use crate::domain::task_compilation::context::IterativeTaskCompilationContext;

/// 声明的任务/执行器时隙域；非时隙上下文使用时隙零。 / Declared task/executor-slot domain; unslotted contexts use slot zero.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskPricingDomain {
    /// 稳定任务索引 / Stable task indices.
    pub tasks: BTreeSet<usize>,
    /// 执行器与时隙身份 / Executor-slot identities.
    pub executor_slots: BTreeSet<(usize, usize)>,
}

impl TaskPricingDomain {
    /// 创建无时隙的完整矩形域。 / Create a complete unslotted rectangular domain.
    pub fn unslotted(tasks: usize, executors: usize) -> Self {
        Self { tasks: (0..tasks).collect(), executor_slots: (0..executors).map(|executor| (executor, 0)).collect() }
    }

    pub(crate) fn covers(&self, other: &Self) -> bool {
        other.tasks.is_subset(&self.tasks) && other.executor_slots.is_subset(&self.executor_slots)
    }
}

/// 待注册的任务-执行器列；数值在既有 context 注册边界投影。 / Task-executor column awaiting projection at the existing context-registration boundary.
#[derive(Debug, Clone)]
pub struct TaskPricingColumn<V: SolveValue = f64> {
    /// 任务索引 / Task index.
    pub task_index: usize,
    /// 执行器索引 / Executor index.
    pub executor_index: usize,
    /// 完整业务成本 / Complete business cost.
    pub cost: Cost<V>,
}

/// 当前 LP 和业务域绑定的只读定价输入。 / Read-only pricing input bound to the current LP and business domain.
#[derive(Debug, Clone)]
pub struct TaskPricingRequest {
    /// 本轮证据作用域；字符串本身不认证来源。 / Current evidence scope; strings alone do not certify provenance.
    pub scope: ColumnGenerationScope,
    /// 当前迭代 / Current iteration.
    pub iteration: usize,
    /// 本次请求域 / Requested domain.
    pub domain: TaskPricingDomain,
    /// 包含显式零值的任务价格 / Task prices including explicit zeros.
    pub shadow_prices: HashMap<usize, f64>,
    /// 全部原模型行的有符号对偶；不省略扩展行或零值。 / Signed duals for every original model row, including extensions and zeros.
    pub row_duals: BTreeMap<String, f64>,
    /// 已固定列 / Fixed columns.
    pub fixed_columns: HashSet<usize>,
    /// 已保留列 / Kept columns.
    pub kept_columns: HashSet<usize>,
    /// 实际 LP 来源；裸外部价格没有该句柄。 / Actual LP provenance; raw external prices have no handle.
    pub verified_duals: Option<VerifiedPricingDuals>,
}

/// 任务定价策略；旧生成器默认不产生证明。 / Task-pricing policy; legacy generators do not produce proof by default.
pub trait TaskCGPolicy: Send + Sync {
    /// 旧候选生成接口，仅启发式。 / Legacy candidate generation, heuristic only.
    fn generate_tasks(&self, request: &TaskPricingRequest) -> GanttResult<Vec<TaskPricingColumn>>;

    /// 返回真实完成度与独立无改进证据。 / Return actual completion and independent no-improvement evidence.
    fn generate_task_batch(&self, request: &TaskPricingRequest) -> GanttResult<ColumnGenerationBatch<TaskPricingColumn>> {
        self.generate_tasks(request).map(ColumnGenerationBatch::heuristic)
    }
}

fn calculation(error: impl std::fmt::Display) -> GanttError {
    GanttError::Calculation { message: error.to_string() }
}

static NEXT_REQUEST: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
struct PricingState {
    domain: Option<TaskPricingDomain>,
    rows: BTreeMap<usize, String>,
    revision: Option<String>,
    fixed: BTreeSet<usize>,
    kept: BTreeSet<usize>,
}

impl PricingState {
    fn new<C: IterativeTaskCompilationContext>(
        context: &C,
        fixed: &HashSet<usize>,
        kept: &HashSet<usize>,
    ) -> Self {
        Self { domain: context.pricing_domain(), rows: context.task_pricing_rows(),
            revision: context.pricing_revision(),
            fixed: fixed.iter().copied().collect(), kept: kept.iter().copied().collect() }
    }
}

#[derive(Default)]
pub(crate) struct TaskPricingSession {
    evidence: Option<PricingEvidence>,
    source_context: Option<PricingState>,
    bound: Option<(f64, PricingState)>,
    completion: Option<ColumnGenerationCompletion>,
}

impl TaskPricingSession {
    pub fn invalidate(&mut self) {
        self.evidence = None;
        self.source_context = None;
        self.bound = None;
        self.completion = None;
    }

    pub fn accept_report<C: IterativeTaskCompilationContext>(
        &mut self,
        model: &MetaModel<f64>,
        context: &C,
        report: &SolveReport<f64>,
    ) -> GanttResult<()> {
        self.invalidate();
        report.validate().map_err(calculation)?;
        if report.is_optimal() {
            self.evidence = Some(PricingEvidence::new(model, report)?);
            self.source_context = Some(PricingState::new(context, &HashSet::new(), &HashSet::new()));
        }
        Ok(())
    }

    pub fn accept_result<C: IterativeTaskCompilationContext>(
        &mut self,
        model: &MetaModel<f64>,
        context: &C,
        result: &LPResult,
    ) -> GanttResult<()> {
        self.invalidate();
        let report = result.report.as_ref().or(result.result.source_report.as_ref())
            .ok_or_else(|| calculation("认证任务定价需要原始 LP 报告 / certified task pricing requires the original LP report"))?;
        // 不使用会改写原报告对偶的兼容转换。 / Do not use compatibility conversion that overwrites source duals.
        for source in result.report.iter().chain(result.result.source_report.iter()) {
            source.validate().map_err(calculation)?;
            let solution = source.solution.as_ref().ok_or_else(|| calculation("LP 投影缺少 incumbent / LP projection lacks an incumbent"))?;
            if solution.values != result.result.solution
                || solution.objective_value.or(solution.objective) != Some(result.result.obj)
                || solution.dual_solution.as_ref() != Some(&result.dual_solution.constraints)
            {
                return Err(calculation("LP primal、目标或对偶投影已改变 / LP primal, objective or dual projection was changed"));
            }
            if source.is_optimal() { PricingEvidence::new(model, source)?; }
        }
        self.accept_report(model, context, report)
    }

    pub fn generate<C: IterativeTaskCompilationContext>(
        &mut self,
        model: &MetaModel<f64>,
        context: &C,
        mut request: TaskPricingRequest,
        policy: &dyn TaskCGPolicy,
    ) -> GanttResult<ColumnGenerationBatch<TaskPricingColumn>> {
        self.bound = None;
        self.completion = None;
        let state = PricingState::new(context, &request.fixed_columns, &request.kept_columns);
        if self.evidence.is_some()
            && self.source_context.as_ref() != Some(&PricingState::new(context, &HashSet::new(), &HashSet::new()))
        {
            return Err(calculation("任务业务域或注册状态已改变 / task business domain or registration state changed"));
        }
        if state.domain.as_ref().is_some_and(|domain| !domain.covers(&request.domain))
            || request.domain.executor_slots.iter().any(|(_, slot)| *slot != 0)
            || request.domain.tasks.iter().any(|task| *task >= context.task_count_for_shadow_price())
        {
            return Err(calculation("任务定价域超出上下文或包含不支持的时隙 / task-pricing domain exceeds context or contains unsupported slots"));
        }
        let mut current = model.try_to_linear_triad_model().map_err(calculation)?;
        current.linear_relax();
        request.scope.model = format!("{}:{:?}", model.model_identity(), linear_model_fingerprint(&current).map_err(calculation)?);
        request.scope.request = format!("task-pricing-{}", NEXT_REQUEST.fetch_add(1, Ordering::Relaxed));
        request.scope.branch = format!("fixed={:?};kept={:?}", state.fixed, state.kept);
        request.scope.groups = request.domain.tasks.iter().map(|task| format!("task:{task}")).collect();
        request.scope.executor_slots = request.domain.executor_slots.iter().map(|(executor, slot)| format!("{executor}:{slot}")).collect();
        request.scope.global = self.evidence.is_some() && state.domain.as_ref() == Some(&request.domain)
            && state.revision.is_some() && state.fixed.is_empty() && state.kept.is_empty();
        if let Some(evidence) = &self.evidence {
            evidence.validate_for(model)?;
            request.row_duals = evidence.row_duals.clone();
            request.shadow_prices = request.domain.tasks.iter().map(|task| {
                let row = state.rows.get(task).ok_or_else(|| calculation("缺少任务行映射 / missing task-row mapping"))?;
                let price = evidence.row_duals.get(row).ok_or_else(|| calculation("缺少任务行对偶 / missing task-row dual"))?;
                Ok((*task, *price))
            }).collect::<GanttResult<_>>()?;
            request.verified_duals = Some(evidence.certificate.clone());
            request.scope.dual = format!("{:?}", evidence.certificate.duals());
        } else {
            request.verified_duals = None;
            request.row_duals.clear();
            request.scope.dual = "heuristic-unverified".into();
            if request.shadow_prices.values().any(|price| !price.is_finite()) {
                return Err(calculation("启发式任务价格非有限 / non-finite heuristic task price"));
            }
        }
        let batch = policy.generate_task_batch(&request)?;
        validate_batch(&batch, &request.scope, &request.domain)?;
        if batch.certifies_no_improving_columns(&request.scope) && let Some(evidence) = &self.evidence {
            self.bound = evidence.lower_bound(model)?.map(|value| (value, state));
        }
        self.completion = Some(batch.completion);
        Ok(batch)
    }

    pub fn lower_bound<C: IterativeTaskCompilationContext>(
        &self,
        model: &MetaModel<f64>,
        context: &C,
        fixed: &HashSet<usize>,
        kept: &HashSet<usize>,
    ) -> GanttResult<Option<f64>> {
        let Some((value, state)) = &self.bound else { return Ok(None); };
        if *state != PricingState::new(context, fixed, kept) { return Ok(None); }
        self.evidence.as_ref().ok_or_else(|| calculation("缺少任务 LP 来源 / missing task LP provenance"))?.validate_for(model)?;
        Ok(Some(*value))
    }

    pub fn completion(&self) -> Option<ColumnGenerationCompletion> {
        self.completion
    }
}

fn validate_batch(
    batch: &ColumnGenerationBatch<TaskPricingColumn>,
    expected: &ColumnGenerationScope,
    domain: &TaskPricingDomain,
) -> GanttResult<()> {
    if batch.scope.as_ref().is_some_and(|scope| scope != expected) {
        return Err(calculation("任务批次作用域不匹配 / task-batch scope mismatch"));
    }
    for column in &batch.columns {
        validate_column(column, domain)?;
    }
    let mut requests = BTreeSet::new();
    for child in &batch.children {
        let scope = child.scope.as_ref().ok_or_else(|| calculation("子批次缺少作用域 / child batch lacks a scope"))?;
        if !requests.insert(&scope.request)
            || !(scope.request == expected.request || scope.request.starts_with(&format!("{}:", expected.request)))
            || scope.model != expected.model || scope.dual != expected.dual || scope.branch != expected.branch
            || !scope.groups.is_subset(&expected.groups) || !scope.executor_slots.is_subset(&expected.executor_slots)
        {
            return Err(calculation("任务子批次作用域不匹配 / task child-batch scope mismatch"));
        }
        let child_domain = TaskPricingDomain {
            tasks: domain.tasks.iter().copied().filter(|task| scope.groups.contains(&format!("task:{task}"))).collect(),
            executor_slots: domain.executor_slots.iter().copied().filter(|(executor, slot)| scope.executor_slots.contains(&format!("{executor}:{slot}"))).collect(),
        };
        validate_batch(child, scope, &child_domain)?;
    }
    Ok(())
}

pub(crate) fn validate_column(column: &TaskPricingColumn, domain: &TaskPricingDomain) -> GanttResult<()> {
    if !domain.tasks.contains(&column.task_index) || !domain.executor_slots.contains(&(column.executor_index, 0))
        || (column.cost.cost_sum.is_none() && !column.cost.items.is_empty())
        || column.cost.cost_sum.is_some_and(|value| !value.is_finite())
        || column.cost.items.iter().any(|item| item.cost_quantity.is_some_and(|value| !value.is_finite()))
    {
        return Err(calculation("任务列索引或成本非法 / invalid task-column indices or costs"));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
