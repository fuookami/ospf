//! CSP1D 应用服务 / CSP1D application services
//!
//! 这里先承接 Kotlin 应用层的公共服务边界与状态类型。
//! This module initially carries the public service boundaries and state types
//! from the Kotlin application layer.

use ospf_rust_core::solver::SolveValue;
use ospf_rust_core::variable::VariableId;

use crate::application::model::{
    Csp1dConfiguration, Csp1dKpiKeys, Csp1dProblem, Csp1dSolution, Csp1dSolutionAnalyzer,
    Csp1dSolveConfig, Csp1dSolutionStatus,
};
use crate::domain::cutting_plan_generation::{
    Csp1dInitialCuttingPlanGenerator, Csp1dPricingGenerator, Csp1dPricingInput,
    Csp1dPricingObjectiveConfig, CuttingPlanGenerationInput, CuttingPlanGenerationReport,
    CuttingPlanGenerationStatistics, ReducedCostPricingGenerator, SimpleInitialCuttingPlanGenerator,
    width_feasibility_check_from_policies,
};
use crate::domain::material::{from_f64, render_cutting_plan, to_f64, CuttingPlan};
use crate::domain::produce::{
    accept_partial_by_policies, allow_recovery_fallback_by_policies,
    filter_initial_plans_by_policies_with_context, is_equivalent_by_policies,
    select_termination_by_policies_with_default, should_stop_by_policies, Csp1dFlowContext,
    Csp1dDefaultShadowPriceMap, Csp1dIterativeContext, Csp1dModelContext, Csp1dModelingMode,
    Csp1dProduceContext, Csp1dProduceContextBuilder, CuttingPlanUsage, Produce, ProduceInput,
};

/// 列生成终止原因 / Column generation termination reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Csp1dTerminationReason {
    IterationLimitReached,
    LpSolveFailed,
    LpInfeasible,
    PricingConverged,
    AllDuplicates,
    NoInitialPlans,
}

/// 列生成每轮迭代记录 / Column generation iteration record
#[derive(Debug, Clone, PartialEq)]
pub struct Csp1dIterationRecord {
    pub iteration: i64,
    pub lp_objective: f64,
    pub plan_count_before: i64,
    pub priced_plan_count: u64,
    pub plan_count_after: i64,
}

/// 最终 MILP 状态 / Final MILP status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Csp1dFinalMilpStatus {
    NotAttempted,
    Solved,
    Failed,
}

/// 列生成求解追踪信息 / Column generation solve trace
#[derive(Debug, Clone, PartialEq)]
pub struct Csp1dColumnGenerationTrace {
    pub initial_plan_count: u64,
    pub final_plan_count: u64,
    pub priced_plan_count: Vec<u64>,
    pub termination_reason: Csp1dTerminationReason,
    pub iterations: Vec<Csp1dIterationRecord>,
    pub initial_generation_statistics: Option<CuttingPlanGenerationStatistics>,
    pub final_milp_status: Csp1dFinalMilpStatus,
    pub partial_solution_available: bool,
    pub failure_message: Option<String>,
    pub pricing_generation_statistics: Option<CuttingPlanGenerationStatistics>,
    pub lp_failure_message: Option<String>,
}

/// 列生成结果 / Column generation result
#[derive(Debug, Clone)]
pub struct Csp1dColumnGenerationResult<V: SolveValue> {
    pub solution: Csp1dSolution<V>,
    pub trace: Csp1dColumnGenerationTrace,
}

/// 恢复状态 / Recovery status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Csp1dRecoveryStatus {
    Solved,
    RetriedWithoutWarmStart,
    FallbackDisabled,
    SolveFailed,
}

/// warm start 处理状态 / Warm start handling status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Csp1dWarmStartStatus {
    NotProvided,
    Ignored,
    AdapterUnsupported,
    Applied,
    Invalid,
}

/// warm start / Warm start
#[derive(Debug, Clone, Default)]
pub struct Csp1dWarmStart<V: SolveValue> {
    pub cutting_plans: Vec<crate::domain::material::CuttingPlan<V>>,
    pub previous_solution: Option<Csp1dSolution<V>>,
}

/// 恢复选项 / Recovery options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Csp1dRecoveryOptions {
    pub retry_without_warm_start: bool,
}

impl Default for Csp1dRecoveryOptions {
    fn default() -> Self {
        Self {
            retry_without_warm_start: true,
        }
    }
}

/// 恢复输入 / Recovery input
#[derive(Debug, Clone)]
pub struct Csp1dRecoveryInput<V: SolveValue> {
    pub problem: Option<Csp1dProblem<V>>,
    pub solve_config: Option<Csp1dSolveConfig<V>>,
    pub warm_start: Option<Csp1dWarmStart<V>>,
    pub options: Csp1dRecoveryOptions,
}

impl<V: SolveValue> Default for Csp1dRecoveryInput<V> {
    fn default() -> Self {
        Self {
            problem: None,
            solve_config: None,
            warm_start: None,
            options: Csp1dRecoveryOptions::default(),
        }
    }
}

/// 恢复追踪 / Recovery trace
#[derive(Debug, Clone, PartialEq)]
pub struct Csp1dRecoveryTrace {
    pub status: Csp1dRecoveryStatus,
    pub warm_start_status: Csp1dWarmStartStatus,
    pub attempt_count: i64,
    pub warm_start_plan_count: i64,
    pub applied_warm_start_plan_count: i64,
    pub applied_warm_start_usage_count: i64,
    pub message: Option<String>,
}

/// 恢复结果 / Recovery result
#[derive(Debug, Clone)]
pub struct Csp1dRecoveryResult<V: SolveValue> {
    pub solution: Csp1dSolution<V>,
    pub trace: Csp1dRecoveryTrace,
}

/// warm start adapter 输入 / Warm start adapter input
#[derive(Debug, Clone)]
pub struct Csp1dWarmStartAdapterInput<V: SolveValue> {
    pub problem: Option<Csp1dProblem<V>>,
    pub solve_config: Option<Csp1dSolveConfig<V>>,
    pub warm_start: Option<Csp1dWarmStart<V>>,
    pub cutting_plans: Vec<crate::domain::material::CuttingPlan<V>>,
}

impl<V: SolveValue> Default for Csp1dWarmStartAdapterInput<V> {
    fn default() -> Self {
        Self {
            problem: None,
            solve_config: None,
            warm_start: None,
            cutting_plans: Vec::new(),
        }
    }
}

/// warm start adapter 结果 / Warm start adapter result
pub struct Csp1dWarmStartAdapterResult<V: SolveValue> {
    pub initial_generator: Option<Box<dyn Fn(&CuttingPlanGenerationInput<V>) -> Vec<CuttingPlan<V>> + Send + Sync>>,
    pub initial_plan_usages: Vec<crate::domain::produce::CuttingPlanUsage<V>>,
    pub applied_plan_count: i64,
    pub applied_usage_count: i64,
    pub message: Option<String>,
}

impl<V: SolveValue> Default for Csp1dWarmStartAdapterResult<V> {
    fn default() -> Self {
        Self {
            initial_generator: None,
            initial_plan_usages: Vec::new(),
            applied_plan_count: 0,
            applied_usage_count: 0,
            message: None,
        }
    }
}

impl<V: SolveValue> std::fmt::Debug for Csp1dWarmStartAdapterResult<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Csp1dWarmStartAdapterResult")
            .field("has_initial_generator", &self.initial_generator.is_some())
            .field("initial_plan_usages", &self.initial_plan_usages)
            .field("applied_plan_count", &self.applied_plan_count)
            .field("applied_usage_count", &self.applied_usage_count)
            .field("message", &self.message)
            .finish()
    }
}

impl<V: SolveValue> Clone for Csp1dWarmStartAdapterResult<V> {
    fn clone(&self) -> Self {
        Self {
            initial_generator: None,
            initial_plan_usages: self.initial_plan_usages.clone(),
            applied_plan_count: self.applied_plan_count,
            applied_usage_count: self.applied_usage_count,
            message: self.message.clone(),
        }
    }
}

/// warm start adapter / Warm start adapter
pub trait Csp1dWarmStartAdapter<V: SolveValue>: Send + Sync {
    fn apply(&self, input: Csp1dWarmStartAdapterInput<V>) -> Csp1dWarmStartAdapterResult<V>;
}

/// 不支持 warm start 的 adapter / Unsupported warm-start adapter
#[derive(Debug, Clone, Default)]
pub struct Csp1dUnsupportedWarmStartAdapter;

impl<V: SolveValue> Csp1dWarmStartAdapter<V> for Csp1dUnsupportedWarmStartAdapter {
    fn apply(&self, _input: Csp1dWarmStartAdapterInput<V>) -> Csp1dWarmStartAdapterResult<V> {
        Csp1dWarmStartAdapterResult {
            initial_generator: None,
            initial_plan_usages: Vec::new(),
            applied_plan_count: 0,
            applied_usage_count: 0,
            message: Some("Warm start adapter is not configured".into()),
        }
    }
}

/// 方案池 warm start adapter / Cutting-plan-pool warm start adapter
#[derive(Debug, Clone)]
pub struct Csp1dWarmStartPlanPoolAdapter {
    pub append_fallback_plans: bool,
}

impl Default for Csp1dWarmStartPlanPoolAdapter {
    fn default() -> Self {
        Self {
            append_fallback_plans: true,
        }
    }
}

impl<V: SolveValue> Csp1dWarmStartAdapter<V> for Csp1dWarmStartPlanPoolAdapter {
    fn apply(&self, input: Csp1dWarmStartAdapterInput<V>) -> Csp1dWarmStartAdapterResult<V> {
        let warm_start_plans = input.cutting_plans.clone();
        let initial_plan_usages = warm_start_plan_usages(&input);
        let append_fallback_plans = self.append_fallback_plans;
        let applied_plan_count = warm_start_plans.len() as i64;
        let applied_usage_count = initial_plan_usages.len() as i64;
        Csp1dWarmStartAdapterResult {
            initial_generator: Some(Box::new(move |generation_input| {
                let mut plans = warm_start_plans.clone();
                if append_fallback_plans {
                    plans.extend(SimpleInitialCuttingPlanGenerator.generate(generation_input));
                }
                plans
            })),
            initial_plan_usages,
            applied_plan_count,
            applied_usage_count,
            message: Some("Warm start cutting plan pool was applied as initial plan pool".into()),
        }
    }
}

fn warm_start_plan_usages<V: SolveValue>(
    input: &Csp1dWarmStartAdapterInput<V>,
) -> Vec<crate::domain::produce::CuttingPlanUsage<V>> {
    let Some(previous_solution) = input
        .warm_start
        .as_ref()
        .and_then(|warm_start| warm_start.previous_solution.as_ref())
    else {
        return Vec::new();
    };
    let compatible_keys = input
        .cutting_plans
        .iter()
        .map(CuttingPlan::canonical_key)
        .collect::<std::collections::HashSet<_>>();
    previous_solution
        .produce
        .cutting_plans
        .iter()
        .filter(|usage| compatible_keys.contains(&usage.plan.canonical_key()))
        .cloned()
        .collect()
}

/// recovery fallback 禁用异常 / Recovery fallback-disabled exception
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct Csp1dRecoveryFallbackDisabledException {
    pub message: String,
    pub trace: Csp1dRecoveryTrace,
}

/// recovery 求解异常 / Recovery solve exception
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct Csp1dRecoverySolveException {
    pub message: String,
    pub trace: Csp1dRecoveryTrace,
}

/// 列生成入口 / Column generation entry
pub struct Csp1dColumnGeneration<V: SolveValue> {
    pub configuration: Csp1dConfiguration,
    pub initial_generator: Box<dyn Csp1dInitialCuttingPlanGenerator<V>>,
    pub pricing_generator: Box<dyn Csp1dPricingGenerator<V>>,
    pub analyzer: Box<dyn Csp1dSolutionAnalyzer<V>>,
    pub yield_config: Option<crate::domain::r#yield::YieldModelingConfig<V>>,
    pub waste_config: Option<crate::domain::wasting_minimization::WasteMinimizationConfig<V>>,
    pub length_config: Option<crate::domain::length_assignment::LengthAssignmentModelingConfig<V>>,
    pub warm_start_plan_usages: Vec<crate::domain::produce::CuttingPlanUsage<V>>,
}

