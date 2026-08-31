//! 任务编译上下文 / Task compilation context
//!
//! 映射 Kotlin `gantt-scheduling-domain-task-compilation-context` 子模块。
//! Maps the Kotlin `gantt-scheduling-domain-task-compilation-context` submodule.
//!
//! # 核心模块 / Core Modules
//!
//! - [`adapter`]: 建模适配器（变量集合、表达式构建、结果提取）
//! - [`model`]: 编译组件（Compilation、TaskTime、Makespan、Switch、Solution）
//! - [`service`]: 服务与限制（SolutionAnalyzer、Limits Pipeline）

pub mod adapter;
pub mod context;
pub mod iterative;
pub mod model;
pub mod service;

// ========================================================================
// 公共重导出 / Public re-exports
// ========================================================================

pub use adapter::{
    IndexedVariableArray1,
    IndexedVariableArray2,
    IndexedVariableArray3,
    ModelComponent,
    extract_value,
    extract_binary,
    extract_binary_values_1,
    extract_binary_values_2,
    extract_values_1,
    extract_values_2,
    sum_to_linear,
    build_linear_expression_symbol,
    symbols_to_indexed_1d,
    symbols_to_indexed_2d,
    optional_symbols_to_indexed,
    IndexedVariableCombination1,
    IndexedVariableCombination2,
    IndexedLinearExpressionSymbols1,
    IndexedLinearExpressionSymbols2,
    OptionalIndexedLinearExpressionSymbols,
};

pub use model::{
    Compilation,
    TaskTime,
    Makespan,
    Switch,
    TaskSolution,
    TaskSolutionSummary,
    TaskTimeInfo,
};

pub use service::{
    SolutionAnalyzer,
};

pub use iterative::{
    IterativeTaskCompilation,
    AddedTaskColumn,
};

pub use context::{
    IterativeTaskCompilationContext,
    BasicTaskCompilationContext,
};

pub use service::limits::{
    TaskCompilationConstraint,
    ExecutorCompilationConstraint,
    TaskConflictConstraint,
    TaskTimeConflictConstraint,
    TaskDelayTimeConstraint,
    TaskAdvanceTimeConstraint,
    TaskOverMaxDelayTimeConstraint,
    TaskOverMaxAdvanceTimeConstraint,
    TaskDelayLastEndTimeConstraint,
    TaskAdvanceEarliestEndTimeConstraint,
    TaskExecutorCostMinimization,
    TaskCostMinimization,
    MakespanMinimization,
    SwitchCostMinimization,
    SwitchTimeMinimization,
    TaskDelayTimeMinimization,
    TaskAdvanceTimeMinimization,
    ExecutorCostMinimization,
    ExecutorLeisureMinimization,
};

// ============================================================================
// Aggregation 类型 / Aggregation Types
// ============================================================================

use std::marker::PhantomData;
use ospf_rust_core::model::MetaModel;
use crate::domain::task::{ExecutorTrait, TaskTrait, AssignmentPolicyTrait};
use crate::GanttResult;

/// 任务编译聚合 / Task compilation aggregation
///
/// 编排 Compilation 和 Switch 组件的注册顺序。
/// Orchestrates the registration order of Compilation and Switch components.
pub struct TaskCompilationAggregation<T, E, A>
where
    E: ExecutorTrait,
    A: AssignmentPolicyTrait<E>,
    T: TaskTrait<E, A>,
{
    /// 编译组件 / Compilation component
    pub compilation: Compilation<T, E, A>,
    /// 切换组件 / Switch component
    pub switch: Switch,
    /// 类型标记 / Type marker
    _marker: PhantomData<A>,
}

impl<T, E, A> std::fmt::Debug for TaskCompilationAggregation<T, E, A>
where
    E: ExecutorTrait,
    A: AssignmentPolicyTrait<E>,
    T: TaskTrait<E, A>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskCompilationAggregation")
            .field("compilation", &self.compilation)
            .field("switch", &self.switch)
            .finish()
    }
}

