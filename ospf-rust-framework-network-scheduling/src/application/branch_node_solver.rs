//! 单分支节点列生成求解 / Single-branch-node column generation.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_core::solver::{
    LinearSolver, ProblemStatus, SolveOptions, SolveReport, TerminationReason,
    linear_model_fingerprint, require_infeasibility_certificate_for_model,
    require_optimal_lp_certificate_for_linear_model, require_optimal_lp_certificate_for_model,
};
use ospf_rust_quantities::unit::UnitConversionValue;

use crate::domain::route_compilation::{RouteCompilationContext, RouteCompilationExtension};
use crate::domain::route_generation::{
    DefaultLabelDominancePolicy, DefaultPricingColumnSelector, EspprcPricer, InitialRouteGenerator,
    PricingRequest, RouteGraphBuilder,
};
use crate::domain::vrp::{
    ArcCostCalculator, ArcFeasibilityPolicy, BranchMask, DistanceCalculator, Route,
    RouteCostPolicy, RouteValidationPolicy, RouteValidator, TravelTimeCalculator, VehicleTypeId,
    VrptwInstance,
};
use crate::error::{NetworkSchedulingError, Result};

use super::{
    BranchNode, BranchNodeSolveContext, BranchNodeSolveResult, BranchNodeSolveStatus,
    LinearProgrammingRequest, LinearProgrammingResult,
};

/// 单节点 provider 配置 / Single-node provider configuration.
pub struct BranchNodeSolverConfig<
    V,
    D,
    T,
    A,
    C,
    L = DefaultLabelDominancePolicy,
    S = DefaultPricingColumnSelector,
> where
    V: SolveValue + UnitConversionValue,
{
    /// 距离计算策略 / Distance calculator.
    pub distance_calculator: D,
    /// 行驶时间计算策略 / Travel-time calculator.
    pub travel_time_calculator: T,
    /// 弧成本计算策略 / Arc-cost calculator.
    pub arc_cost_calculator: A,
    /// 路线成本策略 / Route-cost policy.
    pub route_cost_policy: C,
    /// 静态弧可行性策略 / Static arc-feasibility policy.
    pub arc_feasibility_policy: Arc<dyn ArcFeasibilityPolicy<V>>,
    /// 标签支配策略 / Label-dominance policy.
    pub dominance_policy: L,
    /// 负列选择策略 / Negative-column selector.
    pub column_selector: S,
    /// 额外路线编译扩展 / Extra route-compilation extensions.
    pub extensions: Vec<Arc<dyn RouteCompilationExtension<V>>>,
}