impl<V: SolveValue> std::fmt::Debug for Csp1dColumnGeneration<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Csp1dColumnGeneration")
            .field("configuration", &self.configuration)
            .field("has_yield_config", &self.yield_config.is_some())
            .field("has_waste_config", &self.waste_config.is_some())
            .field("has_length_config", &self.length_config.is_some())
            .field("warm_start_plan_usages", &self.warm_start_plan_usages.len())
            .finish()
    }
}

impl<V: SolveValue> Default for Csp1dColumnGeneration<V> {
    fn default() -> Self {
        let initial_generator = SimpleInitialCuttingPlanGenerator;
        Self {
            configuration: Csp1dConfiguration::default(),
            initial_generator: Box::new(initial_generator),
            pricing_generator: Box::new(ReducedCostPricingGenerator::new(initial_generator)),
            analyzer: Box::new(crate::application::model::DefaultCsp1dSolutionAnalyzer),
            yield_config: None,
            waste_config: None,
            length_config: None,
            warm_start_plan_usages: Vec::new(),
        }
    }
}

impl<V: SolveValue> Csp1dColumnGeneration<V> {
    pub fn with_generators(
        initial_generator: Box<dyn Csp1dInitialCuttingPlanGenerator<V>>,
        pricing_generator: Box<dyn Csp1dPricingGenerator<V>>,
    ) -> Self {
        Self {
            initial_generator,
            pricing_generator,
            ..Self::default()
        }
    }

    pub fn solve(
        &self,
        problem: Csp1dProblem<V>,
        solve_config: Option<Csp1dSolveConfig<V>>,
    ) -> Csp1dSolution<V> {
        self.solve_with_trace(problem, solve_config).solution
    }

    pub fn solve_with_trace(
        &self,
        problem: Csp1dProblem<V>,
        solve_config: Option<Csp1dSolveConfig<V>>,
    ) -> Csp1dColumnGenerationResult<V> {
        self.solve_with_trace_internal(problem, solve_config, None, &self.warm_start_plan_usages)
    }

    fn solve_with_trace_internal(
        &self,
        problem: Csp1dProblem<V>,
        solve_config: Option<Csp1dSolveConfig<V>>,
        initial_generator: Option<&(dyn Fn(&CuttingPlanGenerationInput<V>) -> Vec<CuttingPlan<V>> + Send + Sync)>,
        warm_start_plan_usages: &[crate::domain::produce::CuttingPlanUsage<V>],
    ) -> Csp1dColumnGenerationResult<V> {
        let resolved_config = self.resolve_solve_config(&problem, solve_config);
        let width_feasibility_check = width_check_for_problem(&problem, &resolved_config);
        let generation_input = CuttingPlanGenerationInput {
            products: problem.products.clone(),
            materials: problem.materials.clone(),
            machines: problem.machines.clone(),
            costars: problem.costars.clone(),
            demands: problem.demands.clone(),
            existing_plans: Vec::new(),
            domain_policies: resolved_config.extension_set.domain_policies.clone(),
            generation_strategies: resolved_config.extension_set.generation_strategies.clone(),
            candidate_filters: Vec::new(),
            width_feasibility_check,
            canonical_key_overrides: Vec::new(),
            dominance_accept_overrides: Vec::new(),
        };
        let initial_report = if resolved_config.column_generation.max_initial_plans == 0 {
            CuttingPlanGenerationReport {
                plans: Vec::new(),
                statistics: CuttingPlanGenerationStatistics::default(),
            }
        } else if let Some(initial_generator) = initial_generator {
            let plans = initial_generator(&generation_input);
            CuttingPlanGenerationReport {
                plans: plans.clone(),
                statistics: CuttingPlanGenerationStatistics {
                    generated_candidates: plans.len() as i64,
                    accepted_plans: plans.len() as i64,
                    ..CuttingPlanGenerationStatistics::default()
                },
            }
        } else {
            self.initial_generator.generate_with_report(&generation_input)
        };
        let flow_context = BasicFlowContext {
            iteration: 0,
            current_plans: initial_report.plans.clone(),
            iteration_limit: resolved_config.column_generation.iteration_limit,
            allow_partial_solution: true,
            new_plans: Vec::new(),
            pricing_statistics: None,
            has_valid_lp_result: false,
            warm_start_plan_count: warm_start_plan_usages.len() as u64,
            warm_start_requires_fallback: false,
        };
        let mut current_plans = filter_initial_plans_by_policies_with_context(
            deduplicate_plans(initial_report.plans, &[]),
            &resolved_config.extension_set.flow_policies,
            &flow_context,
        );
        current_plans.truncate(resolved_config.column_generation.max_initial_plans as usize);
        let initial_plan_count = current_plans.len() as u64;

        if current_plans.is_empty() {
            let failure_message = "No initial cutting plans generated".to_string();
            let base_solution = self.analyzer.analyze(
                &problem,
                empty_produce(&problem),
                Vec::new(),
            );
            let solution = enrich_solution(
                base_solution,
                EnrichmentInput {
                    top_plans: Vec::new(),
                    status: Csp1dSolutionStatus::NoInitialPlans,
                    failure_message: Some(failure_message.clone()),
                    termination_reason: Some(Csp1dTerminationReason::NoInitialPlans),
                    final_milp_status: Csp1dFinalMilpStatus::NotAttempted,
                    partial_solution_available: false,
                    initial_statistics: Some(initial_report.statistics.clone()),
                    pricing_statistics: None,
                    lp_failure_message: None,
                    iteration_records: Vec::new(),
                    extraction_policies: &resolved_config.extension_set.extraction_policies,
                    demands: &problem.demands,
                    materials: &problem.materials,
                    machines: &problem.machines,
                    generated_plans: &[],
                },
            );
            return Csp1dColumnGenerationResult {
                solution,
                trace: Csp1dColumnGenerationTrace {
                    initial_plan_count: 0,
                    final_plan_count: 0,
                    priced_plan_count: Vec::new(),
                    termination_reason: Csp1dTerminationReason::NoInitialPlans,
                    iterations: Vec::new(),
                    initial_generation_statistics: Some(initial_report.statistics),
                    final_milp_status: Csp1dFinalMilpStatus::NotAttempted,
                    partial_solution_available: false,
                    failure_message: Some(failure_message),
                    pricing_generation_statistics: None,
                    lp_failure_message: None,
                },
            };
        }

        let mut priced_plan_counts = Vec::new();
        let mut iteration_records = Vec::new();
        let mut pricing_statistics = None;
        let mut termination_reason = Csp1dTerminationReason::PricingConverged;
        for iteration in 0..resolved_config.column_generation.iteration_limit {
            let shadow_prices = self
                .extract_lp_shadow_prices(&problem, &current_plans, &resolved_config)
                .unwrap_or_else(|| optimistic_shadow_prices(&problem));
            let pricing_input = Csp1dPricingInput {
                generation_input: CuttingPlanGenerationInput {
                    existing_plans: current_plans.clone(),
                    ..generation_input.clone()
                },
                shadow_prices,
                max_generated_plans: resolved_config.column_generation.max_pricing_plans,
                objective_config: pricing_objective_config(&resolved_config),
                pricing_cost_modifiers: Vec::new(),
                pricing_benefit_modifiers: Vec::new(),
                is_improving_judges: Vec::new(),
                canonical_key_overrides: Vec::new(),
                pricing_policies: resolved_config.extension_set.pricing_policies.clone(),
            };
            let pricing_report = self.pricing_generator.generate_with_report(&pricing_input);
            pricing_statistics = merge_generation_statistics(pricing_statistics, pricing_report.statistics.clone());
            let plan_count_before = current_plans.len();
            let flow_context = BasicFlowContext {
                iteration: iteration as u64,
                current_plans: current_plans.clone(),
                iteration_limit: resolved_config.column_generation.iteration_limit,
                allow_partial_solution: resolved_config.allow_partial_solution,
                new_plans: pricing_report.plans.clone(),
                pricing_statistics: pricing_statistics.clone(),
                has_valid_lp_result: true,
                warm_start_plan_count: warm_start_plan_usages.len() as u64,
                warm_start_requires_fallback: false,
            };
            let added_plans = deduplicate_plans_by_flow_policies(
                pricing_report.plans,
                &current_plans,
                &resolved_config.extension_set.flow_policies,
                &flow_context,
            );
            if added_plans.is_empty() {
                priced_plan_counts.push(0);
                iteration_records.push(Csp1dIterationRecord {
                    iteration: iteration as i64,
                    lp_objective: plan_count_before as f64,
                    plan_count_before: plan_count_before as i64,
                    priced_plan_count: 0,
                    plan_count_after: plan_count_before as i64,
                });
                termination_reason = if flow_context.new_plans.is_empty() {
                    Csp1dTerminationReason::PricingConverged
                } else {
                    Csp1dTerminationReason::AllDuplicates
                };
                termination_reason = select_termination_reason_by_policies(
                    &flow_context,
                    &resolved_config.extension_set.flow_policies,
                    termination_reason,
                    None,
                )
                .0;
                break;
            }
            current_plans.extend(added_plans.clone());
            priced_plan_counts.push(added_plans.len() as u64);
            iteration_records.push(Csp1dIterationRecord {
                iteration: iteration as i64,
                lp_objective: plan_count_before as f64,
                plan_count_before: plan_count_before as i64,
                priced_plan_count: added_plans.len() as u64,
                plan_count_after: current_plans.len() as i64,
            });
            if iteration + 1 == resolved_config.column_generation.iteration_limit {
                termination_reason = Csp1dTerminationReason::IterationLimitReached;
                termination_reason = select_termination_reason_by_policies(
                    &flow_context,
                    &resolved_config.extension_set.flow_policies,
                    termination_reason,
                    None,
                )
                .0;
            } else {
                let stop_context = BasicFlowContext {
                    current_plans: current_plans.clone(),
                    new_plans: added_plans,
                    ..flow_context
                };
                if should_stop_by_policies(
                    &stop_context,
                    &resolved_config.extension_set.flow_policies,
                ) {
                    termination_reason = select_termination_reason_by_policies(
                        &stop_context,
                        &resolved_config.extension_set.flow_policies,
                        Csp1dTerminationReason::PricingConverged,
                        None,
                    )
                    .0;
                    break;
                }
            }
        }

        let final_result = self.solve_final_heuristic(
            &problem,
            &current_plans,
            &resolved_config,
            warm_start_plan_usages,
        );
        let top_plans = top_cutting_plans(&current_plans, resolved_config.top_k_plan_limit);
        let mut base_solution = self.analyzer.analyze(
            &problem,
            final_result.produce,
            current_plans.clone(),
        );
        base_solution.yield_result = final_result.yield_result;
        base_solution.waste_result = final_result.waste_result;
        base_solution.length_result = final_result.length_result;
        let final_status = final_result.status;
        let allow_partial = if final_status == Csp1dFinalMilpStatus::Failed {
            accept_partial_by_policies(&flow_context, &resolved_config.extension_set.flow_policies)
                && resolved_config.allow_partial_solution
        } else {
            resolved_config.allow_partial_solution
        };
        let solution_status = match final_status {
            Csp1dFinalMilpStatus::Solved => Csp1dSolutionStatus::Feasible,
            Csp1dFinalMilpStatus::Failed if allow_partial => Csp1dSolutionStatus::Partial,
            Csp1dFinalMilpStatus::Failed => Csp1dSolutionStatus::Failed,
            Csp1dFinalMilpStatus::NotAttempted => Csp1dSolutionStatus::Partial,
        };
        let solution = enrich_solution(
            base_solution,
            EnrichmentInput {
                top_plans,
                status: solution_status,
                failure_message: final_result.failure_message.clone(),
                termination_reason: Some(termination_reason),
                final_milp_status: final_status,
                partial_solution_available: final_status == Csp1dFinalMilpStatus::Failed,
                initial_statistics: Some(initial_report.statistics.clone()),
                pricing_statistics: pricing_statistics.clone(),
                lp_failure_message: None,
                iteration_records: iteration_records.clone(),
                extraction_policies: &resolved_config.extension_set.extraction_policies,
                demands: &problem.demands,
                materials: &problem.materials,
                machines: &problem.machines,
                generated_plans: &current_plans,
            },
        );
        Csp1dColumnGenerationResult {
            solution,
            trace: Csp1dColumnGenerationTrace {
                initial_plan_count,
                final_plan_count: current_plans.len() as u64,
                priced_plan_count: priced_plan_counts,
                termination_reason,
                iterations: iteration_records,
                initial_generation_statistics: Some(initial_report.statistics),
                final_milp_status: final_status,
                partial_solution_available: final_status == Csp1dFinalMilpStatus::Failed,
                failure_message: final_result.failure_message,
                pricing_generation_statistics: pricing_statistics,
                lp_failure_message: None,
            },
        }
    }

