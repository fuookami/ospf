//! 任务步骤图 / Task step graph
//!
//! 提供多步任务的步骤、前后向依赖向量和 DAG 构建校验。
//! Provides steps, forward/backward dependency vectors, and DAG build validation
//! for multi-step tasks.

use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};

use time::Duration;

use crate::domain::common::{
    GanttId, TaskPlanId, TaskPlanIdTrait, TaskStepId, TaskStepIdTrait,
};
use crate::domain::task::{ExecutorTrait, TaskStatus};
use crate::{GanttError, GanttResult};

/// 步骤关系 / Step relation
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StepRelation {
    /// 与关系，所有前置或后继步骤均参与 / AND relation, all related steps participate
    And,
    /// 或关系，任一前置或后继步骤参与即可 / OR relation, any related step may participate
    Or,
}

/// 任务步骤 trait / Task step trait
///
/// 表示多步任务中的一个可执行步骤。
/// Represents one executable step in a multi-step task.
pub trait TaskStepTrait<E>: Send + Sync + std::fmt::Debug + Clone + Eq + std::hash::Hash + 'static
where
    E: ExecutorTrait,
{
    /// 步骤 ID 类型 / Step id type
    type Id: TaskStepIdTrait;

    /// 步骤 ID / Step ID
    fn id(&self) -> &Self::Id;

    /// 步骤名称 / Step name
    fn name(&self) -> &str;

    /// 显示名称 / Display name
    fn display_name(&self) -> &str {
        self.name()
    }

    /// 可用执行者 / Enabled executors
    fn enabled_executors(&self) -> &Vec<E>;

    /// 状态集合 / Status set
    fn status(&self) -> &HashSet<TaskStatus>;

    /// 指定执行者下的持续时间 / Duration with specified executor
    fn duration(&self, executor: &E) -> Duration;
}

/// 基础任务步骤 / Basic task step
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct BasicTaskStep<E, I = TaskStepId>
where
    E: ExecutorTrait,
    I: TaskStepIdTrait,
{
    /// 步骤 ID / Step ID
    pub id: I,
    /// 步骤名称 / Step name
    pub name: String,
    /// 可用执行者 / Enabled executors
    pub enabled_executors: Vec<E>,
    /// 状态集合 / Status set
    pub status: HashSet<TaskStatus>,
    /// 默认持续时间 / Default duration
    pub duration: Duration,
}

impl<E, I> PartialEq for BasicTaskStep<E, I>
where
    E: ExecutorTrait,
    I: TaskStepIdTrait,
{
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<E, I> Eq for BasicTaskStep<E, I>
where
    E: ExecutorTrait,
    I: TaskStepIdTrait,
{
}

impl<E, I> Hash for BasicTaskStep<E, I>
where
    E: ExecutorTrait,
    I: TaskStepIdTrait,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<E, I> BasicTaskStep<E, I>
where
    E: ExecutorTrait,
    I: TaskStepIdTrait,
{
    /// 创建基础任务步骤 / Create basic task step
    pub fn new(
        id: impl Into<I>,
        name: impl Into<String>,
        enabled_executors: Vec<E>,
        duration: Duration,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            enabled_executors,
            status: HashSet::new(),
            duration,
        }
    }

    /// 设置状态 / Set status
    pub fn with_status(mut self, status: HashSet<TaskStatus>) -> Self {
        self.status = status;
        self
    }
}

impl<E, I> TaskStepTrait<E> for BasicTaskStep<E, I>
where
    E: ExecutorTrait,
    I: TaskStepIdTrait,
{
    type Id = I;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn enabled_executors(&self) -> &Vec<E> {
        &self.enabled_executors
    }

    fn status(&self) -> &HashSet<TaskStatus> {
        &self.status
    }

    fn duration(&self, _executor: &E) -> Duration {
        self.duration
    }
}

/// 前向任务步骤向量 / Forward task step vector
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardTaskStepVector<I = TaskStepId>
where
    I: TaskStepIdTrait,
{
    /// 源步骤 ID / Source step ID
    pub from: I,
    /// 目标步骤 ID 列表 / Target step ID list
    pub to: Vec<I>,
    /// 步骤关系 / Step relation
    pub relation: StepRelation,
}

