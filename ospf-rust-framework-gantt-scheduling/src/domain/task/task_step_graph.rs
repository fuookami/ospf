//! 任务步骤图 / Task step graph
//!
//! 提供多步任务的步骤、前后向依赖向量和 DAG 构建校验。
//! Provides steps, forward/backward dependency vectors, and DAG build validation
//! for multi-step tasks.

use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};

use time::Duration;

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
    /// 步骤 ID / Step ID
    fn id(&self) -> &str;

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
pub struct BasicTaskStep<E>
where
    E: ExecutorTrait,
{
    /// 步骤 ID / Step ID
    pub id: String,
    /// 步骤名称 / Step name
    pub name: String,
    /// 可用执行者 / Enabled executors
    pub enabled_executors: Vec<E>,
    /// 状态集合 / Status set
    pub status: HashSet<TaskStatus>,
    /// 默认持续时间 / Default duration
    pub duration: Duration,
}

impl<E> PartialEq for BasicTaskStep<E>
where
    E: ExecutorTrait,
{
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<E> Eq for BasicTaskStep<E>
where
    E: ExecutorTrait,
{
}

impl<E> Hash for BasicTaskStep<E>
where
    E: ExecutorTrait,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<E> BasicTaskStep<E>
where
    E: ExecutorTrait,
{
    /// 创建基础任务步骤 / Create basic task step
    pub fn new(
        id: impl Into<String>,
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

impl<E> TaskStepTrait<E> for BasicTaskStep<E>
where
    E: ExecutorTrait,
{
    fn id(&self) -> &str {
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
pub struct ForwardTaskStepVector {
    /// 源步骤 ID / Source step ID
    pub from: String,
    /// 目标步骤 ID 列表 / Target step ID list
    pub to: Vec<String>,
    /// 步骤关系 / Step relation
    pub relation: StepRelation,
}

impl ForwardTaskStepVector {
    /// 创建前向向量 / Create forward vector
    pub fn new(
        from: impl Into<String>,
        to: Vec<impl Into<String>>,
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
pub struct BackwardTaskStepVector {
    /// 源步骤 ID 列表 / Source step ID list
    pub from: Vec<String>,
    /// 目标步骤 ID / Target step ID
    pub to: String,
    /// 步骤关系 / Step relation
    pub relation: StepRelation,
}

impl BackwardTaskStepVector {
    /// 创建后向向量 / Create backward vector
    pub fn new(
        from: Vec<impl Into<String>>,
        to: impl Into<String>,
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
pub struct StartSteps {
    /// 起始步骤 ID 列表 / Start step ID list
    pub steps: Vec<String>,
    /// 步骤关系 / Step relation
    pub relation: StepRelation,
}

impl StartSteps {
    /// 创建起始步骤集合 / Create start step set
    pub fn new(steps: Vec<impl Into<String>>, relation: StepRelation) -> Self {
        Self {
            steps: steps.into_iter().map(Into::into).collect(),
            relation,
        }
    }
}

/// 任务步骤图 / Task step graph
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct TaskStepGraph<E, S>
where
    E: ExecutorTrait,
    S: TaskStepTrait<E>,
{
    /// 图 ID / Graph ID
    pub id: String,
    /// 图名称 / Graph name
    pub name: String,
    /// 步骤列表 / Step list
    pub steps: Vec<S>,
    /// 起始步骤集合 / Start step set
    pub start_steps: StartSteps,
    /// 前向步骤向量映射 / Forward step vector map
    pub forward_task_step_vectors: HashMap<String, ForwardTaskStepVector>,
    /// 后向步骤向量映射 / Backward step vector map
    pub backward_step_relations: HashMap<String, BackwardTaskStepVector>,
    _marker: std::marker::PhantomData<E>,
}

impl<E, S> TaskStepGraph<E, S>
where
    E: ExecutorTrait,
    S: TaskStepTrait<E>,
{
    /// 创建构建器 / Create builder
    pub fn builder(id: impl Into<String>, name: impl Into<String>) -> TaskStepGraphBuilder<E, S> {
        TaskStepGraphBuilder::new(id, name)
    }

    /// 按 ID 获取步骤 / Get step by ID
    pub fn step(&self, step_id: &str) -> Option<&S> {
        self.steps.iter().find(|step| step.id() == step_id)
    }

    /// 获取后继步骤 ID / Get successor step IDs
    pub fn successors(&self, step_id: &str) -> &[String] {
        self.forward_task_step_vectors
            .get(step_id)
            .map(|vector| vector.to.as_slice())
            .unwrap_or(&[])
    }

    /// 获取前置步骤 ID / Get predecessor step IDs
    pub fn predecessors(&self, step_id: &str) -> &[String] {
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

/// 任务步骤图构建器 / Task step graph builder
#[derive(Debug, Clone)]
pub struct TaskStepGraphBuilder<E, S>
where
    E: ExecutorTrait,
    S: TaskStepTrait<E>,
{
    id: String,
    name: String,
    steps: Vec<S>,
    start_steps: Option<StartSteps>,
    forward_task_step_vectors: HashMap<String, ForwardTaskStepVector>,
    backward_step_relations: HashMap<String, BackwardTaskStepVector>,
    _marker: std::marker::PhantomData<E>,
}

impl<E, S> TaskStepGraphBuilder<E, S>
where
    E: ExecutorTrait,
    S: TaskStepTrait<E>,
{
    /// 创建任务步骤图构建器 / Create task step graph builder
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
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
        steps: Vec<impl Into<String>>,
        relation: StepRelation,
    ) -> Self {
        self.start_steps = Some(StartSteps::new(steps, relation));
        self
    }

    /// 添加前向关系 / Add forward relation
    pub fn forward(
        mut self,
        from: impl Into<String>,
        to: Vec<impl Into<String>>,
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
        from: Vec<impl Into<String>>,
        to: impl Into<String>,
        relation: StepRelation,
    ) -> Self {
        let vector = BackwardTaskStepVector::new(from, to, relation);
        self.backward_step_relations
            .insert(vector.to.clone(), vector);
        self
    }

    /// 构建任务步骤图 / Build task step graph
    pub fn build(self) -> GanttResult<TaskStepGraph<E, S>> {
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

    fn validate_steps(&self) -> GanttResult<HashSet<String>> {
        let mut ids = HashSet::with_capacity(self.steps.len());
        for step in &self.steps {
            if step.id().is_empty() {
                return Err(GanttError::Calculation {
                    message: "task step id must not be empty".to_string(),
                });
            }
            if !ids.insert(step.id().to_string()) {
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

    fn validate_start_steps(&self, step_ids: &HashSet<String>) -> GanttResult<StartSteps> {
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

    fn validate_forward_vectors(&self, step_ids: &HashSet<String>) -> GanttResult<()> {
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

    fn validate_backward_vectors(&self, step_ids: &HashSet<String>) -> GanttResult<()> {
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

    fn validate_acyclic(&self, step_ids: &HashSet<String>) -> GanttResult<()> {
        let mut indegrees: HashMap<String, usize> = step_ids
            .iter()
            .map(|id| (id.clone(), 0))
            .collect();
        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();

        for vector in self.forward_task_step_vectors.values() {
            let entry = adjacency.entry(vector.from.clone()).or_default();
            for to in &vector.to {
                entry.push(to.clone());
                if let Some(indegree) = indegrees.get_mut(to) {
                    *indegree += 1;
                }
            }
        }

        let mut queue: VecDeque<String> = indegrees
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
        ids: &[String],
        step_ids: &HashSet<String>,
        context: &str,
    ) -> GanttResult<()> {
        for id in ids {
            self.ensure_step_id_exists(id, step_ids, context)?;
        }
        Ok(())
    }

    fn ensure_step_id_exists(
        &self,
        id: &str,
        step_ids: &HashSet<String>,
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

        assert_eq!(graph.successors("a"), &["b".to_string(), "c".to_string()]);
        assert_eq!(graph.predecessors("b"), &["a".to_string()]);
        assert!(graph.step("c").is_some());
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