    fn extract_lp_shadow_prices(
        &self,
        problem: &Csp1dProblem<V>,
        cutting_plans: &[CuttingPlan<V>],
        solve_config: &Csp1dSolveConfig<V>,
    ) -> Option<crate::domain::material::ShadowPriceMap<V>> {
        let input = ProduceInput {
            cutting_plans: cutting_plans.to_vec(),
            demands: problem.demands.clone(),
            materials: problem.materials.clone(),
            machines: problem.machines.clone(),
            warm_start_plan_usages: Vec::new(),
        };
        let mut builder = Csp1dProduceContextBuilder::new(input);
        builder.mode(Csp1dModelingMode::LP);
        for extension in solve_config.all_extensions() {
            builder.extension(extension);
        }
        for policy in &solve_config.extension_set.objective_policies {
            builder.objective_policy(policy.clone());
        }
        let mut context = builder.build().ok()?;
        let mut model = ospf_rust_core::model::MetaModel::<f64>::new("csp1d_lp_shadow_price");
        context.register(&mut model).ok()?;
        let duals = optimistic_dual_solution(problem, &model);
        context.extract_shadow_price(&model, &duals).ok()
    }

    fn resolve_solve_config(
        &self,
        problem: &Csp1dProblem<V>,
        solve_config: Option<Csp1dSolveConfig<V>>,
    ) -> Csp1dSolveConfig<V> {
        let mut config = solve_config.unwrap_or_else(|| {
            problem.solve_config.clone().unwrap_or_else(|| Csp1dSolveConfig {
                column_generation: problem.configuration,
                ..Csp1dSolveConfig::default()
            })
        });
        if config.yield_config.is_none() {
            config.yield_config = self.yield_config.clone();
        }
        if config.waste_config.is_none() {
            config.waste_config = self.waste_config.clone();
        }
        if config.length_config.is_none() {
            config.length_config = self.length_config.clone();
        }
        config
    }

    fn solve_final_heuristic(
        &self,
        problem: &Csp1dProblem<V>,
        cutting_plans: &[CuttingPlan<V>],
        solve_config: &Csp1dSolveConfig<V>,
        warm_start_plan_usages: &[crate::domain::produce::CuttingPlanUsage<V>],
    ) -> HeuristicMilpResult<V> {
        solve_milp_heuristic(
            problem,
            cutting_plans,
            solve_config,
            warm_start_plan_usages,
            true,
        )
    }
}

/// CSP1D 排程入口 / CSP1D schedule entry point
pub struct Csp1dSchedule<V: SolveValue> {
    /// 默认列生成求解入口 / Default column-generation solve entry
    pub column_generation: Csp1dColumnGeneration<V>,
}

impl<V: SolveValue> std::fmt::Debug for Csp1dSchedule<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Csp1dSchedule")
            .field("column_generation", &self.column_generation)
            .finish()
    }
}

impl<V: SolveValue> Default for Csp1dSchedule<V> {
    fn default() -> Self {
        Self {
            column_generation: Csp1dColumnGeneration::default(),
        }
    }
}

impl<V: SolveValue> Csp1dSchedule<V> {
    /// 使用列生成入口创建 / Create with column generation entry
    pub fn new(column_generation: Csp1dColumnGeneration<V>) -> Self {
        Self { column_generation }
    }

    /// 以列生成作为默认排程求解路径 / Use column generation as the default scheduling path
    pub fn solve(
        &self,
        problem: Csp1dProblem<V>,
        solve_config: Option<Csp1dSolveConfig<V>>,
    ) -> Csp1dSolution<V> {
        self.column_generation.solve(problem, solve_config)
    }
}

/// 普通 MILP 入口 / Plain MILP entry
pub struct Csp1dMilp<V: SolveValue> {
    pub configuration: Csp1dConfiguration,
    pub initial_generator: Box<dyn Csp1dInitialCuttingPlanGenerator<V>>,
    pub analyzer: Box<dyn Csp1dSolutionAnalyzer<V>>,
    pub yield_config: Option<crate::domain::r#yield::YieldModelingConfig<V>>,
    pub waste_config: Option<crate::domain::wasting_minimization::WasteMinimizationConfig<V>>,
    pub length_config: Option<crate::domain::length_assignment::LengthAssignmentModelingConfig<V>>,
    pub warm_start_plan_usages: Vec<crate::domain::produce::CuttingPlanUsage<V>>,
}

impl<V: SolveValue> std::fmt::Debug for Csp1dMilp<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Csp1dMilp")
            .field("configuration", &self.configuration)
            .field("has_yield_config", &self.yield_config.is_some())
            .field("has_waste_config", &self.waste_config.is_some())
            .field("has_length_config", &self.length_config.is_some())
            .field("warm_start_plan_usages", &self.warm_start_plan_usages.len())
            .finish()
    }
}

impl<V: SolveValue> Default for Csp1dMilp<V> {
    fn default() -> Self {
        Self {
            configuration: Csp1dConfiguration::default(),
            initial_generator: Box::new(SimpleInitialCuttingPlanGenerator),
            analyzer: Box::new(crate::application::model::DefaultCsp1dSolutionAnalyzer),
            yield_config: None,
            waste_config: None,
            length_config: None,
            warm_start_plan_usages: Vec::new(),
        }
    }
}

impl<V: SolveValue> Csp1dMilp<V> {
    /// 使用初始生成器创建 / Create with initial generator
    pub fn with_initial_generator(
        initial_generator: Box<dyn Csp1dInitialCuttingPlanGenerator<V>>,
    ) -> Self {
        Self {
            initial_generator,
            ..Self::default()
        }
    }

    /// 求解 / Solve
    pub fn solve(
        &self,
        problem: Csp1dProblem<V>,
        solve_config: Option<Csp1dSolveConfig<V>>,
    ) -> Csp1dSolution<V> {
        self.solve_with_trace(problem, solve_config).solution
    }

    /// 带追踪求解 / Solve with trace
    pub fn solve_with_trace(
        &self,
        problem: Csp1dProblem<V>,
        solve_config: Option<Csp1dSolveConfig<V>>,
    ) -> Csp1dColumnGenerationResult<V> {
        let resolved_config = self.resolve_solve_config(&problem, solve_config);
        let width_feasibility_check = width_check_for_problem(&problem, &resolved_config);
        let generation_input = CuttingPlanGenerationInput {
            products: problem.products.clone(),
            materials: problem.materials.clone(),
            machines: problem.machines.clone(),
            costars: problem.costars.clone(),
            demands: problem.demands.clone(),
            existing_plans: Vec::new(),
            domain_policies: resolved_config.extension_set.domain_policies.clone(),
            generation_strategies: resolved_config.extension_set.generation_strategies.clone(),
            candidate_filters: Vec::new(),
            width_feasibility_check,
            canonical_key_overrides: Vec::new(),
            dominance_accept_overrides: Vec::new(),
        };
        let initial_report = if resolved_config.column_generation.max_initial_plans == 0 {
            CuttingPlanGenerationReport {
                plans: Vec::new(),
                statistics: CuttingPlanGenerationStatistics::default(),
            }
        } else {
            self.initial_generator.generate_with_report(&generation_input)
        };
        let flow_context = BasicFlowContext {
            iteration: 0,
            current_plans: initial_report.plans.clone(),
            iteration_limit: resolved_config.column_generation.iteration_limit,
            allow_partial_solution: true,
            new_plans: Vec::new(),
            pricing_statistics: None,
            has_valid_lp_result: false,
            warm_start_plan_count: self.warm_start_plan_usages.len() as u64,
            warm_start_requires_fallback: false,
        };
        let mut generated_plans = filter_initial_plans_by_policies_with_context(
            deduplicate_plans(initial_report.plans, &[]),
            &resolved_config.extension_set.flow_policies,
            &flow_context,
        );
        generated_plans.truncate(resolved_config.column_generation.max_initial_plans as usize);
        if generated_plans.is_empty() {
            let failure_message = "No initial cutting plans generated".to_string();
            let base_solution = self.analyzer.analyze(
                &problem,
                empty_produce(&problem),
                Vec::new(),
            );
            let solution = enrich_solution(
                base_solution,
                EnrichmentInput {
                    top_plans: Vec::new(),
                    status: Csp1dSolutionStatus::NoInitialPlans,
                    failure_message: Some(failure_message.clone()),
                    termination_reason: Some(Csp1dTerminationReason::NoInitialPlans),
                    final_milp_status: Csp1dFinalMilpStatus::NotAttempted,
                    partial_solution_available: false,
                    initial_statistics: Some(initial_report.statistics.clone()),
                    pricing_statistics: None,
                    lp_failure_message: None,
                    iteration_records: Vec::new(),
                    extraction_policies: &resolved_config.extension_set.extraction_policies,
                    demands: &problem.demands,
                    materials: &problem.materials,
                    machines: &problem.machines,
                    generated_plans: &[],
                },
            );
            return Csp1dColumnGenerationResult {
                solution,
                trace: Csp1dColumnGenerationTrace {
                    initial_plan_count: 0,
                    final_plan_count: 0,
                    priced_plan_count: Vec::new(),
                    termination_reason: Csp1dTerminationReason::NoInitialPlans,
                    iterations: Vec::new(),
                    initial_generation_statistics: Some(initial_report.statistics),
                    final_milp_status: Csp1dFinalMilpStatus::NotAttempted,
                    partial_solution_available: false,
                    failure_message: Some(failure_message),
                    pricing_generation_statistics: None,
                    lp_failure_message: None,
                },
            };
        }

        let milp_result = solve_milp_heuristic(
            &problem,
            &generated_plans,
            &resolved_config,
            &self.warm_start_plan_usages,
            false,
        );
        let top_plans = top_cutting_plans(&generated_plans, resolved_config.top_k_plan_limit);
        let mut base_solution = self.analyzer.analyze(
            &problem,
            milp_result.produce,
            generated_plans.clone(),
        );
        base_solution.yield_result = milp_result.yield_result;
        base_solution.waste_result = milp_result.waste_result;
        base_solution.length_result = milp_result.length_result;
        let status = match milp_result.status {
            Csp1dFinalMilpStatus::Solved => Csp1dSolutionStatus::Feasible,
            Csp1dFinalMilpStatus::Failed if resolved_config.allow_partial_solution => {
                Csp1dSolutionStatus::Partial
            }
            Csp1dFinalMilpStatus::Failed => Csp1dSolutionStatus::Failed,
            Csp1dFinalMilpStatus::NotAttempted => Csp1dSolutionStatus::Partial,
        };
        let solution = enrich_solution(
            base_solution,
            EnrichmentInput {
                top_plans,
                status,
                failure_message: milp_result.failure_message.clone(),
                termination_reason: None,
                final_milp_status: milp_result.status,
                partial_solution_available: milp_result.status == Csp1dFinalMilpStatus::Failed,
                initial_statistics: Some(initial_report.statistics.clone()),
                pricing_statistics: None,
                lp_failure_message: None,
                iteration_records: Vec::new(),
                extraction_policies: &resolved_config.extension_set.extraction_policies,
                demands: &problem.demands,
                materials: &problem.materials,
                machines: &problem.machines,
                generated_plans: &generated_plans,
            },
        );
        Csp1dColumnGenerationResult {
            solution,
            trace: Csp1dColumnGenerationTrace {
                initial_plan_count: generated_plans.len() as u64,
                final_plan_count: generated_plans.len() as u64,
                priced_plan_count: Vec::new(),
                termination_reason: Csp1dTerminationReason::PricingConverged,
                iterations: Vec::new(),
                initial_generation_statistics: Some(initial_report.statistics),
                final_milp_status: milp_result.status,
                partial_solution_available: milp_result.status == Csp1dFinalMilpStatus::Failed,
                failure_message: milp_result.failure_message,
                pricing_generation_statistics: None,
                lp_failure_message: None,
            },
        }
    }

