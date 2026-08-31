//! 任务编译的 CP 组件 / Constraint-programming component for task compilation.

use std::collections::{BTreeMap, BTreeSet};

use ospf_rust_core::model::constraint_programming::{
    BooleanLiteral, ConstraintDefinition, ConstraintProgrammingConstraint,
    ConstraintProgrammingModel, ConstraintProgrammingSnapshot, IntegerDomain, IntegerExpression,
    IntegerObjective, IntegerTerm, IntegerVariable, IntervalDuration, IntervalVariable,
};

use crate::GanttResult;
use crate::domain::task::{AssignmentPolicyTrait, ExecutorTrait, TaskTrait};

use super::model::Compilation;

fn calculation_error(message: impl Into<String>) -> crate::GanttError {
    crate::GanttError::Calculation {
        message: message.into(),
    }
}

/// CP NoOverlap 任务定义 / CP NoOverlap task definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoOverlapTask {
    /// 稳定任务 ID / Stable task ID.
    pub id: String,
    /// 固定持续时间 / Fixed duration.
    pub duration: i64,
}

impl NoOverlapTask {
    /// 创建 NoOverlap 任务 / Create a NoOverlap task.
    pub fn new(id: impl Into<String>, duration: i64) -> GanttResult<Self> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(calculation_error("CP NoOverlap task ID must not be blank"));
        }
        if duration < 0 {
            return Err(calculation_error(
                "CP NoOverlap task duration must not be negative",
            ));
        }
        Ok(Self { id, duration })
    }
}

/// Gantt CP NoOverlap 模型组件 / Gantt CP NoOverlap model component.
///
/// 该组件是生产 task-compilation 的 CP 建模入口：它只负责构造不可变 CP snapshot，
/// 求解和 backend 选择仍由 application/solver 层负责。
///
/// This component is the production CP modeling entry for task compilation. It builds an
/// immutable CP snapshot; solving and backend selection remain owned by the application/solver
/// layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoOverlapConstraintProgrammingComponent {
    /// 任务集合 / Task set.
    pub tasks: Vec<NoOverlapTask>,
    /// 可用执行者稳定 ID；为空时不生成 assignment 层 / Available executor IDs; empty means no assignment layer.
    pub executor_ids: Vec<String>,
    /// 起点值域上界 / Upper bound of the start-time domain.
    pub start_upper_bound: i64,
}

impl NoOverlapConstraintProgrammingComponent {
    /// 创建 NoOverlap CP 组件 / Create a NoOverlap CP component.
    pub fn new(tasks: Vec<NoOverlapTask>, start_upper_bound: i64) -> GanttResult<Self> {
        if start_upper_bound < 0 {
            return Err(calculation_error(
                "CP NoOverlap start upper bound must not be negative",
            ));
        }
        let mut ids = BTreeSet::new();
        for task in &tasks {
            if !ids.insert(task.id.clone()) {
                return Err(calculation_error(format!(
                    "duplicate CP NoOverlap task ID {}",
                    task.id
                )));
            }
        }
        Ok(Self {
            tasks,
            executor_ids: Vec::new(),
            start_upper_bound,
        })
    }

    /// 从生产 Compilation 创建组件 / Create the component from the production Compilation.
    pub fn from_compilation<T, E, A>(
        compilation: &Compilation<T, E, A>,
        durations: &[i64],
        start_upper_bound: i64,
    ) -> GanttResult<Self>
    where
        E: ExecutorTrait,
        A: AssignmentPolicyTrait<E>,
        T: TaskTrait<E, A>,
    {
        if durations.len() != compilation.tasks.len() {
            return Err(calculation_error(format!(
                "CP NoOverlap duration count {} does not match task count {}",
                durations.len(),
                compilation.tasks.len()
            )));
        }
        let tasks = compilation
            .tasks
            .iter()
            .zip(durations)
            .map(|(task, duration)| NoOverlapTask::new(task.id().to_string(), *duration))
            .collect::<GanttResult<Vec<_>>>()?;
        let mut component = Self::new(tasks, start_upper_bound)?;
        let mut executor_ids = BTreeSet::new();
        for executor in &compilation.executors {
            let id = executor.id().to_string();
            if id.trim().is_empty() || !executor_ids.insert(id) {
                return Err(calculation_error(
                    "CP NoOverlap executor IDs must be non-empty and unique",
                ));
            }
        }
        component.executor_ids = executor_ids.into_iter().collect();
        Ok(component)
    }

    /// 返回任务 start 变量的稳定 ID / Return the stable ID of a task start variable.
    pub fn start_variable_id(&self, task_id: &str) -> String {
        format!(
            "gantt/cp/no-overlap/task/{}/start",
            Self::escaped_id(task_id)
        )
    }

    /// 返回任务 interval 的稳定 ID / Return the stable ID of a task interval.
    pub fn interval_id(&self, task_id: &str) -> String {
        format!(
            "gantt/cp/no-overlap/task/{}/interval",
            Self::escaped_id(task_id)
        )
    }

    /// 返回任务-执行者 interval 的稳定 ID / Return the stable ID of a task-executor interval.
    pub fn interval_id_for_executor(&self, task_id: &str, executor_id: &str) -> String {
        format!(
            "gantt/cp/no-overlap/task/{}/executor/{}/interval",
            Self::escaped_id(task_id),
            Self::escaped_id(executor_id)
        )
    }