impl<I> ForwardTaskStepVector<I>
where
    I: TaskStepIdTrait,
{
    /// 创建前向向量 / Create forward vector
    pub fn new(
        from: impl Into<I>,
        to: Vec<impl Into<I>>,
        relation: StepRelation,
    ) -> Self {
        Self {
            from: from.into(),
            to: to.into_iter().map(Into::into).collect(),
            relation,
        }
    }
}

/// 后向任务步骤向量 / Backward task step vector
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackwardTaskStepVector<I = TaskStepId>
where
    I: TaskStepIdTrait,
{
    /// 源步骤 ID 列表 / Source step ID list
    pub from: Vec<I>,
    /// 目标步骤 ID / Target step ID
    pub to: I,
    /// 步骤关系 / Step relation
    pub relation: StepRelation,
}

impl<I> BackwardTaskStepVector<I>
where
    I: TaskStepIdTrait,
{
    /// 创建后向向量 / Create backward vector
    pub fn new(
        from: Vec<impl Into<I>>,
        to: impl Into<I>,
        relation: StepRelation,
    ) -> Self {
        Self {
            from: from.into_iter().map(Into::into).collect(),
            to: to.into(),
            relation,
        }
    }
}

/// 起始步骤集合 / Start step set
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartSteps<I = TaskStepId>
where
    I: TaskStepIdTrait,
{
    /// 起始步骤 ID 列表 / Start step ID list
    pub steps: Vec<I>,
    /// 步骤关系 / Step relation
    pub relation: StepRelation,
}

impl<I> StartSteps<I>
where
    I: TaskStepIdTrait,
{
    /// 创建起始步骤集合 / Create start step set
    pub fn new(steps: Vec<impl Into<I>>, relation: StepRelation) -> Self {
        Self {
            steps: steps.into_iter().map(Into::into).collect(),
            relation,
        }
    }
}

/// 任务步骤图 / Task step graph
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "S: serde::Serialize, S::Id: serde::Serialize, PI: serde::Serialize",
        deserialize = "S: serde::Deserialize<'de>, S::Id: serde::Deserialize<'de>, PI: serde::Deserialize<'de>"
    ))
)]
#[derive(Debug, Clone)]
pub struct TaskStepGraph<E, S, PI = TaskPlanId>
where
    E: ExecutorTrait,
    S: TaskStepTrait<E>,
    PI: TaskPlanIdTrait,
{
    /// 图 ID / Graph ID
    pub id: PI,
    /// 图名称 / Graph name
    pub name: String,
    /// 步骤列表 / Step list
    pub steps: Vec<S>,
    /// 起始步骤集合 / Start step set
    pub start_steps: StartSteps<S::Id>,
    /// 前向步骤向量映射 / Forward step vector map
    pub forward_task_step_vectors: HashMap<S::Id, ForwardTaskStepVector<S::Id>>,
    /// 后向步骤向量映射 / Backward step vector map
    pub backward_step_relations: HashMap<S::Id, BackwardTaskStepVector<S::Id>>,
    _marker: std::marker::PhantomData<E>,
}