    fn resolve_solve_config(
        &self,
        problem: &Csp1dProblem<V>,
        solve_config: Option<Csp1dSolveConfig<V>>,
    ) -> Csp1dSolveConfig<V> {
        let mut config = solve_config.unwrap_or_else(|| {
            problem.solve_config.clone().unwrap_or_else(|| Csp1dSolveConfig {
                column_generation: problem.configuration,
                ..Csp1dSolveConfig::default()
            })
        });
        if config.yield_config.is_none() {
            config.yield_config = self.yield_config.clone();
        }
        if config.waste_config.is_none() {
            config.waste_config = self.waste_config.clone();
        }
        if config.length_config.is_none() {
            config.length_config = self.length_config.clone();
        }
        config
    }
}

/// CSP1D MILP 求解结果 / CSP1D MILP solve result
pub struct Csp1dMilpSolveResult<V: SolveValue> {
    /// 产出结果 / Produce result
    pub produce: Produce<V>,
    /// 产出率结果 / Yield result
    pub yield_result: Option<crate::domain::r#yield::YieldModelingResult<V>>,
    /// 损耗结果 / Waste result
    pub waste_result: Option<crate::domain::wasting_minimization::WasteMinimizationResult<V>>,
    /// 长度分配结果 / Length assignment result
    pub length_result: Option<crate::domain::length_assignment::LengthAssignmentResult<V>>,
    /// 轻量模型 / Lightweight model
    pub model: ospf_rust_core::model::MetaModel<f64>,
    /// 启发式变量解 / Heuristic variable solution
    pub solution_by_id: std::collections::HashMap<VariableId, f64>,
}

impl<V: SolveValue> std::fmt::Debug for Csp1dMilpSolveResult<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Csp1dMilpSolveResult")
            .field("produce", &self.produce)
            .field("yield_result", &self.yield_result)
            .field("waste_result", &self.waste_result)
            .field("length_result", &self.length_result)
            .field("model_token_count", &self.model.tokens().len())
            .field("solution_by_id", &self.solution_by_id)
            .finish()
    }
}

/// CSP1D LP 求解结果 / CSP1D LP solve result
pub struct Csp1dLpSolveResult<V: SolveValue> {
    /// 影子价格 / Shadow prices
    pub shadow_prices: crate::domain::material::ShadowPriceMap<V>,
    /// 轻量模型 / Lightweight model
    pub model: ospf_rust_core::model::MetaModel<f64>,
    /// 占位对偶解 / Placeholder dual solution
    pub dual_solution: Vec<f64>,
    /// framework 影子价格映射 / Framework shadow price map
    pub framework_shadow_price_map: Csp1dDefaultShadowPriceMap,
}

impl<V: SolveValue> std::fmt::Debug for Csp1dLpSolveResult<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Csp1dLpSolveResult")
            .field("shadow_prices", &self.shadow_prices)
            .field("model_constraint_count", &self.model.constraints().len())
            .field("dual_solution", &self.dual_solution)
            .field("framework_shadow_price_map", &self.framework_shadow_price_map)
            .finish()
    }
}

/// CSP1D MILP/LP 求解器 / CSP1D MILP/LP solver
#[derive(Debug, Clone, Default)]
pub struct Csp1dMilpSolver;

impl Csp1dMilpSolver {
    /// 创建求解器 / Create solver
    pub fn new() -> Self {
        Self
    }

    /// 求解 MILP / Solve MILP
    pub fn solve<V: SolveValue>(
        &self,
        input: ProduceInput<V>,
        yield_config: Option<crate::domain::r#yield::YieldModelingConfig<V>>,
        waste_config: Option<crate::domain::wasting_minimization::WasteMinimizationConfig<V>>,
        length_config: Option<crate::domain::length_assignment::LengthAssignmentModelingConfig<V>>,
        extensions: Vec<crate::domain::produce::Csp1dModelingExtension<V>>,
        objective_policies: Vec<std::sync::Arc<dyn crate::domain::produce::Csp1dObjectivePolicy<V>>>,
        is_final_milp: bool,
    ) -> Option<Csp1dMilpSolveResult<V>> {
        solve_milp_input_heuristic(
            input,
            yield_config,
            waste_config,
            length_config,
            extensions,
            objective_policies,
            is_final_milp,
        )
        .ok()
    }

    /// 求解 LP 松弛 / Solve LP relaxation
    pub fn solve_lp<V: SolveValue>(
        &self,
        input: ProduceInput<V>,
        extensions: Vec<crate::domain::produce::Csp1dModelingExtension<V>>,
    ) -> Option<Csp1dLpSolveResult<V>> {
        solve_lp_input_heuristic(input, extensions).ok()
    }
}

/// 恢复入口 / Recovery entry
pub struct Csp1dRecovery<V: SolveValue> {
    pub milp: Csp1dColumnGeneration<V>,
    pub warm_start_adapter: Box<dyn Csp1dWarmStartAdapter<V>>,
}

impl<V: SolveValue> std::fmt::Debug for Csp1dRecovery<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Csp1dRecovery")
            .field("milp", &self.milp)
            .field("has_warm_start_adapter", &true)
            .finish()
    }
}

impl<V: SolveValue> Default for Csp1dRecovery<V> {
    fn default() -> Self {
        Self {
            milp: Csp1dColumnGeneration::default(),
            warm_start_adapter: Box::new(Csp1dUnsupportedWarmStartAdapter),
        }
    }
}

impl<V: SolveValue> Csp1dRecovery<V> {
    pub fn with_warm_start_adapter(
        milp: Csp1dColumnGeneration<V>,
        warm_start_adapter: Box<dyn Csp1dWarmStartAdapter<V>>,
    ) -> Self {
        Self {
            milp,
            warm_start_adapter,
        }
    }

    pub fn solve(
        &self,
        problem: Csp1dProblem<V>,
        solve_config: Option<Csp1dSolveConfig<V>>,
    ) -> Csp1dSolution<V> {
        self.milp.solve(problem, solve_config)
    }

    pub fn solve_with_trace(
        &self,
        input: Csp1dRecoveryInput<V>,
    ) -> crate::Csp1dResult<Csp1dRecoveryResult<V>> {
        solve_recovery_with_trace(&self.milp, self.warm_start_adapter.as_ref(), input)
    }
}

/// 列生成恢复入口 / Column-generation recovery entry
pub struct Csp1dColumnGenerationRecovery<V: SolveValue> {
    pub column_generation: Csp1dColumnGeneration<V>,
    pub warm_start_adapter: Box<dyn Csp1dWarmStartAdapter<V>>,
}

impl<V: SolveValue> std::fmt::Debug for Csp1dColumnGenerationRecovery<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Csp1dColumnGenerationRecovery")
            .field("column_generation", &self.column_generation)
            .finish()
    }
}

impl<V: SolveValue> Default for Csp1dColumnGenerationRecovery<V> {
    fn default() -> Self {
        Self {
            column_generation: Csp1dColumnGeneration::default(),
            warm_start_adapter: Box::new(Csp1dUnsupportedWarmStartAdapter),
        }
    }
}

impl<V: SolveValue> Csp1dColumnGenerationRecovery<V> {
    /// 使用 warm start adapter 创建 / Create with warm-start adapter
    pub fn with_warm_start_adapter(
        column_generation: Csp1dColumnGeneration<V>,
        warm_start_adapter: Box<dyn Csp1dWarmStartAdapter<V>>,
    ) -> Self {
        Self {
            column_generation,
            warm_start_adapter,
        }
    }

    /// 求解 / Solve
    pub fn solve(
        &self,
        problem: Csp1dProblem<V>,
        solve_config: Option<Csp1dSolveConfig<V>>,
    ) -> Csp1dSolution<V> {
        self.column_generation.solve(problem, solve_config)
    }

    /// 带恢复追踪求解 / Solve with recovery trace
    pub fn solve_with_trace(
        &self,
        input: Csp1dRecoveryInput<V>,
    ) -> crate::Csp1dResult<Csp1dRecoveryResult<V>> {
        solve_recovery_with_trace(
            &self.column_generation,
            self.warm_start_adapter.as_ref(),
            input,
        )
    }
}

fn solve_recovery_with_trace<V: SolveValue>(
    solver: &Csp1dColumnGeneration<V>,
    warm_start_adapter: &dyn Csp1dWarmStartAdapter<V>,
    input: Csp1dRecoveryInput<V>,
) -> crate::Csp1dResult<Csp1dRecoveryResult<V>> {
    let problem = input.problem.clone().ok_or_else(|| crate::Csp1dError::InvalidInput {
        message: "Csp1dRecoveryInput.problem is required".into(),
    })?;
    let warm_start_resolution = resolve_warm_start(&input, warm_start_adapter);
    let fallback_required = requires_fallback(warm_start_resolution.status);
    let allow_fallback = allow_recovery_fallback(&input, &warm_start_resolution);
    if fallback_required && !allow_fallback {
        let trace = fallback_disabled_trace(
            warm_start_resolution.status,
            warm_start_resolution.plans.len() as i64,
        );
        return Err(crate::Csp1dError::RecoveryFallbackDisabled {
            message: format!(
                "Warm start cannot be applied and fallback is disabled: {:?}",
                warm_start_resolution.status
            ),
            trace,
        });
    }
    let solve_config = input.solve_config.clone();
    let adapter_summary = warm_start_resolution
        .adapter_result
        .as_ref()
        .map(WarmStartAdapterSummary::from);
    let initial_generator = warm_start_resolution
        .adapter_result
        .as_ref()
        .and_then(|result| result.initial_generator.as_deref());
    let warm_start_plan_usages = warm_start_resolution
        .adapter_result
        .as_ref()
        .map(|result| result.initial_plan_usages.as_slice())
        .unwrap_or(&[]);
    let result = solver.solve_with_trace_internal(
        problem,
        solve_config,
        initial_generator,
        warm_start_plan_usages,
    );
    let status = if warm_start_resolution.status == Csp1dWarmStartStatus::Applied {
        Csp1dRecoveryStatus::Solved
    } else if fallback_required {
        Csp1dRecoveryStatus::RetriedWithoutWarmStart
    } else {
        Csp1dRecoveryStatus::Solved
    };
    Ok(Csp1dRecoveryResult {
        solution: result.solution,
        trace: Csp1dRecoveryTrace {
            status,
            warm_start_status: warm_start_resolution.status,
            attempt_count: 1,
            warm_start_plan_count: warm_start_resolution.plans.len() as i64,
            applied_warm_start_plan_count: adapter_summary
                .as_ref()
                .map(|summary| summary.applied_plan_count)
                .unwrap_or(0),
            applied_warm_start_usage_count: adapter_summary
                .as_ref()
                .map(|summary| summary.applied_usage_count)
                .unwrap_or(0),
            message: adapter_summary
                .and_then(|summary| summary.message)
                .or_else(|| warm_start_message(warm_start_resolution.status)),
        },
    })
}