    /// 返回任务-执行者 assignment 变量的稳定 ID / Return the stable ID of a task-executor assignment variable.
    pub fn assignment_variable_id(&self, task_id: &str, executor_id: &str) -> String {
        format!(
            "gantt/cp/no-overlap/assignment/{}/{}",
            Self::escaped_id(task_id),
            Self::escaped_id(executor_id)
        )
    }

    /// 构造不可变 CP snapshot / Build an immutable CP snapshot.
    pub fn freeze(&self) -> GanttResult<ConstraintProgrammingSnapshot> {
        let mut model = ConstraintProgrammingModel::new("gantt/cp/no-overlap");
        let mut intervals_by_executor = BTreeMap::<String, Vec<_>>::new();
        let mut objective_terms = Vec::with_capacity(self.tasks.len());

        for task in &self.tasks {
            let start = IntegerVariable::new(self.start_variable_id(&task.id));
            let start_domain = IntegerDomain::range(0, self.start_upper_bound)
                .map_err(|error| calculation_error(error.to_string()))?;
            model
                .register_variable(start.clone(), start_domain)
                .map_err(|error| calculation_error(error.to_string()))?;
            let assignment_literals = self
                .executor_ids
                .iter()
                .map(|executor_id| {
                    let assignment =
                        IntegerVariable::new(self.assignment_variable_id(&task.id, executor_id));
                    model
                        .register_variable(assignment.clone(), IntegerDomain::boolean())
                        .map_err(|error| calculation_error(error.to_string()))?;
                    Ok(BooleanLiteral::positive(assignment))
                })
                .collect::<GanttResult<Vec<_>>>()?;
            if !assignment_literals.is_empty() {
                model
                    .add_constraint(ConstraintDefinition::new(
                        format!(
                            "gantt/cp/assignment/{}/exactly-one",
                            Self::escaped_id(&task.id)
                        ),
                        ConstraintProgrammingConstraint::exactly_one(assignment_literals.clone()),
                    ))
                    .map_err(|error| calculation_error(error.to_string()))?;
            }
            let end = IntegerExpression::linear(
                task.duration,
                [IntegerTerm {
                    variable: start.clone(),
                    coefficient: 1,
                }],
            )
            .map_err(|error| calculation_error(error.to_string()))?;
            if self.executor_ids.is_empty() {
                let interval = IntervalVariable::new(
                    self.interval_id(&task.id),
                    IntegerExpression::variable(start.clone()),
                    IntervalDuration::Fixed(task.duration),
                    end.clone(),
                    None,
                )
                .map_err(|error| calculation_error(error.to_string()))?;
                model
                    .register_interval(interval.clone())
                    .map_err(|error| calculation_error(error.to_string()))?;
                intervals_by_executor
                    .entry(String::new())
                    .or_default()
                    .push(interval.id);
            } else {
                for (executor_id, presence) in self.executor_ids.iter().zip(assignment_literals) {
                    let interval = IntervalVariable::new(
                        self.interval_id_for_executor(&task.id, executor_id),
                        IntegerExpression::variable(start.clone()),
                        IntervalDuration::Fixed(task.duration),
                        end.clone(),
                        Some(presence),
                    )
                    .map_err(|error| calculation_error(error.to_string()))?;
                    model
                        .register_interval(interval.clone())
                        .map_err(|error| calculation_error(error.to_string()))?;
                    intervals_by_executor
                        .entry(executor_id.clone())
                        .or_default()
                        .push(interval.id);
                }
            }
            objective_terms.push(IntegerTerm {
                variable: start,
                coefficient: 1,
            });
        }

        for (executor_id, intervals) in intervals_by_executor {
            let constraint_id = if executor_id.is_empty() {
                "gantt/cp/no-overlap".to_owned()
            } else {
                format!(
                    "gantt/cp/no-overlap/executor/{}",
                    Self::escaped_id(&executor_id)
                )
            };
            model
                .add_constraint(ConstraintDefinition::new(
                    constraint_id,
                    ConstraintProgrammingConstraint::NoOverlap { intervals },
                ))
                .map_err(|error| calculation_error(error.to_string()))?;
        }
        if !objective_terms.is_empty() {
            let objective = IntegerExpression::linear(0, objective_terms)
                .map_err(|error| calculation_error(error.to_string()))?;
            model.set_objective(IntegerObjective::minimize(objective));
        }
        model
            .freeze()
            .map_err(|error| calculation_error(error.to_string()))
    }

    fn escaped_id(id: &str) -> String {
        format!("{}:{}", id.len(), id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_overlap_component_rejects_duplicate_tasks_and_builds_snapshot() {
        let component = NoOverlapConstraintProgrammingComponent::new(
            vec![NoOverlapTask::new("a", 2).expect("task")],
            3,
        )
        .expect("component");
        let snapshot = component.freeze().expect("snapshot");
        assert!(snapshot.constraint(&"gantt/cp/no-overlap".into()).is_some());
        assert!(
            snapshot
                .variable(&component.start_variable_id("a").into())
                .is_some()
        );
        assert!(
            NoOverlapConstraintProgrammingComponent::new(
                vec![
                    NoOverlapTask::new("a", 1).expect("task"),
                    NoOverlapTask::new("a", 2).expect("task"),
                ],
                3,
            )
            .is_err()
        );
    }
}
