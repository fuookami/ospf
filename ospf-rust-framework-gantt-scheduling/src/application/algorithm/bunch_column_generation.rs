//! 束级分支定价算法 / Bunch-level branch and price algorithm
//!
//! 实现任务束调度的分支定价算法，包含：
//! - 初始 MILP 求解
//! - 全局列生成（所有执行器）
//! - 局部列生成（自由执行器）
//! - 固定/保留/隐藏列
//! - 列移除
//! - 收敛检测
//!
//! Implements branch and price algorithm for bunch scheduling, including:
//! - Initial MILP solve
//! - Global column generation (all executors)
//! - Local column generation (free executors)
//! - Fix/keep/hide columns
//! - Column removal
//! - Convergence detection

use std::collections::{HashMap, HashSet};

use ospf_rust_core::model::MetaModel;
use ospf_rust_framework::solver::column_generation_solver::{
    ColumnGenerationSolver, FeasibleSolution, LPResult,
};
use ospf_rust_framework::solver::framework_solve_options::FrameworkSolveOptions;

use crate::GanttError;
use crate::GanttResult;
use crate::application::algorithm::branch_and_price::{BranchNode, BranchNodeSolveOutput};
use crate::application::algorithm::policy::ColumnGenerationPolicy;
use crate::application::iteration::Iteration;
use crate::domain::bunch_compilation::context::IterativeBunchCompilationContext;
use crate::domain::bunch_compilation::model::{BunchEntry, BunchSolution};
use crate::domain::common::{
    ConstraintIndexMap, ExecutorId, ExecutorIdTrait, GanttDynamicModelLifecycle,
    GanttModelStateFacade,
};

/// 束级列生成策略 / Bunch column generation policy
///
/// 定义列生成算法的注入策略：上下文构建、影子价格映射、
/// reduced cost 计算和束生成器。
///
/// Defines injection strategies for column generation:
/// context building, shadow price mapping, reduced cost calculation,
/// and bunch generation.
/// 执行器分支组 / Executor branch group
///
/// 普通束以执行器为组；时隙束以 `(executor, slot)` 为组。
/// Ordinary bunches use an executor group; slot-based bunches use an
/// `(executor, slot)` group.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BranchGroup<I = ExecutorId>
where
    I: ExecutorIdTrait,
{
    /// 执行器 ID / Executor id
    pub executor_id: I,
    /// 时隙索引；普通束为 None / Slot index; None for ordinary bunches
    pub slot_index: Option<usize>,
}

impl<I> From<&BunchEntry<I>> for BranchGroup<I>
where
    I: ExecutorIdTrait,
{
    fn from(bunch: &BunchEntry<I>) -> Self {
        Self {
            executor_id: bunch.executor_id.clone(),
            slot_index: bunch.slot_index,
        }
    }
}

/// 分支组跟踪器 / Branch group tracker
///
/// 只有执行器的全部已知 group 均固定时才把它从本地定价中移除。
/// An executor leaves local pricing only after every known group is fixed.
#[derive(Debug, Clone)]
pub struct BranchGroupTracker<I = ExecutorId>
where
    I: ExecutorIdTrait,
{
    groups_by_executor: HashMap<I, HashSet<Option<usize>>>,
    fixed_groups: HashSet<BranchGroup<I>>,
}

impl<I> Default for BranchGroupTracker<I>
where
    I: ExecutorIdTrait,
{
    fn default() -> Self {
        Self {
            groups_by_executor: HashMap::new(),
            fixed_groups: HashSet::new(),
        }
    }
}

impl<I> BranchGroupTracker<I>
where
    I: ExecutorIdTrait,
{
    /// 登记束所属的分支组 / Register a bunch branch group
    pub fn observe(&mut self, bunch: &BunchEntry<I>) {
        self.groups_by_executor
            .entry(bunch.executor_id.clone())
            .or_default()
            .insert(bunch.slot_index);
    }

    /// 标记一个束所属 group 已固定 / Mark a bunch group as fixed
    pub fn mark_fixed(&mut self, bunch: &BunchEntry<I>) {
        self.observe(bunch);
        self.fixed_groups.insert(BranchGroup::from(bunch));
    }

    /// 清空固定状态并保留已知 group / Clear fixed state while retaining known groups
    pub fn clear_fixed(&mut self) {
        self.fixed_groups.clear();
    }

    /// 返回指定执行器是否所有已知 group 均固定 / Check whether all known groups are fixed
    pub fn is_executor_fully_fixed(&self, executor_id: &I) -> bool {
        let Some(groups) = self.groups_by_executor.get(executor_id) else {
            return false;
        };
        !groups.is_empty() && groups.iter().all(|slot_index| {
            self.fixed_groups.contains(&BranchGroup {
                executor_id: executor_id.clone(),
                slot_index: *slot_index,
            })
        })
    }

    /// 返回所有固定 group / Return all fixed groups
    pub fn fixed_groups(&self) -> &HashSet<BranchGroup<I>> {
        &self.fixed_groups
    }
}

/// 束定价请求 / Bunch pricing request
///
/// 承载时隙级分支状态和最小列配额，供精确定价器消费。
/// Carries slot-level branch state and column quota for exact pricing.
#[derive(Debug, Clone)]
pub struct BunchPricingRequest<I = ExecutorId>
where
    I: ExecutorIdTrait,
{
    /// 当前迭代 / Current iteration
    pub iteration: usize,
    /// 可定价执行器 / Executors available for pricing
    pub executor_ids: Vec<I>,
    /// 任务影子价格 / Task shadow prices
    pub shadow_prices: HashMap<usize, f64>,
    /// 执行器-时隙影子价格 / Executor-slot shadow prices
    pub executor_slot_shadow_prices: HashMap<(I, usize), f64>,
    /// 已固定 group / Fixed groups
    pub fixed_groups: HashSet<BranchGroup<I>>,
    /// 已保留 group / Kept groups
    pub kept_groups: HashSet<BranchGroup<I>>,
    /// 隐藏执行器 / Hidden executors
    pub hidden_executors: HashSet<I>,
    /// 每个未固定执行器的最小列数 / Minimum columns per non-fixed executor
    pub min_column_amount_per_executor: usize,
}

/// 束级列生成策略 / Bunch column generation policy
pub trait BunchCGPolicy<I = ExecutorId>: Send + Sync
where
    I: ExecutorIdTrait,
{
    /// 构建影子价格映射 / Build shadow price map
    fn build_shadow_price_map(&self) -> HashMap<usize, f64>;

    /// 计算 reduced cost / Calculate reduced cost
    fn reduced_cost(&self, shadow_prices: &HashMap<usize, f64>, bunch: &BunchEntry<I>) -> f64;

    /// 生成新束 / Generate new bunches
    fn generate_bunches(
        &self,
        iteration: usize,
        executor_ids: &[I],
        shadow_prices: &HashMap<usize, f64>,
    ) -> Vec<BunchEntry<I>>;

    /// 使用完整定价请求生成新束 / Generate bunches with a complete pricing request
    ///
    /// 默认实现保留旧接口行为，避免已有策略被迫同步迁移。
    /// The default preserves the legacy interface so existing policies remain compatible.
    fn generate_bunches_with_request(
        &self,
        request: &BunchPricingRequest<I>,
    ) -> GanttResult<Vec<BunchEntry<I>>> {
        Ok(self.generate_bunches(
            request.iteration,
            &request.executor_ids,
            &request.shadow_prices,
        ))
    }
}