fn allow_recovery_fallback<V: SolveValue>(
    input: &Csp1dRecoveryInput<V>,
    warm_start_resolution: &WarmStartResolution<V>,
) -> bool {
    if !requires_fallback(warm_start_resolution.status) {
        return input.options.retry_without_warm_start;
    }
    let solve_config_for_policy = input
        .solve_config
        .as_ref()
        .or_else(|| input.problem.as_ref().and_then(|problem| problem.solve_config.as_ref()));
    let Some(solve_config) = solve_config_for_policy else {
        return input.options.retry_without_warm_start;
    };
    let flow_context = BasicFlowContext {
        iteration: 0,
        current_plans: Vec::new(),
        iteration_limit: solve_config.column_generation.iteration_limit,
        allow_partial_solution: solve_config.allow_partial_solution,
        new_plans: Vec::new(),
        pricing_statistics: None,
        has_valid_lp_result: false,
        warm_start_plan_count: warm_start_resolution.plans.len() as u64,
        warm_start_requires_fallback: true,
    };
    allow_recovery_fallback_by_policies(
        &flow_context,
        &solve_config.extension_set.flow_policies,
        input.options.retry_without_warm_start,
    )
}

struct WarmStartResolution<V: SolveValue> {
    plans: Vec<CuttingPlan<V>>,
    status: Csp1dWarmStartStatus,
    adapter_result: Option<Csp1dWarmStartAdapterResult<V>>,
}

struct WarmStartAdapterSummary {
    applied_plan_count: i64,
    applied_usage_count: i64,
    message: Option<String>,
}

impl<V: SolveValue> From<&Csp1dWarmStartAdapterResult<V>> for WarmStartAdapterSummary {
    fn from(result: &Csp1dWarmStartAdapterResult<V>) -> Self {
        Self {
            applied_plan_count: result.applied_plan_count,
            applied_usage_count: result.applied_usage_count,
            message: result.message.clone(),
        }
    }
}

fn resolve_warm_start<V: SolveValue>(
    input: &Csp1dRecoveryInput<V>,
    warm_start_adapter: &dyn Csp1dWarmStartAdapter<V>,
) -> WarmStartResolution<V> {
    let (plans, selected_status) = warm_start_plan_selection(input);
    let initial_status = warm_start_status(input, &plans, selected_status);
    let adapter_result = if initial_status == Csp1dWarmStartStatus::AdapterUnsupported {
        input.warm_start.as_ref().map(|warm_start| {
            warm_start_adapter.apply(Csp1dWarmStartAdapterInput {
                problem: input.problem.clone(),
                solve_config: input.solve_config.clone(),
                warm_start: Some(warm_start.clone()),
                cutting_plans: plans.clone(),
            })
        })
    } else {
        None
    };
    let status = if adapter_result
        .as_ref()
        .and_then(|result| result.initial_generator.as_ref())
        .is_some()
    {
        Csp1dWarmStartStatus::Applied
    } else {
        initial_status
    };
    WarmStartResolution {
        plans,
        status,
        adapter_result,
    }
}