/// 可注入 LP 求解端口 / Injectable LP-solving port.
pub trait LinearProgrammingSolver: Send + Sync {
    /// 求解当前 RMP 的 LP / Solve the current RMP as an LP.
    fn solve_lp(&self, request: &LinearProgrammingRequest<'_>) -> Result<LinearProgrammingResult>;
}

/// 将 core 线性求解器适配到列生成 LP 端口 / Adapt a core linear solver to the column-generation LP port.
impl<S> LinearProgrammingSolver for S
where
    S: LinearSolver,
{
    fn solve_lp(&self, request: &LinearProgrammingRequest<'_>) -> Result<LinearProgrammingResult> {
        if request
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return Ok(LinearProgrammingResult::time_limit());
        }
        let linear_model = request
            .model
            .try_to_linear_triad_model()
            .map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
        let options = SolveOptions::new().with_cancellation_handle(request.cancellation.as_ref());
        let report = self
            .solve_linear_report_with_options(&linear_model, &options)
            .map_err(|error| NetworkSchedulingError::solver(error.to_string()))?;
        let status = match (report.problem_status, report.termination_reason) {
            (ProblemStatus::Feasible, TerminationReason::Completed) if report.is_optimal() => {
                crate::application::LinearProgrammingStatus::Optimal
            }
            (ProblemStatus::Feasible, TerminationReason::Completed)
            | (ProblemStatus::Feasible, TerminationReason::IterationLimit)
            | (ProblemStatus::Feasible, TerminationReason::NodeLimit)
            | (ProblemStatus::Feasible, TerminationReason::TotalNodeLimit)
            | (ProblemStatus::Feasible, TerminationReason::StallNodeLimit)
            | (ProblemStatus::Feasible, TerminationReason::SolutionLimit)
            | (ProblemStatus::Feasible, TerminationReason::BestSolutionLimit)
            | (ProblemStatus::Feasible, TerminationReason::GapLimit)
            | (ProblemStatus::Feasible, TerminationReason::MemoryLimit)
            | (ProblemStatus::Feasible, TerminationReason::WorkLimit)
            | (ProblemStatus::Feasible, TerminationReason::ObjectiveLimit)
            | (ProblemStatus::Feasible, TerminationReason::RestartLimit)
            | (ProblemStatus::Feasible, TerminationReason::Suboptimal) => {
                crate::application::LinearProgrammingStatus::SolverStopped
            }
            (ProblemStatus::Infeasible, TerminationReason::Completed) => {
                crate::application::LinearProgrammingStatus::Infeasible
            }
            (ProblemStatus::Unknown, TerminationReason::TimeLimit) => {
                crate::application::LinearProgrammingStatus::TimeLimit
            }
            (_, TerminationReason::Cancelled | TerminationReason::Interrupted) => {
                crate::application::LinearProgrammingStatus::SolverStopped
            }
            (_, TerminationReason::TimeLimit) => {
                crate::application::LinearProgrammingStatus::TimeLimit
            }
            (
                _,
                TerminationReason::NodeLimit
                | TerminationReason::TotalNodeLimit
                | TerminationReason::StallNodeLimit
                | TerminationReason::SolutionLimit
                | TerminationReason::BestSolutionLimit
                | TerminationReason::GapLimit
                | TerminationReason::MemoryLimit
                | TerminationReason::WorkLimit
                | TerminationReason::ObjectiveLimit
                | TerminationReason::RestartLimit
                | TerminationReason::Suboptimal,
            ) => crate::application::LinearProgrammingStatus::SolverStopped,
            (problem_status, termination_reason) => {
                return Err(NetworkSchedulingError::solver(format!(
                    "LP 报告无法用于列生成：{:?}/{:?} / LP report cannot be consumed by column generation: {:?}/{:?}",
                    problem_status, termination_reason, problem_status, termination_reason
                )));
            }
        };
        if status == crate::application::LinearProgrammingStatus::Infeasible {
            let model_fingerprint = linear_model_fingerprint(&linear_model).map_err(|error| {
                NetworkSchedulingError::solver(format!(
                    "LP 模型指纹生成失败：{} / failed to fingerprint LP model: {}",
                    error, error
                ))
            })?;
            require_infeasibility_certificate_for_model(&report, &model_fingerprint).map_err(|error| {
                    NetworkSchedulingError::solver(format!(
                        "LP 不可行报告缺少匹配模型的已验证证书：{} / LP infeasibility report lacks a verified certificate matching the model: {}",
                        error, error
                    ))
                })?;
        }
        let dual_solution = if status == crate::application::LinearProgrammingStatus::Optimal {
            let certificate = require_optimal_lp_certificate_for_linear_model(&report, &linear_model)
                .map_err(|error| {
                    NetworkSchedulingError::solver(format!(
                        "LP 已报告最优但未通过模型绑定的对偶证书复核：{} / LP reported optimality but failed the model-bound dual certificate gate: {}",
                        error, error
                    ))
                })?;
            let raw_dual_solution = certificate.dual;
            let dual_solution = normalize_core_dual_solution(request.model, raw_dual_solution)?;
            if contains_unreliable_dual(&dual_solution) {
                return Err(NetworkSchedulingError::solver(
                    "归一化 LP 对偶证书包含无效值，不能继续定价 / normalized LP dual certificate contains unreliable values; pricing cannot continue",
                ));
            }
            Some(dual_solution)
        } else {
            None
        };
        let solution = report
            .solution
            .as_ref()
            .map(|solution| solution.values.clone());
        let objective = report
            .solution
            .as_ref()
            .and_then(|solution| solution.objective_value.or(solution.objective));
        Ok(LinearProgrammingResult {
            status,
            objective,
            solution,
            dual_solution,
            report: Some(report),
        })
    }
}

const MAX_RELIABLE_DUAL_ABS: f64 = 1e90;

/// 判断 LP 对偶是否可用于定价 / Check whether LP duals are reliable for pricing.
///
/// 部分后端在当前解阶段会用超大有限值表示无效对偶；这些值不能仅通过
/// `is_finite` 过滤，否则会污染 reduced cost 和节点下界。
/// Some backends use very large finite values for unavailable duals in the current
/// solution stage; checking only `is_finite` would contaminate reduced costs and node bounds.
fn contains_unreliable_dual(duals: &[f64]) -> bool {
    duals
        .iter()
        .any(|value| !value.is_finite() || value.abs() >= MAX_RELIABLE_DUAL_ABS)
}

/// 折叠 core 线性模型展开的约束对偶 / Collapse duals from core's expanded linear rows.
///
/// `MetaModel` 等式会在 `LinearTriadModel` 中拆为正向和反向两条 `<=` 行；该函数恢复
/// `LinearProgrammingResult` 约定的原模型约束顺序和符号。
/// An equality in `MetaModel` becomes forward and reverse `<=` rows in
/// `LinearTriadModel`; this restores the original order and sign convention required by
/// `LinearProgrammingResult`.
fn normalize_core_dual_solution(
    model: &MetaModel<f64>,
    expanded_duals: &[f64],
) -> Result<Vec<f64>> {
    let mut expanded_index = 0usize;
    let mut duals = Vec::with_capacity(model.constraints().len());
    for constraint in model.constraints() {
        let first = expanded_duals.get(expanded_index).copied().ok_or_else(|| {
            NetworkSchedulingError::solver(format!(
                "对偶解长度不足：第 {} 个展开约束 / dual solution is shorter than expanded row {}",
                expanded_index, expanded_index
            ))
        })?;
        expanded_index += 1;
        let normalized = match constraint.inequality.relation {
            ConstraintRelation::LessEqual => first,
            ConstraintRelation::GreaterEqual => -first,
            ConstraintRelation::Equal => {
                let second = expanded_duals.get(expanded_index).copied().ok_or_else(|| {
                    NetworkSchedulingError::solver(format!(
                        "等式对偶缺少反向展开行：第 {} 个展开约束 / equality dual is missing reverse expanded row {}",
                        expanded_index, expanded_index
                    ))
                })?;
                expanded_index += 1;
                first - second
            }
        };
        duals.push(normalized);
    }
    if expanded_index != expanded_duals.len() {
        return Err(NetworkSchedulingError::solver(format!(
            "对偶解包含多余展开约束：已消费 {} 行，实际 {} 行 / dual solution has extra expanded rows: consumed {}, received {}",
            expanded_index,
            expanded_duals.len(),
            expanded_index,
            expanded_duals.len()
        )));
    }
    Ok(duals)
}