impl<E, S, PI> TaskStepGraph<E, S, PI>
where
    E: ExecutorTrait,
    S: TaskStepTrait<E>,
    PI: TaskPlanIdTrait,
{
    /// 使用业务 ID 创建构建器 / Create builder with domain id
    pub fn builder_with_id(
        id: impl Into<PI>,
        name: impl Into<String>,
    ) -> TaskStepGraphBuilder<E, S, PI> {
        TaskStepGraphBuilder::new(id, name)
    }

    /// 按 ID 获取步骤 / Get step by ID
    pub fn step(&self, step_id: &S::Id) -> Option<&S> {
        self.steps.iter().find(|step| step.id() == step_id)
    }

    /// 获取后继步骤 ID / Get successor step IDs
    pub fn successors(&self, step_id: &S::Id) -> &[S::Id] {
        self.forward_task_step_vectors
            .get(step_id)
            .map(|vector| vector.to.as_slice())
            .unwrap_or(&[])
    }

    /// 获取前置步骤 ID / Get predecessor step IDs
    pub fn predecessors(&self, step_id: &S::Id) -> &[S::Id] {
        self.backward_step_relations
            .get(step_id)
            .map(|vector| vector.from.as_slice())
            .unwrap_or(&[])
    }

    /// 是否为空图 / Whether the graph has no steps
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

impl<E, S> TaskStepGraph<E, S, TaskPlanId>
where
    E: ExecutorTrait,
    S: TaskStepTrait<E>,
{
    /// 创建构建器 / Create builder
    pub fn builder(
        id: impl Into<TaskPlanId>,
        name: impl Into<String>,
    ) -> TaskStepGraphBuilder<E, S, TaskPlanId> {
        TaskStepGraphBuilder::new(id, name)
    }
}

/// 任务步骤图构建器 / Task step graph builder
#[derive(Debug, Clone)]
pub struct TaskStepGraphBuilder<E, S, PI = TaskPlanId>
where
    E: ExecutorTrait,
    S: TaskStepTrait<E>,
    PI: TaskPlanIdTrait,
{
    id: PI,
    name: String,
    steps: Vec<S>,
    start_steps: Option<StartSteps<S::Id>>,
    forward_task_step_vectors: HashMap<S::Id, ForwardTaskStepVector<S::Id>>,
    backward_step_relations: HashMap<S::Id, BackwardTaskStepVector<S::Id>>,
    _marker: std::marker::PhantomData<E>,
}

impl<E, S, PI> TaskStepGraphBuilder<E, S, PI>
where
    E: ExecutorTrait,
    S: TaskStepTrait<E>,
    PI: TaskPlanIdTrait,
{
    /// 创建任务步骤图构建器 / Create task step graph builder
    pub fn new(id: impl Into<PI>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            steps: Vec::new(),
            start_steps: None,
            forward_task_step_vectors: HashMap::new(),
            backward_step_relations: HashMap::new(),
            _marker: std::marker::PhantomData,
        }
    }

    /// 添加步骤 / Add step
    pub fn add_step(mut self, step: S) -> Self {
        self.steps.push(step);
        self
    }

    /// 批量添加步骤 / Add steps
    pub fn add_steps(mut self, steps: impl IntoIterator<Item = S>) -> Self {
        self.steps.extend(steps);
        self
    }

    /// 设置起始步骤 / Set start steps
    pub fn start_steps(
        mut self,
        steps: Vec<impl Into<S::Id>>,
        relation: StepRelation,
    ) -> Self {
        self.start_steps = Some(StartSteps::new(steps, relation));
        self
    }

    /// 添加前向关系 / Add forward relation
    pub fn forward(
        mut self,
        from: impl Into<S::Id>,
        to: Vec<impl Into<S::Id>>,
        relation: StepRelation,
    ) -> Self {
        let vector = ForwardTaskStepVector::new(from, to, relation);
        self.forward_task_step_vectors
            .insert(vector.from.clone(), vector);
        self
    }

    /// 添加后向关系 / Add backward relation
    pub fn backward(
        mut self,
        from: Vec<impl Into<S::Id>>,
        to: impl Into<S::Id>,
        relation: StepRelation,
    ) -> Self {
        let vector = BackwardTaskStepVector::new(from, to, relation);
        self.backward_step_relations
            .insert(vector.to.clone(), vector);
        self
    }

    /// 构建任务步骤图 / Build task step graph
    pub fn build(self) -> GanttResult<TaskStepGraph<E, S, PI>> {
        let step_ids = self.validate_steps()?;
        let start_steps = self.validate_start_steps(&step_ids)?;
        self.validate_forward_vectors(&step_ids)?;
        self.validate_backward_vectors(&step_ids)?;
        self.validate_forward_backward_consistency()?;
        self.validate_acyclic(&step_ids)?;

        Ok(TaskStepGraph {
            id: self.id,
            name: self.name,
            steps: self.steps,
            start_steps,
            forward_task_step_vectors: self.forward_task_step_vectors,
            backward_step_relations: self.backward_step_relations,
            _marker: std::marker::PhantomData,
        })
    }

    fn validate_steps(&self) -> GanttResult<HashSet<S::Id>> {
        let mut ids = HashSet::with_capacity(self.steps.len());
        for step in &self.steps {
            if step.id().is_empty() {
                return Err(GanttError::Calculation {
                    message: "task step id must not be empty".to_string(),
                });
            }
            if !ids.insert(step.id().clone()) {
                return Err(GanttError::Calculation {
                    message: format!("duplicated task step id `{}`", step.id()),
                });
            }
        }
        if ids.is_empty() {
            return Err(GanttError::Calculation {
                message: "task step graph must contain at least one step".to_string(),
            });
        }
        Ok(ids)
    }

    fn validate_start_steps(
        &self,
        step_ids: &HashSet<S::Id>,
    ) -> GanttResult<StartSteps<S::Id>> {
        let start_steps = self.start_steps.clone().ok_or_else(|| GanttError::Calculation {
            message: "task step graph start steps must be set".to_string(),
        })?;
        if start_steps.steps.is_empty() {
            return Err(GanttError::Calculation {
                message: "task step graph start steps must not be empty".to_string(),
            });
        }
        self.ensure_step_ids_exist(&start_steps.steps, step_ids, "start steps")?;
        Ok(start_steps)
    }

    fn validate_forward_vectors(&self, step_ids: &HashSet<S::Id>) -> GanttResult<()> {
        for vector in self.forward_task_step_vectors.values() {
            self.ensure_step_id_exists(&vector.from, step_ids, "forward source")?;
            if vector.to.is_empty() {
                return Err(GanttError::Calculation {
                    message: format!("forward vector `{}` target list must not be empty", vector.from),
                });
            }
            self.ensure_step_ids_exist(&vector.to, step_ids, "forward targets")?;
        }
        Ok(())
    }

    fn validate_backward_vectors(&self, step_ids: &HashSet<S::Id>) -> GanttResult<()> {
        for vector in self.backward_step_relations.values() {
            self.ensure_step_id_exists(&vector.to, step_ids, "backward target")?;
            if vector.from.is_empty() {
                return Err(GanttError::Calculation {
                    message: format!("backward vector `{}` source list must not be empty", vector.to),
                });
            }
            self.ensure_step_ids_exist(&vector.from, step_ids, "backward sources")?;
        }
        Ok(())
    }

    fn validate_forward_backward_consistency(&self) -> GanttResult<()> {
        for vector in self.forward_task_step_vectors.values() {
            for to in &vector.to {
                let Some(backward) = self.backward_step_relations.get(to) else {
                    return Err(GanttError::Calculation {
                        message: format!("missing backward vector for step `{}`", to),
                    });
                };
                if !backward.from.contains(&vector.from) {
                    return Err(GanttError::Calculation {
                        message: format!(
                            "backward vector for step `{}` does not contain source `{}`",
                            to, vector.from
                        ),
                    });
                }
            }
        }

        for vector in self.backward_step_relations.values() {
            for from in &vector.from {
                let Some(forward) = self.forward_task_step_vectors.get(from) else {
                    return Err(GanttError::Calculation {
                        message: format!("missing forward vector for step `{}`", from),
                    });
                };
                if !forward.to.contains(&vector.to) {
                    return Err(GanttError::Calculation {
                        message: format!(
                            "forward vector for step `{}` does not contain target `{}`",
                            from, vector.to
                        ),
                    });
                }
            }
        }
        Ok(())
    }

    fn validate_acyclic(&self, step_ids: &HashSet<S::Id>) -> GanttResult<()> {
        let mut indegrees: HashMap<S::Id, usize> = step_ids
            .iter()
            .map(|id| (id.clone(), 0))
            .collect();
        let mut adjacency: HashMap<S::Id, Vec<S::Id>> = HashMap::new();

        for vector in self.forward_task_step_vectors.values() {
            let entry = adjacency.entry(vector.from.clone()).or_default();
            for to in &vector.to {
                entry.push(to.clone());
                if let Some(indegree) = indegrees.get_mut(to) {
                    *indegree += 1;
                }
            }
        }

        let mut queue: VecDeque<S::Id> = indegrees
            .iter()
            .filter_map(|(id, indegree)| (*indegree == 0).then_some(id.clone()))
            .collect();
        let mut visited = 0usize;

        while let Some(step_id) = queue.pop_front() {
            visited += 1;
            for next in adjacency.get(&step_id).into_iter().flatten() {
                let indegree = indegrees.get_mut(next).expect("validated step id");
                *indegree -= 1;
                if *indegree == 0 {
                    queue.push_back(next.clone());
                }
            }
        }

        if visited != step_ids.len() {
            return Err(GanttError::Calculation {
                message: "task step graph must be a DAG".to_string(),
            });
        }
        Ok(())
    }

    fn ensure_step_ids_exist(
        &self,
        ids: &[S::Id],
        step_ids: &HashSet<S::Id>,
        context: &str,
    ) -> GanttResult<()> {
        for id in ids {
            self.ensure_step_id_exists(id, step_ids, context)?;
        }
        Ok(())
    }

    fn ensure_step_id_exists(
        &self,
        id: &S::Id,
        step_ids: &HashSet<S::Id>,
        context: &str,
    ) -> GanttResult<()> {
        if !step_ids.contains(id) {
            return Err(GanttError::Calculation {
                message: format!("unknown task step id `{}` in {}", id, context),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task::BasicExecutor;

    fn step(id: &str) -> BasicTaskStep<BasicExecutor> {
        BasicTaskStep::new(
            id,
            id,
            vec![BasicExecutor::new("exec_1", "Executor 1")],
            Duration::hours(1),
        )
    }

    #[test]
    fn test_task_step_graph_builds_dag() {
        let graph = TaskStepGraph::builder("graph_1", "Graph 1")
            .add_steps([step("a"), step("b"), step("c")])
            .start_steps(vec!["a"], StepRelation::And)
            .forward("a", vec!["b", "c"], StepRelation::And)
            .backward(vec!["a"], "b", StepRelation::And)
            .backward(vec!["a"], "c", StepRelation::And)
            .build()
            .unwrap();

        assert_eq!(
            graph.successors(&TaskStepId::from("a")),
            &[TaskStepId::from("b"), TaskStepId::from("c")],
        );
        assert_eq!(
            graph.predecessors(&TaskStepId::from("b")),
            &[TaskStepId::from("a")],
        );
        assert!(graph.step(&TaskStepId::from("c")).is_some());
    }

    #[test]
    fn test_task_step_graph_rejects_duplicate_steps() {
        let err = TaskStepGraph::builder("graph_1", "Graph 1")
            .add_steps([step("a"), step("a")])
            .start_steps(vec!["a"], StepRelation::And)
            .build()
            .unwrap_err();

        assert!(format!("{err}").contains("duplicated"));
    }

    #[test]
    fn test_task_step_graph_rejects_unknown_start_step() {
        let err = TaskStepGraph::builder("graph_1", "Graph 1")
            .add_step(step("a"))
            .start_steps(vec!["missing"], StepRelation::And)
            .build()
            .unwrap_err();

        assert!(format!("{err}").contains("unknown task step id"));
    }

    #[test]
    fn test_task_step_graph_rejects_inconsistent_vectors() {
        let err = TaskStepGraph::builder("graph_1", "Graph 1")
            .add_steps([step("a"), step("b")])
            .start_steps(vec!["a"], StepRelation::And)
            .forward("a", vec!["b"], StepRelation::And)
            .build()
            .unwrap_err();

        assert!(format!("{err}").contains("missing backward vector"));
    }

    #[test]
    fn test_task_step_graph_rejects_cycle() {
        let err = TaskStepGraph::builder("graph_1", "Graph 1")
            .add_steps([step("a"), step("b")])
            .start_steps(vec!["a"], StepRelation::And)
            .forward("a", vec!["b"], StepRelation::And)
            .forward("b", vec!["a"], StepRelation::And)
            .backward(vec!["b"], "a", StepRelation::And)
            .backward(vec!["a"], "b", StepRelation::And)
            .build()
            .unwrap_err();

        assert!(format!("{err}").contains("DAG"));
    }
}
