//! 甘特排程领域框架 / Gantt scheduling domain framework
//!
//! 本 crate 承接 Kotlin `ospf-kotlin-framework-gantt-scheduling` 的 Rust 迁移。
//! This crate hosts the Rust migration of Kotlin `ospf-kotlin-framework-gantt-scheduling`.
//!
//! 迁移目标不是逐文件翻译 Kotlin，而是用 Rust 风格复刻 Gantt Scheduling 领域框架能力，
//! 并严格对齐当前 Rust 项目的 framework 架构规范。
//! The migration goal is not file-by-file Kotlin translation, but Rust-style reimplementation
//! of Gantt Scheduling domain framework capabilities, strictly aligned with the existing
//! Rust project's framework architecture conventions.

pub mod application;
pub mod domain;
pub mod infrastructure;

// 领域错误 re-export / Domain error re-exports
pub use domain::error::{
    GanttSchedulingCapabilityError, GanttSchedulingError, GanttSchedulingLifecycleError,
    GanttSchedulingSolvingError, GanttSchedulingValidationError,
};

/// 甘特排程错误类型 / Gantt scheduling error type
#[derive(Debug, thiserror::Error)]
pub enum GanttError {
    /// 无效的时间范围 / Invalid time range
    #[error("invalid time range: {message}")]
    InvalidTimeRange { message: String },

    /// 无效的持续时间 / Invalid duration
    #[error("invalid duration: {message}")]
    InvalidDuration { message: String },

    /// 空结果 / Empty result
    #[error("empty result: {message}")]
    EmptyResult { message: String },

    /// 不支持的操作 / Unsupported operation
    #[error("unsupported operation: {message}")]
    Unsupported { message: String },

    /// 计算错误 / Calculation error
    #[error("calculation error: {message}")]
    Calculation { message: String },
}

/// 甘特排程结果类型 / Gantt scheduling result type
pub type GanttResult<T> = Result<T, GanttError>;

#[cfg(test)]
mod constraint_programming_differential_tests {
    use std::collections::BTreeMap;

    use ospf_rust_core::model::MetaModel;
    use ospf_rust_core::model::ObjectiveCategory;
    use ospf_rust_core::model::constraint_programming::{
        BooleanLiteral, ConstraintDefinition, ConstraintProgrammingConstraint,
        ConstraintProgrammingModel, IntegerDomain, IntegerExpression, IntegerObjective,
        IntegerTerm, IntegerVariable,
    };
    use ospf_rust_core::model::intermediate::{
        BasicLinearTriadModel, LinearTriadModel, SparseVector,
    };
    use ospf_rust_core::solver::constraint_programming::{
        ConstraintProgrammingSolveOptions, ConstraintProgrammingSolver,
        FakeConstraintProgrammingSolver,
    };
    use ospf_rust_core::solver::{ProblemStatus, StableVariableId};
    use ospf_rust_core::token::Token;
    use ospf_rust_core::variable::{
        BinaryVariableItem, IntegerVariableItem, VariableId, VariableType,
    };

    use crate::domain::task::{
        AssignmentPolicyTrait, BasicAssignmentPolicy, BasicExecutor, ExecutorTrait, TaskTrait,
    };
    use crate::domain::task_compilation::{Compilation, NoOverlapConstraintProgrammingComponent};

    #[derive(Debug, Clone)]
    struct DifferentialTask {
        id: String,
        name: String,
    }

    impl DifferentialTask {
        fn new(id: &str) -> Self {
            Self {
                id: id.to_owned(),
                name: id.to_owned(),
            }
        }
    }

    impl<E: ExecutorTrait, A: AssignmentPolicyTrait<E>> TaskTrait<E, A> for DifferentialTask {
        type Id = String;

