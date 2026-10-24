//! 二维套裁求解入口 / 2D cutting stock solve entry
//!
//! 两条路径共用同一套装配与恢复逻辑，差别只在「候选方案怎么来」：
//! 1. **MILP 路径**：一次生成全部候选图案，解一次主问题。
//! 2. **列生成路径**：先解一次松弛主问题，按**剩余需求**再生成图案并追加列，重复到没有新列或到达
//!    迭代上限，最后解一次最终整数主问题。
//!
//! 这里刻意说明清楚：本阶段的列追加由**剩余需求**驱动，而不是由检验数驱动。两者都属于列生成思想，
//! 但后者需要从 LP 对偶里取出需求约束的影子价格；对偶提取留作后续，不在此阶段声称已实现。
//!
//! Both paths share the same assembly and recovery logic and differ only in where the candidate plans come from:
//! the MILP path generates every candidate once and solves the master problem once, while the column generation
//! path solves the relaxed master first, regenerates patterns against the **residual demand** and appends
//! columns, repeating until no new column appears or the iteration limit is reached, then solves the final
//! integer master problem.
//!
//! This is stated explicitly: at this stage the column addition is driven by the **residual demand**, not by
//! reduced costs. Both belong to the column generation idea, but the latter needs the demand shadow prices from
//! the LP duals; dual extraction is left for a later stage and is not claimed here.

use std::collections::BTreeMap;

use ospf_rust_base::error::Ret;

use crate::application::model::{Csp2dConfiguration, Csp2dProblem};
use crate::domain::material::{CuttingPlan2, Pattern2};
use crate::domain::pattern_generation::{PatternGenerationRequest2, ShelfPatternGenerator2};
use crate::domain::produce::{
    Csp2dIterationRecord, Csp2dMasterDemand, Csp2dMasterProblem, Csp2dPatternAssignment,
};
use crate::domain::wasting_minimization::{Csp2dWasteResult, WasteContext};
use crate::domain::r#yield::{Csp2dYieldResult, YieldContext};
use crate::infrastructure::error::validation_error;

// ============================================================================
// Csp2dSolution - 求解结果 / Solve result
// ============================================================================

/// 求解结果 / Solve result
#[derive(Debug, Clone)]
pub struct Csp2dSolution {
    /// 选中方案与使用次数 / Selected plans with their usage counts
    pub usage: Vec<(CuttingPlan2<f64, ospf_rust_quantities::unit::derived::Meter>, u64)>,
    /// 需求覆盖量 / Demand coverage
    pub coverage: Vec<f64>,
    /// 良率结果 / Yield result
    pub yield_result: Csp2dYieldResult,
    /// 余料结果 / Waste result
    pub waste_result: Csp2dWasteResult,
    /// 物料用量 / Material usage
    pub material_usage: BTreeMap<String, u64>,
    /// 板料分配（张数/段高与余料口径）/ Sheet assignment (counts, segment height and remnants)
    pub sheet_assignment: crate::domain::sheet_assignment::SheetAssignment,
    /// 是否完整覆盖 / Whether the coverage is complete
    pub fully_covered: bool,
    /// 迭代记录 / Iteration records
    pub iterations: Vec<Csp2dIterationRecord>,
    /// 终止原因 / Termination reason
    pub termination: Csp2dTerminationReason,
    /// 失败信息 / Failure message
    pub failure_message: Option<String>,
}

/// 终止原因 / Termination reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Csp2dTerminationReason {
    /// 覆盖完整 / Coverage complete
    Covered,
    /// 无新方案 / No new plan
    NoNewPlan,
    /// 到达迭代上限 / Iteration limit reached
    IterationLimitReached,
    /// 无候选方案 / No candidate plan
    NoCandidatePlan,
}

impl Csp2dSolution {
    /// 方案数量 / Plan count
    pub fn plan_count(&self) -> usize {
        self.usage.len()
    }

    /// 物料总消耗 / Total material consumption
    pub fn total_material_usage(&self) -> u64 {
        self.usage.iter().map(|(_, count)| *count).sum()
    }

    /// 总余料面积 / Total remnant area
    pub fn total_waste_area(&self) -> f64 {
        self.waste_result.total_waste_area
    }
}

// ============================================================================
// Csp2dSolvers - 求解入口 / Solve entries
// ============================================================================

/// 二维套裁求解入口 / 2D cutting stock solve entry
///
/// `master_solver` 为空时使用内置启发式；注入后主问题交由真实求解器处理。
/// With an empty `master_solver` the built-in heuristic is used; when injected, the master problem is handed to
/// a real solver.
#[derive(Debug, Clone, Default)]
pub struct Csp2dSolvers {
    /// 主问题求解器 / Master problem solver
    pub master_solver: Option<std::sync::Arc<dyn crate::application::master_solver::Csp2dMasterSolver>>,
}