/// 真实单节点 Branch-and-Price provider / Real single-node Branch-and-Price provider.
pub struct BranchNodeSolver<
    V,
    LpSolver,
    D,
    T,
    A,
    C,
    L = DefaultLabelDominancePolicy,
    S = DefaultPricingColumnSelector,
> where
    V: SolveValue + UnitConversionValue,
{
    /// VRPTW 实例 / VRPTW instance.
    pub instance: Arc<VrptwInstance<V>>,
    /// LP 求解器 / LP solver.
    pub solver: Arc<LpSolver>,
    /// 单节点 provider 配置 / Single-node provider configuration.
    pub config: BranchNodeSolverConfig<V, D, T, A, C, L, S>,
}

impl<V, LpSolver, D, T, A, C, L, S> BranchNodeSolver<V, LpSolver, D, T, A, C, L, S>
where
    V: SolveValue + UnitConversionValue,
    LpSolver: LinearProgrammingSolver,
    D: DistanceCalculator<V> + Clone,
    T: TravelTimeCalculator<V> + Clone,
    A: ArcCostCalculator<V> + Clone,
    C: RouteCostPolicy<V> + Clone,
    L: crate::domain::route_generation::LabelDominancePolicy + Clone,
    S: crate::domain::route_generation::PricingColumnSelector<V> + Clone,
{
    /// 创建单节点 provider / Create a single-node provider.
    pub fn new(
        instance: Arc<VrptwInstance<V>>,
        solver: Arc<LpSolver>,
        config: BranchNodeSolverConfig<V, D, T, A, C, L, S>,
    ) -> Result<Self> {
        Ok(Self {
            instance,
            solver,
            config,
        })
    }

    fn solve_phase(
        &self,
        node: &BranchNode,
        context: &BranchNodeSolveContext,
        compilation: &mut RouteCompilationContext<V>,
        model: &mut MetaModel<f64>,
        phase: crate::domain::vrp::PricingPhase,
    ) -> Result<PhaseResult> {
        let mut pricing_calls = 0usize;
        let mut generated_columns = 0usize;
        for iteration in 1..=context.max_cg_iterations {
            if let Some(status) = interruption_status(context) {
                return Ok(PhaseResult::interrupted(
                    status,
                    iteration - 1,
                    pricing_calls,
                    generated_columns,
                ));
            }
            let request = LinearProgrammingRequest {
                node_id: node.id,
                phase,
                iteration,
                model,
                deadline: context.deadline,
                cancellation: context.cancellation.as_ref().map(|token| token.handle()),
            };
            let lp = self.solver.solve_lp(&request)?;
            match lp.status {
                crate::application::LinearProgrammingStatus::Optimal => {
                    validate_lp_result(model, &lp, node.id, phase, iteration)?;
                    let duals = verified_lp_duals(model, &lp, node.id, phase, iteration)?;
                    model.set_solution(lp.solution.as_ref().ok_or_else(|| {
                        NetworkSchedulingError::solver(format!(
                            "节点 {} 第 {:?} 轮 LP 缺少解向量 / node {} {:?} LP round {} has no solution",
                            node.id, phase, node.id, phase, iteration
                        ))
                    })?);
                    if phase == crate::domain::vrp::PricingPhase::PhaseOne
                        && lp.objective.unwrap_or(f64::INFINITY)
                            <= self.instance.tolerances.feasibility
                    {
                        return Ok(PhaseResult::complete(
                            lp,
                            iteration,
                            pricing_calls,
                            generated_columns,
                        ));
                    }
                    let duals = compilation.extract_pricing_duals(model, &duals)?;
                    let graph_builder = RouteGraphBuilder::new(
                        self.instance.clone(),
                        self.config.distance_calculator.clone(),
                        self.config.travel_time_calculator.clone(),
                        self.config.arc_cost_calculator.clone(),
                        self.config.route_cost_policy.clone(),
                    )
                    .with_arc_feasibility_policy(self.config.arc_feasibility_policy.clone());
                    let pricer = EspprcPricer::with_policies(
                        graph_builder,
                        self.config.dominance_policy.clone(),
                        self.config.column_selector.clone(),
                    );
                    let mut new_routes = Vec::new();
                    let mut pricing_complete = true;
                    for vehicle_type in &self.instance.vehicle_types {
                        if let Some(status) = interruption_status(context) {
                            return Ok(PhaseResult::interrupted(
                                status,
                                iteration,
                                pricing_calls,
                                generated_columns,
                            ));
                        }
                        pricing_calls += 1;
                        let request = PricingRequest {
                            instance: self.instance.clone(),
                            duals: duals.clone(),
                            branch_mask: Some(node.path.mask.clone()),
                            pricing_tolerance: self.instance.tolerances.pricing,
                            max_columns_per_pricing: context.max_columns_per_pricing,
                            vehicle_type_id: vehicle_type.id.clone(),
                            cancellation: context.cancellation.clone(),
                            deadline: context.deadline,
                            max_labels: context.max_labels,
                            max_depth: None,
                        };
                        let pricing = pricer.price_request(&request)?;
                        new_routes.extend(pricing.routes);
                        pricing_complete &= pricing.exact_pricing_complete;
                        if pricing.interrupted {
                            let status = if context
                                .deadline
                                .is_some_and(|deadline| Instant::now() >= deadline)
                            {
                                BranchNodeSolveStatus::TimeLimit
                            } else {
                                BranchNodeSolveStatus::SolverStopped
                            };
                            return Ok(PhaseResult::interrupted(
                                status,
                                iteration,
                                pricing_calls,
                                generated_columns,
                            ));
                        }
                    }
                    if !pricing_complete {
                        return Ok(PhaseResult::interrupted(
                            BranchNodeSolveStatus::SolverStopped,
                            iteration,
                            pricing_calls,
                            generated_columns,
                        ));
                    }
                    if new_routes.is_empty() {
                        return Ok(PhaseResult::complete(
                            lp,
                            iteration,
                            pricing_calls,
                            generated_columns,
                        ));
                    }
                    generated_columns +=
                        compilation.add_columns(iteration, new_routes, model)?.len();
                }
                crate::application::LinearProgrammingStatus::Infeasible => {
                    return Ok(PhaseResult::infeasible(
                        verified_infeasibility_report(model, lp.report.as_ref()),
                        iteration,
                        pricing_calls,
                        generated_columns,
                    ));
                }
                crate::application::LinearProgrammingStatus::TimeLimit => {
                    let last_lp =
                        retain_interrupted_solution(model, &lp, node.id, phase, iteration)?;
                    return Ok(PhaseResult::interrupted(
                        BranchNodeSolveStatus::TimeLimit,
                        iteration,
                        pricing_calls,
                        generated_columns,
                    )
                    .with_last_lp(last_lp));
                }
                crate::application::LinearProgrammingStatus::SolverStopped => {
                    let last_lp =
                        retain_interrupted_solution(model, &lp, node.id, phase, iteration)?;
                    return Ok(PhaseResult::interrupted(
                        BranchNodeSolveStatus::SolverStopped,
                        iteration,
                        pricing_calls,
                        generated_columns,
                    )
                    .with_last_lp(last_lp));
                }
            }
        }
        Err(NetworkSchedulingError::pricing(format!(
            "节点 {} {:?} 定价在 {} 轮内未收敛 / node {} {:?} pricing did not converge within {} iterations",
            node.id, phase, context.max_cg_iterations, node.id, phase, context.max_cg_iterations
        )))
    }
}

