//! 任务束编译服务 / Bunch compilation services
//!
//! 实现任务束编译的约束和目标 Pipeline。
//! Implements constraint and objective pipelines for bunch compilation.

pub mod limits {
    //! 任务束编译限制与目标族 / Bunch compilation limits and objective families
    //!
    //! 实现束级编译约束、任务级约束和成本目标。
    //! Implements bunch-level compilation constraints, task-level constraints, and cost objectives.

    use ospf_rust_core::error::Result;
    use ospf_rust_core::model::MetaModel;
    use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
    use ospf_rust_core::model::mechanism::constraint_group::ConstraintGroup;
    use ospf_rust_core::model::object::SubObjective;
    use ospf_rust_framework::model::pipeline::Pipeline;

    use crate::domain::bunch_compilation::model::BunchCompilation;
    use crate::domain::bunch_compilation::slot_based::SlotBasedBunchCompilationContext;
    use crate::domain::common::ExecutorIdTrait;

    // ============================================================================
    // 约束型 Pipeline / Constraint Pipelines
    // ============================================================================

    /// 任务编译约束（束模式）/ Task compilation constraint (bunch mode)
    ///
    /// 确保每个任务被恰好分配到一个束或被取消：
    /// `taskCompilation[task] == 1` 即 `sum(x[bunch] for bunch containing task) + y[task] == 1`
    ///
    /// Ensures each task is assigned to exactly one bunch or canceled:
    /// `taskCompilation[task] == 1` i.e. `sum(x[bunch] for bunch containing task) + y[task] == 1`
    /// 任务编译约束（束模式）
    ///
    /// `task_polynomials` 是注册期构建缓冲区：在 `from_compilation()` 中从 BunchCompilation 状态
    /// 预计算，在 `Pipeline::register()` 期间消费以注册约束。创建后不再修改。
    ///
    /// Task compilation constraint (bunch mode).
    /// `task_polynomials` is a register-time builder buffer: pre-computed from BunchCompilation
    /// state in `from_compilation()`, consumed during `Pipeline::register()` to register constraints.
    /// Not modified after creation.
    #[derive(Debug)]
    pub struct BunchTaskCompilationConstraint {
        name: String,
        group: Option<ConstraintGroup>,
        /// 每个任务的编译多项式项：task_idx -> [(var_index, coefficient)]
        /// Register-time buffer: pre-computed in `from_compilation()`, consumed in `register()`.
        pub task_polynomials: Vec<Vec<(usize, f64)>>,
    }

    impl BunchTaskCompilationConstraint {
        /// 从 BunchCompilation 创建任务编译约束 / Create from BunchCompilation
        pub fn from_compilation(compilation: &BunchCompilation) -> Self {
            let mut task_polynomials = Vec::with_capacity(compilation.n_tasks);

            for ti in 0..compilation.n_tasks {
                let mut terms = Vec::new();

                // y[task] 项
                terms.push((compilation.y_indices[ti], 1.0));

                // x[bunch] 项：遍历所有迭代的所有束
                for (iter_idx, x_indices) in compilation.x_indices.iter().enumerate() {
                    let bunches = compilation.aggregation.bunches_for_iteration(iter_idx);
                    for (local_idx, bunch) in bunches.iter().enumerate() {
                        if bunch.task_indices.contains(&ti)
                            && let Some(&x_idx) = x_indices.get(local_idx)
                        {
                            terms.push((x_idx, 1.0));
                        }
                    }
                }

                task_polynomials.push(terms);
            }

            Self {
                name: "bunch_task_compilation".to_string(),
                group: None,
                task_polynomials,
            }
        }
    }

    impl Pipeline<MetaModel<f64>> for BunchTaskCompilationConstraint {
        fn name(&self) -> &str {
            &self.name
        }
        fn constraint_group(&self) -> Option<&ConstraintGroup> {
            self.group.as_ref()
        }

        fn register(&self, model: &mut MetaModel<f64>) {
            for (ti, terms) in self.task_polynomials.iter().enumerate() {
                if let Err(e) =
                    model.add_eq_constraint(terms, 1.0, &format!("{}_{}", self.name, ti))
                {
                    log::warn!("Failed to register {}_{}: {:?}", self.name, ti, e);
                }
            }
        }

        fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
            Ok(())
        }
    }

    /// 执行器编译约束（束模式）/ Executor compilation constraint (bunch mode)
    ///
    /// 确保每个执行器被束选择或空闲变量覆盖：
    /// `executorCompilation[executor] == 1`
    ///
    /// Ensures each executor is covered by bunch selection or leisure variable:
    /// `executorCompilation[executor] == 1`
    /// 执行器编译约束（束模式）
    ///
    /// `executor_polynomials` 是注册期构建缓冲区：在 `from_compilation()` 中预计算，
    /// 在 `Pipeline::register()` 期间消费以注册约束。
    ///
    /// Executor compilation constraint (bunch mode).
    /// `executor_polynomials` is a register-time builder buffer: pre-computed in `from_compilation()`,
    /// consumed during `Pipeline::register()` to register constraints.
    #[derive(Debug)]
    pub struct BunchExecutorCompilationConstraint {
        name: String,
        group: Option<ConstraintGroup>,
        /// 每个执行器的编译多项式项 / Executor compilation polynomial terms
        /// Register-time buffer: pre-computed in `from_compilation()`, consumed in `register()`.
        pub executor_polynomials: Vec<Vec<(usize, f64)>>,
    }

    impl BunchExecutorCompilationConstraint {
        /// 从 BunchCompilation 创建执行器编译约束 / Create from BunchCompilation
        pub fn from_compilation(compilation: &BunchCompilation) -> Self {
            let mut executor_polynomials = Vec::new();

            for (ei, _) in compilation.executor_ids.iter().enumerate() {
                let mut terms = Vec::new();

                // z[executor] 项
                if compilation.with_executor_leisure {
                    terms.push((compilation.z_indices[ei], 1.0));
                }

                // x[bunch] 项：属于该执行器的束
                for (iter_idx, x_indices) in compilation.x_indices.iter().enumerate() {
                    let bunches = compilation.aggregation.bunches_for_iteration(iter_idx);
                    for (local_idx, bunch) in bunches.iter().enumerate() {
                        if bunch.executor_id == compilation.executor_ids[ei]
                            && let Some(&x_idx) = x_indices.get(local_idx)
                        {
                            terms.push((x_idx, 1.0));
                        }
                    }
                }

                executor_polynomials.push(terms);
            }

            Self {
                name: "bunch_executor_compilation".to_string(),
                group: None,
                executor_polynomials,
            }
        }
    }

    impl Pipeline<MetaModel<f64>> for BunchExecutorCompilationConstraint {
        fn name(&self) -> &str {
            &self.name
        }
        fn constraint_group(&self) -> Option<&ConstraintGroup> {
            self.group.as_ref()
        }

        fn register(&self, model: &mut MetaModel<f64>) {
            for (ei, terms) in self.executor_polynomials.iter().enumerate() {
                if let Err(e) =
                    model.add_eq_constraint(terms, 1.0, &format!("{}_{}", self.name, ei))
                {
                    log::warn!("Failed to register {}_{}: {:?}", self.name, ei, e);
                }
            }
        }

        fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
            Ok(())
        }
    }

    /// 执行器-时隙编译约束 / Executor-slot compilation constraint
    ///
    /// 每个执行器在每个时隙恰好选择一列。构造时从 slot context 读取权威
    /// `(executor, slot) -> x` 映射，保证约束与动态加列保持一致。
    /// Exactly one column is selected for each executor and slot. The constraint
    /// reads the authoritative `(executor, slot) -> x` mapping from the slot context.
    #[derive(Debug)]
    pub struct ExecutorSlotCompilationConstraint<I>
    where
        I: ExecutorIdTrait,
    {
        name: String,
        group: Option<ConstraintGroup>,
        /// 执行器-时隙线性项 / Executor-slot linear terms
        #[allow(clippy::type_complexity)]
        pub executor_slot_polynomials: Vec<(I, usize, Vec<(usize, f64)>)>,
    }

    impl<I> ExecutorSlotCompilationConstraint<I>
    where
        I: ExecutorIdTrait,
    {
        /// 从 slot compilation context 创建约束 / Create constraint from slot compilation context
        pub fn from_context<S, C>(context: &C) -> Self
        where
            S: crate::infrastructure::TimeSlot + Clone,
            C: SlotBasedBunchCompilationContext<S, ExecutorId = I>,
        {
            let mut executor_slot_polynomials = context
                .x_by_executor_slot()
                .into_iter()
                .map(|((executor_id, slot_index), indices)| {
                    (
                        executor_id,
                        slot_index,
                        indices.into_iter().map(|index| (index, 1.0)).collect(),
                    )
                })
                .collect::<Vec<_>>();
            executor_slot_polynomials
                .sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0).then_with(|| lhs.1.cmp(&rhs.1)));
            Self {
                name: "executor_slot_compilation".to_string(),
                group: None,
                executor_slot_polynomials,
            }
        }

        /// 创建空约束 / Create an empty constraint
        ///
        /// 调用方可在模型注册前填充 `executor_slot_polynomials`，用于没有具体
        /// slot context 类型信息的组合场景。
        /// Callers may populate `executor_slot_polynomials` before registration when
        /// composing without a concrete slot-context type.
        pub fn new() -> Self {
            Self {
                name: "executor_slot_compilation".to_string(),
                group: None,
                executor_slot_polynomials: Vec::new(),
            }
        }

        /// 约束名 / Constraint name
        pub fn constraint_name(executor_id: &I, slot_index: usize) -> String {
            format!("executor_slot_compilation_{}_{}", executor_id, slot_index)
        }
    }

    impl<I> Default for ExecutorSlotCompilationConstraint<I>
    where
        I: ExecutorIdTrait,
    {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<I> Pipeline<MetaModel<f64>> for ExecutorSlotCompilationConstraint<I>
    where
        I: ExecutorIdTrait,
    {
        fn name(&self) -> &str {
            &self.name
        }
        fn constraint_group(&self) -> Option<&ConstraintGroup> {
            self.group.as_ref()
        }

        fn register(&self, model: &mut MetaModel<f64>) {
            for (executor_id, slot_index, terms) in &self.executor_slot_polynomials {
                let name = Self::constraint_name(executor_id, *slot_index);
                if let Err(error) = model.add_eq_constraint(terms, 1.0, &name) {
                    log::warn!("Failed to register {}: {:?}", name, error);
                }
            }
        }

        fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
            Ok(())
        }
    }

    // ============================================================================
    // 目标型 Pipeline / Objective Pipelines
    // ============================================================================

    /// 束成本最小化 / Bunch cost minimization
    ///
    /// 最小化 `sum(bunch.cost * x[bunch])`。
    /// Minimizes total bunch cost.
    /// 束成本最小化
    ///
    /// `cost_terms` 是注册期构建缓冲区：在 `from_compilation()` 中预计算，
    /// 在 `Pipeline::register()` 期间消费以注册目标函数。
    ///
    /// Bunch cost minimization.
    /// `cost_terms` is a register-time builder buffer: pre-computed in `from_compilation()`,
    /// consumed during `Pipeline::register()` to register the objective.
    #[derive(Debug)]
    pub struct BunchCostMinimization {
        name: String,
        /// 成本项：x 变量索引和系数 / Cost terms: x variable indices and coefficients
        /// Register-time buffer: pre-computed in `from_compilation()`, consumed in `register()`.
        pub cost_terms: Vec<(usize, f64)>,
    }

    impl BunchCostMinimization {
        /// 从 BunchCompilation 创建束成本最小化 / Create from BunchCompilation
        pub fn from_compilation(compilation: &BunchCompilation) -> Self {
            let mut cost_terms = Vec::new();

            for (iter_idx, x_indices) in compilation.x_indices.iter().enumerate() {
                let bunches = compilation.aggregation.bunches_for_iteration(iter_idx);
                for (local_idx, bunch) in bunches.iter().enumerate() {
                    if bunch.cost != 0.0
                        && let Some(&x_idx) = x_indices.get(local_idx)
                    {
                        cost_terms.push((x_idx, bunch.cost));
                    }
                }
            }

            Self {
                name: "bunch_cost_minimization".to_string(),
                cost_terms,
            }
        }
    }

    impl Pipeline<MetaModel<f64>> for BunchCostMinimization {
        fn name(&self) -> &str {
            &self.name
        }
        fn constraint_group(&self) -> Option<&ConstraintGroup> {
            None
        }

        fn register(&self, model: &mut MetaModel<f64>) {
            if self.cost_terms.is_empty() {
                return;
            }
            let monomials: Vec<LinearMonomial<f64>> = self
                .cost_terms
                .iter()
                .map(|&(idx, coeff)| LinearMonomial::new(coeff, idx))
                .collect();
            let polynomial = Linear::new(monomials, 0.0);
            let sub_obj = SubObjective::minimize(polynomial, &self.name);
            model.add_sub_objective(sub_obj);
        }

        fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use std::collections::HashMap;

        use super::*;
        use crate::domain::bunch_compilation::context::{
            BasicBunchCompilationContext, IterativeBunchCompilationContext,
        };
        use crate::domain::bunch_compilation::model::{BunchCompilation, BunchEntry};
        use crate::domain::bunch_compilation::slot_based::{
            BasicSlotBasedBunchCompilationContext, StaticSlotBasedCapacityPreSolver,
        };
        use crate::domain::bunch_generation::{CapacityIntermediateValues, SlotConstraints};
        use crate::domain::task::BasicExecutor;
        use crate::infrastructure::{TimeRange, TimeSlot};
        use time::macros::datetime;

        #[derive(Debug, Clone)]
        struct TestSlot(TimeRange);

        impl TimeSlot for TestSlot {
            fn time(&self) -> &TimeRange {
                &self.0
            }

            fn sub_of(&self, sub_time: &TimeRange) -> Option<Self> {
                self.0.intersection(sub_time).map(Self)
            }
        }

        #[test]
        fn executor_slot_constraint_registers_each_pair() {
            let slots = vec![
                TestSlot(TimeRange::new(
                    datetime!(2020-08-30 08:00 UTC),
                    datetime!(2020-08-30 09:00 UTC),
                )),
                TestSlot(TimeRange::new(
                    datetime!(2020-08-30 09:00 UTC),
                    datetime!(2020-08-30 10:00 UTC),
                )),
            ];
            let executor = BasicExecutor::new("exec_1", "Executor 1");
            let values = CapacityIntermediateValues::new(
                slots.clone(),
                HashMap::from([
                    (0, SlotConstraints::default()),
                    (1, SlotConstraints::default()),
                ]),
            );
            let mut context = BasicSlotBasedBunchCompilationContext::new(
                BasicBunchCompilationContext::new(0, vec![executor.id.clone()], false),
                slots,
                Box::new(StaticSlotBasedCapacityPreSolver::new(values)),
            );
            let mut model = MetaModel::<f64>::new("executor_slot_constraint");
            context.register(&mut model).unwrap();
            context
                .add_columns(
                    0,
                    vec![
                        BunchEntry {
                            index: 0,
                            executor_id: executor.id.clone(),
                            task_indices: vec![],
                            cost: 1.0,
                            iteration: 0,
                            slot_index: Some(0),
                        },
                        BunchEntry {
                            index: 1,
                            executor_id: executor.id.clone(),
                            task_indices: vec![],
                            cost: 1.0,
                            iteration: 0,
                            slot_index: Some(1),
                        },
                    ],
                    &mut model,
                )
                .unwrap();

            let constraint =
                ExecutorSlotCompilationConstraint::from_context::<TestSlot, _>(&context);
            constraint.register(&mut model);
            assert_eq!(constraint.executor_slot_polynomials.len(), 2);
            assert_eq!(model.num_constraints(), 2);
        }

        #[test]
        fn test_bunch_task_compilation_constraint() {
            let mut model = MetaModel::<f64>::new("test_bunch_task_constraint");

            let mut compilation = BunchCompilation::new(
                2, // 2 tasks
                vec!["exec_1".to_string()],
                true,
            );
            compilation.register(&mut model).unwrap();

            // 添加初始列
            let bunches = vec![BunchEntry {
                index: 0,
                executor_id: "exec_1".into(),
                task_indices: vec![0, 1],
                cost: 10.0,
                iteration: 0,
                slot_index: None,
            }];
            compilation.add_columns(0, bunches, &mut model).unwrap();

            let constraint = BunchTaskCompilationConstraint::from_compilation(&compilation);
            assert_eq!(constraint.task_polynomials.len(), 2);
            constraint.register(&mut model);
            constraint.invoke(&model).unwrap();
        }

        #[test]
        fn test_bunch_executor_compilation_constraint() {
            let mut model = MetaModel::<f64>::new("test_bunch_exec_constraint");

            let mut compilation = BunchCompilation::new(2, vec!["exec_1".to_string()], true);
            compilation.register(&mut model).unwrap();

            let bunches = vec![BunchEntry {
                index: 0,
                executor_id: "exec_1".into(),
                task_indices: vec![0],
                cost: 5.0,
                iteration: 0,
                slot_index: None,
            }];
            compilation.add_columns(0, bunches, &mut model).unwrap();

            let constraint = BunchExecutorCompilationConstraint::from_compilation(&compilation);
            assert_eq!(constraint.executor_polynomials.len(), 1);
            constraint.register(&mut model);
            constraint.invoke(&model).unwrap();
        }

        #[test]
        fn test_bunch_cost_minimization() {
            let mut model = MetaModel::<f64>::new("test_bunch_cost");

            let mut compilation = BunchCompilation::new(2, vec!["exec_1".to_string()], true);
            compilation.register(&mut model).unwrap();

            let bunches = vec![
                BunchEntry {
                    index: 0,
                    executor_id: "exec_1".into(),
                    task_indices: vec![0],
                    cost: 5.0,
                    iteration: 0,
                    slot_index: None,
                },
                BunchEntry {
                    index: 1,
                    executor_id: "exec_1".into(),
                    task_indices: vec![1],
                    cost: 8.0,
                    iteration: 0,
                    slot_index: None,
                },
            ];
            compilation.add_columns(0, bunches, &mut model).unwrap();

            let obj = BunchCostMinimization::from_compilation(&compilation);
            assert_eq!(obj.cost_terms.len(), 2);
            obj.register(&mut model);
            obj.invoke(&model).unwrap();
        }
    }
}