impl<T, E, A> TaskCompilationAggregation<T, E, A>
where
    E: ExecutorTrait,
    A: AssignmentPolicyTrait<E>,
    T: TaskTrait<E, A>,
{
    /// 创建新的任务编译聚合 / Create new task compilation aggregation
    pub fn new(
        tasks: Vec<T>,
        executors: Vec<E>,
        task_cancel_enabled: bool,
        with_executor_leisure: bool,
        switch_enabled: bool,
    ) -> Self {
        Self {
            compilation: Compilation::new(tasks, executors, task_cancel_enabled, with_executor_leisure),
            switch: Switch::new(switch_enabled),
            _marker: PhantomData,
        }
    }

    /// 注册所有组件到模型 / Register all components to model
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        self.compilation.register(model)?;
        self.switch.register_with_compilation(&self.compilation, model)?;
        Ok(())
    }
}

/// 带时间的任务编译聚合 / Task compilation aggregation with time
///
/// 编排 Compilation、Switch、TaskTime 和 Makespan 组件的注册顺序。
/// Orchestrates the registration order of Compilation, Switch, TaskTime and Makespan components.
pub struct TaskCompilationAggregationWithTime<T, E, A>
where
    E: ExecutorTrait,
    A: AssignmentPolicyTrait<E>,
    T: TaskTrait<E, A>,
{
    /// 编译组件 / Compilation component
    pub compilation: Compilation<T, E, A>,
    /// 切换组件 / Switch component
    pub switch: Switch,
    /// 时间组件 / Task time component
    pub task_time: TaskTime,
    /// 完工时间组件 / Makespan component
    pub makespan: Makespan,
    /// 类型标记 / Type marker
    _marker: PhantomData<A>,
}

impl<T, E, A> std::fmt::Debug for TaskCompilationAggregationWithTime<T, E, A>
where
    E: ExecutorTrait,
    A: AssignmentPolicyTrait<E>,
    T: TaskTrait<E, A>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskCompilationAggregationWithTime")
            .field("compilation", &self.compilation)
            .field("switch", &self.switch)
            .field("task_time", &self.task_time)
            .field("makespan", &self.makespan)
            .finish()
    }
}

impl<T, E, A> TaskCompilationAggregationWithTime<T, E, A>
where
    E: ExecutorTrait,
    A: AssignmentPolicyTrait<E>,
    T: TaskTrait<E, A>,
{
    /// 创建新的带时间的任务编译聚合 / Create new task compilation aggregation with time
    pub fn new(
        tasks: Vec<T>,
        executors: Vec<E>,
        task_cancel_enabled: bool,
        with_executor_leisure: bool,
        switch_enabled: bool,
        delay_enabled: bool,
        over_max_delay_enabled: bool,
        advance_enabled: bool,
        over_max_advance_enabled: bool,
        makespan_extra: bool,
    ) -> Self {
        Self {
            compilation: Compilation::new(tasks, executors, task_cancel_enabled, with_executor_leisure),
            switch: Switch::new(switch_enabled),
            task_time: TaskTime::new(delay_enabled, over_max_delay_enabled, advance_enabled, over_max_advance_enabled),
            makespan: Makespan::new(makespan_extra),
            _marker: PhantomData,
        }
    }

    /// 注册所有组件到模型 / Register all components to model
    ///
    /// `scheduled_starts` 和 `durations` 用于 TaskTime 的时间约束和 slack 变量。
    /// `scheduled_starts` and `durations` are used by TaskTime for time constraints and slack variables.
    pub fn register(
        &mut self,
        scheduled_starts: Vec<Option<f64>>,
        durations: Vec<f64>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<()> {
        self.compilation.register(model)?;
        self.task_time.register(
            self.compilation.tasks.len(),
            scheduled_starts,
            durations,
            model,
        )?;
        self.switch.register_with_task_time(&self.compilation, &self.task_time, model)?;
        self.makespan.register(&self.task_time, model)?;
        Ok(())
    }
}

/// 迭代任务编译聚合兼容壳（已迁移至 iterative 模块）/ Iterative task compilation aggregation compatibility shell (moved to iterative module)
#[derive(Debug, Clone, Default)]
#[deprecated(note = "Use IterativeTaskCompilationAggregation from iterative module instead")]
pub struct IterativeAggregation;

/// 迭代任务编译上下文兼容壳（已迁移至 iterative 模块）/ Iterative task compilation context compatibility shell (moved to iterative module)
#[derive(Debug, Clone, Default)]
#[deprecated(note = "Use IterativeTaskCompilationContext from context module instead")]
pub struct IterativeContext;