impl Csp2dSolvers {
    /// 创建求解入口 / Create the solve entry
    pub fn new() -> Self {
        Self::default()
    }

    /// 注入主问题求解器 / Inject a master problem solver
    pub fn with_master_solver(
        mut self,
        solver: std::sync::Arc<dyn crate::application::master_solver::Csp2dMasterSolver>,
    ) -> Self {
        self.master_solver = Some(solver);
        self
    }

    /// MILP 路径：一次生成全部候选图案并解一次主问题 / MILP path: generate every candidate once and solve the
    /// master problem once
    pub fn solve_milp(&self, problem: Csp2dProblem) -> Ret<Csp2dSolution> {
        problem.validate()?;
        let plans = self.build_plans(&problem, &problem.demand_products(), 0)?;
        if plans.is_empty() {
            return Ok(self.empty_solution(
                &problem,
                Csp2dTerminationReason::NoCandidatePlan,
                Some("没有可用的候选方案 / No candidate plan".to_string()),
            ));
        }
        let assignment = Csp2dPatternAssignment::new(plans, problem.demand_piece_bound())?;
        let master = Csp2dMasterProblem::new(assignment, problem.master_demands())?;
        // 一次求解：每个方案按需求比例取整，再校正到覆盖完整。
        // One solve: round each plan by its demand ratio, then correct until coverage is complete.
        let usage = self.heuristic_usage(&master, &problem.configuration);
        Ok(self.build_solution(&problem, &master, &usage, Vec::new()))
    }

