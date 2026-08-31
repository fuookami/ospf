//! 批次应用服务 / Bunch application services
//!
//! 提供束级列生成和分支定价算法入口。
//! Provides bunch-level column generation and branch-and-price algorithm entry points.

use std::collections::HashMap;

use ospf_rust_core::model::MetaModel;
use ospf_rust_framework::solver::column_generation_solver::ColumnGenerationSolver;

use crate::application::algorithm::bunch_column_generation::{
    BunchBranchAndPriceAlgorithm, BunchCGPolicy,
};
use crate::application::algorithm::policy::ColumnGenerationPolicy;
use crate::application::algorithm::{
    BranchAndPriceTreeSearch, BranchSearchConfig, BranchSearchHooks, BranchSearchResult,
};
use crate::domain::bunch_compilation::context::IterativeBunchCompilationContext;
use crate::domain::bunch_compilation::model::{BunchEntry, BunchSolution};
use crate::domain::bunch_generation::{
    BunchFeasibilityPolicy, BunchTaskCandidate, CapacityIntermediateValues, SlotBasedBunchGenerator,
};
use crate::domain::task::{
    AssignmentPolicyTrait, BunchCostPolicy, DefaultBunchCostPolicy, ExecutorTrait, TaskTrait,
};
use crate::infrastructure::TimeSlot;

/// 束级列生成算法入口 / Bunch column-generation algorithm entry
///
/// 创建束级分支定价算法实例。
/// Creates a bunch-level branch-and-price algorithm instance.
pub fn create_bunch_branch_and_price<C, S, P>(
    context: C,
    solver: S,
    policy: P,
    executor_ids: Vec<String>,
    configuration: ColumnGenerationPolicy,
) -> BunchBranchAndPriceAlgorithm<C, S, P>
where
    C: IterativeBunchCompilationContext,
    S: ColumnGenerationSolver,
    P: BunchCGPolicy,
{
    BunchBranchAndPriceAlgorithm::new(context, solver, policy, executor_ids, configuration)
}

/// 搜索隔离的束级分支定价树 / Search isolated bunch branch-and-price tree
///
/// 使用 `solve_branch_node_with_fresh_model` 求解每个节点，避免树搜索过程中复用同一
/// `MetaModel` 或 context 注册态。
/// Uses `solve_branch_node_with_fresh_model` for each node so tree search does
/// not reuse one `MetaModel` or context registration state across nodes.
pub fn search_bunch_branch_and_price_with_fresh_model<C, S, P, F>(
    algorithm: &mut BunchBranchAndPriceAlgorithm<C, S, P>,
    config: BranchSearchConfig,
    mut build_model: F,
) -> BranchSearchResult<BunchSolution>
where
    C: IterativeBunchCompilationContext + Clone,
    S: ColumnGenerationSolver,
    P: BunchCGPolicy,
    F: FnMut() -> MetaModel<f64>,
{
    let search = BranchAndPriceTreeSearch::new(config);
    search.search(|node| algorithm.solve_branch_node_with_fresh_model(node, &mut build_model))
}

/// 带 hooks 搜索隔离的束级分支定价树 / Search isolated bunch branch-and-price tree with hooks
pub fn search_bunch_branch_and_price_with_hooks<C, S, P, F>(
    algorithm: &mut BunchBranchAndPriceAlgorithm<C, S, P>,
    config: BranchSearchConfig,
    mut build_model: F,
    hooks: BranchSearchHooks<'_, BunchSolution>,
) -> BranchSearchResult<BunchSolution>
where
    C: IterativeBunchCompilationContext + Clone,
    S: ColumnGenerationSolver,
    P: BunchCGPolicy,
    F: FnMut() -> MetaModel<f64>,
{
    let search = BranchAndPriceTreeSearch::new(config);
    search.search_with_hooks(
        |node| algorithm.solve_branch_node_with_fresh_model(node, &mut build_model),
        hooks,
    )
}