        fn id(&self) -> &Self::Id {
            &self.id
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    const VARIABLE_IDS: [&str; 4] = [
        "assignment/task-0/resource-0",
        "assignment/task-0/resource-1",
        "assignment/task-1/resource-0",
        "assignment/task-1/resource-1",
    ];

    const COSTS: [i64; 4] = [1, 4, 3, 2];

    fn cp_snapshot() -> ospf_rust_core::model::constraint_programming::ConstraintProgrammingSnapshot
    {
        let variables = VARIABLE_IDS.map(IntegerVariable::new);
        let mut model = ConstraintProgrammingModel::new("gantt-assignment-cp-differential");
        for variable in &variables {
            model
                .register_variable(variable.clone(), IntegerDomain::boolean())
                .expect("assignment variable");
        }

        for (task, pair) in [(0, [0_usize, 1_usize]), (1, [2_usize, 3_usize])] {
            model
                .add_constraint(ConstraintDefinition::new(
                    format!("task-{task}-exactly-one"),
                    ConstraintProgrammingConstraint::exactly_one(
                        pair.into_iter()
                            .map(|index| BooleanLiteral::positive(variables[index].clone())),
                    ),
                ))
                .expect("task assignment constraint");
        }
        for (resource, pair) in [(0, [0_usize, 2_usize]), (1, [1_usize, 3_usize])] {
            model
                .add_constraint(ConstraintDefinition::new(
                    format!("resource-{resource}-at-most-one"),
                    ConstraintProgrammingConstraint::at_most_one(
                        pair.into_iter()
                            .map(|index| BooleanLiteral::positive(variables[index].clone())),
                    ),
                ))
                .expect("resource capacity constraint");
        }
        let objective = IntegerExpression::linear(
            0,
            variables
                .into_iter()
                .zip(COSTS)
                .map(|(variable, coefficient)| IntegerTerm {
                    variable,
                    coefficient,
                }),
        )
        .expect("assignment objective");
        model.set_objective(IntegerObjective::minimize(objective));
        model.freeze().expect("CP snapshot")
    }

    fn mip_baseline() -> LinearTriadModel {
        let mut basic = BasicLinearTriadModel::new("gantt-assignment-mip-baseline");
        for (index, name) in VARIABLE_IDS.iter().enumerate() {
            let variable = BinaryVariableItem::create(VariableId::standalone(index), name);
            basic.add_variable_with_bounds(
                Token::from_generic(variable, index),
                0.0,
                1.0,
                VariableType::Binary,
            );
        }
        for (name, indices, rhs) in [
            ("task-0-lower", [0_usize, 1_usize], -1.0),
            ("task-0-upper", [0_usize, 1_usize], 1.0),
            ("task-1-lower", [2_usize, 3_usize], -1.0),
            ("task-1-upper", [2_usize, 3_usize], 1.0),
            ("resource-0-capacity", [0_usize, 2_usize], 1.0),
            ("resource-1-capacity", [1_usize, 3_usize], 1.0),
        ] {
            let sign = if name.ends_with("lower") { -1.0 } else { 1.0 };
            let mut row = SparseVector::new();
            for index in indices {
                row.add(index, sign);
            }
            basic.add_constraint_with_metadata(
                row,
                rhs,
                name.to_owned(),
                None,
                false,
                0,
                None,
                None,
            );
        }
        let mut model = LinearTriadModel::from_basic(basic);
        model.set_objective(
            COSTS.into_iter().map(|value| value as f64).collect(),
            ObjectiveCategory::Minimum,
        );
        model
    }

    fn mip_feasible(values: &[i64], model: &LinearTriadModel) -> bool {
        model
            .basic
            .A
            .rows
            .iter()
            .zip(&model.basic.b)
            .all(|(row, rhs)| {
                row.entries
                    .iter()
                    .map(|(index, coefficient)| values[*index] as f64 * coefficient)
                    .sum::<f64>()
                    <= *rhs + f64::EPSILON
            })
    }

    #[test]
    fn gantt_assignment_cp_matches_the_linear_mip_differential_baseline() {
        let snapshot = cp_snapshot();
        let cp_report = FakeConstraintProgrammingSolver::new()
            .solve_constraint_programming(&snapshot, &ConstraintProgrammingSolveOptions::new())
            .expect("CP assignment solve");
        assert_eq!(cp_report.problem_status, ProblemStatus::Feasible);
        assert_eq!(
            cp_report
                .solution
                .as_ref()
                .and_then(|solution| solution.objective),
            Some(3)
        );

        let mip = mip_baseline();
        let mut mip_feasible_assignments = Vec::new();
        let mut mip_best_objective = f64::INFINITY;
        for mask in 0_u8..16 {
            let values = (0..4)
                .map(|index| i64::from((mask >> index) & 1))
                .collect::<Vec<_>>();
            if mip_feasible(&values, &mip) {
                let objective = mip
                    .c
                    .iter()
                    .zip(&values)
                    .map(|(coefficient, value)| coefficient * *value as f64)
                    .sum::<f64>();
                mip_best_objective = mip_best_objective.min(objective);
                mip_feasible_assignments.push(values);
            }
        }
        assert_eq!(mip_feasible_assignments.len(), 2);
        assert_eq!(mip_best_objective, 3.0);

        let mut cp_feasible_count = 0;
        for mask in 0_u8..16 {
            let assignment = VARIABLE_IDS
                .iter()
                .enumerate()
                .map(|(index, id)| (StableVariableId::from(*id), i64::from((mask >> index) & 1)))
                .collect::<BTreeMap<_, _>>();
            if snapshot.validate_assignment(&assignment).is_ok() {
                cp_feasible_count += 1;
            }
        }
        assert_eq!(cp_feasible_count, mip_feasible_assignments.len());
        snapshot
            .validate_assignment(
                &cp_report
                    .solution
                    .as_ref()
                    .expect("CP solution")
                    .stable_values,
            )
            .expect("CP assignment must satisfy the source model");
    }

    #[test]
    fn gantt_no_overlap_cp_matches_the_integer_mip_differential_baseline() {
        let tasks = vec![
            DifferentialTask::new("task-a"),
            DifferentialTask::new("task-b"),
        ];
        let executors = vec![BasicExecutor::new("executor-0", "Executor 0")];
        let mut compilation: Compilation<
            DifferentialTask,
            BasicExecutor,
            BasicAssignmentPolicy<BasicExecutor>,
        > = Compilation::new(tasks, executors, false, false);
        let mut source_model = MetaModel::<f64>::new("gantt-no-overlap-compilation");
        compilation
            .register(&mut source_model)
            .expect("Gantt Compilation registration");
        let task_a_id = compilation.tasks[0].id.clone();
        let task_b_id = compilation.tasks[1].id.clone();
        assert_eq!(
            compilation
                .x
                .as_ref()
                .expect("Compilation assignment variables")
                .len(),
            2
        );

        let component =
            NoOverlapConstraintProgrammingComponent::from_compilation(&compilation, &[2, 2], 3)
                .expect("Gantt CP NoOverlap component");
        let executor_id = compilation.executors[0].id.to_string();
        let start_a_id = component.start_variable_id(&task_a_id);
        let start_b_id = component.start_variable_id(&task_b_id);
        let assignment_a_id = component.assignment_variable_id(&task_a_id, &executor_id);
        let assignment_b_id = component.assignment_variable_id(&task_b_id, &executor_id);
        let interval_a_id = component.interval_id_for_executor(&task_a_id, &executor_id);
        let interval_b_id = component.interval_id_for_executor(&task_b_id, &executor_id);
        let snapshot = component.freeze().expect("NoOverlap CP snapshot");
        let cp_report = FakeConstraintProgrammingSolver::new()
            .solve_constraint_programming(&snapshot, &ConstraintProgrammingSolveOptions::new())
            .expect("NoOverlap CP solve");
        assert_eq!(cp_report.problem_status, ProblemStatus::Feasible);
        assert_eq!(
            cp_report
                .solution
                .as_ref()
                .and_then(|solution| solution.objective),
            Some(2)
        );

        let mut basic = BasicLinearTriadModel::new("gantt-no-overlap-mip-baseline");
        for (index, (name, variable_type)) in [
            ("gantt/no-overlap/start-a", VariableType::Integer),
            ("gantt/no-overlap/start-b", VariableType::Integer),
            ("gantt/no-overlap/order-a-before-b", VariableType::Binary),
        ]
        .into_iter()
        .enumerate()
        {
            let token = if variable_type == VariableType::Binary {
                Token::from_generic(
                    BinaryVariableItem::create(VariableId::standalone(index), name),
                    index,
                )
            } else {
                Token::from_generic(
                    IntegerVariableItem::create(VariableId::standalone(index), name),
                    index,
                )
            };
            basic.add_variable_with_bounds(
                token,
                0.0,
                if variable_type == VariableType::Binary {
                    1.0
                } else {
                    3.0
                },
                variable_type,
            );
        }
        let mut first_order = SparseVector::new();
        first_order.add(0, 1.0);
        first_order.add(1, -1.0);
        first_order.add(2, 5.0);
        basic.add_constraint_with_metadata(
            first_order,
            3.0,
            "a-before-b".to_owned(),
            None,
            false,
            0,
            None,
            None,
        );
        let mut second_order = SparseVector::new();
        second_order.add(0, -1.0);
        second_order.add(1, 1.0);
        second_order.add(2, -5.0);
        basic.add_constraint_with_metadata(
            second_order,
            -2.0,
            "b-before-a".to_owned(),
            None,
            false,
            0,
            None,
            None,
        );
        let mut mip = LinearTriadModel::from_basic(basic);
        mip.set_objective(vec![1.0, 1.0, 0.0], ObjectiveCategory::Minimum);

        let mut mip_feasible_pairs = std::collections::BTreeSet::new();
        let mut mip_best_objective = f64::INFINITY;
        for start_a_value in 0..=3 {
            for start_b_value in 0..=3 {
                for order in 0..=1 {
                    let values = vec![start_a_value, start_b_value, order];
                    if mip_feasible(&values, &mip) {
                        mip_best_objective = mip_best_objective.min(
                            mip.c
                                .iter()
                                .zip(&values)
                                .map(|(coefficient, value)| coefficient * *value as f64)
                                .sum(),
                        );
                        mip_feasible_pairs.insert(vec![start_a_value, start_b_value]);
                    }
                }
            }
        }
        assert_eq!(mip_feasible_pairs.len(), 6);
        assert_eq!(mip_best_objective, 2.0);

        let mut cp_feasible = std::collections::BTreeSet::new();
        for start_a_value in 0..=3 {
            for start_b_value in 0..=3 {
                let assignment = [
                    (StableVariableId::from(start_a_id.as_str()), start_a_value),
                    (StableVariableId::from(start_b_id.as_str()), start_b_value),
                    (StableVariableId::from(assignment_a_id.as_str()), 1),
                    (StableVariableId::from(assignment_b_id.as_str()), 1),
                ]
                .into_iter()
                .collect::<BTreeMap<_, _>>();
                if snapshot.validate_assignment(&assignment).is_ok() {
                    cp_feasible.insert(vec![start_a_value, start_b_value]);
                }
            }
        }
        assert_eq!(cp_feasible, mip_feasible_pairs);
        let cp_solution = cp_report.solution.as_ref().expect("NoOverlap solution");
        let intervals = snapshot
            .validate_assignment(&cp_solution.stable_values)
            .expect("CP solution validates");
        assert!(
            intervals[&interval_a_id.clone().into()].end
                <= intervals[&interval_b_id.clone().into()].start
                || intervals[&interval_b_id.clone().into()].end
                    <= intervals[&interval_a_id.clone().into()].start
        );
    }

    #[test]
    fn gantt_no_overlap_multi_executor_cp_matches_the_integer_mip_differential_baseline() {
        let tasks = vec![
            DifferentialTask::new("task-a"),
            DifferentialTask::new("task-b"),
        ];
        let executors = vec![
            BasicExecutor::new("executor-0", "Executor 0"),
            BasicExecutor::new("executor-1", "Executor 1"),
        ];
        let mut compilation: Compilation<
            DifferentialTask,
            BasicExecutor,
            BasicAssignmentPolicy<BasicExecutor>,
        > = Compilation::new(tasks, executors, false, false);
        let mut source_model = MetaModel::<f64>::new("gantt-no-overlap-multi-executor");
        compilation
            .register(&mut source_model)
            .expect("Gantt Compilation registration");

        let component =
            NoOverlapConstraintProgrammingComponent::from_compilation(&compilation, &[2, 2], 2)
                .expect("Gantt CP NoOverlap component");
        let task_a_id = compilation.tasks[0].id.clone();
        let task_b_id = compilation.tasks[1].id.clone();
        let executor_ids = compilation
            .executors
            .iter()
            .map(|executor| executor.id.to_string())
            .collect::<Vec<_>>();
        let start_a_id = component.start_variable_id(&task_a_id);
        let start_b_id = component.start_variable_id(&task_b_id);
        let assignment_ids = executor_ids
            .iter()
            .flat_map(|executor_id| {
                [
                    component.assignment_variable_id(&task_a_id, executor_id),
                    component.assignment_variable_id(&task_b_id, executor_id),
                ]
            })
            .collect::<Vec<_>>();
        let snapshot = component.freeze().expect("NoOverlap CP snapshot");

        let cp_report = FakeConstraintProgrammingSolver::new()
            .solve_constraint_programming(&snapshot, &ConstraintProgrammingSolveOptions::new())
            .expect("NoOverlap CP solve");
        assert_eq!(cp_report.problem_status, ProblemStatus::Feasible);
        assert_eq!(
            cp_report
                .solution
                .as_ref()
                .and_then(|solution| solution.objective),
            Some(0)
        );

        // This is the same finite optional-interval formulation as the production CP component:
        // each executor has its own order binary, activated only when both assignments are true.
        // 该有限 optional interval formulation 与生产 CP component 相同：每个 executor 有独立
        // 顺序 binary，只有两个任务同时分配给它时才激活顺序约束。
        let mut basic = BasicLinearTriadModel::new("gantt-no-overlap-multi-executor-mip");
        let variables = [
            ("start-a", VariableType::Integer),
            ("start-b", VariableType::Integer),
            ("assignment-a-executor-0", VariableType::Binary),
            ("assignment-a-executor-1", VariableType::Binary),
            ("assignment-b-executor-0", VariableType::Binary),
            ("assignment-b-executor-1", VariableType::Binary),
            ("order-executor-0", VariableType::Binary),
            ("order-executor-1", VariableType::Binary),
        ];
        for (index, (name, variable_type)) in variables.into_iter().enumerate() {
            let token = if variable_type == VariableType::Binary {
                Token::from_generic(
                    BinaryVariableItem::create(VariableId::standalone(index), name),
                    index,
                )
            } else {
                Token::from_generic(
                    IntegerVariableItem::create(VariableId::standalone(index), name),
                    index,
                )
            };
            basic.add_variable_with_bounds(
                token,
                0.0,
                if variable_type == VariableType::Binary {
                    1.0
                } else {
                    2.0
                },
                variable_type,
            );
        }
        let mut add_row = |entries: &[(usize, f64)], rhs: f64, name: &str| {
            let mut row = SparseVector::new();
            for (index, coefficient) in entries {
                row.add(*index, *coefficient);
            }
            basic.add_constraint_with_metadata(
                row,
                rhs,
                name.to_owned(),
                None,
                false,
                0,
                None,
                None,
            );
        };
        add_row(&[(2, -1.0), (3, -1.0)], -1.0, "task-a-lower");
        add_row(&[(2, 1.0), (3, 1.0)], 1.0, "task-a-upper");
        add_row(&[(4, -1.0), (5, -1.0)], -1.0, "task-b-lower");
        add_row(&[(4, 1.0), (5, 1.0)], 1.0, "task-b-upper");

        let big_m = 8.0;
        for (executor, (task_a_assignment, task_b_assignment, order)) in
            [(2, 4, 6), (3, 5, 7)].into_iter().enumerate()
        {
            add_row(
                &[
                    (0, 1.0),
                    (1, -1.0),
                    (order, big_m),
                    (task_a_assignment, big_m),
                    (task_b_assignment, big_m),
                ],
                3.0 * big_m - 2.0,
                &format!("executor-{executor}-a-before-b"),
            );
            add_row(
                &[
                    (1, 1.0),
                    (0, -1.0),
                    (order, -big_m),
                    (task_a_assignment, big_m),
                    (task_b_assignment, big_m),
                ],
                2.0 * big_m - 2.0,
                &format!("executor-{executor}-b-before-a"),
            );
        }
        let mut mip = LinearTriadModel::from_basic(basic);
        mip.set_objective(
            vec![1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            ObjectiveCategory::Minimum,
        );

        let mut mip_feasible_assignments = std::collections::BTreeSet::new();
        for start_a in 0_i64..=2 {
            for start_b in 0_i64..=2 {
                for assignment_a_executor_0 in 0_i64..=1 {
                    for assignment_a_executor_1 in 0_i64..=1 {
                        for assignment_b_executor_0 in 0_i64..=1 {
                            for assignment_b_executor_1 in 0_i64..=1 {
                                for order_executor_0 in 0_i64..=1 {
                                    for order_executor_1 in 0_i64..=1 {
                                        let values = vec![
                                            start_a,
                                            start_b,
                                            assignment_a_executor_0,
                                            assignment_a_executor_1,
                                            assignment_b_executor_0,
                                            assignment_b_executor_1,
                                            order_executor_0,
                                            order_executor_1,
                                        ];
                                        if mip_feasible(&values, &mip) {
                                            mip_feasible_assignments.insert(values[..6].to_vec());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut cp_feasible = std::collections::BTreeSet::new();
        for start_a in 0_i64..=2 {
            for start_b in 0_i64..=2 {
                for assignment_a_executor_0 in 0_i64..=1 {
                    for assignment_a_executor_1 in 0_i64..=1 {
                        for assignment_b_executor_0 in 0_i64..=1 {
                            for assignment_b_executor_1 in 0_i64..=1 {
                                let values = [
                                    start_a,
                                    start_b,
                                    assignment_a_executor_0,
                                    assignment_a_executor_1,
                                    assignment_b_executor_0,
                                    assignment_b_executor_1,
                                ];
                                let assignment = [
                                    (start_a_id.as_str(), values[0]),
                                    (start_b_id.as_str(), values[1]),
                                    (assignment_ids[0].as_str(), values[2]),
                                    (assignment_ids[1].as_str(), values[4]),
                                    (assignment_ids[2].as_str(), values[3]),
                                    (assignment_ids[3].as_str(), values[5]),
                                ]
                                .into_iter()
                                .map(|(id, value)| (StableVariableId::from(id), value))
                                .collect::<BTreeMap<_, _>>();
                                if snapshot.validate_assignment(&assignment).is_ok() {
                                    cp_feasible.insert(values.to_vec());
                                }
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(cp_feasible, mip_feasible_assignments);
    }

    #[test]
    fn gantt_no_overlap_allows_parallel_tasks_on_different_executors() {
        let component = NoOverlapConstraintProgrammingComponent {
            tasks: vec![
                super::domain::task_compilation::NoOverlapTask::new("task-a", 2).expect("task a"),
                super::domain::task_compilation::NoOverlapTask::new("task-b", 2).expect("task b"),
            ],
            executor_ids: vec!["executor-0".to_owned(), "executor-1".to_owned()],
            start_upper_bound: 2,
        };
        let snapshot = component.freeze().expect("NoOverlap CP snapshot");
        let parallel = [
            (component.start_variable_id("task-a"), 0),
            (component.start_variable_id("task-b"), 0),
            (component.assignment_variable_id("task-a", "executor-0"), 1),
            (component.assignment_variable_id("task-a", "executor-1"), 0),
            (component.assignment_variable_id("task-b", "executor-0"), 0),
            (component.assignment_variable_id("task-b", "executor-1"), 1),
        ]
        .into_iter()
        .map(|(id, value)| (StableVariableId::from(id), value))
        .collect::<BTreeMap<_, _>>();
        snapshot
            .validate_assignment(&parallel)
            .expect("different executors must allow parallel tasks");

        let same_executor = [
            (component.start_variable_id("task-a"), 0),
            (component.start_variable_id("task-b"), 0),
            (component.assignment_variable_id("task-a", "executor-0"), 1),
            (component.assignment_variable_id("task-a", "executor-1"), 0),
            (component.assignment_variable_id("task-b", "executor-0"), 1),
            (component.assignment_variable_id("task-b", "executor-1"), 0),
        ]
        .into_iter()
        .map(|(id, value)| (StableVariableId::from(id), value))
        .collect::<BTreeMap<_, _>>();
        assert!(
            snapshot.validate_assignment(&same_executor).is_err(),
            "tasks assigned to the same executor must not overlap"
        );
    }
}