/// 束级分支定价算法 / Bunch-level branch and price algorithm
///
/// 实现完整的分支定价算法主循环。
/// Implements the full branch and price algorithm main loop.
pub struct BunchBranchAndPriceAlgorithm<C, S, P>
where
    C: IterativeBunchCompilationContext,
    S: ColumnGenerationSolver,
    P: BunchCGPolicy<C::ExecutorId>,
{
    /// 编译上下文 / Compilation context
    pub context: C,
    /// 求解器 / Solver
    pub solver: S,
    /// 策略 / Policy
    pub policy: P,
    /// 配置 / Configuration
    pub configuration: ColumnGenerationPolicy,
    /// 执行器 ID 列表 / Executor ID list
    pub executor_ids: Vec<C::ExecutorId>,

    /// 迭代状态 / Iteration state
    pub iteration: Iteration,
    /// 影子价格 / Shadow prices
    pub shadow_prices: HashMap<usize, f64>,
    /// 执行器-时隙影子价格 / Executor-slot shadow prices
    pub executor_slot_shadow_prices: HashMap<(C::ExecutorId, usize), f64>,
    /// 已固定的束 / Fixed bunches
    pub fixed_bunches: HashSet<usize>,
    /// 保留的束 / Kept bunches
    pub kept_bunches: HashSet<usize>,
    /// 隐藏的执行器 / Hidden executors
    pub hidden_executors: HashSet<C::ExecutorId>,
    /// 分支组状态 / Branch group state
    pub branch_groups: BranchGroupTracker<C::ExecutorId>,
    /// 动态模型状态 / Dynamic model state
    pub model_state: GanttModelStateFacade,
    /// 动态模型生命周期 / Dynamic model lifecycle
    pub lifecycle: GanttDynamicModelLifecycle,
    /// 约束索引映射 / Constraint index map
    pub constraint_index_map: ConstraintIndexMap,
    /// 约束名到索引 fallback / Constraint name to index fallback
    pub constraint_name_to_index: HashMap<String, usize>,

    /// 最佳解 / Best solution
    pub best_solution: Option<BunchSolution>,
    /// 最佳目标值 / Best objective value
    pub best_obj: f64,

    /// 列索引计数器 / Column index counter
    bunch_index_counter: usize,
}

/// 分支节点求解快照 / Branch node solve snapshot
///
/// 保存 application 层可回滚状态，避免同一算法实例连续求解多个节点时串状态。
/// Stores rollback-capable application state so consecutive node solves on the same
/// algorithm instance do not leak state into each other.
#[derive(Debug, Clone)]
struct BunchBranchAndPriceSnapshot<I = ExecutorId>
where
    I: ExecutorIdTrait,
{
    /// 迭代状态 / Iteration state
    iteration: Iteration,
    /// 影子价格 / Shadow prices
    shadow_prices: HashMap<usize, f64>,
    /// 执行器-时隙影子价格 / Executor-slot shadow prices
    executor_slot_shadow_prices: HashMap<(I, usize), f64>,
    /// 已固定的束 / Fixed bunches
    fixed_bunches: HashSet<usize>,
    /// 保留的束 / Kept bunches
    kept_bunches: HashSet<usize>,
    /// 隐藏的执行器 / Hidden executors
    hidden_executors: HashSet<I>,
    /// 分支组状态 / Branch group state
    branch_groups: BranchGroupTracker<I>,
    /// 动态模型状态 / Dynamic model state
    model_state: GanttModelStateFacade,
    /// 动态模型生命周期 / Dynamic model lifecycle
    lifecycle: GanttDynamicModelLifecycle,
    /// 最佳解 / Best solution
    best_solution: Option<BunchSolution>,
    /// 最佳目标值 / Best objective value
    best_obj: f64,
    /// 列索引计数器 / Column index counter
    bunch_index_counter: usize,
}

/// 分支节点隔离求解快照 / Branch node isolated solve snapshot
///
/// 在 application 状态之外额外保存 context 状态。fresh `MetaModel` 由调用方构建，
/// 当前快照只负责恢复算法实例内部状态。
/// Stores context state in addition to application state. A fresh `MetaModel` is
/// built by the caller; this snapshot restores only the algorithm instance state.
#[derive(Debug, Clone)]
struct BunchBranchAndPriceIsolatedSnapshot<C>
where
    C: IterativeBunchCompilationContext,
{
    /// application 状态快照 / Application-state snapshot
    application: BunchBranchAndPriceSnapshot<C::ExecutorId>,
    /// 编译上下文 / Compilation context
    context: C,
}

impl<C, S, P> std::fmt::Debug for BunchBranchAndPriceAlgorithm<C, S, P>
where
    C: IterativeBunchCompilationContext,
    S: ColumnGenerationSolver,
    P: BunchCGPolicy<C::ExecutorId>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BunchBranchAndPriceAlgorithm")
            .field("iteration", &self.iteration.iteration)
            .field("best_obj", &self.best_obj)
            .field("fixed_count", &self.fixed_bunches.len())
            .finish()
    }
}