fn warm_start_plan_selection<V: SolveValue>(
    input: &Csp1dRecoveryInput<V>,
) -> (Vec<CuttingPlan<V>>, Option<Csp1dWarmStartStatus>) {
    let Some(warm_start) = &input.warm_start else {
        return (Vec::new(), Some(Csp1dWarmStartStatus::NotProvided));
    };
    if !warm_start.cutting_plans.is_empty() {
        if !is_warm_start_compatible(&warm_start.cutting_plans, input.problem.as_ref()) {
            return (
                warm_start.cutting_plans.clone(),
                Some(Csp1dWarmStartStatus::Invalid),
            );
        }
        return (warm_start.cutting_plans.clone(), None);
    }
    let plans = warm_start
        .previous_solution
        .as_ref()
        .map(|solution| {
            solution
                .generated_plans
                .iter()
                .filter(|plan| is_warm_start_plan_compatible(plan, input.problem.as_ref()))
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    (plans, None)
}

fn warm_start_status<V: SolveValue>(
    input: &Csp1dRecoveryInput<V>,
    plans: &[CuttingPlan<V>],
    selected_status: Option<Csp1dWarmStartStatus>,
) -> Csp1dWarmStartStatus {
    if let Some(status) = selected_status {
        return status;
    }
    if input.warm_start.is_none() {
        return Csp1dWarmStartStatus::NotProvided;
    }
    if plans.is_empty() {
        return Csp1dWarmStartStatus::Ignored;
    }
    if !is_warm_start_compatible(plans, input.problem.as_ref()) {
        return Csp1dWarmStartStatus::Invalid;
    }
    Csp1dWarmStartStatus::AdapterUnsupported
}

fn is_warm_start_compatible<V: SolveValue>(
    plans: &[CuttingPlan<V>],
    problem: Option<&Csp1dProblem<V>>,
) -> bool {
    plans
        .iter()
        .all(|plan| is_warm_start_plan_compatible(plan, problem))
}

fn is_warm_start_plan_compatible<V: SolveValue>(
    plan: &CuttingPlan<V>,
    problem: Option<&Csp1dProblem<V>>,
) -> bool {
    let Some(problem) = problem else {
        return true;
    };
    let Some(material) = problem
        .materials
        .iter()
        .find(|material| material.id == plan.material.id)
    else {
        return false;
    };
    if !material.enabled(plan, &problem.machines) {
        return false;
    }
    if let Some(machine_id) = &plan.machine_id {
        if !problem.machines.is_empty()
            && !problem.machines.iter().any(|machine| machine.id == *machine_id)
        {
            return false;
        }
    }
    plan.demand_contributions.iter().all(|contribution| {
        problem
            .products
            .iter()
            .any(|product| product.id == contribution.product.id)
    })
}

fn requires_fallback(status: Csp1dWarmStartStatus) -> bool {
    matches!(
        status,
        Csp1dWarmStartStatus::Invalid | Csp1dWarmStartStatus::AdapterUnsupported
    )
}

fn warm_start_message(status: Csp1dWarmStartStatus) -> Option<String> {
    match status {
        Csp1dWarmStartStatus::NotProvided => None,
        Csp1dWarmStartStatus::Ignored => Some("Warm start was empty and ignored".into()),
        Csp1dWarmStartStatus::AdapterUnsupported => {
            Some("Warm start is compatible but current adapter did not apply it".into())
        }
        Csp1dWarmStartStatus::Applied => Some("Warm start was applied".into()),
        Csp1dWarmStartStatus::Invalid => {
            Some("Warm start is incompatible with current problem".into())
        }
    }
}

fn fallback_disabled_trace(
    status: Csp1dWarmStartStatus,
    plan_count: i64,
) -> Csp1dRecoveryTrace {
    Csp1dRecoveryTrace {
        status: Csp1dRecoveryStatus::FallbackDisabled,
        warm_start_status: status,
        attempt_count: 0,
        warm_start_plan_count: plan_count,
        applied_warm_start_plan_count: 0,
        applied_warm_start_usage_count: 0,
        message: Some(fallback_disabled_message(status)),
    }
}

fn fallback_disabled_message(status: Csp1dWarmStartStatus) -> String {
    match status {
        Csp1dWarmStartStatus::AdapterUnsupported => {
            "Warm start is compatible but current adapter does not support applying it; fallback is disabled".into()
        }
        Csp1dWarmStartStatus::Invalid => {
            "Warm start is incompatible with current problem; fallback is disabled".into()
        }
        _ => "Warm start cannot be applied and fallback is disabled".into(),
    }
}

struct HeuristicMilpResult<V: SolveValue> {
    status: Csp1dFinalMilpStatus,
    produce: Produce<V>,
    yield_result: Option<crate::domain::r#yield::YieldModelingResult<V>>,
    waste_result: Option<crate::domain::wasting_minimization::WasteMinimizationResult<V>>,
    length_result: Option<crate::domain::length_assignment::LengthAssignmentResult<V>>,
    failure_message: Option<String>,
}

fn solve_milp_heuristic<V: SolveValue>(
    problem: &Csp1dProblem<V>,
    cutting_plans: &[CuttingPlan<V>],
    solve_config: &Csp1dSolveConfig<V>,
    warm_start_plan_usages: &[crate::domain::produce::CuttingPlanUsage<V>],
    is_final_milp: bool,
) -> HeuristicMilpResult<V> {
    let mut selected = select_plans_heuristically(problem, cutting_plans);
    selected.extend(warm_start_plan_usages.iter().cloned());
    let input = ProduceInput {
        cutting_plans: cutting_plans.to_vec(),
        demands: problem.demands.clone(),
        materials: problem.materials.clone(),
        machines: problem.machines.clone(),
        warm_start_plan_usages: selected.clone(),
    };
    match solve_milp_input_heuristic(
        input,
        solve_config.yield_config.clone(),
        solve_config.waste_config.clone(),
        solve_config.length_config.clone(),
        solve_config.all_extensions(),
        solve_config.extension_set.objective_policies.clone(),
        is_final_milp,
    ) {
        Ok(result) => HeuristicMilpResult {
            status: Csp1dFinalMilpStatus::Solved,
            produce: result.produce,
            yield_result: result.yield_result,
            waste_result: result.waste_result,
            length_result: result.length_result,
            failure_message: None,
        },
        Err(message) => HeuristicMilpResult {
            status: Csp1dFinalMilpStatus::Failed,
            produce: empty_produce(problem),
            yield_result: None,
            waste_result: None,
            length_result: None,
            failure_message: Some(message),
        },
    }
}

fn solve_milp_input_heuristic<V: SolveValue>(
    input: ProduceInput<V>,
    yield_config: Option<crate::domain::r#yield::YieldModelingConfig<V>>,
    waste_config: Option<crate::domain::wasting_minimization::WasteMinimizationConfig<V>>,
    length_config: Option<crate::domain::length_assignment::LengthAssignmentModelingConfig<V>>,
    extensions: Vec<crate::domain::produce::Csp1dModelingExtension<V>>,
    objective_policies: Vec<std::sync::Arc<dyn crate::domain::produce::Csp1dObjectivePolicy<V>>>,
    is_final_milp: bool,
) -> Result<Csp1dMilpSolveResult<V>, String> {
    if input.cutting_plans.is_empty() {
        return Err("No cutting plans for CSP1D MILP".into());
    }
    let selected = input.warm_start_plan_usages.clone();
    let resolved_length_config = resolve_default_length_bounds(&input, length_config);
    let mut builder = Csp1dProduceContextBuilder::new(input);
    builder.mode(Csp1dModelingMode::MILP).is_final_milp(is_final_milp);
    if let Some(config) = yield_config {
        builder.yield_config(config);
    }
    if let Some(config) = waste_config {
        builder.waste_config(config);
    }
    if let Some(config) = resolved_length_config {
        builder.length_config(config);
    }
    for extension in extensions {
        builder.extension(extension);
    }
    for policy in objective_policies {
        builder.objective_policy(policy);
    }
    let mut context = builder.build().map_err(|error| error.to_string())?;
    let model_name = if is_final_milp {
        "csp1d_final_heuristic"
    } else {
        "csp1d_milp_heuristic"
    };
    let mut model = ospf_rust_core::model::MetaModel::<f64>::new(model_name);
    context.register(&mut model).map_err(|error| error.to_string())?;
    let mut solution_by_id = std::collections::HashMap::new();
    let usage_by_key = selected
        .iter()
        .fold(std::collections::BTreeMap::new(), |mut acc, usage| {
            acc.entry(usage.plan.canonical_key())
                .and_modify(|amount| *amount += usage.amount)
                .or_insert(usage.amount);
            acc
        });
    for (plan_index, plan) in context.produce.cutting_plans.iter().enumerate() {
        let Some(amount) = usage_by_key.get(&plan.canonical_key()).copied() else {
            continue;
        };
        let Some(variable_index) = context.produce.plan_variable_index(plan_index) else {
            continue;
        };
        if let Some(token) = model.tokens().get(variable_index) {
            solution_by_id.insert(token.id(), amount as f64);
        }
    }
    insert_domain_slack_solution(&context, &selected, &model, &mut solution_by_id);
    model.set_solution_by_id(&solution_by_id);
    let produce = context
        .extract_solution(&model)
        .map_err(|error| error.to_string())?;
    let yield_result = context.extract_yield_result(&model);
    let waste_result = context.extract_waste_result(&model);
    let length_result = context.extract_length_result(&model);
    Ok(Csp1dMilpSolveResult {
        produce,
        yield_result,
        waste_result,
        length_result,
        model,
        solution_by_id,
    })
}

fn resolve_default_length_bounds<V: SolveValue>(
    input: &ProduceInput<V>,
    length_config: Option<crate::domain::length_assignment::LengthAssignmentModelingConfig<V>>,
) -> Option<crate::domain::length_assignment::LengthAssignmentModelingConfig<V>> {
    let Some(mut config) = length_config else {
        return None;
    };
    if config.dynamic_product_ids.is_empty() {
        return Some(config);
    }
    let needs_derivation = config
        .dynamic_product_ids
        .iter()
        .any(|product_id| {
            !config.assigned_length_lower_bound.contains_key(product_id)
                || !config.assigned_length_upper_bound.contains_key(product_id)
        });
    if !needs_derivation {
        return Some(config);
    }
    let dynamic_product_ids = config
        .dynamic_product_ids
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    for product_id in dynamic_product_ids {
        let demand = input
            .demands
            .iter()
            .find(|demand| demand.product.id == product_id);
        let contribution = input
            .cutting_plans
            .iter()
            .flat_map(|plan| plan.demand_contributions.iter())
            .find(|contribution| contribution.product.id == product_id);
        if !config.assigned_length_lower_bound.contains_key(&product_id) {
            let has_zero_source = demand.is_some()
                || contribution.is_some()
                || !config.over_length_penalty.is_empty()
                || config.total_length_penalty.is_some()
                || config.batch_min_penalty.is_some();
            if let Some(zero) = has_zero_source.then(|| from_f64(0.0)).flatten() {
                config
                    .assigned_length_lower_bound
                    .insert(product_id.clone(), zero);
            }
        }
        if !config.assigned_length_upper_bound.contains_key(&product_id) {
            let product = demand
                .map(|demand| &demand.product)
                .or_else(|| contribution.map(|contribution| &contribution.product));
            if let Some(max_over_length) = product
                .and_then(|product| product.max_over_produce_length.as_ref())
            {
                config
                    .assigned_length_upper_bound
                    .insert(product_id, max_over_length.value.clone());
            }
        }
    }
    Some(config)
}

fn solve_lp_input_heuristic<V: SolveValue>(
    input: ProduceInput<V>,
    extensions: Vec<crate::domain::produce::Csp1dModelingExtension<V>>,
) -> Result<Csp1dLpSolveResult<V>, String> {
    if input.cutting_plans.is_empty() {
        return Err("No cutting plans for CSP1D LP".into());
    }
    let domain_value_sample = input
        .demands
        .first()
        .map(|demand| demand.quantity.value.clone())
        .or_else(|| {
            input
                .materials
                .first()
                .map(|material| material.width_range.lower_bound.value.clone())
        })
        .or_else(|| {
            input
                .cutting_plans
                .first()
                .and_then(|plan| plan.rest_width())
                .map(|width| width.value)
        })
        .ok_or_else(|| "Cannot derive V sample from ProduceInput for shadow price extraction".to_string())?;
    let mut builder = Csp1dProduceContextBuilder::new(input.clone());
    builder.mode(Csp1dModelingMode::LP);
    for extension in extensions {
        builder.extension(extension);
    }
    let mut context = builder.build().map_err(|error| error.to_string())?;
    let mut model = ospf_rust_core::model::MetaModel::<f64>::new("csp1d_produce_lp");
    context.register(&mut model).map_err(|error| error.to_string())?;
    let dual_solution = optimistic_dual_solution_for_demands(&input.demands, &model);
    let mut lifecycle = crate::domain::produce::Csp1dShadowPriceLifecycle::with_pipelines(
        domain_value_sample,
        context.cg_pipelines.clone(),
    );
    let shadow_prices = lifecycle
        .try_extract_from_dual_solution(&model, &dual_solution)
        .map_err(|error| error.to_string())?;
    Ok(Csp1dLpSolveResult {
        shadow_prices,
        model,
        dual_solution,
        framework_shadow_price_map: lifecycle.framework_shadow_price_map,
    })
}

fn width_check_for_problem<V: SolveValue>(
    problem: &Csp1dProblem<V>,
    solve_config: &Csp1dSolveConfig<V>,
) -> Option<crate::domain::cutting_plan_generation::Csp1dWidthFeasibilityCheck<V>> {
    let domain_value_sample = problem
        .demands
        .first()
        .map(|demand| demand.quantity.value.clone())
        .or_else(|| {
            problem
                .materials
                .first()
                .map(|material| material.width_range.upper_bound.value.clone())
        })?;
    width_feasibility_check_from_policies(
        &solve_config.extension_set.domain_policies,
        domain_value_sample,
    )
}

#[derive(Debug, Clone)]
struct BasicFlowContext<V: SolveValue> {
    iteration: u64,
    current_plans: Vec<CuttingPlan<V>>,
    iteration_limit: u64,
    allow_partial_solution: bool,
    new_plans: Vec<CuttingPlan<V>>,
    pricing_statistics: Option<CuttingPlanGenerationStatistics>,
    has_valid_lp_result: bool,
    warm_start_plan_count: u64,
    warm_start_requires_fallback: bool,
}

impl<V: SolveValue> Csp1dFlowContext<V> for BasicFlowContext<V> {
    fn iteration(&self) -> u64 {
        self.iteration
    }

    fn current_plans(&self) -> &[CuttingPlan<V>] {
        &self.current_plans
    }

    fn iteration_limit(&self) -> u64 {
        self.iteration_limit
    }

    fn allow_partial_solution(&self) -> bool {
        self.allow_partial_solution
    }

    fn new_plans(&self) -> &[CuttingPlan<V>] {
        &self.new_plans
    }

    fn pricing_statistics(&self) -> Option<&CuttingPlanGenerationStatistics> {
        self.pricing_statistics.as_ref()
    }

    fn has_valid_lp_result(&self) -> bool {
        self.has_valid_lp_result
    }

    fn warm_start_plan_count(&self) -> u64 {
        self.warm_start_plan_count
    }

    fn warm_start_requires_fallback(&self) -> bool {
        self.warm_start_requires_fallback
    }
}

struct EnrichmentInput<'a, V: SolveValue> {
    top_plans: Vec<CuttingPlan<V>>,
    status: Csp1dSolutionStatus,
    failure_message: Option<String>,
    termination_reason: Option<Csp1dTerminationReason>,
    final_milp_status: Csp1dFinalMilpStatus,
    partial_solution_available: bool,
    initial_statistics: Option<CuttingPlanGenerationStatistics>,
    pricing_statistics: Option<CuttingPlanGenerationStatistics>,
    lp_failure_message: Option<String>,
    iteration_records: Vec<Csp1dIterationRecord>,
    extraction_policies: &'a [std::sync::Arc<dyn crate::domain::produce::Csp1dExtractionPolicy<V>>],
    demands: &'a [crate::domain::material::ProductDemand<V>],
    materials: &'a [crate::domain::material::Material<V>],
    machines: &'a [crate::domain::material::Machine<V>],
    generated_plans: &'a [CuttingPlan<V>],
}

fn empty_produce<V: SolveValue>(problem: &Csp1dProblem<V>) -> Produce<V> {
    Produce {
        cutting_plans: Vec::new(),
        material_usages: Vec::new(),
        machine_usages: Vec::new(),
        unmet_demands: problem.demands.clone(),
    }
}

fn deduplicate_plans<V: SolveValue>(
    candidates: Vec<CuttingPlan<V>>,
    existing: &[CuttingPlan<V>],
) -> Vec<CuttingPlan<V>> {
    let mut ids = existing
        .iter()
        .map(|plan| plan.id.clone())
        .collect::<std::collections::HashSet<_>>();
    let mut keys = existing
        .iter()
        .map(CuttingPlan::canonical_key)
        .collect::<std::collections::HashSet<_>>();
    let mut plans = Vec::new();
    for plan in candidates {
        let key = plan.canonical_key();
        if ids.contains(&plan.id) || keys.contains(&key) {
            continue;
        }
        ids.insert(plan.id.clone());
        keys.insert(key);
        plans.push(plan);
    }
    plans
}

fn deduplicate_plans_by_flow_policies<V: SolveValue>(
    candidates: Vec<CuttingPlan<V>>,
    existing: &[CuttingPlan<V>],
    flow_policies: &[std::sync::Arc<dyn crate::domain::produce::Csp1dFlowPolicy<V>>],
    _flow_context: &dyn Csp1dFlowContext<V>,
) -> Vec<CuttingPlan<V>> {
    let mut ids = existing
        .iter()
        .map(|plan| plan.id.clone())
        .collect::<std::collections::HashSet<_>>();
    let mut keys = existing
        .iter()
        .map(CuttingPlan::canonical_key)
        .collect::<std::collections::HashSet<_>>();
    let mut plans = Vec::new();
    for plan in candidates {
        let key = plan.canonical_key();
        let equivalent_to_existing = existing
            .iter()
            .chain(plans.iter())
            .any(|existing| is_equivalent_by_policies(existing, &plan, flow_policies));
        if ids.contains(&plan.id) || keys.contains(&key) || equivalent_to_existing {
            continue;
        }
        ids.insert(plan.id.clone());
        keys.insert(key);
        plans.push(plan);
    }
    plans
}

fn select_termination_reason_by_policies<V: SolveValue>(
    context: &dyn Csp1dFlowContext<V>,
    policies: &[std::sync::Arc<dyn crate::domain::produce::Csp1dFlowPolicy<V>>],
    default_reason: Csp1dTerminationReason,
    default_message: Option<String>,
) -> (Csp1dTerminationReason, Option<String>) {
    let (reason, message) = select_termination_by_policies_with_default(
        context,
        policies,
        termination_reason_name(default_reason).to_string(),
        default_message,
    );
    (parse_termination_reason(&reason).unwrap_or(default_reason), message)
}

fn termination_reason_name(reason: Csp1dTerminationReason) -> &'static str {
    match reason {
        Csp1dTerminationReason::IterationLimitReached => "IterationLimitReached",
        Csp1dTerminationReason::LpSolveFailed => "LpSolveFailed",
        Csp1dTerminationReason::LpInfeasible => "LpInfeasible",
        Csp1dTerminationReason::PricingConverged => "PricingConverged",
        Csp1dTerminationReason::AllDuplicates => "AllDuplicates",
        Csp1dTerminationReason::NoInitialPlans => "NoInitialPlans",
    }
}

fn parse_termination_reason(value: &str) -> Option<Csp1dTerminationReason> {
    match value {
        "IterationLimitReached" => Some(Csp1dTerminationReason::IterationLimitReached),
        "LpSolveFailed" => Some(Csp1dTerminationReason::LpSolveFailed),
        "LpInfeasible" => Some(Csp1dTerminationReason::LpInfeasible),
        "PricingConverged" => Some(Csp1dTerminationReason::PricingConverged),
        "AllDuplicates" => Some(Csp1dTerminationReason::AllDuplicates),
        "NoInitialPlans" => Some(Csp1dTerminationReason::NoInitialPlans),
        _ => None,
    }
}

fn optimistic_shadow_prices<V: SolveValue>(
    problem: &Csp1dProblem<V>,
) -> crate::domain::material::ShadowPriceMap<V> {
    problem
        .demands
        .iter()
        .filter_map(|demand| {
            Some((
                crate::domain::material::Csp1dShadowPriceKey::ProductDemand(
                    crate::domain::material::ProductDemandShadowPriceKey {
                        product_id: demand.product.id.clone(),
                        unit_symbol: crate::domain::material::shadow_price_unit_symbol(&demand.quantity.unit),
                    },
                ),
                from_f64(1.0)?,
            ))
        })
        .collect()
}