/// 默认束生成策略 / Default bunch generation policy
///
/// 将基于时隙的束生成器、候选任务、产能中间值和可行性策略组合为 application 可用的
/// `BunchCGPolicy`。
/// Combines slot-based bunch generator, task candidates, capacity intermediate values,
/// and feasibility policy into an application-level `BunchCGPolicy`.
#[derive(Debug, Clone)]
pub struct DefaultBunchGenerationPolicy<'a, E, T, A, S, P, C = DefaultBunchCostPolicy>
where
    E: ExecutorTrait,
    T: TaskTrait<E, A>,
    A: AssignmentPolicyTrait<E>,
    S: TimeSlot,
    P: BunchFeasibilityPolicy<E, T, A>,
    C: BunchCostPolicy,
{
    /// 基于时隙的生成器 / Slot-based generator
    pub generator: SlotBasedBunchGenerator<E>,
    /// 产能中间值 / Capacity intermediate values
    pub intermediate_values: &'a CapacityIntermediateValues<S>,
    /// 候选任务 / Candidate tasks
    pub candidates: Vec<BunchTaskCandidate<'a, T>>,
    /// 可行性策略 / Feasibility policy
    pub feasibility_policy: P,
    /// 成本策略 / Cost policy
    pub cost_policy: C,
    /// 分配策略 marker / Assignment policy marker
    _assignment: std::marker::PhantomData<A>,
}

impl<'a, E, T, A, S, P> DefaultBunchGenerationPolicy<'a, E, T, A, S, P, DefaultBunchCostPolicy>
where
    E: ExecutorTrait,
    T: TaskTrait<E, A>,
    A: AssignmentPolicyTrait<E>,
    S: TimeSlot,
    P: BunchFeasibilityPolicy<E, T, A>,
{
    /// 创建默认束生成策略 / Create default bunch generation policy
    pub fn new(
        generator: SlotBasedBunchGenerator<E>,
        intermediate_values: &'a CapacityIntermediateValues<S>,
        candidates: Vec<BunchTaskCandidate<'a, T>>,
        feasibility_policy: P,
    ) -> Self {
        Self {
            generator,
            intermediate_values,
            candidates,
            feasibility_policy,
            cost_policy: DefaultBunchCostPolicy,
            _assignment: std::marker::PhantomData,
        }
    }
}

impl<'a, E, T, A, S, P, C> DefaultBunchGenerationPolicy<'a, E, T, A, S, P, C>
where
    E: ExecutorTrait,
    T: TaskTrait<E, A>,
    A: AssignmentPolicyTrait<E>,
    S: TimeSlot,
    P: BunchFeasibilityPolicy<E, T, A>,
    C: BunchCostPolicy,
{
    /// 创建带成本策略的默认束生成策略 / Create default bunch generation policy with cost policy
    pub fn with_cost_policy(
        generator: SlotBasedBunchGenerator<E>,
        intermediate_values: &'a CapacityIntermediateValues<S>,
        candidates: Vec<BunchTaskCandidate<'a, T>>,
        feasibility_policy: P,
        cost_policy: C,
    ) -> Self {
        Self {
            generator,
            intermediate_values,
            candidates,
            feasibility_policy,
            cost_policy,
            _assignment: std::marker::PhantomData,
        }
    }
}