impl<C, S, P> BunchBranchAndPriceAlgorithm<C, S, P>
where
    C: IterativeBunchCompilationContext,
    S: ColumnGenerationSolver,
    P: BunchCGPolicy<C::ExecutorId>,
{
    /// 创建算法实例 / Create algorithm instance
    pub fn new(
        context: C,
        solver: S,
        policy: P,
        executor_ids: Vec<impl Into<C::ExecutorId>>,
        configuration: ColumnGenerationPolicy,
    ) -> Self {
        let executor_ids = executor_ids.into_iter().map(Into::into).collect();
        Self {
            context,
            solver,
            policy,
            configuration,
            executor_ids,
            iteration: Iteration::new(),
            shadow_prices: HashMap::new(),
            executor_slot_shadow_prices: HashMap::new(),
            fixed_bunches: HashSet::new(),
            kept_bunches: HashSet::new(),
            hidden_executors: HashSet::new(),
            branch_groups: BranchGroupTracker::default(),
            model_state: GanttModelStateFacade::new(),
            lifecycle: GanttDynamicModelLifecycle::new(),
            constraint_index_map: ConstraintIndexMap::new(),
            constraint_name_to_index: HashMap::new(),
            best_solution: None,
            best_obj: f64::NEG_INFINITY,
            bunch_index_counter: 0,
        }
    }

    /// 执行分支定价算法 / Execute branch and price algorithm
    ///
    /// 主循环结构：
    /// 1. 注册模型 + 添加初始列
    /// 2. 求解初始 MILP
    /// 3. 固定/保留初始解中的束
    /// 4. 主循环（全局列生成 + 局部列生成 + 最终 MILP）
    ///
    /// Main loop structure:
    /// 1. Register model + add initial columns
    /// 2. Solve initial MILP
    /// 3. Fix/keep bunches from initial solution
    /// 4. Main loop (global CG + local CG + final MILP)
    pub fn run(&mut self, model: &mut MetaModel<f64>) -> GanttResult<BunchSolution> {
        // 1. 注册 + 添加初始列
        self.context.register(model)?;

        // 2. 求解初始 MILP
        let ip_result = self.solve_milp(model)?;
        self.after_milp_solve(model, &ip_result.solution);
        let mut current_ip_solution = ip_result.solution.clone();
        let initial_obj = ip_result.obj;

        if initial_obj == 0.0 {
            let solution = self
                .context
                .analyze_solution_with_lifecycle(&ip_result.solution, &self.lifecycle);
            self.best_solution = Some(solution.clone());
            self.best_obj = initial_obj;
            return Ok(solution);
        }

        self.iteration.record_ip(initial_obj);
        self.iteration.set_upper_bound(initial_obj);

        let mut solution = self
            .context
            .analyze_solution_with_lifecycle(&ip_result.solution, &self.lifecycle);
        self.best_solution = Some(solution.clone());
        self.best_obj = initial_obj;

        // 3. 固定/保留初始解中的束
        let fixed = self.context.extract_fixed(&ip_result.solution);
        self.fixed_bunches = fixed;
        self.refresh_branch_groups();

        let kept = self.context.extract_kept(&ip_result.solution);
        self.kept_bunches = kept;

        // 4. 主循环
        let mut maximum_reduced_cost_1 = 50.0;
        let mut maximum_reduced_cost_2 = 3000.0;

        while !self.iteration.is_improvement_slow()
            && self.iteration.iteration < self.configuration.max_iterations
            && self.iteration.elapsed() < self.configuration.time_limit
        {
            // ---- 全局列生成 ----
        self.shadow_prices = match self.solve_rmp_lp(model) {
            Ok(lp_result) => {
                self.executor_slot_shadow_prices = self.extract_executor_slot_shadow_prices(&lp_result);
                if self.iteration.record_lp(lp_result.result.obj) {
                        self.kept_bunches
                            .extend(self.context.extract_kept(&lp_result.result.solution));
                    }
                    self.extract_shadow_prices(&lp_result)
                }
                Err(_) => break,
            };

            // 隐藏执行器
            self.hide_executors(&current_ip_solution);

            // 全局列生成（1 次）
            self.iteration.next_iteration();
            let new_bunches = self.generate_bunches(&self.executor_ids)?;

            if !new_bunches.is_empty() {
                self.add_columns(self.iteration.iteration, new_bunches, model)?;

                // 重新求解 RMP
                if let Ok(lp_result) = self.solve_rmp_lp(model) {
                    if self.iteration.record_lp(lp_result.result.obj) {
                        self.kept_bunches
                            .extend(self.context.extract_kept(&lp_result.result.solution));
                    }
                    self.shadow_prices = self.extract_shadow_prices(&lp_result);
                    self.executor_slot_shadow_prices = self.extract_executor_slot_shadow_prices(&lp_result);
                }

                // 列移除
                if self.context.column_count() > self.configuration.max_column_amount {
                    let shadow_prices_clone = self.shadow_prices.clone();
                    self.remove_columns(maximum_reduced_cost_1, &shadow_prices_clone, model)?;
                }
            }

            // 选择自由执行器 + 全局固定
            let free_executors = self.select_free_executors();
            let current_fixed = self.globally_fix(&free_executors, model)?;
            let mut free_executor_list = free_executors;

            // ---- 局部列生成 ----
            loop {
                self.shadow_prices = match self.solve_rmp_lp(model) {
                    Ok(lp_result) => {
                        self.executor_slot_shadow_prices = self.extract_executor_slot_shadow_prices(&lp_result);
                        self.iteration.record_lp(lp_result.result.obj);
                        self.extract_shadow_prices(&lp_result)
                    }
                    Err(_) => break,
                };

                self.iteration.next_iteration();
                let local_bunches = self.generate_bunches(&free_executor_list)?;

                if local_bunches.is_empty() {
                    break;
                }

                self.add_columns(self.iteration.iteration, local_bunches, model)?;

                // 局部固定
                let new_fixed = self.locally_fix(
                    self.iteration.iteration,
                    &current_fixed,
                    &current_ip_solution,
                    model,
                )?;
                if !new_fixed.is_empty() {
                    // 仅当一个执行器的全部时隙 group 已固定时才移除它。
                    // Remove an executor only after every slot group is fixed.
                    free_executor_list = self.select_free_executors();
                } else {
                    break;
                }

                // 列移除
                if self.context.column_count() > self.configuration.max_column_amount {
                    let shadow_prices_clone = self.shadow_prices.clone();
                    maximum_reduced_cost_2 =
                        self.remove_columns(maximum_reduced_cost_2, &shadow_prices_clone, model)?;
                }
            }

            // ---- 最终 MILP ----
            if let Ok(ip_result) = self.solve_milp(model) {
                self.after_milp_solve(model, &ip_result.solution);
                current_ip_solution = ip_result.solution.clone();
                if self.iteration.record_ip(ip_result.obj) {
                    solution = self
                        .context
                        .analyze_solution_with_lifecycle(&current_ip_solution, &self.lifecycle);
                    self.best_solution = Some(solution.clone());
                    self.best_obj = ip_result.obj;
                    self.fixed_bunches = self.context.extract_fixed(&current_ip_solution);
                    self.kept_bunches = self.context.extract_kept(&current_ip_solution);

                    if ip_result.obj == 0.0 {
                        break;
                    }
                }
            }

            // 刷新 + 步长减半
            self.flush(model)?;
            self.iteration.halve_step();
            maximum_reduced_cost_1 = 50.0;
        }

        self.best_solution
            .clone()
            .ok_or_else(|| GanttError::Calculation {
                message: "No feasible solution found".to_string(),
            })
    }

    /// 求解分支节点 / Solve a branch node
    ///
    /// 将节点决策映射到领域列状态，再复用当前单节点分支定价主流程。
    /// Maps node decisions to domain column state, then reuses the current single-node
    /// branch-and-price main flow.
    pub fn solve_branch_node(
        &mut self,
        node: &BranchNode,
        model: &mut MetaModel<f64>,
    ) -> BranchNodeSolveOutput<BunchSolution> {
        let snapshot = self.snapshot();
        self.apply_branch_node(node);
        let output = match self.run(model) {
            Ok(solution) => BranchNodeSolveOutput {
                lower_bound: self.iteration.lower_bound,
                branch_target: self.next_branch_target_for_solution(&solution),
                solution: Some(solution),
                objective: Some(self.best_obj),
                infeasible: false,
            },
            Err(_) => BranchNodeSolveOutput::infeasible(),
        };
        self.restore(snapshot);
        output
    }

    /// 使用 fresh 模型求解分支节点 / Solve a branch node with a fresh model
    ///
    /// 该入口在每个节点求解前快照 application 与 context 状态，并使用调用方构建的
    /// fresh `MetaModel` 执行求解。求解结束后丢弃模型并恢复算法实例状态，避免同一算法
    /// 实例连续求解兄弟节点时污染 context 注册态。
    ///
    /// This entry snapshots application and context state before each node solve,
    /// then runs with a caller-built fresh `MetaModel`. After solving, the model is
    /// discarded and the algorithm instance is restored so sibling node solves do
    /// not leak context registration state.
    pub fn solve_branch_node_with_fresh_model<F>(
        &mut self,
        node: &BranchNode,
        mut build_model: F,
    ) -> BranchNodeSolveOutput<BunchSolution>
    where
        C: Clone,
        F: FnMut() -> MetaModel<f64>,
    {
        let snapshot = self.isolated_snapshot();
        let mut model = build_model();
        self.apply_branch_node(node);
        let output = match self.run(&mut model) {
            Ok(solution) => BranchNodeSolveOutput {
                lower_bound: self.iteration.lower_bound,
                branch_target: self.next_branch_target_for_solution(&solution),
                solution: Some(solution),
                objective: Some(self.best_obj),
                infeasible: false,
            },
            Err(_) => BranchNodeSolveOutput::infeasible(),
        };
        self.restore_isolated(snapshot);
        output
    }

    // ---- 内部方法 / Internal methods ----

    /// 创建 application 状态快照 / Create application-state snapshot
    fn snapshot(&self) -> BunchBranchAndPriceSnapshot<C::ExecutorId> {
        BunchBranchAndPriceSnapshot {
            iteration: self.iteration.clone(),
            shadow_prices: self.shadow_prices.clone(),
            executor_slot_shadow_prices: self.executor_slot_shadow_prices.clone(),
            fixed_bunches: self.fixed_bunches.clone(),
            kept_bunches: self.kept_bunches.clone(),
            hidden_executors: self.hidden_executors.clone(),
            branch_groups: self.branch_groups.clone(),
            model_state: self.model_state.clone(),
            lifecycle: self.lifecycle.clone(),
            best_solution: self.best_solution.clone(),
            best_obj: self.best_obj,
            bunch_index_counter: self.bunch_index_counter,
        }
    }

    /// 创建隔离求解快照 / Create isolated-solve snapshot
    fn isolated_snapshot(&self) -> BunchBranchAndPriceIsolatedSnapshot<C>
    where
        C: Clone,
    {
        BunchBranchAndPriceIsolatedSnapshot {
            application: self.snapshot(),
            context: self.context.clone(),
        }
    }

    /// 恢复 application 状态快照 / Restore application-state snapshot
    fn restore(&mut self, snapshot: BunchBranchAndPriceSnapshot<C::ExecutorId>) {
        self.iteration = snapshot.iteration;
        self.shadow_prices = snapshot.shadow_prices;
        self.executor_slot_shadow_prices = snapshot.executor_slot_shadow_prices;
        self.fixed_bunches = snapshot.fixed_bunches;
        self.kept_bunches = snapshot.kept_bunches;
        self.hidden_executors = snapshot.hidden_executors;
        self.branch_groups = snapshot.branch_groups;
        self.model_state = snapshot.model_state;
        self.lifecycle = snapshot.lifecycle;
        self.best_solution = snapshot.best_solution;
        self.best_obj = snapshot.best_obj;
        self.bunch_index_counter = snapshot.bunch_index_counter;
    }

    /// 恢复隔离求解快照 / Restore isolated-solve snapshot
    fn restore_isolated(&mut self, snapshot: BunchBranchAndPriceIsolatedSnapshot<C>) {
        self.context = snapshot.context;
        self.restore(snapshot.application);
    }

    /// 应用分支节点决策 / Apply branch-node decisions
    fn apply_branch_node(&mut self, node: &BranchNode) {
        for decision in &node.decisions {
            self.lifecycle
                .fix_binary_column(decision.target_index, decision.fixed_value);
            if decision.fixed_value == 0 {
                self.fixed_bunches.remove(&decision.target_index);
                self.kept_bunches.remove(&decision.target_index);
            } else {
                self.fixed_bunches.insert(decision.target_index);
            }
        }
        self.refresh_branch_groups();
        self.sync_model_state_from_lifecycle();
    }

    /// 选择下一个分支目标 / Select next branch target
    fn next_branch_target(&self) -> Option<usize> {
        self.kept_bunches
            .iter()
            .chain(self.fixed_bunches.iter())
            .copied()
            .find(|bunch_index| self.lifecycle.state().is_selectable(*bunch_index))
    }

    /// 从节点解选择分支目标 / Select branch target from node solution
    fn next_branch_target_for_solution(&self, solution: &BunchSolution) -> Option<usize> {
        self.next_branch_target().or_else(|| {
            solution
                .selected_bunches
                .iter()
                .copied()
                .find(|bunch_index| self.lifecycle.state().is_selectable(*bunch_index))
        })
    }

    /// 求解 MILP / Solve MILP
    fn solve_milp(&self, model: &mut MetaModel<f64>) -> GanttResult<FeasibleSolution> {
        self.lifecycle.apply_solution_to_model(model);
        let options = FrameworkSolveOptions::new();
        solve_with_options_sync(&self.solver, model, options).map_err(|e| GanttError::Calculation {
            message: format!("MILP solve failed: {:?}", e),
        })
    }

    /// 求解 RMP LP / Solve RMP LP
    fn solve_rmp_lp(&mut self, model: &mut MetaModel<f64>) -> GanttResult<LPResult> {
        let options = FrameworkSolveOptions::new();
        let triad_model =
                model
                .try_to_linear_triad_model()
                .map_err(|e| GanttError::Calculation {
                    message: format!("Failed to convert model: {:?}", e),
                })?;

        // LP 对偶向量按展平模型的约束顺序返回。每次动态加列或刷新模型后，
        // 重新从当前约束名构建索引，确保 task/executor-slot 对偶不会读取过期位置。
        // LP duals are returned in flattened constraint order. Rebuild the index after
        // every dynamic model refresh so task and executor-slot duals use current rows.
        self.constraint_name_to_index = triad_model
            .basic
            .constraint_names
            .iter()
            .enumerate()
            .map(|(index, name)| (name.clone(), index))
            .collect();
        self.constraint_index_map = self
            .context
            .build_constraint_index_map(&self.constraint_name_to_index);

        let result = solve_lp_with_options_sync(&self.solver, &triad_model, options).map_err(|e| {
            GanttError::Calculation {
                message: format!("LP solve failed: {:?}", e),
            }
        })?;
        model.set_solution(&result.result.solution);
        Ok(result)
    }

    /// 提取影子价格 / Extract shadow prices
    fn extract_shadow_prices(&self, lp_result: &LPResult) -> HashMap<usize, f64> {
        self.context.extract_shadow_price_with_index_map(
            &lp_result.dual_solution,
            &self.constraint_index_map,
            &self.constraint_name_to_index,
        )
    }

    /// 添加列 / Add columns
    fn add_columns(
        &mut self,
        iteration: usize,
        new_bunches: Vec<BunchEntry<C::ExecutorId>>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<Vec<usize>> {
        // 更新束索引
        let offset = self.bunch_index_counter;
        let mut updated_bunches = Vec::with_capacity(new_bunches.len());
        for mut bunch in new_bunches {
            bunch.index = offset + bunch.index;
            bunch.iteration = iteration;
            updated_bunches.push(bunch);
        }
        self.bunch_index_counter += updated_bunches.len();

        let added = self.context.add_columns(iteration, updated_bunches, model)?;
        self.refresh_branch_groups();
        Ok(added)
    }

    /// MILP 求解后刷新生命周期 / Refresh lifecycle after MILP solve
    fn after_milp_solve(&mut self, model: &mut MetaModel<f64>, solution: &[f64]) {
        self.lifecycle.set_solution_to_model(model, solution.to_vec());
        self.lifecycle
            .set_warm_start_from_solution(self.configuration.local_fix_threshold);
        self.sync_model_state_from_lifecycle();
    }

    /// 隐藏执行器 / Hide executors
    fn hide_executors(&mut self, solution: &[f64]) {
        self.hidden_executors
            .extend(self.context.extract_hidden_executors(solution));
    }

    /// 选择自由执行器 / Select free executors
    fn select_free_executors(&self) -> Vec<C::ExecutorId> {
        self.executor_ids
            .iter()
            .filter(|id| {
                !self.hidden_executors.contains(*id)
                    && !self.branch_groups.is_executor_fully_fixed(id)
            })
            .cloned()
            .collect()
    }

    /// 提取执行器-时隙影子价格 / Extract executor-slot shadow prices
    fn extract_executor_slot_shadow_prices(
        &self,
        lp_result: &LPResult,
    ) -> HashMap<(C::ExecutorId, usize), f64> {
        self.context.extract_executor_slot_shadow_prices(
            &lp_result.dual_solution,
            &self.constraint_index_map,
        )
    }

    /// 全局固定 / Globally fix
    fn globally_fix(
        &mut self,
        free_executors: &[C::ExecutorId],
        model: &mut MetaModel<f64>,
    ) -> GanttResult<HashSet<usize>> {
        let free_set = free_executors.iter().cloned().collect::<HashSet<_>>();
        let current_fixed = self
            .fixed_bunches
            .iter()
            .copied()
            .filter(|bunch_index| {
                self.context.get_bunch_entry(*bunch_index).map_or(true, |entry| {
                    entry.slot_index.is_some() || !free_set.contains(&entry.executor_id)
                })
            })
            .collect::<HashSet<_>>();

        self.context.globally_fix_in_model(&current_fixed, model)?;
        self.lifecycle.fix_columns(current_fixed.iter().copied());
        self.fixed_bunches = current_fixed.clone();
        self.refresh_branch_groups();
        self.sync_model_state_from_lifecycle();
        Ok(current_fixed)
    }

    /// 局部固定 / Locally fix
    fn locally_fix(
        &mut self,
        iteration: usize,
        current_fixed: &HashSet<usize>,
        solution: &[f64],
        model: &mut MetaModel<f64>,
    ) -> GanttResult<HashSet<usize>> {
        let newly_fixed = self.context.locally_fix_in_model(
            iteration,
            self.configuration.local_fix_threshold,
            solution,
            current_fixed,
            model,
        )?;
        self.fixed_bunches.extend(newly_fixed.iter().copied());
        self.refresh_branch_groups();
        self.lifecycle.fix_columns(newly_fixed.iter().copied());
        self.sync_model_state_from_lifecycle();
        Ok(newly_fixed)
    }

    /// 移除列 / Remove columns
    fn remove_columns(
        &mut self,
        maximum_reduced_cost: f64,
        _shadow_prices: &HashMap<usize, f64>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<f64> {
        let cutoff = (maximum_reduced_cost.floor() as i64 * 2 / 3).max(5) as f64;
        let removing = (0..self.bunch_index_counter)
            .filter(|bunch_index| {
                if self.fixed_bunches.contains(bunch_index)
                    || self.kept_bunches.contains(bunch_index)
                    || self.lifecycle.state().is_removed(*bunch_index)
                {
                    return false;
                }
                self.context
                    .get_bunch_entry(*bunch_index)
                    .map(|entry| self.policy.reduced_cost(_shadow_prices, &entry) >= maximum_reduced_cost)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        if removing.is_empty() {
            return Ok(maximum_reduced_cost);
        }

        self.lifecycle.remove_columns(removing.iter().copied());
        self.context.hide_bunches_in_model(&removing, model)?;
        self.context.remove_columns(&removing);
        self.sync_model_state_from_lifecycle();

        if self.context.column_count() > self.configuration.max_column_amount {
            Ok(cutoff)
        } else {
            Ok(maximum_reduced_cost)
        }
    }

    /// 刷新状态 / Flush state
    fn flush(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        self.fixed_bunches.clear();
        self.kept_bunches.clear();
        self.hidden_executors.clear();
        self.branch_groups.clear_fixed();
        self.shadow_prices.clear();
        self.lifecycle.flush();
        model.flush(false);
        self.context.flush();
        self.context.apply_lifecycle(&mut self.lifecycle);
        self.context.restore_non_removed_ranges_in_model(model)?;
        self.context.hide_bunches_in_model(&self.lifecycle.state().removed_columns(), model)?;
        self.sync_model_state_from_lifecycle();
        Ok(())
    }

    /// 同步兼容字段 / Sync compatibility field
    pub(crate) fn sync_model_state_from_lifecycle(&mut self) {
        self.model_state = self.lifecycle.column_state_facade();
    }

    /// 构造并执行定价请求 / Build and execute pricing request
    fn generate_bunches(
        &self,
        executor_ids: &[C::ExecutorId],
    ) -> GanttResult<Vec<BunchEntry<C::ExecutorId>>> {
        let kept_groups = self
            .kept_bunches
            .iter()
            .filter_map(|bunch_index| self.context.get_bunch_entry(*bunch_index))
            .map(|bunch| BranchGroup::from(&bunch))
            .collect();
        self.policy.generate_bunches_with_request(&BunchPricingRequest {
            iteration: self.iteration.iteration,
            executor_ids: executor_ids.to_vec(),
            shadow_prices: self.shadow_prices.clone(),
            executor_slot_shadow_prices: self.executor_slot_shadow_prices.clone(),
            fixed_groups: self.branch_groups.fixed_groups().clone(),
            kept_groups,
            hidden_executors: self.hidden_executors.clone(),
            min_column_amount_per_executor: self.configuration.min_column_amount_per_executor,
        })
    }

    /// 根据当前固定列重建分支 group 状态 / Rebuild branch groups from current fixed columns
    fn refresh_branch_groups(&mut self) {
        self.branch_groups.clear_fixed();
        for bunch_index in 0..self.bunch_index_counter {
            let Some(bunch) = self.context.get_bunch_entry(bunch_index) else {
                continue;
            };
            if self.fixed_bunches.contains(&bunch_index) {
                self.branch_groups.mark_fixed(&bunch);
            } else {
                self.branch_groups.observe(&bunch);
            }
        }
    }
}

#[cfg(feature = "async")]
fn solve_lp_with_options_sync<S>(
    solver: &S,
    model: &ospf_rust_core::model::intermediate::LinearTriadModel,
    options: FrameworkSolveOptions,
) -> ospf_rust_core::error::Result<LPResult>
where
    S: ColumnGenerationSolver,
{
    futures::executor::block_on(solver.solve_lp_with_options(model, options))
}

#[cfg(not(feature = "async"))]
fn solve_lp_with_options_sync<S>(
    solver: &S,
    model: &ospf_rust_core::model::intermediate::LinearTriadModel,
    options: FrameworkSolveOptions,
) -> ospf_rust_core::error::Result<LPResult>
where
    S: ColumnGenerationSolver,
{
    solver.solve_lp_with_options(model, options)
}

#[cfg(feature = "async")]
fn solve_with_options_sync<S>(
    solver: &S,
    model: &MetaModel<f64>,
    options: FrameworkSolveOptions,
) -> ospf_rust_core::error::Result<FeasibleSolution>
where
    S: ColumnGenerationSolver,
{
    futures::executor::block_on(solver.solve_with_options(model, options))
}

#[cfg(not(feature = "async"))]
fn solve_with_options_sync<S>(
    solver: &S,
    model: &MetaModel<f64>,
    options: FrameworkSolveOptions,
) -> ospf_rust_core::error::Result<FeasibleSolution>
where
    S: ColumnGenerationSolver,
{
    solver.solve_with_options(model, options)
}

#[cfg(test)]
mod tests {
    use std::fmt::{Display, Formatter};

    use super::*;
    use ospf_rust_core::error::{CoreError, Result as CoreResult, SolverError};
    use ospf_rust_core::model::intermediate::LinearTriadModel;
    use ospf_rust_framework::solver::column_generation_solver::LinearDualSolution;

    #[test]
    fn branch_group_tracker_keeps_executor_until_all_slots_are_fixed() {
        let mut tracker = BranchGroupTracker::<ExecutorId>::default();
        let first = BunchEntry {
            index: 0,
            executor_id: "exec_1".into(),
            task_indices: vec![0],
            cost: 1.0,
            iteration: 0,
            slot_index: Some(0),
        };
        let second = BunchEntry {
            index: 1,
            executor_id: "exec_1".into(),
            task_indices: vec![1],
            cost: 1.0,
            iteration: 0,
            slot_index: Some(1),
        };
        tracker.observe(&first);
        tracker.observe(&second);
        tracker.mark_fixed(&first);
        assert!(!tracker.is_executor_fully_fixed(&"exec_1".into()));
        tracker.mark_fixed(&second);
        assert!(tracker.is_executor_fully_fixed(&"exec_1".into()));
    }

    #[test]
    fn test_bunch_branch_and_price_policy_defaults() {
        let policy = ColumnGenerationPolicy::default();
        assert_eq!(policy.max_iterations, 100);
        assert_eq!(policy.max_column_amount, 50000);
        assert!((policy.local_fix_threshold - 0.9).abs() < f64::EPSILON);
    }

    #[derive(Debug, Clone)]
    struct EmptyBunchPolicy;

    impl BunchCGPolicy for EmptyBunchPolicy {
        fn build_shadow_price_map(&self) -> HashMap<usize, f64> {
            HashMap::new()
        }

        fn reduced_cost(&self, _shadow_prices: &HashMap<usize, f64>, bunch: &BunchEntry) -> f64 {
            bunch.cost
        }

        fn generate_bunches(
            &self,
            _iteration: usize,
            _executor_ids: &[ExecutorId],
            _shadow_prices: &HashMap<usize, f64>,
        ) -> Vec<BunchEntry> {
            Vec::new()
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
    struct WorkUnitId(u64);

    impl Display for WorkUnitId {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
            write!(formatter, "{}", self.0)
        }
    }

    impl ExecutorIdTrait for WorkUnitId {}

    #[derive(Debug, Clone)]
    struct WorkUnitBunchPolicy;

    impl BunchCGPolicy<WorkUnitId> for WorkUnitBunchPolicy {
        fn build_shadow_price_map(&self) -> HashMap<usize, f64> {
            HashMap::new()
        }

        fn reduced_cost(
            &self,
            _shadow_prices: &HashMap<usize, f64>,
            bunch: &BunchEntry<WorkUnitId>,
        ) -> f64 {
            bunch.cost
        }

        fn generate_bunches(
            &self,
            _iteration: usize,
            _executor_ids: &[WorkUnitId],
            _shadow_prices: &HashMap<usize, f64>,
        ) -> Vec<BunchEntry<WorkUnitId>> {
            Vec::new()
        }
    }

    #[derive(Debug, Clone)]
    struct MockColumnGenerationSolver;

    #[cfg_attr(feature = "async", async_trait::async_trait)]
    impl ColumnGenerationSolver for MockColumnGenerationSolver {
        fn name(&self) -> &str {
            "mock_gantt_bunch_column_generation"
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
    fn test_business_executor_id_builds_and_solves_minimal_model() {
        let executor_id = WorkUnitId(42);
        let context = crate::domain::bunch_compilation::context::BasicBunchCompilationContext::<
            WorkUnitId,
        >::new_with_ids(1, vec![executor_id.clone()], false);
        let mut algorithm = BunchBranchAndPriceAlgorithm::new(
            context,
            MockColumnGenerationSolver,
            WorkUnitBunchPolicy,
            vec![executor_id.clone()],
            ColumnGenerationPolicy::default(),
        );
        let mut model = MetaModel::<f64>::new("work_unit_id_minimal_model");

        let solution = algorithm.run(&mut model).unwrap();

        assert!(solution.selected_bunches.is_empty());
        assert_eq!(algorithm.context.compilation.base.executor_ids, vec![executor_id]);
    }

    #[derive(Debug, Clone)]
    struct FailingColumnGenerationSolver;

    #[cfg_attr(feature = "async", async_trait::async_trait)]
    impl ColumnGenerationSolver for FailingColumnGenerationSolver {
        fn name(&self) -> &str {
            "failing_gantt_bunch_column_generation"
        }

        #[cfg(feature = "async")]
        async fn solve_milp_with_options(
            &self,
            _model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> CoreResult<FeasibleSolution> {
            Err(CoreError::Solver(SolverError::Infeasible))
        }

        #[cfg(not(feature = "async"))]
        fn solve_milp_with_options(
            &self,
            _model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> CoreResult<FeasibleSolution> {
            Err(CoreError::Solver(SolverError::Infeasible))
        }

        #[cfg(feature = "async")]
        async fn solve_lp_with_options(
            &self,
            _model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> CoreResult<LPResult> {
            Err(CoreError::Solver(SolverError::Infeasible))
        }

        #[cfg(not(feature = "async"))]
        fn solve_lp_with_options(
            &self,
            _model: &LinearTriadModel,
            _options: FrameworkSolveOptions,
        ) -> CoreResult<LPResult> {
            Err(CoreError::Solver(SolverError::Infeasible))
        }
    }

    #[test]
    fn test_branch_node_decisions_update_column_state() {
        let context = crate::domain::bunch_compilation::context::BasicBunchCompilationContext::new(
            1,
            vec!["exec_1".to_string()],
            false,
        );
        let mut algorithm = BunchBranchAndPriceAlgorithm::new(
            context,
            MockColumnGenerationSolver,
            EmptyBunchPolicy,
            vec!["exec_1".to_string()],
            ColumnGenerationPolicy::default(),
        );
        algorithm.kept_bunches.insert(1);
        algorithm.fixed_bunches.insert(2);
        let node = BranchNode {
            id: 1,
            parent_id: Some(0),
            depth: 1,
            decisions: vec![
                crate::application::algorithm::BranchDecision::zero(
                    1,
                    crate::application::algorithm::BranchDirection::Left,
                ),
                crate::application::algorithm::BranchDecision::one(
                    3,
                    crate::application::algorithm::BranchDirection::Right,
                ),
            ],
            lower_bound: 0.0,
            upper_bound: None,
            status: crate::application::algorithm::BranchNodeStatus::Pending,
        };

        algorithm.apply_branch_node(&node);

        assert!(algorithm.model_state.is_hidden(1));
        assert!(!algorithm.kept_bunches.contains(&1));
        assert!(algorithm.model_state.is_fixed(3));
        assert!(algorithm.fixed_bunches.contains(&3));
        let branch_target = algorithm.next_branch_target().unwrap();
        assert_ne!(branch_target, 1);
        assert!(algorithm.model_state.is_selectable(branch_target));
    }

    #[test]
    fn test_branch_node_snapshot_restore_isolates_sibling_state() {
        let context = crate::domain::bunch_compilation::context::BasicBunchCompilationContext::new(
            1,
            vec!["exec_1".to_string()],
            false,
        );
        let mut algorithm = BunchBranchAndPriceAlgorithm::new(
            context,
            MockColumnGenerationSolver,
            EmptyBunchPolicy,
            vec!["exec_1".to_string()],
            ColumnGenerationPolicy::default(),
        );
        algorithm.kept_bunches.insert(1);
        algorithm.fixed_bunches.insert(9);
        algorithm.shadow_prices.insert(0, 2.0);
        algorithm.iteration.next_iteration();
        algorithm.iteration.record_lp(7.0);
        algorithm.iteration.record_ip(8.0);
        algorithm.best_solution = Some(BunchSolution {
            selected_bunches: vec![9],
            canceled_tasks: vec![],
        });
        algorithm.best_obj = 3.0;

        let root_snapshot = algorithm.snapshot();
        let left_node = BranchNode {
            id: 1,
            parent_id: Some(0),
            depth: 1,
            decisions: vec![crate::application::algorithm::BranchDecision::zero(
                1,
                crate::application::algorithm::BranchDirection::Left,
            )],
            lower_bound: 0.0,
            upper_bound: None,
            status: crate::application::algorithm::BranchNodeStatus::Pending,
        };
        let right_node = BranchNode {
            id: 2,
            parent_id: Some(0),
            depth: 1,
            decisions: vec![crate::application::algorithm::BranchDecision::one(
                2,
                crate::application::algorithm::BranchDirection::Right,
            )],
            lower_bound: 0.0,
            upper_bound: None,
            status: crate::application::algorithm::BranchNodeStatus::Pending,
        };

        algorithm.apply_branch_node(&left_node);
        assert!(algorithm.model_state.is_hidden(1));
        assert!(!algorithm.kept_bunches.contains(&1));

        algorithm.restore(root_snapshot.clone());
        assert!(!algorithm.model_state.is_hidden(1));
        assert!(algorithm.kept_bunches.contains(&1));
        assert_eq!(algorithm.shadow_prices.get(&0), Some(&2.0));
        assert_eq!(algorithm.iteration.iteration, 1);
        assert_eq!(algorithm.iteration.best_lp_obj, 7.0);
        assert_eq!(algorithm.iteration.best_ip_obj, 8.0);
        assert_eq!(algorithm.best_obj, 3.0);

        algorithm.apply_branch_node(&right_node);
        assert!(algorithm.model_state.is_fixed(2));
        assert!(!algorithm.model_state.is_hidden(1));

        algorithm.restore(root_snapshot);
        assert!(!algorithm.model_state.is_fixed(2));
        assert!(algorithm.fixed_bunches.contains(&9));
        assert_eq!(
            algorithm.best_solution.as_ref().unwrap().selected_bunches,
            vec![9],
        );
    }

    #[test]
    fn test_after_milp_solve_writes_solution_to_model() {
        let context = crate::domain::bunch_compilation::context::BasicBunchCompilationContext::new(
            1,
            vec!["exec_1".to_string()],
            false,
        );
        let mut algorithm = BunchBranchAndPriceAlgorithm::new(
            context,
            MockColumnGenerationSolver,
            EmptyBunchPolicy,
            vec!["exec_1".to_string()],
            ColumnGenerationPolicy::default(),
        );
        let mut model = MetaModel::<f64>::new("test_after_milp_solution_write");
        model
            .as_basic_mut()
            .register_auto_variable::<ospf_rust_core::variable::Binary>("x")
            .unwrap();

        algorithm.after_milp_solve(&mut model, &[1.0]);

        assert_eq!(algorithm.lifecycle.solution(), Some([1.0].as_slice()));
        assert_eq!(model.solution_by_solver_order(), vec![Some(1.0)]);
    }

    #[test]
    fn test_solve_branch_node_restores_application_state_after_failure() {
        let context = crate::domain::bunch_compilation::context::BasicBunchCompilationContext::new(
            1,
            vec!["exec_1".to_string()],
            false,
        );
        let mut algorithm = BunchBranchAndPriceAlgorithm::new(
            context,
            FailingColumnGenerationSolver,
            EmptyBunchPolicy,
            vec!["exec_1".to_string()],
            ColumnGenerationPolicy::default(),
        );
        algorithm.kept_bunches.insert(1);
        algorithm.shadow_prices.insert(0, 2.0);
        algorithm.iteration.next_iteration();
        algorithm.iteration.record_lp(7.0);
        algorithm.best_obj = 3.0;
        let node = BranchNode {
            id: 1,
            parent_id: Some(0),
            depth: 1,
            decisions: vec![crate::application::algorithm::BranchDecision::zero(
                1,
                crate::application::algorithm::BranchDirection::Left,
            )],
            lower_bound: 0.0,
            upper_bound: None,
            status: crate::application::algorithm::BranchNodeStatus::Pending,
        };
        let mut model = MetaModel::<f64>::new("test_branch_node_failure_restore");

        let output = algorithm.solve_branch_node(&node, &mut model);

        assert!(output.infeasible);
        assert!(!algorithm.model_state.is_hidden(1));
        assert!(algorithm.kept_bunches.contains(&1));
        assert_eq!(algorithm.shadow_prices.get(&0), Some(&2.0));
        assert_eq!(algorithm.iteration.iteration, 1);
        assert_eq!(algorithm.iteration.best_lp_obj, 7.0);
        assert_eq!(algorithm.best_obj, 3.0);
    }

    #[test]
    fn test_solve_branch_node_with_fresh_model_restores_context_state() {
        #[derive(Debug, Clone)]
        struct OneBunchPolicy;

        impl BunchCGPolicy for OneBunchPolicy {
            fn build_shadow_price_map(&self) -> HashMap<usize, f64> {
                HashMap::new()
            }

            fn reduced_cost(
                &self,
                _shadow_prices: &HashMap<usize, f64>,
                bunch: &BunchEntry,
            ) -> f64 {
                bunch.cost
            }

            fn generate_bunches(
                &self,
                iteration: usize,
                executor_ids: &[ExecutorId],
                _shadow_prices: &HashMap<usize, f64>,
            ) -> Vec<BunchEntry> {
                executor_ids
                    .first()
                    .map(|executor_id| BunchEntry {
                        index: 0,
                        executor_id: executor_id.clone(),
                        task_indices: vec![0],
                        cost: 1.0,
                        iteration,
                        slot_index: None,
                    })
                    .into_iter()
                    .collect()
            }
        }

        let context = crate::domain::bunch_compilation::context::BasicBunchCompilationContext::new(
            1,
            vec!["exec_1".to_string()],
            false,
        );
        let mut configuration = ColumnGenerationPolicy::default();
        configuration.max_column_amount = 0;
        let mut algorithm = BunchBranchAndPriceAlgorithm::new(
            context,
            MockColumnGenerationSolver,
            OneBunchPolicy,
            vec!["exec_1".to_string()],
            configuration,
        );
        algorithm.kept_bunches.insert(7);
        let node = BranchNode {
            id: 1,
            parent_id: Some(0),
            depth: 1,
            decisions: vec![crate::application::algorithm::BranchDecision::zero(
                7,
                crate::application::algorithm::BranchDirection::Left,
            )],
            lower_bound: 0.0,
            upper_bound: None,
            status: crate::application::algorithm::BranchNodeStatus::Pending,
        };

        let output = algorithm.solve_branch_node_with_fresh_model(&node, || {
            MetaModel::<f64>::new("test_fresh_branch_node")
        });

        assert!(!output.infeasible);
        assert_eq!(output.objective, Some(0.0));
        assert_eq!(algorithm.context.column_count(), 0);
        assert!(algorithm.context.compilation.base.y_indices.is_empty());
        assert!(
            algorithm
                .context
                .compilation
                .base
                .task_compilation_symbols
                .is_empty()
        );
        assert_eq!(algorithm.bunch_index_counter, 0);
        assert!(!algorithm.model_state.is_hidden(7));
        assert!(algorithm.kept_bunches.contains(&7));
    }

    #[test]
    fn test_solve_branch_node_with_fresh_model_restores_lifecycle_state() {
        let context = crate::domain::bunch_compilation::context::BasicBunchCompilationContext::new(
            1,
            vec!["exec_1".to_string()],
            false,
        );
        let mut algorithm = BunchBranchAndPriceAlgorithm::new(
            context,
            MockColumnGenerationSolver,
            EmptyBunchPolicy,
            vec!["exec_1".to_string()],
            ColumnGenerationPolicy::default(),
        );
        algorithm.lifecycle.set_solution(vec![0.0, 1.0]);
        algorithm.lifecycle.set_warm_start([1]);
        algorithm.sync_model_state_from_lifecycle();
        let snapshot = algorithm.lifecycle.snapshot();
        let left_node = BranchNode {
            id: 1,
            parent_id: Some(0),
            depth: 1,
            decisions: vec![crate::application::algorithm::BranchDecision::zero(
                1,
                crate::application::algorithm::BranchDirection::Left,
            )],
            lower_bound: 0.0,
            upper_bound: None,
            status: crate::application::algorithm::BranchNodeStatus::Pending,
        };
        let right_node = BranchNode {
            id: 2,
            parent_id: Some(0),
            depth: 1,
            decisions: vec![crate::application::algorithm::BranchDecision::one(
                2,
                crate::application::algorithm::BranchDirection::Right,
            )],
            lower_bound: 0.0,
            upper_bound: None,
            status: crate::application::algorithm::BranchNodeStatus::Pending,
        };

        let left = algorithm.solve_branch_node_with_fresh_model(&left_node, || {
            MetaModel::<f64>::new("test_lifecycle_left_node")
        });
        let right = algorithm.solve_branch_node_with_fresh_model(&right_node, || {
            MetaModel::<f64>::new("test_lifecycle_right_node")
        });

        assert!(!left.infeasible);
        assert!(!right.infeasible);
        assert_eq!(algorithm.lifecycle.solution(), snapshot.solution.as_deref());
        assert_eq!(
            algorithm.lifecycle.warm_start_columns(),
            snapshot.state.warm_start_columns(),
        );
        assert_eq!(algorithm.lifecycle.flush_count(), snapshot.flush_count);
        assert!(!algorithm.lifecycle.state().is_hidden(1));
        assert!(!algorithm.lifecycle.state().is_fixed(2));
    }
}