fn optimistic_dual_solution<V: SolveValue>(
    problem: &Csp1dProblem<V>,
    model: &ospf_rust_core::model::MetaModel<f64>,
) -> Vec<f64> {
    optimistic_dual_solution_for_demands(&problem.demands, model)
}

fn optimistic_dual_solution_for_demands<V: SolveValue>(
    demands: &[crate::domain::material::ProductDemand<V>],
    model: &ospf_rust_core::model::MetaModel<f64>,
) -> Vec<f64> {
    let demand_keys = demands
        .iter()
        .map(|demand| {
            crate::domain::material::shadow_price_key_to_string(
                &crate::domain::material::Csp1dShadowPriceKey::ProductDemand(
                    crate::domain::material::ProductDemandShadowPriceKey {
                        product_id: demand.product.id.clone(),
                        unit_symbol: crate::domain::material::shadow_price_unit_symbol(&demand.quantity.unit),
                    },
                ),
            )
        })
        .collect::<std::collections::HashSet<_>>();
    model
        .constraints()
        .iter()
        .map(|constraint| {
            constraint
                .args
                .as_ref()
                .filter(|args| demand_keys.contains(*args))
                .map(|_| 1.0)
                .unwrap_or(0.0)
        })
        .collect()
}

fn pricing_objective_config<V: SolveValue>(
    _solve_config: &Csp1dSolveConfig<V>,
) -> Csp1dPricingObjectiveConfig<V> {
    Csp1dPricingObjectiveConfig {
        plan_usage_penalty: None,
        trim_width_penalty: None,
        rest_material_penalty: None,
        material_cost_penalty: std::collections::BTreeMap::new(),
    }
}

fn merge_generation_statistics(
    left: Option<CuttingPlanGenerationStatistics>,
    right: CuttingPlanGenerationStatistics,
) -> Option<CuttingPlanGenerationStatistics> {
    Some(match left {
        Some(left) => CuttingPlanGenerationStatistics {
            visited_nodes: left.visited_nodes + right.visited_nodes,
            generated_candidates: left.generated_candidates + right.generated_candidates,
            accepted_plans: left.accepted_plans + right.accepted_plans,
            infeasible_candidates: left.infeasible_candidates + right.infeasible_candidates,
            duplicate_candidates: left.duplicate_candidates + right.duplicate_candidates,
            dominated_candidates: left.dominated_candidates + right.dominated_candidates,
            width_bound_pruned_nodes: left.width_bound_pruned_nodes + right.width_bound_pruned_nodes,
            knife_bound_pruned_nodes: left.knife_bound_pruned_nodes + right.knife_bound_pruned_nodes,
            length_bound_pruned_entries: left.length_bound_pruned_entries
                + right.length_bound_pruned_entries,
            material_width_index_cache_hits: left.material_width_index_cache_hits
                + right.material_width_index_cache_hits,
            material_slice_template_cache_hits: left.material_slice_template_cache_hits
                + right.material_slice_template_cache_hits,
            quantity_cache_hits: left.quantity_cache_hits + right.quantity_cache_hits,
            quantity_cache_misses: left.quantity_cache_misses + right.quantity_cache_misses,
            material_slice_template_cache_misses: left.material_slice_template_cache_misses
                + right.material_slice_template_cache_misses,
            cross_worker_duplicate_candidates: left.cross_worker_duplicate_candidates
                + right.cross_worker_duplicate_candidates,
            cross_contribution_dominated: left.cross_contribution_dominated
                + right.cross_contribution_dominated,
            elapsed_milliseconds: left.elapsed_milliseconds + right.elapsed_milliseconds,
            stop_reason: right.stop_reason,
        },
        None => right,
    })
}

fn select_plans_heuristically<V: SolveValue>(
    problem: &Csp1dProblem<V>,
    cutting_plans: &[CuttingPlan<V>],
) -> Vec<crate::domain::produce::CuttingPlanUsage<V>> {
    let mut selected = Vec::new();
    let mut supplied = problem
        .demands
        .iter()
        .map(|demand| ((demand.product.id.clone(), demand.quantity.unit.symbol().to_string()), 0.0))
        .collect::<std::collections::BTreeMap<_, _>>();
    let required = problem
        .demands
        .iter()
        .filter_map(|demand| {
            Some((
                (demand.product.id.clone(), demand.quantity.unit.symbol().to_string()),
                to_f64(&demand.quantity.value)?,
            ))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut material_usage = std::collections::BTreeMap::<String, u64>::new();
    for demand in &problem.demands {
        let key = (demand.product.id.clone(), demand.quantity.unit.symbol().to_string());
        let target = *required.get(&key).unwrap_or(&0.0);
        while *supplied.get(&key).unwrap_or(&0.0) + f64::EPSILON < target {
            let Some(plan) = cutting_plans.iter().find(|plan| {
                let current_material_usage = *material_usage.get(&plan.material.id).unwrap_or(&0);
                if current_material_usage >= plan.material.available_batches {
                    return false;
                }
                plan.demand_contributions.iter().any(|contribution| {
                    contribution.product.id == demand.product.id
                        && contribution.quantity.unit == demand.quantity.unit
                        && to_f64(&contribution.quantity.value).unwrap_or(0.0) > 0.0
                })
            }) else {
                break;
            };
            selected.push(crate::domain::produce::CuttingPlanUsage {
                plan: plan.clone(),
                amount: 1,
            });
            material_usage
                .entry(plan.material.id.clone())
                .and_modify(|amount| *amount = amount.saturating_add(1))
                .or_insert(1);
            for contribution in &plan.demand_contributions {
                let contribution_key = (
                    contribution.product.id.clone(),
                    contribution.quantity.unit.symbol().to_string(),
                );
                let Some(value) = to_f64(&contribution.quantity.value) else {
                    continue;
                };
                supplied
                    .entry(contribution_key)
                    .and_modify(|amount| *amount += value)
                    .or_insert(value);
            }
        }
    }
    selected
}

fn top_cutting_plans<V: SolveValue>(
    plans: &[CuttingPlan<V>],
    limit: Option<u64>,
) -> Vec<CuttingPlan<V>> {
    let Some(limit) = limit else {
        return Vec::new();
    };
    if limit == 0 {
        return Vec::new();
    }
    let mut plans = plans.to_vec();
    plans.sort_by(|left, right| {
        let left_used = left
            .used_width()
            .and_then(|quantity| to_f64(&quantity.value))
            .unwrap_or(f64::NEG_INFINITY);
        let right_used = right
            .used_width()
            .and_then(|quantity| to_f64(&quantity.value))
            .unwrap_or(f64::NEG_INFINITY);
        right_used
            .partial_cmp(&left_used)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    plans.truncate(limit as usize);
    plans
}

fn insert_domain_slack_solution<V: SolveValue>(
    context: &Csp1dProduceContext<V>,
    selected: &[CuttingPlanUsage<V>],
    model: &ospf_rust_core::model::MetaModel<f64>,
    solution_by_id: &mut std::collections::HashMap<VariableId, f64>,
) {
    if let Some(r#yield) = &context.r#yield {
        for (demand_index, demand) in r#yield.demands.iter().enumerate() {
            let supplied = selected
                .iter()
                .flat_map(|usage| {
                    usage.plan.demand_contributions.iter().filter_map(|contribution| {
                        if contribution.product.id == demand.product.id
                            && contribution.quantity.unit == demand.quantity.unit
                        {
                            Some(to_f64(&contribution.quantity.value)? * usage.amount as f64)
                        } else {
                            None
                        }
                    })
                })
                .sum::<f64>();
            let Some(required) = to_f64(&demand.quantity.value) else {
                continue;
            };
            let under_value = (required - supplied).max(0.0);
            let over_value = (supplied - required).max(0.0);
            if let Some(token) = r#yield
                .under_production
                .get(demand_index)
                .copied()
                .flatten()
                .and_then(|variable| model.tokens().get(variable))
            {
                solution_by_id.insert(token.id(), under_value);
            }
            if let Some(token) = r#yield
                .over_production
                .get(demand_index)
                .copied()
                .flatten()
                .and_then(|variable| model.tokens().get(variable))
            {
                solution_by_id.insert(token.id(), over_value);
            }
        }
    }

    if let Some(length) = &context.length {
        for (demand_index, demand) in length.demands.iter().enumerate() {
            if !length.config.is_dynamic_product(&demand.product) {
                continue;
            }
            let Some(assigned_value) = demand
                .product
                .length
                .as_ref()
                .or(Some(&demand.quantity))
                .and_then(|quantity| to_f64(&quantity.value))
            else {
                continue;
            };
            if let Some(token) = length
                .assigned_length
                .get(demand_index)
                .copied()
                .flatten()
                .and_then(|variable| model.tokens().get(variable))
            {
                solution_by_id.insert(token.id(), assigned_value);
            }
            let over_value = demand
                .product
                .max_over_produce_length
                .as_ref()
                .and_then(|max_length| {
                    if max_length.unit == demand.quantity.unit {
                        Some((assigned_value - to_f64(&max_length.value)?).max(0.0))
                    } else {
                        None
                    }
                })
                .unwrap_or(0.0);
            if let Some(token) = length
                .over_length
                .get(demand_index)
                .copied()
                .flatten()
                .and_then(|variable| model.tokens().get(variable))
            {
                solution_by_id.insert(token.id(), over_value);
            }
        }
    }
}

fn enrich_solution<V: SolveValue>(
    mut solution: Csp1dSolution<V>,
    input: EnrichmentInput<'_, V>,
) -> Csp1dSolution<V> {
    solution.status = input.status;
    solution.failure_message = input.failure_message.clone();
    solution.top_plans = input.top_plans;
    solution.kpi.top_plan_count = solution.top_plans.len() as u64;
    solution.kpi.yield_metric_count = solution
        .yield_result
        .as_ref()
        .map(|result| {
            result.analysis.under_productions.len()
                + result.analysis.over_productions.len()
                + result.analysis.outputs.len()
        })
        .unwrap_or(0) as u64;
    solution.kpi.waste_metric_count = solution
        .waste_result
        .as_ref()
        .map(|result| result.analysis.metrics.len())
        .unwrap_or(0) as u64;
    solution.kpi.length_metric_count = solution
        .length_result
        .as_ref()
        .map(|result| result.assignments.len() + result.over_length_records.len())
        .unwrap_or(0) as u64;

    let mut details = solution.kpi.details.clone();
    let mut render_kpi = solution.render.kpi.clone();
    insert_detail(
        &mut details,
        &mut render_kpi,
        Csp1dKpiKeys::TopPlanCount,
        solution.kpi.top_plan_count.to_string(),
    );
    insert_detail(
        &mut details,
        &mut render_kpi,
        Csp1dKpiKeys::YieldMetricCount,
        solution.kpi.yield_metric_count.to_string(),
    );
    insert_detail(
        &mut details,
        &mut render_kpi,
        Csp1dKpiKeys::WasteMetricCount,
        solution.kpi.waste_metric_count.to_string(),
    );
    insert_detail(
        &mut details,
        &mut render_kpi,
        Csp1dKpiKeys::LengthMetricCount,
        solution.kpi.length_metric_count.to_string(),
    );
    insert_detail(&mut details, &mut render_kpi, Csp1dKpiKeys::SolutionStatus, format!("{:?}", input.status));
    if let Some(reason) = input.termination_reason {
        insert_detail(&mut details, &mut render_kpi, Csp1dKpiKeys::TerminationReason, format!("{:?}", reason));
        insert_detail(
            &mut details,
            &mut render_kpi,
            Csp1dKpiKeys::ColumnGenerationTerminationReason,
            format!("{:?}", reason),
        );
    }
    insert_detail(
        &mut details,
        &mut render_kpi,
        Csp1dKpiKeys::FinalMilpStatus,
        format!("{:?}", input.final_milp_status),
    );
    insert_detail(
        &mut details,
        &mut render_kpi,
        Csp1dKpiKeys::PartialSolutionAvailable,
        input.partial_solution_available.to_string(),
    );
    if let Some(message) = &input.failure_message {
        insert_detail(&mut details, &mut render_kpi, Csp1dKpiKeys::FailureMessage, message.clone());
    }
    if let Some(message) = &input.lp_failure_message {
        insert_detail(&mut details, &mut render_kpi, Csp1dKpiKeys::LpFailureMessage, message.clone());
    }
    insert_detail(
        &mut details,
        &mut render_kpi,
        Csp1dKpiKeys::ColumnGenerationIterationCount,
        input.iteration_records.len().to_string(),
    );
    if let Some(last) = input.iteration_records.last() {
        insert_detail(
            &mut details,
            &mut render_kpi,
            Csp1dKpiKeys::ColumnGenerationLastLpObjective,
            last.lp_objective.to_string(),
        );
        insert_detail(
            &mut details,
            &mut render_kpi,
            Csp1dKpiKeys::ColumnGenerationLastPlanCount,
            last.plan_count_after.to_string(),
        );
    }
    let priced_total = input
        .iteration_records
        .iter()
        .fold(0_u64, |acc, record| acc.saturating_add(record.priced_plan_count));
    insert_detail(
        &mut details,
        &mut render_kpi,
        Csp1dKpiKeys::ColumnGenerationPricedPlanCount,
        priced_total.to_string(),
    );
    if let Some(statistics) = input.initial_statistics {
        insert_generation_statistics(
            &mut details,
            &mut render_kpi,
            &statistics,
            GenerationKpiPrefix::Initial,
        );
    }
    if let Some(statistics) = input.pricing_statistics.as_ref() {
        insert_generation_statistics(
            &mut details,
            &mut render_kpi,
            statistics,
            GenerationKpiPrefix::Pricing,
        );
    }
    insert_domain_result_details(&solution, &mut details, &mut render_kpi);
    let termination_reason = input.termination_reason.map(|reason| match reason {
        Csp1dTerminationReason::IterationLimitReached => "IterationLimitReached",
        Csp1dTerminationReason::LpSolveFailed => "LpSolveFailed",
        Csp1dTerminationReason::LpInfeasible => "LpInfeasible",
        Csp1dTerminationReason::PricingConverged => "PricingConverged",
        Csp1dTerminationReason::AllDuplicates => "AllDuplicates",
        Csp1dTerminationReason::NoInitialPlans => "NoInitialPlans",
    });
    let final_milp_status = Some(match input.final_milp_status {
        Csp1dFinalMilpStatus::NotAttempted => "NotAttempted",
        Csp1dFinalMilpStatus::Solved => "Solved",
        Csp1dFinalMilpStatus::Failed => "Failed",
    });
    for policy in input.extraction_policies {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            policy.enrich_output(
                &mut details,
                &mut render_kpi,
                &solution.produce,
                input.demands,
                input.materials,
                input.machines,
                input.generated_plans,
                input.iteration_records.len() as u64,
                termination_reason,
                final_milp_status,
                input.pricing_statistics.as_ref(),
            );
        }));
    }
    solution.kpi.details = details;
    solution.render.kpi = render_kpi;
    solution.render.cutting_plans = solution
        .produce
        .cutting_plans
        .iter()
        .map(|usage| render_cutting_plan(&usage.plan, usage.amount))
        .collect();
    solution
}