impl<V, LpSolver, D, T, A, C, L, S> super::BranchNodeSolverProvider<V>
    for BranchNodeSolver<V, LpSolver, D, T, A, C, L, S>
where
    V: SolveValue + UnitConversionValue,
    LpSolver: LinearProgrammingSolver,
    D: DistanceCalculator<V> + Clone,
    T: TravelTimeCalculator<V> + Clone,
    A: ArcCostCalculator<V> + Clone,
    C: RouteCostPolicy<V> + Clone,
    L: crate::domain::route_generation::LabelDominancePolicy + Clone,
    S: crate::domain::route_generation::PricingColumnSelector<V> + Clone,
{
    fn solve_node(
        &self,
        node: &BranchNode,
        inherited_columns: &[Route<V>],
        context: &BranchNodeSolveContext,
    ) -> Result<BranchNodeSolveResult<V>> {
        let mut model = MetaModel::<f64>::new(&format!("node_{}", node.id));
        let mut compilation = RouteCompilationContext::with_extensions(
            self.instance.clone(),
            self.config.extensions.clone(),
        );
        compilation.register(&mut model)?;

        let inherited_columns = inherited_columns
            .iter()
            .filter(|route| route_matches_branch(route, &node.path.mask))
            .cloned()
            .collect::<Vec<_>>();
        let generator = InitialRouteGenerator::new(
            self.instance.clone(),
            self.config.distance_calculator.clone(),
            self.config.travel_time_calculator.clone(),
            self.config.arc_cost_calculator.clone(),
            self.config.route_cost_policy.clone(),
        )
        .with_arc_feasibility_policy(self.config.arc_feasibility_policy.clone());
        let mut initial_routes = inherited_columns;
        initial_routes.extend(generator.generate(Some(&node.path.mask))?);
        let initial_routes = compilation.add_columns(0, initial_routes, &mut model)?;

        let phase_one = self.solve_phase(
            node,
            context,
            &mut compilation,
            &mut model,
            crate::domain::vrp::PricingPhase::PhaseOne,
        )?;
        let mut iterations = phase_one.iterations;
        let mut generated_columns = initial_routes.len();
        generated_columns += phase_one.generated_columns;
        match phase_one.status {
            PhaseStatus::Infeasible => {
                return node_result(
                    BranchNodeSolveStatus::Infeasible,
                    true,
                    false,
                    f64::INFINITY,
                    f64::INFINITY,
                    &compilation,
                    &model,
                    iterations,
                    0,
                    generated_columns,
                    false,
                    phase_one.infeasibility_report,
                );
            }
            PhaseStatus::Interrupted(status) => {
                return node_result(
                    status,
                    false,
                    true,
                    node.inherited_lower_bound,
                    phase_one.objective(),
                    &compilation,
                    &model,
                    iterations,
                    0,
                    generated_columns,
                    true,
                    None,
                );
            }
            PhaseStatus::Complete => {}
        }
        if !compilation.is_phase_one_converged(&model, self.instance.tolerances.feasibility)? {
            return node_result(
                BranchNodeSolveStatus::Infeasible,
                true,
                false,
                f64::INFINITY,
                phase_one.objective(),
                &compilation,
                &model,
                iterations,
                0,
                generated_columns,
                false,
                None,
            );
        }
        compilation.switch_to_phase_two(&mut model)?;
        let phase_two = self.solve_phase(
            node,
            context,
            &mut compilation,
            &mut model,
            crate::domain::vrp::PricingPhase::PhaseTwo,
        )?;
        iterations += phase_two.iterations;
        generated_columns += phase_two.generated_columns;
        match phase_two.status {
            PhaseStatus::Infeasible => {
                return node_result(
                    BranchNodeSolveStatus::Infeasible,
                    true,
                    false,
                    f64::INFINITY,
                    f64::INFINITY,
                    &compilation,
                    &model,
                    iterations,
                    phase_one.pricing_calls + phase_two.pricing_calls,
                    generated_columns,
                    false,
                    phase_two.infeasibility_report,
                );
            }
            PhaseStatus::Interrupted(status) => {
                return node_result(
                    status,
                    false,
                    true,
                    node.inherited_lower_bound,
                    phase_two.objective(),
                    &compilation,
                    &model,
                    iterations,
                    phase_one.pricing_calls + phase_two.pricing_calls,
                    generated_columns,
                    true,
                    None,
                );
            }
            PhaseStatus::Complete => {}
        }
        let objective = phase_two.objective();
        if !objective.is_finite() {
            return Err(NetworkSchedulingError::contract(format!(
                "节点 {} 最终 LP 目标不是有限值 / node {} final LP objective is not finite",
                node.id, node.id
            )));
        }
        let mut route_values = compilation.extract_route_values(&model)?;
        let is_integer = !route_values.is_empty()
            && route_values.iter().all(|(_, value)| {
                value.is_finite()
                    && (*value - value.round()).abs() <= self.instance.tolerances.integrality
            });
        if is_integer {
            // 最终整数 RMP 才触发扩展 enrich，并按原列签名回填，避免扩展偷偷改变模型列身份。
            // Only an integral final RMP triggers extension enrichment; routes are matched by their
            // original signatures so an extension cannot silently change model-column identity.
            let enriched_routes =
                compilation.extract_solution(&model, self.instance.tolerances.integrality)?;
            let mut enriched_by_signature = BTreeMap::new();
            for route in enriched_routes {
                let signature = route.signature();
                if enriched_by_signature
                    .insert(signature.clone(), route)
                    .is_some()
                {
                    return Err(NetworkSchedulingError::contract(format!(
                        "扩展产生重复路线签名：{} / extension produced duplicate route signature: {}",
                        signature, signature
                    )));
                }
            }
            for (route, value) in &mut route_values {
                if *value < 1.0 - self.instance.tolerances.integrality {
                    continue;
                }
                let signature = route.signature();
                *route = enriched_by_signature.remove(&signature).ok_or_else(|| {
                    NetworkSchedulingError::contract(format!(
                        "扩展遗漏已选路线：{} / extension omitted selected route: {}",
                        signature, signature
                    ))
                })?;
            }
            if let Some(signature) = enriched_by_signature.keys().next() {
                return Err(NetworkSchedulingError::contract(format!(
                    "扩展返回模型外路线：{} / extension returned a route outside the model: {}",
                    signature, signature
                )));
            }
        }
        Ok(BranchNodeSolveResult {
            status: BranchNodeSolveStatus::Optimal,
            pricing_complete: true,
            is_feasible: true,
            is_integer,
            lower_bound: objective,
            lp_objective: objective,
            route_values,
            columns: compilation.aggregation.routes().to_vec(),
            iterations,
            pricing_calls: phase_one.pricing_calls + phase_two.pricing_calls,
            generated_columns,
            interrupted: false,
            solver_report: phase_two.last_lp.as_ref().and_then(|lp| lp.report.clone()),
            solver_model_fingerprint: Some(
                linear_model_fingerprint(
                    &model
                        .try_to_linear_triad_model()
                        .map_err(|error| NetworkSchedulingError::model(error.to_string()))?,
                )
                .map_err(|error| NetworkSchedulingError::solver(error.to_string()))?,
            ),
        })
    }

    fn validate_routes(
        &self,
        instance: &VrptwInstance<V>,
        branch_mask: &BranchMask<VehicleTypeId>,
        routes: &[Route<V>],
    ) -> Result<()> {
        super::validate_solution_structure(instance, branch_mask, routes)?;
        for route in routes {
            RouteValidator::validate_with_policy(
                instance,
                route,
                RouteValidationPolicy {
                    distance_calculator: &self.config.distance_calculator,
                    travel_time_calculator: &self.config.travel_time_calculator,
                    arc_cost_calculator: &self.config.arc_cost_calculator,
                    route_cost_policy: &self.config.route_cost_policy,
                    arc_feasibility_policy: self.config.arc_feasibility_policy.as_ref(),
                    branch_mask: Some(branch_mask),
                },
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct PhaseResult {
    status: PhaseStatus,
    last_lp: Option<LinearProgrammingResult>,
    iterations: usize,
    pricing_calls: usize,
    generated_columns: usize,
    infeasibility_report: Option<SolveReport<f64>>,
}

impl PhaseResult {
    fn complete(
        lp: LinearProgrammingResult,
        iterations: usize,
        pricing_calls: usize,
        generated_columns: usize,
    ) -> Self {
        Self {
            status: PhaseStatus::Complete,
            last_lp: Some(lp),
            iterations,
            pricing_calls,
            generated_columns,
            infeasibility_report: None,
        }
    }

    fn infeasible(
        infeasibility_report: Option<SolveReport<f64>>,
        iterations: usize,
        pricing_calls: usize,
        generated_columns: usize,
    ) -> Self {
        Self {
            status: PhaseStatus::Infeasible,
            last_lp: None,
            iterations,
            pricing_calls,
            generated_columns,
            infeasibility_report,
        }
    }

    fn interrupted(
        status: BranchNodeSolveStatus,
        iterations: usize,
        pricing_calls: usize,
        generated_columns: usize,
    ) -> Self {
        Self {
            status: PhaseStatus::Interrupted(status),
            last_lp: None,
            iterations,
            pricing_calls,
            generated_columns,
            infeasibility_report: None,
        }
    }

    fn with_last_lp(mut self, last_lp: Option<LinearProgrammingResult>) -> Self {
        self.last_lp = last_lp;
        self
    }

    fn objective(&self) -> f64 {
        self.last_lp
            .as_ref()
            .and_then(|lp| lp.objective)
            .unwrap_or(f64::INFINITY)
    }
}

#[derive(Debug, Clone, Copy)]
enum PhaseStatus {
    Complete,
    Infeasible,
    Interrupted(BranchNodeSolveStatus),
}

fn interruption_status(context: &BranchNodeSolveContext) -> Option<BranchNodeSolveStatus> {
    if context
        .deadline
        .is_some_and(|deadline| Instant::now() >= deadline)
    {
        return Some(BranchNodeSolveStatus::TimeLimit);
    }
    context
        .cancellation
        .as_ref()
        .is_some_and(crate::domain::route_generation::CancellationToken::is_cancelled)
        .then_some(BranchNodeSolveStatus::SolverStopped)
}

fn validate_lp_result(
    model: &MetaModel<f64>,
    result: &LinearProgrammingResult,
    node_id: usize,
    phase: crate::domain::vrp::PricingPhase,
    iteration: usize,
) -> Result<()> {
    let objective = result.objective.ok_or_else(|| {
        NetworkSchedulingError::solver(format!(
            "节点 {} {:?} 第 {} 轮 LP 缺少目标值 / node {} {:?} LP round {} has no objective",
            node_id, phase, iteration, node_id, phase, iteration
        ))
    })?;
    if !objective.is_finite() {
        return Err(NetworkSchedulingError::solver(format!(
            "节点 {} {:?} 第 {} 轮 LP 目标不是有限值 / node {} {:?} LP round {} objective is not finite",
            node_id, phase, iteration, node_id, phase, iteration
        )));
    }
    let solution = result.solution.as_ref().ok_or_else(|| {
        NetworkSchedulingError::solver(format!(
            "节点 {} {:?} 第 {} 轮 LP 缺少解向量 / node {} {:?} LP round {} has no solution",
            node_id, phase, iteration, node_id, phase, iteration
        ))
    })?;
    if solution.len() < model.tokens_in_solver_order().len() {
        return Err(NetworkSchedulingError::solver(format!(
            "节点 {} {:?} 第 {} 轮 LP 解向量长度不足 / node {} {:?} LP round {} solution is too short",
            node_id, phase, iteration, node_id, phase, iteration
        )));
    }
    if solution.iter().any(|value| !value.is_finite()) {
        return Err(NetworkSchedulingError::solver(format!(
            "节点 {} {:?} 第 {} 轮 LP 解向量包含非有限值 / node {} {:?} LP round {} solution contains a non-finite value",
            node_id, phase, iteration, node_id, phase, iteration
        )));
    }
    Ok(())
}

fn verified_lp_duals(
    model: &MetaModel<f64>,
    result: &LinearProgrammingResult,
    node_id: usize,
    phase: crate::domain::vrp::PricingPhase,
    iteration: usize,
) -> Result<Vec<f64>> {
    let report = result.report.as_ref().ok_or_else(|| {
        NetworkSchedulingError::solver(format!(
            "节点 {} {:?} 第 {} 轮 LP 缺少统一报告，不能继续精确定价 / node {} {:?} LP round {} has no unified report; exact pricing cannot continue",
            node_id, phase, iteration, node_id, phase, iteration
        ))
    })?;
    let linear_model = model
        .try_to_linear_triad_model()
        .map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
    let model_fingerprint = linear_model_fingerprint(&linear_model)
        .map_err(|error| NetworkSchedulingError::solver(error.to_string()))?;
    let certificate = require_optimal_lp_certificate_for_model(report, &model_fingerprint)
        .map_err(|error| {
            NetworkSchedulingError::solver(format!(
                "节点 {} {:?} 第 {} 轮 LP 缺少匹配模型的已验证最优证书：{} / node {} {:?} LP round {} lacks a verified optimal certificate for the matching model: {}",
                node_id, phase, iteration, error, node_id, phase, iteration, error
            ))
        })?;
    let normalized_certificate_duals = normalize_core_dual_solution(model, certificate.dual)?;
    let report_solution = report.solution.as_ref().ok_or_else(|| {
        NetworkSchedulingError::solver(format!(
            "节点 {} {:?} 第 {} 轮 LP 报告缺少解 / node {} {:?} LP round {} report has no solution",
            node_id, phase, iteration, node_id, phase, iteration
        ))
    })?;
    let report_objective = report_solution
        .objective_value
        .or(report_solution.objective)
        .ok_or_else(|| {
            NetworkSchedulingError::solver(format!(
                "节点 {} {:?} 第 {} 轮 LP 报告缺少目标值 / node {} {:?} LP round {} report has no objective",
                node_id, phase, iteration, node_id, phase, iteration
            ))
        })?;
    let objective = result.objective.ok_or_else(|| {
        NetworkSchedulingError::solver(format!(
            "节点 {} {:?} 第 {} 轮 LP 缺少目标值 / node {} {:?} LP round {} has no objective",
            node_id, phase, iteration, node_id, phase, iteration
        ))
    })?;
    if (report_objective - objective).abs()
        > 1e-8 * report_objective.abs().max(objective.abs()).max(1.0)
    {
        return Err(NetworkSchedulingError::solver(format!(
            "节点 {} {:?} 第 {} 轮 LP 报告目标与结果不一致 / node {} {:?} LP round {} report objective disagrees with result",
            node_id, phase, iteration, node_id, phase, iteration
        )));
    }
    let result_solution = result.solution.as_ref().ok_or_else(|| {
        NetworkSchedulingError::solver(format!(
            "节点 {} {:?} 第 {} 轮 LP 缺少解向量 / node {} {:?} LP round {} has no solution",
            node_id, phase, iteration, node_id, phase, iteration
        ))
    })?;
    if result_solution.len() < report_solution.values.len()
        || report_solution
            .values
            .iter()
            .zip(result_solution)
            .any(|(report, result)| {
                (report - result).abs() > 1e-8 * report.abs().max(result.abs()).max(1.0)
            })
    {
        return Err(NetworkSchedulingError::solver(format!(
            "节点 {} {:?} 第 {} 轮 LP 报告解与结果不一致 / node {} {:?} LP round {} report solution disagrees with result",
            node_id, phase, iteration, node_id, phase, iteration
        )));
    }
    if let Some(result_duals) = &result.dual_solution
        && (result_duals.len() != normalized_certificate_duals.len()
            || result_duals
                .iter()
                .zip(&normalized_certificate_duals)
                .any(|(report, result)| {
                    (report - result).abs() > 1e-8 * report.abs().max(result.abs()).max(1.0)
                }))
    {
        return Err(NetworkSchedulingError::solver(format!(
            "节点 {} {:?} 第 {} 轮 LP 对偶与报告不一致 / node {} {:?} LP round {} dual solution disagrees with report",
            node_id, phase, iteration, node_id, phase, iteration
        )));
    }
    Ok(normalized_certificate_duals)
}

fn retain_interrupted_solution(
    model: &mut MetaModel<f64>,
    result: &LinearProgrammingResult,
    node_id: usize,
    phase: crate::domain::vrp::PricingPhase,
    iteration: usize,
) -> Result<Option<LinearProgrammingResult>> {
    match (&result.objective, &result.solution) {
        (None, None) => Ok(None),
        (Some(_), Some(_)) => {
            validate_lp_result(model, result, node_id, phase, iteration)?;
            model.set_solution(result.solution.as_ref().ok_or_else(|| {
                NetworkSchedulingError::solver(format!(
                    "节点 {} {:?} 第 {} 轮中断 LP 缺少解向量 / node {} {:?} interrupted LP round {} has no solution",
                    node_id, phase, iteration, node_id, phase, iteration
                ))
            })?);
            Ok(Some(result.clone()))
        }
        _ => Err(NetworkSchedulingError::solver(format!(
            "节点 {} {:?} 第 {} 轮中断 LP 的目标值和解向量必须同时存在或同时缺失 / node {} {:?} interrupted LP round {} must provide both objective and solution or neither",
            node_id, phase, iteration, node_id, phase, iteration
        ))),
    }
}

fn route_matches_branch<V>(route: &Route<V>, branch_mask: &BranchMask<VehicleTypeId>) -> bool
where
    V: SolveValue + UnitConversionValue,
{
    branch_mask.is_route_compatible(
        &route.vehicle_type_id,
        &route
            .stops
            .iter()
            .map(|stop| stop.node_id.clone())
            .collect::<Vec<_>>(),
        &route.effective_arc_ids(),
    )
}

#[allow(clippy::too_many_arguments)]
fn node_result<V>(
    status: BranchNodeSolveStatus,
    pricing_complete: bool,
    is_feasible: bool,
    lower_bound: f64,
    lp_objective: f64,
    compilation: &crate::domain::route_compilation::RouteCompilationContext<V>,
    model: &MetaModel<f64>,
    iterations: usize,
    pricing_calls: usize,
    generated_columns: usize,
    interrupted: bool,
    solver_report: Option<SolveReport<f64>>,
) -> Result<BranchNodeSolveResult<V>>
where
    V: SolveValue + UnitConversionValue,
{
    let route_values = if model.has_solution() {
        compilation.extract_route_values(model)?
    } else {
        Vec::new()
    };
    let has_integer_candidate = !route_values.is_empty()
        && route_values
            .iter()
            .all(|(_, value)| value.is_finite() && (*value - value.round()).abs() <= 1e-7)
        && route_values.iter().any(|(_, value)| *value >= 1.0 - 1e-7);
    let candidate_is_feasible = interrupted && has_integer_candidate;
    let solver_model_fingerprint = linear_model_fingerprint(
        &model
            .try_to_linear_triad_model()
            .map_err(|error| NetworkSchedulingError::model(error.to_string()))?,
    )
    .map_err(|error| NetworkSchedulingError::solver(error.to_string()))?;
    Ok(BranchNodeSolveResult {
        status,
        pricing_complete,
        is_feasible: is_feasible || candidate_is_feasible,
        is_integer: has_integer_candidate,
        lower_bound,
        lp_objective,
        route_values,
        columns: compilation.aggregation.routes().to_vec(),
        iterations,
        pricing_calls,
        generated_columns,
        interrupted,
        solver_report,
        solver_model_fingerprint: Some(solver_model_fingerprint),
    })
}

fn verified_infeasibility_report(
    model: &MetaModel<f64>,
    report: Option<&SolveReport<f64>>,
) -> Option<SolveReport<f64>> {
    let linear_model = model.try_to_linear_triad_model().ok()?;
    let model_fingerprint = linear_model_fingerprint(&linear_model).ok()?;
    report
        .filter(|report| {
            require_infeasibility_certificate_for_model(report, &model_fingerprint).is_ok()
        })
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::{contains_unreliable_dual, normalize_core_dual_solution};
    use ospf_rust_core::model::{ConstraintRelation, MetaModel};

    #[test]
    fn core_duals_are_collapsed_back_to_meta_model_rows() {
        let mut model = MetaModel::<f64>::new("dual-normalization");
        model
            .add_linear_constraint(&[], ConstraintRelation::Equal, 0.0, "equal")
            .expect("register equal constraint");
        model
            .add_linear_constraint(&[], ConstraintRelation::LessEqual, 0.0, "less")
            .expect("register less-equal constraint");
        model
            .add_linear_constraint(&[], ConstraintRelation::GreaterEqual, 0.0, "greater")
            .expect("register greater-equal constraint");

        let duals = normalize_core_dual_solution(&model, &[2.0, -3.0, -4.0, -5.0])
            .expect("normalize core duals");
        assert_eq!(duals, vec![5.0, -4.0, 5.0]);
    }

    #[test]
    fn backend_sentinel_duals_are_rejected_before_pricing() {
        assert!(!contains_unreliable_dual(&[0.0, -4.5, 1e12]));
        assert!(contains_unreliable_dual(&[0.0, 1e99]));
        assert!(contains_unreliable_dual(&[f64::NAN]));
        assert!(contains_unreliable_dual(&[f64::INFINITY]));
    }
}