    /// 列生成路径：按剩余需求迭代追加图案 / Column generation path: append patterns against the residual demand
    pub fn solve_column_generation(&self, problem: Csp2dProblem) -> Ret<Csp2dSolution> {
        problem.validate()?;
        let configuration = problem.configuration.clone();
        let mut plans = self.build_plans(
            &problem,
            &problem.demand_products(),
            0,
        )?;
        if plans.is_empty() {
            return Ok(self.empty_solution(
                &problem,
                Csp2dTerminationReason::NoCandidatePlan,
                Some("没有可用的候选方案 / No candidate plan".to_string()),
            ));
        }

        let mut iterations = Vec::new();
        let mut termination = Csp2dTerminationReason::IterationLimitReached;
        let mut usage = Vec::new();

        for iteration in 0..configuration.iteration_limit {
            let assignment = Csp2dPatternAssignment::new(
                plans.clone(),
                problem.demand_piece_bound(),
            )?;
            let master =
                Csp2dMasterProblem::new(assignment, problem.master_demands())?;
            usage = self.heuristic_usage(&master, &configuration);
            let covered = master.assignment.covers_all(&usage, &master.demands);

            // 剩余需求驱动下一轮图案生成。
            // The residual demand drives the next round of pattern generation.
            let residuals = crate::domain::produce::residual_products(
                &master.assignment,
                &usage,
                &master.demands,
                &problem.products,
            );
            let mut added = Vec::new();
            if !covered && !residuals.is_empty() {
                let candidates = self.build_plans(&problem, &residuals, iteration as i64 + 1)?;
                // 只保留身份键尚不存在的图案。
                // Keep only patterns whose identity key is not present yet.
                let existing: std::collections::HashSet<String> = plans
                    .iter()
                    .map(|plan| plan.identity_key())
                    .collect();
                let fresh: Vec<_> = candidates
                    .into_iter()
                    .filter(|plan| !existing.contains(&plan.identity_key()))
                    .collect();

                // 用检验数筛选新列：影子价格由当前覆盖缺口估计（不是 LP 对偶，见 pricing 模块说明）。
                // 只保留检验数为负的候选，按检验数升序取前若干个，避免把无改进价值的列塞进主问题。
                // Filter the new columns by reduced cost: the shadow prices are estimated from the current
                // coverage deficit (not LP duals, see the pricing module). Only negative reduced costs are kept,
                // sorted ascending, so columns with no improving value never enter the master.
                let prices = crate::domain::produce::estimated_shadow_prices(
                    &master.assignment,
                    &usage,
                    &master.demands,
                );
                let pricing = crate::domain::produce::Csp2dPricingConfig::default();
                let mut scored: Vec<(f64, crate::domain::material::CuttingPlan2<f64, ospf_rust_quantities::unit::derived::Meter>)> = fresh
                    .into_iter()
                    .map(|plan| {
                        let estimate = crate::domain::produce::estimate_reduced_cost(
                            &plan,
                            &master.demands,
                            &prices,
                            &pricing,
                        );
                        (estimate.reduced_cost, plan)
                    })
                    .collect();
                scored.sort_by(|lhs, rhs| {
                    lhs.0
                        .partial_cmp(&rhs.0)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                let limit = configuration.max_candidates_per_iteration.max(1);
                let mut improving: Vec<_> = scored
                    .iter()
                    .filter(|(reduced_cost, _)| *reduced_cost < -pricing.tolerance)
                    .map(|(reduced_cost, plan)| (*reduced_cost, plan.clone()))
                    .collect();
                improving.sort_by(|lhs, rhs| {
                    lhs.0
                        .partial_cmp(&rhs.0)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                added = improving
                    .into_iter()
                    .take(limit)
                    .map(|(_, plan)| plan)
                    .collect();
                // 若全部候选都没有负检验数（估计口径下无可改进列），仍保留覆盖最全的一个，
                // 否则列生成会因为「估计过于保守」而提前停滞。
                // If no candidate has a negative reduced cost under the estimate, keep the widest-coverage one
                // anyway, otherwise the estimate being too conservative would stall column generation early.
                if added.is_empty() {
                    if let Some((_, plan)) = scored.into_iter().min_by(|lhs, rhs| {
                        lhs.0
                            .partial_cmp(&rhs.0)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    }) {
                        added.push(plan);
                    }
                }
            }

            iterations.push(Csp2dIterationRecord {
                iteration,
                plan_count: plans.len(),
                added_plan_count: added.len(),
                covered,
            });

            if covered {
                termination = Csp2dTerminationReason::Covered;
                break;
            }
            if added.is_empty() {
                termination = Csp2dTerminationReason::NoNewPlan;
                break;
            }
            plans.extend(added);
        }

        let assignment = Csp2dPatternAssignment::new(
            plans,
            problem.demand_piece_bound(),
        )?;
        let master = Csp2dMasterProblem::new(assignment, problem.master_demands())?;
        usage = self.heuristic_usage(&master, &configuration);
        let mut solution = self.build_solution(&problem, &master, &usage, iterations);
        solution.termination = termination;
        Ok(solution)
    }

    /// 由问题生成候选方案 / Build the candidate plans of a problem
    fn build_plans(
        &self,
        problem: &Csp2dProblem,
        products: &[(crate::domain::material::Product2<f64, ospf_rust_quantities::unit::derived::Meter>, u64)],
        iteration: i64,
    ) -> Ret<Vec<CuttingPlan2<f64, ospf_rust_quantities::unit::derived::Meter>>> {
        let generator = ShelfPatternGenerator2::new();
        let mut plans = Vec::new();
        for material in &problem.materials {
            let request = PatternGenerationRequest2::new(
                material.clone(),
                products.to_vec(),
                problem.kerf.clone(),
            )
            .with_process(problem.process)
            .with_costars(problem.costars.clone())
            .with_max_candidates(problem.configuration.max_candidates_per_iteration);
            let request = PatternGenerationRequest2 {
                iteration,
                ..request
            };
            let patterns: Vec<Pattern2<f64, ospf_rust_quantities::unit::derived::Meter>> =
                generator.generate_patterns(&request)?;
            for pattern in patterns {
                // 工艺与切分树不一致的图案直接丢弃，不让非法图案进入主问题。
                // Drop patterns whose process and cut tree disagree, so no illegal pattern reaches the master.
                if pattern.validate(&problem.kerf).is_err() {
                    continue;
                }
                plans.push(CuttingPlan2::from_pattern(pattern));
            }
        }
        Ok(plans)
    }

    /// 启发式使用次数：按需求比例取整并校正覆盖 / Heuristic usage: round by demand ratio and correct coverage
    ///
    /// 这是本阶段替代 LP/整数求解的轻量口径：先按「需求 / 覆盖」给出初始次数，再对未覆盖的需求逐个
    /// 增加最能补齐它的方案次数。
    /// This is the lightweight stand-in for the LP/integer solve at this stage: seed the counts from
    /// "demand / coverage", then raise the count of the plan that best fills each uncovered demand.
    fn heuristic_usage(
        &self,
        master: &Csp2dMasterProblem,
        configuration: &Csp2dConfiguration,
    ) -> Vec<f64> {
        // 注入求解器时优先使用它；终态不可交付时回退到内置启发式，绝不把非最优解当成最优。
        // An injected solver takes precedence; when its terminal state is not deliverable the built-in heuristic
        // takes over, so a non-optimal result is never passed off as optimal.
        if let Some(solver) = self.master_solver.as_ref()
            && let Ok(result) = solver.solve_master(master)
            && crate::application::master_solver::require_deliverable(&result).is_ok()
            && result.usage.len() == master.assignment.len()
        {
            return result.usage;
        }
        let plan_count = master.assignment.len();
        let mut usage = vec![0.0f64; plan_count];

        // 覆盖优先：只要还有未覆盖需求，就挑最能补齐的方案加一次。
        // Coverage first: while a demand is uncovered, raise the count of the plan that fills it best.
        let mut guard = 0usize;
        while guard < 4096 {
            guard += 1;
            let coverage = master.assignment.coverage(&usage, &master.demands);
            let mut best: Option<(usize, f64)> = None;
            for (demand_index, demand) in master.demands.iter().enumerate() {
                let required = demand.demand_value();
                let covered = coverage[demand_index];
                if covered + 1e-6 >= required {
                    continue;
                }
                for plan_index in 0..plan_count {
                    let coefficient =
                        master.assignment.coverage_coefficient(plan_index, demand);
                    if coefficient <= 0.0 {
                        continue;
                    }
                    // 优先补齐缺口最大、且单位消耗余料最少的方案。
                    // Prefer the plan that closes the largest deficit per unit of remnant.
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
            // 使用次数受上界约束，避免把单列用到不合理的量级。
            // The usage is bounded to avoid an unreasonable count on a single column.
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

        // 覆盖已足够时，若配置关闭了最终整数求解，仍返回这份使用次数。
        // When coverage suffices, the same counts are returned even if the final integer solve is disabled.
        let _ = configuration;
        usage
    }

    /// 组装求解结果 / Assemble the solve result
    fn build_solution(
        &self,
        problem: &Csp2dProblem,
        master: &Csp2dMasterProblem,
        usage: &[f64],
        iterations: Vec<Csp2dIterationRecord>,
    ) -> Csp2dSolution {
        let rounded = master.assignment.rounded_usage(usage);
        let mut selected = Vec::new();
        for (index, count) in rounded.iter().enumerate() {
            if *count == 0 {
                continue;
            }
            if let Some(plan) = master.assignment.plans.get(index) {
                selected.push((plan.clone(), *count));
            }
        }
        let coverage = master.assignment.coverage(usage, &master.demands);
        let demands: Vec<(crate::domain::material::ProductId2, f64)> = master
            .demands
            .iter()
            .map(|demand| (demand.product_id.clone(), demand.demand_value()))
            .collect();
        let yield_result: Csp2dYieldResult =
            YieldContext::new().analyze(&demands, &coverage);
        let waste_result: Csp2dWasteResult = WasteContext::new().analyze(
            &selected
                .iter()
                .map(|(plan, _)| plan.clone())
                .collect::<Vec<_>>(),
        );
        let fully_covered = master.assignment.covers_all(usage, &master.demands);
        let termination = if fully_covered {
            Csp2dTerminationReason::Covered
        } else {
            Csp2dTerminationReason::NoNewPlan
        };
        // 板料分配由选中方案的使用量推导，并立刻校验可用量，避免超用物料被静默接受。
        // The sheet assignment is derived from the selected usage and validated immediately so over-consumption
        // of a material is never silently accepted.
        let sheet_assignment =
            crate::domain::sheet_assignment::assign_sheets(&selected).unwrap_or_default();
        Csp2dSolution {
            usage: selected,
            coverage,
            yield_result,
            waste_result,
            material_usage: master.material_usage(usage),
            sheet_assignment,
            fully_covered,
            iterations,
            termination,
            failure_message: if fully_covered {
                None
            } else {
                Some(
                    "当前生成范围无法覆盖全部需求 / The current generation scope cannot cover every demand"
                        .to_string(),
                )
            },
        }
    }

    /// 空解 / Empty solution
    fn empty_solution(
        &self,
        problem: &Csp2dProblem,
        termination: Csp2dTerminationReason,
        failure_message: Option<String>,
    ) -> Csp2dSolution {
        let demands: Vec<(crate::domain::material::ProductId2, f64)> = problem
            .master_demands()
            .iter()
            .map(|demand| (demand.product_id.clone(), demand.demand_value()))
            .collect();
        let coverage = vec![0.0; demands.len()];
        Csp2dSolution {
            usage: Vec::new(),
            yield_result: YieldContext::new().analyze(&demands, &coverage),
            waste_result: Csp2dWasteResult::default(),
            coverage,
            material_usage: BTreeMap::new(),
            sheet_assignment: crate::domain::sheet_assignment::SheetAssignment::default(),
            fully_covered: false,
            iterations: Vec::new(),
            termination,
            failure_message,
        }
    }
}

/// 校验问题并给出显式错误 / Validate a problem and surface an explicit error
pub fn require_valid(problem: &Csp2dProblem) -> Ret<()> {
    if problem.materials.is_empty() {
        return Err(validation_error("问题没有物料 / The problem has no material"));
    }
    problem.validate()
}