fn insert_domain_result_details<V: SolveValue>(
    solution: &Csp1dSolution<V>,
    details: &mut std::collections::BTreeMap<String, String>,
    render_kpi: &mut std::collections::BTreeMap<String, String>,
) {
    if let Some(yield_result) = &solution.yield_result {
        for under in &yield_result.analysis.under_productions {
            insert_detail(
                details,
                render_kpi,
                &Csp1dKpiKeys::underProduction(
                    &under.demand.product.id,
                    &crate::domain::material::shadow_price_unit_symbol(&under.shortfall.unit),
                ),
                format!("{:?}", under.shortfall.value),
            );
        }
        for over in &yield_result.analysis.over_productions {
            insert_detail(
                details,
                render_kpi,
                &Csp1dKpiKeys::overProduction(
                    &over.demand.product.id,
                    &crate::domain::material::shadow_price_unit_symbol(&over.surplus.unit),
                ),
                format!("{:?}", over.surplus.value),
            );
        }
    }
    if let Some(waste_result) = &solution.waste_result {
        if let Some(value) = &waste_result.total_trim_width {
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::TotalTrimWidth,
                format!("{:?}", value),
            );
        }
        if let Some(value) = &waste_result.total_rest_material {
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::TotalRestMaterial,
                format!("{:?}", value),
            );
        }
        if let Some(value) = &waste_result.over_production_area {
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::OverProductionArea,
                format!("{:?}", value),
            );
        }
        insert_detail(
            details,
            render_kpi,
            Csp1dKpiKeys::OverProductionAreaMeasure,
            format!("{:?}", waste_result.over_production_area_measure),
        );
        insert_detail(
            details,
            render_kpi,
            Csp1dKpiKeys::RestMaterialMeasure,
            format!("{:?}", waste_result.rest_material_measure),
        );
        for cost in &waste_result.material_costs {
            insert_detail(
                details,
                render_kpi,
                &Csp1dKpiKeys::materialCost(&cost.material_id),
                format!("{:?}", cost.cost),
            );
        }
    }
    if let Some(length_result) = &solution.length_result {
        for assignment in &length_result.assignments {
            insert_detail(
                details,
                render_kpi,
                &Csp1dKpiKeys::assignedLength(&assignment.product.id),
                format!("{:?}", assignment.assigned_length.value),
            );
        }
        for record in &length_result.over_length_records {
            insert_detail(
                details,
                render_kpi,
                &Csp1dKpiKeys::overLength(&record.product.id),
                format!("{:?}", record.over_length.value),
            );
        }
    }
}

fn insert_detail(
    details: &mut std::collections::BTreeMap<String, String>,
    render_kpi: &mut std::collections::BTreeMap<String, String>,
    key: &str,
    value: String,
) {
    details.insert(key.to_string(), value.clone());
    render_kpi.insert(key.to_string(), value);
}

enum GenerationKpiPrefix {
    Initial,
    Pricing,
}

fn insert_generation_statistics(
    details: &mut std::collections::BTreeMap<String, String>,
    render_kpi: &mut std::collections::BTreeMap<String, String>,
    statistics: &CuttingPlanGenerationStatistics,
    prefix: GenerationKpiPrefix,
) {
    match prefix {
        GenerationKpiPrefix::Initial => {
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::InitialGenerationVisitedNodes,
                statistics.visited_nodes.to_string(),
            );
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::InitialGenerationGeneratedCandidates,
                statistics.generated_candidates.to_string(),
            );
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::InitialGenerationAcceptedPlans,
                statistics.accepted_plans.to_string(),
            );
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::InitialGenerationInfeasibleCandidates,
                statistics.infeasible_candidates.to_string(),
            );
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::InitialGenerationDuplicateCandidates,
                statistics.duplicate_candidates.to_string(),
            );
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::InitialGenerationDominatedCandidates,
                statistics.dominated_candidates.to_string(),
            );
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::InitialGenerationWidthBoundPrunedNodes,
                statistics.width_bound_pruned_nodes.to_string(),
            );
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::InitialGenerationKnifeBoundPrunedNodes,
                statistics.knife_bound_pruned_nodes.to_string(),
            );
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::InitialGenerationLengthBoundPrunedEntries,
                statistics.length_bound_pruned_entries.to_string(),
            );
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::InitialGenerationMaterialWidthIndexCacheHits,
                statistics.material_width_index_cache_hits.to_string(),
            );
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::InitialGenerationMaterialSliceTemplateCacheHits,
                statistics.material_slice_template_cache_hits.to_string(),
            );
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::InitialGenerationQuantityCacheHits,
                statistics.quantity_cache_hits.to_string(),
            );
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::InitialGenerationQuantityCacheMisses,
                statistics.quantity_cache_misses.to_string(),
            );
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::InitialGenerationMaterialSliceTemplateCacheMisses,
                statistics.material_slice_template_cache_misses.to_string(),
            );
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::InitialGenerationCrossWorkerDuplicateCandidates,
                statistics.cross_worker_duplicate_candidates.to_string(),
            );
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::InitialGenerationCrossContributionDominated,
                statistics.cross_contribution_dominated.to_string(),
            );
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::InitialGenerationElapsedMilliseconds,
                statistics.elapsed_milliseconds.to_string(),
            );
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::InitialGenerationStopReason,
                statistics.stop_reason.stable_name().to_string(),
            );
            render_kpi.insert(
                Csp1dKpiKeys::InitialVisitedNodes.to_string(),
                statistics.visited_nodes.to_string(),
            );
            render_kpi.insert(
                Csp1dKpiKeys::InitialGeneratedCandidates.to_string(),
                statistics.generated_candidates.to_string(),
            );
            render_kpi.insert(
                Csp1dKpiKeys::InitialAcceptedPlans.to_string(),
                statistics.accepted_plans.to_string(),
            );
            render_kpi.insert(
                Csp1dKpiKeys::InitialInfeasibleCandidates.to_string(),
                statistics.infeasible_candidates.to_string(),
            );
            render_kpi.insert(
                Csp1dKpiKeys::InitialDuplicateCandidates.to_string(),
                statistics.duplicate_candidates.to_string(),
            );
            render_kpi.insert(
                Csp1dKpiKeys::InitialDominatedCandidates.to_string(),
                statistics.dominated_candidates.to_string(),
            );
            render_kpi.insert(
                Csp1dKpiKeys::InitialWidthBoundPrunedNodes.to_string(),
                statistics.width_bound_pruned_nodes.to_string(),
            );
            render_kpi.insert(
                Csp1dKpiKeys::InitialKnifeBoundPrunedNodes.to_string(),
                statistics.knife_bound_pruned_nodes.to_string(),
            );
            render_kpi.insert(
                Csp1dKpiKeys::InitialLengthBoundPrunedEntries.to_string(),
                statistics.length_bound_pruned_entries.to_string(),
            );
            render_kpi.insert(
                Csp1dKpiKeys::InitialMaterialWidthIndexCacheHits.to_string(),
                statistics.material_width_index_cache_hits.to_string(),
            );
            render_kpi.insert(
                Csp1dKpiKeys::InitialMaterialSliceTemplateCacheHits.to_string(),
                statistics.material_slice_template_cache_hits.to_string(),
            );
            render_kpi.insert(
                Csp1dKpiKeys::InitialGenerationElapsedMillisecondsRender.to_string(),
                statistics.elapsed_milliseconds.to_string(),
            );
            render_kpi.insert(
                Csp1dKpiKeys::InitialGenerationStopReasonRender.to_string(),
                statistics.stop_reason.stable_name().to_string(),
            );
        }
        GenerationKpiPrefix::Pricing => {
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::PricingVisitedNodes,
                statistics.visited_nodes.to_string(),
            );
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::PricingGeneratedCandidates,
                statistics.generated_candidates.to_string(),
            );
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::PricingAcceptedPlans,
                statistics.accepted_plans.to_string(),
            );
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::PricingInfeasibleCandidates,
                statistics.infeasible_candidates.to_string(),
            );
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::PricingDuplicateCandidates,
                statistics.duplicate_candidates.to_string(),
            );
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::PricingDominatedCandidates,
                statistics.dominated_candidates.to_string(),
            );
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::PricingElapsedMilliseconds,
                statistics.elapsed_milliseconds.to_string(),
            );
            insert_detail(
                details,
                render_kpi,
                Csp1dKpiKeys::PricingStopReason,
                statistics.stop_reason.stable_name().to_string(),
            );
        }
    }
}