impl<E, T, A, S, P, C> BunchCGPolicy for DefaultBunchGenerationPolicy<'_, E, T, A, S, P, C>
where
    E: ExecutorTrait,
    T: TaskTrait<E, A>,
    A: AssignmentPolicyTrait<E>,
    S: TimeSlot,
    P: BunchFeasibilityPolicy<E, T, A>,
    C: BunchCostPolicy,
{
    fn build_shadow_price_map(&self) -> HashMap<usize, f64> {
        HashMap::new()
    }

    fn reduced_cost(&self, shadow_prices: &HashMap<usize, f64>, bunch: &BunchEntry) -> f64 {
        self.cost_policy.reduced_cost(bunch, shadow_prices)
    }

    fn generate_bunches(
        &self,
        iteration: usize,
        executor_ids: &[String],
        shadow_prices: &HashMap<usize, f64>,
    ) -> Vec<BunchEntry> {
        let mut generator = self.generator.clone();
        generator
            .executors
            .retain(|executor| executor_ids.iter().any(|id| id == executor.id()));

        generator
            .generate_all_with_policy(
                iteration,
                self.intermediate_values,
                &self.candidates,
                shadow_prices,
                &self.feasibility_policy,
            )
            .unwrap_or_default()
            .into_iter()
            .map(|entry| entry.bunch)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration as StdDuration;

    use time::macros::datetime;
    use time::{Duration, OffsetDateTime};

    use ospf_rust_core::error::Result as CoreResult;
    use ospf_rust_core::model::intermediate::LinearTriadModel;
    use ospf_rust_framework::solver::column_generation_solver::{
        FeasibleSolution, LPResult, LinearDualSolution,
    };
    use ospf_rust_framework::solver::framework_solve_options::FrameworkSolveOptions;

    use super::*;
    use crate::GanttResult;
    use crate::application::algorithm::{
        BranchAndPriceTreeSearch, BranchNodeSolveOutput, BranchSearchConfig, BranchSearchOrder,
    };
    use crate::domain::bunch_compilation::context::BasicBunchCompilationContext;
    use crate::domain::bunch_generation::{
        BunchGenerationConfig, DefaultBunchFeasibilityPolicy, SlotConstraints,
    };
    use crate::domain::task::{
        BasicAssignmentPolicy, BasicExecutor, CostBreakdown, FunctionalBunchCostPolicy,
    };
    use crate::infrastructure::TimeRange;

    #[derive(Debug, Clone)]
    struct TestSlot {
        time: TimeRange,
    }

    impl TimeSlot for TestSlot {
        fn time(&self) -> &TimeRange {
            &self.time
        }

        fn sub_of(&self, sub_time: &TimeRange) -> Option<Self> {
            self.time.intersection(sub_time).map(|time| Self { time })
        }
    }

    #[derive(Debug, Clone)]
    struct TestTask {
        id: String,
        name: String,
        time: TimeRange,
    }

    impl TaskTrait<BasicExecutor, BasicAssignmentPolicy<BasicExecutor>> for TestTask {
        fn id(&self) -> &str {
            &self.id
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn scheduled_time(&self) -> Option<&TimeRange> {
            Some(&self.time)
        }

        fn time(&self) -> Option<&TimeRange> {
            Some(&self.time)
        }
    }

    fn h(hour: i64) -> OffsetDateTime {
        datetime!(2020-08-30 00:00 UTC) + Duration::hours(hour)
    }

    #[test]
    fn test_default_bunch_generation_policy_generates_entries() {
        let executor = BasicExecutor::new("exec_1", "Executor 1");
        let generator =
            SlotBasedBunchGenerator::new(vec![executor], BunchGenerationConfig::default());
        let slots = vec![TestSlot {
            time: TimeRange::new(h(8), h(18)),
        }];
        let mut constraints = HashMap::new();
        constraints.insert(0, SlotConstraints::default());
        let intermediate_values = CapacityIntermediateValues::new(slots, constraints);
        let task = TestTask {
            id: "task_1".to_string(),
            name: "Task 1".to_string(),
            time: TimeRange::new(h(9), h(10)),
        };
        let candidates = vec![BunchTaskCandidate::new(0, &task, true)];
        let policy = DefaultBunchGenerationPolicy::new(
            generator,
            &intermediate_values,
            candidates,
            DefaultBunchFeasibilityPolicy,
        );

        let bunches =
            policy.generate_bunches(1, &["exec_1".to_string()], &HashMap::from([(0, 2.0)]));

        assert!(!bunches.is_empty());
        assert!(bunches.iter().all(|bunch| bunch.executor_id == "exec_1"));
    }

    #[test]
    fn test_default_bunch_generation_policy_accepts_custom_cost_policy() {
        let executor = BasicExecutor::new("exec_1", "Executor 1");
        let generator =
            SlotBasedBunchGenerator::new(vec![executor], BunchGenerationConfig::default());
        let slots = vec![TestSlot {
            time: TimeRange::new(h(8), h(18)),
        }];
        let mut constraints = HashMap::new();
        constraints.insert(0, SlotConstraints::default());
        let intermediate_values = CapacityIntermediateValues::new(slots, constraints);
        let task = TestTask {
            id: "task_1".to_string(),
            name: "Task 1".to_string(),
            time: TimeRange::new(h(9), h(10)),
        };
        let candidates = vec![BunchTaskCandidate::new(0, &task, true)];
        let cost_policy = FunctionalBunchCostPolicy::new(|bunch, _shadow_prices| CostBreakdown {
            task_cost: bunch.cost,
            executor_cost: 4.0,
            connection_cost: 0.5,
            capacity_cost: 1.5,
            penalty_cost: 2.0,
        });
        let policy = DefaultBunchGenerationPolicy::with_cost_policy(
            generator,
            &intermediate_values,
            candidates,
            DefaultBunchFeasibilityPolicy,
            cost_policy,
        );
        let bunch = BunchEntry {
            index: 0,
            executor_id: "exec_1".to_string(),
            task_indices: vec![0],
            cost: 3.0,
            iteration: 0,
        };

        let reduced = policy.reduced_cost(&HashMap::from([(0, 2.0)]), &bunch);

        assert!((reduced - 9.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_small_branch_and_price_flow_generates_columns_and_searches_tree() {
        let executor = BasicExecutor::new("exec_1", "Executor 1");
        let generator = SlotBasedBunchGenerator::new(
            vec![executor],
            BunchGenerationConfig {
                max_columns_per_executor: 4,
                max_tasks_per_bunch: 2,
                allow_order_change: false,
                allow_empty_slot: false,
            },
        );
        let slots = vec![TestSlot {
            time: TimeRange::new(h(8), h(18)),
        }];
        let mut constraints = HashMap::new();
        constraints.insert(0, SlotConstraints::default());
        let intermediate_values = CapacityIntermediateValues::new(slots, constraints);
        let tasks = [
            TestTask {
                id: "task_1".to_string(),
                name: "Task 1".to_string(),
                time: TimeRange::new(h(9), h(10)),
            },
            TestTask {
                id: "task_2".to_string(),
                name: "Task 2".to_string(),
                time: TimeRange::new(h(11), h(12)),
            },
        ];
        let candidates = tasks
            .iter()
            .enumerate()
            .map(|(index, task)| BunchTaskCandidate::new(index, task, false))
            .collect::<Vec<_>>();
        let policy = DefaultBunchGenerationPolicy::new(
            generator,
            &intermediate_values,
            candidates,
            DefaultBunchFeasibilityPolicy,
        );
        let generated_bunches = policy.generate_bunches(
            0,
            &["exec_1".to_string()],
            &HashMap::from([(0, 3.0), (1, 4.0)]),
        );

        assert!(!generated_bunches.is_empty());

        let search = BranchAndPriceTreeSearch::new(BranchSearchConfig {
            order: BranchSearchOrder::DepthFirst,
            max_nodes: 4,
            max_depth: 2,
            gap_tolerance: 1e-9,
            time_limit: StdDuration::from_secs(1),
        });
        let result = search.search(|node| {
            if node.depth == 0 {
                BranchNodeSolveOutput {
                    lower_bound: -10.0,
                    solution: None,
                    objective: None,
                    branch_target: generated_bunches.first().map(|bunch| bunch.index),
                    infeasible: false,
                }
            } else {
                let selected = generated_bunches
                    .iter()
                    .filter(|bunch| {
                        node.decisions.iter().all(|decision| {
                            decision.fixed_value == 0 || decision.target_index == bunch.index
                        })
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                BranchNodeSolveOutput {
                    lower_bound: -1.0,
                    objective: Some(selected.iter().map(|bunch| bunch.cost).sum::<f64>()),
                    solution: Some(selected),
                    branch_target: None,
                    infeasible: false,
                }
            }
        });

        assert!(result.processed_nodes >= 2);
        assert!(result.incumbent.is_some());
        assert!(
            result
                .incumbent
                .as_ref()
                .unwrap()
                .iter()
                .all(|bunch| bunch.executor_id == "exec_1")
        );
    }

    #[derive(Debug, Clone)]
    struct EmptyPolicy;

    impl BunchCGPolicy for EmptyPolicy {
        fn build_shadow_price_map(&self) -> HashMap<usize, f64> {
            HashMap::new()
        }

        fn reduced_cost(&self, _shadow_prices: &HashMap<usize, f64>, bunch: &BunchEntry) -> f64 {
            bunch.cost
        }

        fn generate_bunches(
            &self,
            _iteration: usize,
            _executor_ids: &[String],
            _shadow_prices: &HashMap<usize, f64>,
        ) -> Vec<BunchEntry> {
            Vec::new()
        }
    }

    #[derive(Debug, Clone)]
    struct ZeroSolver;

    #[cfg_attr(feature = "async", async_trait::async_trait)]
    impl ColumnGenerationSolver for ZeroSolver {
        fn name(&self) -> &str {
            "zero_bunch_service_solver"
        }

        #[cfg(feature = "async")]
        async fn solve_milp_with_options(
            &self,
            model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> CoreResult<FeasibleSolution> {
            Ok(FeasibleSolution::new(0.0, vec![0.0; model.num_variables()]))
        }

        #[cfg(not(feature = "async"))]
        fn solve_milp_with_options(
            &self,
            model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> CoreResult<FeasibleSolution> {
            Ok(FeasibleSolution::new(0.0, vec![0.0; model.num_variables()]))
        }

        #[cfg(feature = "async")]
        async fn solve_lp_with_options(
            &self,
            model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> CoreResult<LPResult> {
            Ok(LPResult::new(
                FeasibleSolution::new(0.0, vec![0.0; model.num_variables()]),
                LinearDualSolution::default(),
            ))
        }

        #[cfg(not(feature = "async"))]
        fn solve_lp_with_options(
            &self,
            model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> CoreResult<LPResult> {
            Ok(LPResult::new(
                FeasibleSolution::new(0.0, vec![0.0; model.num_variables()]),
                LinearDualSolution::default(),
            ))
        }
    }

    #[test]
    fn test_search_bunch_branch_and_price_with_fresh_model_uses_isolated_nodes() -> GanttResult<()>
    {
        let context = BasicBunchCompilationContext::new(1, vec!["exec_1".to_string()], false);
        let mut algorithm = create_bunch_branch_and_price(
            context,
            ZeroSolver,
            EmptyPolicy,
            vec!["exec_1".to_string()],
            ColumnGenerationPolicy::default(),
        );
        let result = search_bunch_branch_and_price_with_fresh_model(
            &mut algorithm,
            BranchSearchConfig {
                order: BranchSearchOrder::DepthFirst,
                max_nodes: 2,
                max_depth: 1,
                gap_tolerance: 1e-9,
                time_limit: StdDuration::from_secs(1),
            },
            || MetaModel::<f64>::new("test_service_isolated_node"),
        );

        assert_eq!(result.processed_nodes, 1);
        assert_eq!(result.incumbent_objective, Some(0.0));
        assert_eq!(algorithm.context.column_count(), 0);
        assert!(algorithm.context.compilation.base.y_indices.is_empty());
        Ok(())
    }

    #[test]
    fn test_isolated_branch_search_solution_respects_fixed_state() -> GanttResult<()> {
        let context = BasicBunchCompilationContext::new(1, vec!["exec_1".to_string()], false);
        let mut algorithm = create_bunch_branch_and_price(
            context,
            ZeroSolver,
            EmptyPolicy,
            vec!["exec_1".to_string()],
            ColumnGenerationPolicy::default(),
        );
        algorithm.lifecycle.fix_columns([3]);
        algorithm.lifecycle.hide_columns([2]);
        algorithm.sync_model_state_from_lifecycle();

        let result = search_bunch_branch_and_price_with_fresh_model(
            &mut algorithm,
            BranchSearchConfig {
                order: BranchSearchOrder::DepthFirst,
                max_nodes: 2,
                max_depth: 1,
                gap_tolerance: 1e-9,
                time_limit: StdDuration::from_secs(1),
            },
            || MetaModel::<f64>::new("test_service_fixed_state_solution"),
        );

        assert_eq!(result.incumbent_objective, Some(0.0));
        assert_eq!(result.incumbent.unwrap().selected_bunches, vec![3]);
        assert!(algorithm.model_state.is_fixed(3));
        assert!(algorithm.model_state.is_hidden(2));
        Ok(())
    }

    #[derive(Debug, Clone, Default)]
    struct FinalMilpFlowState {
        milp_variable_counts: Vec<usize>,
        lp_variable_counts: Vec<usize>,
    }

    #[derive(Debug, Clone, Default)]
    struct FinalMilpFlowSolver {
        state: Arc<Mutex<FinalMilpFlowState>>,
    }

    impl FinalMilpFlowSolver {
        fn solve_milp_model(&self, model: &LinearTriadModel) -> FeasibleSolution {
            let variable_count = model.num_variables();
            self.state
                .lock()
                .unwrap()
                .milp_variable_counts
                .push(variable_count);

            if variable_count <= 1 {
                FeasibleSolution::new(-10.0, vec![1.0; variable_count])
            } else {
                FeasibleSolution::new(1.0, vec![1.0; variable_count])
            }
        }

        fn solve_lp_model(&self, model: &LinearTriadModel) -> LPResult {
            let variable_count = model.num_variables();
            self.state
                .lock()
                .unwrap()
                .lp_variable_counts
                .push(variable_count);
            LPResult::new(
                FeasibleSolution::new(5.0, vec![0.0; variable_count]),
                LinearDualSolution::default(),
            )
        }
    }

    #[cfg_attr(feature = "async", async_trait::async_trait)]
    impl ColumnGenerationSolver for FinalMilpFlowSolver {
        fn name(&self) -> &str {
            "final_milp_flow_solver"
        }

        #[cfg(feature = "async")]
        async fn solve_milp_with_options(
            &self,
            model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> CoreResult<FeasibleSolution> {
            Ok(self.solve_milp_model(model))
        }

        #[cfg(not(feature = "async"))]
        fn solve_milp_with_options(
            &self,
            model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> CoreResult<FeasibleSolution> {
            Ok(self.solve_milp_model(model))
        }

        #[cfg(feature = "async")]
        async fn solve_lp_with_options(
            &self,
            model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> CoreResult<LPResult> {
            Ok(self.solve_lp_model(model))
        }

        #[cfg(not(feature = "async"))]
        fn solve_lp_with_options(
            &self,
            model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> CoreResult<LPResult> {
            Ok(self.solve_lp_model(model))
        }
    }

    #[derive(Debug, Clone)]
    struct SingleIterationBunchPolicy;

    impl BunchCGPolicy for SingleIterationBunchPolicy {
        fn build_shadow_price_map(&self) -> HashMap<usize, f64> {
            HashMap::new()
        }

        fn reduced_cost(&self, _shadow_prices: &HashMap<usize, f64>, bunch: &BunchEntry) -> f64 {
            bunch.cost
        }

        fn generate_bunches(
            &self,
            iteration: usize,
            executor_ids: &[String],
            _shadow_prices: &HashMap<usize, f64>,
        ) -> Vec<BunchEntry> {
            if iteration != 1 {
                return Vec::new();
            }

            executor_ids
                .first()
                .map(|executor_id| BunchEntry {
                    index: 0,
                    executor_id: executor_id.clone(),
                    task_indices: vec![0],
                    cost: 1.0,
                    iteration,
                })
                .into_iter()
                .collect()
        }
    }

    #[test]
    fn test_fresh_model_tree_search_runs_final_milp_end_to_end() -> GanttResult<()> {
        let context = BasicBunchCompilationContext::new(1, vec!["exec_1".to_string()], false);
        let solver = FinalMilpFlowSolver::default();
        let solver_state = solver.state.clone();
        let mut configuration = ColumnGenerationPolicy::default();
        configuration.max_iterations = 1;
        configuration.time_limit = StdDuration::from_secs(1);
        let mut algorithm = create_bunch_branch_and_price(
            context,
            solver,
            SingleIterationBunchPolicy,
            vec!["exec_1".to_string()],
            configuration,
        );

        let result = search_bunch_branch_and_price_with_fresh_model(
            &mut algorithm,
            BranchSearchConfig {
                order: BranchSearchOrder::DepthFirst,
                max_nodes: 3,
                max_depth: 1,
                gap_tolerance: 1e-9,
                time_limit: StdDuration::from_secs(1),
            },
            || MetaModel::<f64>::new("test_final_milp_flow"),
        );

        let trace = solver_state.lock().unwrap().clone();
        assert_eq!(result.processed_nodes, 3);
        assert_eq!(result.incumbent_objective, Some(1.0));
        assert_eq!(result.incumbent.as_ref().unwrap().selected_bunches, vec![0],);
        assert!(result.incumbent.as_ref().unwrap().canceled_tasks.is_empty());
        assert!(trace.milp_variable_counts.iter().any(|count| *count <= 1));
        assert!(trace.milp_variable_counts.iter().any(|count| *count > 1));
        assert!(trace.lp_variable_counts.iter().any(|count| *count <= 1));
        assert!(trace.lp_variable_counts.iter().any(|count| *count > 1));
        assert_eq!(algorithm.context.column_count(), 0);
        assert!(algorithm.context.compilation.base.y_indices.is_empty());
        assert!(algorithm.context.compilation.bunch_x_map.is_empty());
        Ok(())
    }
}
