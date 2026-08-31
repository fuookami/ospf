//! 任务编译模型组件 / Task compilation model components
//!
//! 包含任务调度的核心建模组件：Compilation、TaskTime、Makespan、Switch、Solution。
//! Contains core modeling components for task scheduling: Compilation, TaskTime, Makespan, Switch, Solution.

use std::marker::PhantomData;
use std::sync::Arc;

use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::expression_symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::function::{
    AndFunction, InequalityFunction, InequalityKind, MaskingFunction,
};
use ospf_rust_core::symbol::functions::max_min::MinMaxFunction;
use ospf_rust_core::symbol::functions::slack::SlackFunction;
use ospf_rust_core::symbol::LinearIntermediateSymbol;
use ospf_rust_core::variable::{Binary, UContinuous};

use crate::domain::task::{AssignmentPolicyTrait, ExecutorTrait, TaskTrait};
use crate::domain::task_compilation::adapter::{
    build_linear_expression_symbol, next_gantt_symbol_id,
    IndexedVariableArray1, IndexedVariableArray2,
    symbols_to_indexed_1d,
    IndexedLinearExpressionSymbols1,
};
use crate::GanttError;
use crate::GanttResult;

// ============================================================================
// Compilation 组件 / Compilation Component
// ============================================================================

/// 编译组件 / Compilation component
///
/// 管理任务分配变量 `x[task, executor]`、取消变量 `y[task]`、
/// 空闲变量 `z[executor]`，以及编译中间表达式。
///
/// Manages task assignment variables `x[task, executor]`, cancellation variables
/// `y[task]`, leisure variables `z[executor]`, and compilation intermediate expressions.
pub struct Compilation<T, E, A>
where
    E: ExecutorTrait,
    A: AssignmentPolicyTrait<E>,
    T: TaskTrait<E, A>,
{
    /// 任务列表 / Task list
    pub tasks: Vec<T>,
    /// 执行者列表 / Executor list
    pub executors: Vec<E>,
    /// 是否允许取消任务 / Whether task cancellation is enabled
    pub task_cancel_enabled: bool,
    /// 是否包含执行者空闲变量 / Whether executor leisure variables are included
    pub with_executor_leisure: bool,

    /// 分配变量 x[task, executor] / Assignment variable x[task, executor]
    pub x: Option<IndexedVariableArray2<usize, usize, Binary>>,
    /// 取消变量 y[task] / Cancellation variable y[task]
    pub y: Option<IndexedVariableArray1<usize, Binary>>,
    /// 空闲变量 z[executor] / Leisure variable z[executor]
    pub z: Option<IndexedVariableArray1<usize, Binary>>,

    /// 任务分配中间符号 / Task assignment intermediate symbols
    /// task_assignment[task] = sum(x[task, executor] for executor)
    pub task_assignment_symbols: Vec<Arc<LinearExpressionSymbol<f64>>>,
    /// 任务编译中间符号 / Task compilation intermediate symbols
    /// task_compilation[task] = sum(x[task, executor]) + y[task] (if cancel)
    pub task_compilation_symbols: Vec<Arc<LinearExpressionSymbol<f64>>>,
    /// 执行者编译中间符号 / Executor compilation intermediate symbols
    /// executor_compilation[executor] = sum(x[task, executor] for task) + z[executor] (if leisure)
    pub executor_compilation_symbols: Vec<Arc<LinearExpressionSymbol<f64>>>,

    /// 索引任务分配中间符号 / Indexed task assignment intermediate symbols
    pub task_assignment_indexed: Option<IndexedLinearExpressionSymbols1<usize>>,
    /// 索引任务编译中间符号 / Indexed task compilation intermediate symbols
    pub task_compilation_indexed: Option<IndexedLinearExpressionSymbols1<usize>>,
    /// 索引执行者编译中间符号 / Indexed executor compilation intermediate symbols
    pub executor_compilation_indexed: Option<IndexedLinearExpressionSymbols1<usize>>,

    /// 类型标记 / Type marker
    _marker: PhantomData<A>,
}

impl<T, E, A> std::fmt::Debug for Compilation<T, E, A>
where
    E: ExecutorTrait,
    A: AssignmentPolicyTrait<E>,
    T: TaskTrait<E, A>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Compilation")
            .field("task_cancel_enabled", &self.task_cancel_enabled)
            .field("with_executor_leisure", &self.with_executor_leisure)
            .field("n_tasks", &self.tasks.len())
            .field("n_executors", &self.executors.len())
            .finish()
    }
}

impl<T, E, A> Compilation<T, E, A>
where
    E: ExecutorTrait,
    A: AssignmentPolicyTrait<E>,
    T: TaskTrait<E, A>,
{
    /// 创建新的编译组件 / Create new compilation component
    pub fn new(
        tasks: Vec<T>,
        executors: Vec<E>,
        task_cancel_enabled: bool,
        with_executor_leisure: bool,
    ) -> Self {
        Self {
            tasks,
            executors,
            task_cancel_enabled,
            with_executor_leisure,
            x: None,
            y: None,
            z: None,
            task_assignment_symbols: Vec::new(),
            task_compilation_symbols: Vec::new(),
            executor_compilation_symbols: Vec::new(),
            task_assignment_indexed: None,
            task_compilation_indexed: None,
            executor_compilation_indexed: None,
            _marker: PhantomData,
        }
    }

    /// 注册到模型 / Register to model
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        let n_tasks = self.tasks.len();
        let n_executors = self.executors.len();

        // Register x[task, executor] - binary assignment variables
        let task_indices: Vec<usize> = (0..n_tasks).collect();
        let executor_indices: Vec<usize> = (0..n_executors).collect();
        self.x = Some(IndexedVariableArray2::new(
            "x", &task_indices, &executor_indices, model,
        )?);

        // Register y[task] - binary cancellation variables (if enabled)
        if self.task_cancel_enabled {
            self.y = Some(IndexedVariableArray1::new("y", &task_indices, model)?);
        }

        // Register z[executor] - binary leisure variables (if enabled)
        if self.with_executor_leisure {
            self.z = Some(IndexedVariableArray1::new("z", &executor_indices, model)?);
        }

        // Build intermediate expression symbols
        let x = self.x.as_ref()
            .ok_or_else(|| GanttError::Calculation {
                message: "x variables must be initialized before building intermediate symbols".to_string(),
            })?;

        // 构建 task_assignment[task] = sum(x[task, executor] for executor)
        for ti in 0..n_tasks {
            let mut terms = Vec::new();
            for ei in 0..n_executors {
                if let Some(idx) = x.model_index(&ti, &ei) {
                    terms.push((idx, 1.0));
                }
            }
            let symbol = build_linear_expression_symbol(
                &format!("task_assignment_{}", ti), &terms, 0.0,
            );
            model.add_symbol(symbol.clone())
                .map_err(|e| GanttError::Calculation {
                    message: format!("Failed to register task_assignment_{}: {:?}", ti, e),
                })?;
            self.task_assignment_symbols.push(symbol);
        }

        // 构建 task_compilation[task] = sum(x[task, executor]) + y[task] (if cancel)
        for ti in 0..n_tasks {
            let mut terms = Vec::new();
            for ei in 0..n_executors {
                if let Some(idx) = x.model_index(&ti, &ei) {
                    terms.push((idx, 1.0));
                }
            }
            if let Some(ref y) = self.y {
                if let Some(idx) = y.model_index(&ti) {
                    terms.push((idx, 1.0));
                }
            }
            let symbol = build_linear_expression_symbol(
                &format!("task_compilation_{}", ti), &terms, 0.0,
            );
            model.add_symbol(symbol.clone())
                .map_err(|e| GanttError::Calculation {
                    message: format!("Failed to register task_compilation_{}: {:?}", ti, e),
                })?;
            self.task_compilation_symbols.push(symbol);
        }

        // 构建 executor_compilation[executor] = sum(x[task, executor]) + z[executor] (if leisure)
        for ei in 0..n_executors {
            let mut terms = Vec::new();
            for ti in 0..n_tasks {
                if let Some(idx) = x.model_index(&ti, &ei) {
                    terms.push((idx, 1.0));
                }
            }
            if let Some(ref z) = self.z {
                if let Some(idx) = z.model_index(&ei) {
                    terms.push((idx, 1.0));
                }
            }
            let symbol = build_linear_expression_symbol(
                &format!("executor_compilation_{}", ei), &terms, 0.0,
            );
            model.add_symbol(symbol.clone())
                .map_err(|e| GanttError::Calculation {
                    message: format!("Failed to register executor_compilation_{}: {:?}", ei, e),
                })?;
            self.executor_compilation_symbols.push(symbol);
        }

        // 构建索引符号组合 / Build indexed symbol combinations
        let task_keys: Vec<usize> = (0..n_tasks).collect();
        let executor_keys: Vec<usize> = (0..n_executors).collect();

        self.task_assignment_indexed = Some(symbols_to_indexed_1d(
            "task_assignment", &task_keys, &self.task_assignment_symbols,
        ));
        self.task_compilation_indexed = Some(symbols_to_indexed_1d(
            "task_compilation", &task_keys, &self.task_compilation_symbols,
        ));
        self.executor_compilation_indexed = Some(symbols_to_indexed_1d(
            "executor_compilation", &executor_keys, &self.executor_compilation_symbols,
        ));

        Ok(())
    }

    /// 获取 task_compilation[task] 的所有变量模型索引 / Get all variable model indices for task_compilation[task]
    ///
    /// 返回 `Vec<Vec<usize>>`，其中外层是任务索引，内层是组成表达式的变量索引。
    /// Returns `Vec<Vec<usize>>`, where outer is task index, inner is variable indices composing the expression.
    pub fn task_compilation_model_indices(&self) -> Vec<Vec<usize>> {
        self.task_compilation_symbols.iter()
            .map(|s| {
                let poly = s.to_linear_polynomial();
                poly.monomials().iter().map(|m: &ospf_rust_core::model::flatten::LinearMonomial<f64>| m.var_index()).collect()
            })
            .collect()
    }

    /// 获取 executor_compilation[executor] 的所有变量模型索引 / Get all variable model indices for executor_compilation[executor]
    pub fn executor_compilation_model_indices(&self) -> Vec<Vec<usize>> {
        self.executor_compilation_symbols.iter()
            .map(|s| {
                let poly = s.to_linear_polynomial();
                poly.monomials().iter().map(|m: &ospf_rust_core::model::flatten::LinearMonomial<f64>| m.var_index()).collect()
            })
            .collect()
    }
}

// ============================================================================
// TaskTime 组件 / TaskTime Component
// ============================================================================

/// 任务的排程时间信息 / Task scheduling time information
///
/// 保存从 SolutionAnalyzer 提取的时间结果。
/// Stores time results extracted by SolutionAnalyzer.
#[derive(Debug, Clone)]
pub struct TaskTimeInfo {
    /// 任务索引 / Task index
    pub task_index: usize,
    /// 预估开始时间 / Estimate start time
    pub estimate_start_time: f64,
    /// 预估结束时间 / Estimate end time
    pub estimate_end_time: f64,
    /// 延迟时间 / Delay time
    pub delay_time: f64,
    /// 提前时间 / Advance time
    pub advance_time: f64,
}

/// 任务时间组件 / Task time component
///
/// 管理预估开始时间 `est[task]`、延迟/提前时间变量和相关中间表达式。
/// Manages estimate start time `est[task]`, delay/advance time variables,
/// and related intermediate expressions.
pub struct TaskTime {
    /// 是否允许延迟 / Whether delay is enabled
    pub delay_enabled: bool,
    /// 是否允许超最大延迟 / Whether over-max delay is enabled
    pub over_max_delay_enabled: bool,
    /// 是否允许提前 / Whether advance is enabled
    pub advance_enabled: bool,
    /// 是否允许超最大提前 / Whether over-max advance is enabled
    pub over_max_advance_enabled: bool,

    /// 预估开始时间变量 est[task] / Estimate start time variable est[task]
    pub est: Option<IndexedVariableArray1<usize, UContinuous>>,

    /// 预估开始时间中间符号 / Estimate start time intermediate symbols
    /// estimate_start_time[task] = est[task]
    pub estimate_start_time_symbols: Vec<Arc<LinearExpressionSymbol<f64>>>,

    /// 预估结束时间中间符号 / Estimate end time intermediate symbols
    /// estimate_end_time[task] = est[task] + duration[task]
    pub estimate_end_time_symbols: Vec<Arc<LinearExpressionSymbol<f64>>>,

    /// 索引预估开始时间中间符号 / Indexed estimate start time intermediate symbols
    pub estimate_start_time_indexed: Option<IndexedLinearExpressionSymbols1<usize>>,
    /// 索引预估结束时间中间符号 / Indexed estimate end time intermediate symbols
    pub estimate_end_time_indexed: Option<IndexedLinearExpressionSymbols1<usize>>,

    /// 延迟时间 slack 函数 / Delay time slack functions
    /// delay_time[task] = max(0, est[task] - scheduled_start[task])
    pub delay_time_slacks: Vec<Option<Arc<SlackFunction<f64>>>>,

    /// 提前时间 slack 函数 / Advance time slack functions
    /// advance_time[task] = max(0, scheduled_start[task] - est[task])
    pub advance_time_slacks: Vec<Option<Arc<SlackFunction<f64>>>>,

    /// 延迟时间结果变量模型索引 / Delay time result variable model indices
    pub delay_time_model_indices: Vec<Option<usize>>,

    /// 提前时间结果变量模型索引 / Advance time result variable model indices
    pub advance_time_model_indices: Vec<Option<usize>>,

    /// 任务的排程开始时间（solver 值域）/ Scheduled start times in solver value domain
    /// scheduled_start[task] — 由调用方提供
    pub scheduled_starts: Vec<Option<f64>>,

    /// 任务的持续时间（solver 值域）/ Task durations in solver value domain
    pub durations: Vec<f64>,
}

impl std::fmt::Debug for TaskTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskTime")
            .field("delay_enabled", &self.delay_enabled)
            .field("advance_enabled", &self.advance_enabled)
            .field("n_tasks", &self.scheduled_starts.len())
            .finish()
    }
}

impl TaskTime {
    /// 创建新的任务时间组件 / Create new task time component
    pub fn new(
        delay_enabled: bool,
        over_max_delay_enabled: bool,
        advance_enabled: bool,
        over_max_advance_enabled: bool,
    ) -> Self {
        Self {
            delay_enabled,
            over_max_delay_enabled,
            advance_enabled,
            over_max_advance_enabled,
            est: None,
            estimate_start_time_symbols: Vec::new(),
            estimate_end_time_symbols: Vec::new(),
            estimate_start_time_indexed: None,
            estimate_end_time_indexed: None,
            delay_time_slacks: Vec::new(),
            advance_time_slacks: Vec::new(),
            delay_time_model_indices: Vec::new(),
            advance_time_model_indices: Vec::new(),
            scheduled_starts: Vec::new(),
            durations: Vec::new(),
        }
    }

    /// 注册到模型 / Register to model
    ///
    /// `scheduled_starts` 和 `durations` 提供每个任务的排程开始时间和持续时间
    /// （solver 值域，None 表示无排程时间约束）。
    ///
    /// `scheduled_starts` and `durations` provide each task's scheduled start time and duration
    /// (in solver value domain, None means no scheduling time constraint).
    pub fn register(
        &mut self,
        n_tasks: usize,
        scheduled_starts: Vec<Option<f64>>,
        durations: Vec<f64>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<()> {
        assert_eq!(scheduled_starts.len(), n_tasks, "scheduled_starts length mismatch");
        assert_eq!(durations.len(), n_tasks, "durations length mismatch");

        self.scheduled_starts = scheduled_starts.clone();
        self.durations = durations.clone();

        let task_indices: Vec<usize> = (0..n_tasks).collect();
        self.est = Some(IndexedVariableArray1::new("est", &task_indices, model)?);

        let est = self.est.as_ref()
            .ok_or_else(|| GanttError::Calculation {
                message: "est variables must be initialized".to_string(),
            })?;

        // 构建 estimate_start_time[task] = est[task]
        // 和 estimate_end_time[task] = est[task] + duration[task]
        for ti in 0..n_tasks {
            if let Some(est_idx) = est.model_index(&ti) {
                // estimate_start_time
                let start_symbol = build_linear_expression_symbol(
                    &format!("estimate_start_time_{}", ti),
                    &[(est_idx, 1.0)],
                    0.0,
                );
                model.add_symbol(start_symbol.clone())
                    .map_err(|e| GanttError::Calculation {
                        message: format!("Failed to register estimate_start_time_{}: {:?}", ti, e),
                    })?;
                self.estimate_start_time_symbols.push(start_symbol);

                // estimate_end_time
                let duration = self.durations[ti];
                let end_symbol = build_linear_expression_symbol(
                    &format!("estimate_end_time_{}", ti),
                    &[(est_idx, 1.0)],
                    duration,
                );
                model.add_symbol(end_symbol.clone())
                    .map_err(|e| GanttError::Calculation {
                        message: format!("Failed to register estimate_end_time_{}: {:?}", ti, e),
                    })?;
                self.estimate_end_time_symbols.push(end_symbol);
            }
        }

        // 构建延迟和提前 slack 变量 / Build delay and advance slack variables
        for ti in 0..n_tasks {
            let scheduled_start = self.scheduled_starts[ti];

            if let (Some(est_idx), Some(ss)) = (est.model_index(&ti), scheduled_start) {
                // delay_time = max(0, est - scheduled_start)
                if self.delay_enabled {
                    let est_linear = Linear::new(
                        vec![ospf_rust_core::model::flatten::LinearMonomial::new(1.0, est_idx)],
                        0.0,
                    );
                    let ss_linear = Linear::new(
                        vec![],
                        ss,
                    );
                    let delay_slack = Arc::new(SlackFunction::named(
                        &format!("delay_time_{}", ti),
                        est_linear,
                        ss_linear,
                    ));
                    // SlackFunction 注册到模型时自动注册其 result_variable
                    // SlackFunction auto-registers its result_variable when added to model
                    model.add_symbol(delay_slack.clone())
                        .map_err(|e| GanttError::Calculation {
                            message: format!("Failed to register delay_time_{} symbol: {:?}", ti, e),
                        })?;
                    // 通过 find_token 获取 result_variable 的求解器索引
                    // Get solver index for the result_variable via find_token
                    let delay_var_id = delay_slack.result_variable().id();
                    let delay_var_idx = model.find_token(delay_var_id)
                        .map(|t| t.solver_index)
                        .ok_or_else(|| GanttError::Calculation {
                            message: format!("delay_time_{} result variable not found in model tokens", ti),
                        })?;
                    self.delay_time_slacks.push(Some(delay_slack));
                    self.delay_time_model_indices.push(Some(delay_var_idx));
                } else {
                    self.delay_time_slacks.push(None);
                    self.delay_time_model_indices.push(None);
                }

                // advance_time = max(0, scheduled_start - est)
                if self.advance_enabled {
                    let ss_linear = Linear::new(
                        vec![],
                        ss,
                    );
                    let est_linear = Linear::new(
                        vec![ospf_rust_core::model::flatten::LinearMonomial::new(1.0, est_idx)],
                        0.0,
                    );
                    let advance_slack = Arc::new(SlackFunction::named(
                        &format!("advance_time_{}", ti),
                        ss_linear,
                        est_linear,
                    ));
                    model.add_symbol(advance_slack.clone())
                        .map_err(|e| GanttError::Calculation {
                            message: format!("Failed to register advance_time_{} symbol: {:?}", ti, e),
                        })?;
                    let advance_var_id = advance_slack.result_variable().id();
                    let advance_var_idx = model.find_token(advance_var_id)
                        .map(|t| t.solver_index)
                        .ok_or_else(|| GanttError::Calculation {
                            message: format!("advance_time_{} result variable not found in model tokens", ti),
                        })?;
                    self.advance_time_slacks.push(Some(advance_slack));
                    self.advance_time_model_indices.push(Some(advance_var_idx));
                } else {
                    self.advance_time_slacks.push(None);
                    self.advance_time_model_indices.push(None);
                }
            } else {
                // 无排程时间的任务不需要 slack 变量
                self.delay_time_slacks.push(None);
                self.delay_time_model_indices.push(None);
                self.advance_time_slacks.push(None);
                self.advance_time_model_indices.push(None);
            }
        }

        // 构建索引符号组合 / Build indexed symbol combinations
        let task_keys: Vec<usize> = (0..n_tasks).collect();
        self.estimate_start_time_indexed = Some(symbols_to_indexed_1d(
            "estimate_start_time", &task_keys, &self.estimate_start_time_symbols,
        ));
        self.estimate_end_time_indexed = Some(symbols_to_indexed_1d(
            "estimate_end_time", &task_keys, &self.estimate_end_time_symbols,
        ));

        Ok(())
    }

    /// 获取 est[task] 的模型索引列表 / Get model indices for est variables
    pub fn est_model_indices(&self) -> Vec<Option<usize>> {
        self.est.as_ref()
            .map(|est| {
                (0..est.len()).map(|ti| est.model_index(&ti)).collect()
            })
            .unwrap_or_default()
    }
}

// ============================================================================
// Makespan 组件 / Makespan Component
// ============================================================================

/// 完工时间组件 / Makespan component
///
/// 表示所有任务中最晚的结束时间，使用 `MinMaxFunction` 实现。
/// Represents the latest end time among all tasks, implemented with `MinMaxFunction`.
pub struct Makespan {
    /// 是否使用 MinMax（精确最大值）/ Whether to use MinMax (exact maximum)
    pub extra: bool,
    /// makespan 结果变量模型索引 / Makespan result variable model index
    pub makespan_model_index: Option<usize>,
}

impl std::fmt::Debug for Makespan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Makespan")
            .field("extra", &self.extra)
            .field("registered", &self.makespan_model_index.is_some())
            .finish()
    }
}

impl Makespan {
    /// 创建新的完工时间组件 / Create new makespan component
    pub fn new(extra: bool) -> Self {
        Self {
            extra,
            makespan_model_index: None,
        }
    }

    /// 注册到模型 / Register to model
    ///
    /// 收集 `task_time.estimate_end_time` 的表达式，创建 `MinMaxFunction` 并注册。
    /// Collects `task_time.estimate_end_time` expressions, creates `MinMaxFunction` and registers it.
    pub fn register(
        &mut self,
        task_time: &TaskTime,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<()> {
        let end_time_polynomials: Vec<Linear<f64>> = task_time.estimate_end_time_symbols.iter()
            .map(|s: &Arc<LinearExpressionSymbol<f64>>| s.to_linear_polynomial())
            .collect();

        if end_time_polynomials.is_empty() {
            return Ok(());
        }

        let makespan_fn = MinMaxFunction::new(
            crate::domain::task_compilation::adapter::next_gantt_symbol_id(),
            "makespan",
            end_time_polynomials,
        );

        // MinMaxFunction 注册到模型时自动注册其辅助变量
        // MinMaxFunction auto-registers its auxiliary variables when added to model
        model.add_symbol(Arc::new(makespan_fn.clone()))
            .map_err(|e| GanttError::Calculation {
                message: format!("Failed to register makespan symbol: {:?}", e),
            })?;

        // 通过 find_token 获取 result_variable 的求解器索引
        // Get solver index for the result_variable via find_token
        let makespan_var_id = makespan_fn.result_variable().id();
        let makespan_idx = model.find_token(makespan_var_id)
            .map(|t| t.solver_index)
            .ok_or_else(|| GanttError::Calculation {
                message: "makespan result variable not found in model tokens".to_string(),
            })?;

        self.makespan_model_index = Some(makespan_idx);
        Ok(())
    }
}

// ============================================================================
// Switch 组件 / Switch Component
// ============================================================================

/// 切换组件 / Switch component
///
/// 管理执行器内任务间的切换符号和切换时间符号。
///
/// Manages executor switch symbols and switch-time symbols.
#[derive(Debug)]
pub struct Switch {
    /// 是否启用 / Whether enabled
    pub enabled: bool,
    /// front_of[from_task, to_task] 指示符号 / front_of[from_task, to_task] indicator symbols
    pub front_of_symbols: Vec<Option<Arc<InequalityFunction<f64>>>>,
    /// between_in[middle_task, from_task, to_task] 指示符号 / between_in[middle_task, from_task, to_task] indicator symbols
    pub between_in_symbols: Vec<Option<Arc<AndFunction<f64>>>>,
    /// switch[executor, from_task, to_task] 符号 / switch[executor, from_task, to_task] symbols
    pub switch_symbols: Vec<Arc<AndFunction<f64>>>,
    /// switch_time[executor, from_task, to_task] 遮罩符号 / switch_time[executor, from_task, to_task] masking symbols
    pub switch_time_mask_symbols: Vec<Option<Arc<MaskingFunction<f64>>>>,
    /// switch_time[from_task, to_task] 汇总符号 / switch_time[from_task, to_task] aggregate symbols
    pub switch_time_symbols: Vec<Option<Arc<LinearExpressionSymbol<f64>>>>,
    /// switch 符号的模型索引 / Model indices of switch symbols
    pub switch_model_indices: Vec<Option<usize>>,
    /// 任务数量 / Task count
    pub task_count: usize,
    /// 执行器数量 / Executor count
    pub executor_count: usize,
}

impl Switch {
    /// 创建新的切换组件 / Create new switch component
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            front_of_symbols: Vec::new(),
            between_in_symbols: Vec::new(),
            switch_symbols: Vec::new(),
            switch_time_mask_symbols: Vec::new(),
            switch_time_symbols: Vec::new(),
            switch_model_indices: Vec::new(),
            task_count: 0,
            executor_count: 0,
        }
    }

    /// 注册到模型 / Register to model
    ///
    /// 兼容旧入口；启用切换时应通过 `register_with_compilation` 提供编译上下文。
    /// Compatibility entry; enabled switch registration needs `register_with_compilation`.
    pub fn register(&mut self, _model: &mut MetaModel<f64>) -> GanttResult<()> {
        if self.enabled {
            return Err(GanttError::Unsupported {
                message: "enabled switch requires register_with_compilation".to_string(),
            });
        }
        Ok(())
    }

    /// 携带任务编译上下文注册切换符号 / Register switch symbols with task compilation context
    pub fn register_with_compilation<T, E, A>(
        &mut self,
        compilation: &Compilation<T, E, A>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<()>
    where
        E: ExecutorTrait,
        A: AssignmentPolicyTrait<E>,
        T: TaskTrait<E, A>,
    {
        self.task_count = compilation.tasks.len();
        self.executor_count = compilation.executors.len();
        if !self.enabled {
            return Ok(());
        }

        self.reset_registered_state();

        let Some(x) = compilation.x.as_ref() else {
            return Err(GanttError::Calculation {
                message: "x variables must be initialized before registering switch".to_string(),
            });
        };
        if self.task_count > 1 && compilation.tasks.iter().any(|task| task.time().is_none()) {
            return Err(GanttError::Calculation {
                message: "enabled static switch requires every task to have a time range".to_string(),
            });
        }

        for ei in 0..self.executor_count {
            for from in 0..self.task_count {
                for to in 0..self.task_count {
                    let flat_index = self.switch_flat_index(ei, from, to);
                    if from == to || !Self::is_static_adjacent(&compilation.tasks, from, to) {
                        continue;
                    }

                    let from_idx = x.model_index(&from, &ei)
                        .ok_or_else(|| GanttError::Calculation {
                            message: format!("x[{}, {}] model index not found for switch", from, ei),
                        })?;
                    let to_idx = x.model_index(&to, &ei)
                        .ok_or_else(|| GanttError::Calculation {
                            message: format!("x[{}, {}] model index not found for switch", to, ei),
                        })?;
                    let switch_symbol = Arc::new(AndFunction::new(
                        next_gantt_symbol_id(),
                        &format!("switch_{}_{}_{}", ei, from, to),
                        vec![
                            Linear::new(vec![LinearMonomial::new(1.0, from_idx)], 0.0),
                            Linear::new(vec![LinearMonomial::new(1.0, to_idx)], 0.0),
                        ],
                    ));
                    model.add_symbol(switch_symbol.clone())
                        .map_err(|e| GanttError::Calculation {
                            message: format!("Failed to register switch_{}_{}_{}: {:?}", ei, from, to, e),
                        })?;
                    let switch_idx = model.find_token(switch_symbol.result_variable().id())
                        .map(|token| token.solver_index)
                        .ok_or_else(|| GanttError::Calculation {
                            message: format!("switch_{}_{}_{} result variable not found", ei, from, to),
                        })?;
                    self.switch_model_indices[flat_index] = Some(switch_idx);
                    self.switch_symbols.push(switch_symbol);
                }
            }
        }

        for from in 0..self.task_count {
            for to in 0..self.task_count {
                let flat_index = self.switch_time_flat_index(from, to);
                let distance = Self::static_switch_time_value(&compilation.tasks, from, to);
                let mut switch_terms = Vec::new();
                let mut switch_indicator_terms = Vec::new();
                for ei in 0..self.executor_count {
                    if let Some(switch_idx) = self.switch_model_index(ei, from, to) {
                        switch_indicator_terms.push((switch_idx, 1.0));
                        switch_terms.push(LinearMonomial::new(distance, switch_idx));
                    }
                }
                if switch_terms.is_empty() {
                    continue;
                }
                model.add_le_constraint(
                    &switch_indicator_terms,
                    1.0,
                    &format!("switch_sum_{}_{}", from, to),
                ).map_err(|e| GanttError::Calculation {
                    message: format!("Failed to register switch_sum_{}_{}: {:?}", from, to, e),
                })?;

                let symbol = Arc::new(LinearExpressionSymbol::new(
                    next_gantt_symbol_id(),
                    &format!("switch_time_{}_{}", from, to),
                    switch_terms,
                    0.0,
                ));
                model.add_symbol(symbol.clone())
                    .map_err(|e| GanttError::Calculation {
                        message: format!("Failed to register switch_time_{}_{}: {:?}", from, to, e),
                    })?;
                self.switch_time_symbols[flat_index] = Some(symbol);
            }
        }

        Ok(())
    }

    /// 携带任务时间组件注册动态切换符号 / Register dynamic switch symbols with task-time component
    ///
    /// 对齐 Kotlin `TaskSchedulingSwitch` 中 `taskTime != null` 的可选分支：
    /// 通过 `front_of`、`between_in`、`switch` 和 `switch_time` 表达动态任务先后关系。
    ///
    /// Aligns the optional Kotlin `TaskSchedulingSwitch` branch where `taskTime != null`:
    /// dynamic task order is expressed through `front_of`, `between_in`, `switch`, and `switch_time`.
    pub fn register_with_task_time<T, E, A>(
        &mut self,
        compilation: &Compilation<T, E, A>,
        task_time: &TaskTime,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<()>
    where
        E: ExecutorTrait,
        A: AssignmentPolicyTrait<E>,
        T: TaskTrait<E, A>,
    {
        self.task_count = compilation.tasks.len();
        self.executor_count = compilation.executors.len();
        if !self.enabled {
            return Ok(());
        }

        self.reset_registered_state();

        let Some(x) = compilation.x.as_ref() else {
            return Err(GanttError::Calculation {
                message: "x variables must be initialized before registering switch".to_string(),
            });
        };
        if task_time.estimate_start_time_symbols.len() != self.task_count
            || task_time.estimate_end_time_symbols.len() != self.task_count
        {
            return Err(GanttError::Calculation {
                message: "task_time estimate symbols must be initialized before registering dynamic switch".to_string(),
            });
        }

        self.front_of_symbols = vec![None; self.task_count * self.task_count];
        self.between_in_symbols = vec![None; self.task_count * self.task_count * self.task_count];
        self.switch_model_indices = vec![None; self.executor_count * self.task_count * self.task_count];
        self.switch_time_mask_symbols =
            vec![None; self.executor_count * self.task_count * self.task_count];
        self.switch_time_symbols = vec![None; self.task_count * self.task_count];

        for from in 0..self.task_count {
            for to in 0..self.task_count {
                if from == to {
                    continue;
                }
                let left = Self::linear_difference(
                    &task_time.estimate_start_time_symbols[from],
                    &task_time.estimate_start_time_symbols[to],
                );
                let front_of = Arc::new(InequalityFunction::new(
                    next_gantt_symbol_id(),
                    &format!("front_of_{}_{}", from, to),
                    left,
                    0.0,
                    InequalityKind::LessEqual,
                    Self::dynamic_switch_big_m(task_time),
                ));
                model.add_symbol(front_of.clone())
                    .map_err(|e| GanttError::Calculation {
                        message: format!("Failed to register front_of_{}_{}: {:?}", from, to, e),
                    })?;
                let flat_index = self.switch_time_flat_index(from, to);
                self.front_of_symbols[flat_index] = Some(front_of);
            }
        }

        for middle in 0..self.task_count {
            for from in 0..self.task_count {
                for to in 0..self.task_count {
                    if from == to || from == middle || to == middle {
                        continue;
                    }
                    let Some(front_middle) = self.front_of_model_polynomial(model, from, middle)? else {
                        continue;
                    };
                    let Some(middle_to) = self.front_of_model_polynomial(model, middle, to)? else {
                        continue;
                    };
                    let between = Arc::new(AndFunction::new(
                        next_gantt_symbol_id(),
                        &format!("between_in_{}_{}_{}", middle, from, to),
                        vec![front_middle, middle_to],
                    ));
                    model.add_symbol(between.clone())
                        .map_err(|e| GanttError::Calculation {
                            message: format!(
                                "Failed to register between_in_{}_{}_{}: {:?}",
                                middle, from, to, e,
                            ),
                        })?;
                    let flat_index = self.between_in_flat_index(middle, from, to);
                    self.between_in_symbols[flat_index] = Some(between);
                }
            }
        }

        for ei in 0..self.executor_count {
            for from in 0..self.task_count {
                for to in 0..self.task_count {
                    if from == to {
                        continue;
                    }
                    let Some(front_of) = self.front_of_model_polynomial(model, from, to)? else {
                        continue;
                    };
                    let from_idx = x.model_index(&from, &ei)
                        .ok_or_else(|| GanttError::Calculation {
                            message: format!("x[{}, {}] model index not found for dynamic switch", from, ei),
                        })?;
                    let to_idx = x.model_index(&to, &ei)
                        .ok_or_else(|| GanttError::Calculation {
                            message: format!("x[{}, {}] model index not found for dynamic switch", to, ei),
                        })?;

                    let mut switch_conditions = vec![
                        Linear::new(vec![LinearMonomial::new(1.0, from_idx)], 0.0),
                        Linear::new(vec![LinearMonomial::new(1.0, to_idx)], 0.0),
                        front_of,
                    ];
                    for middle in 0..self.task_count {
                        if middle == from || middle == to {
                            continue;
                        }
                        if let Some(not_between) =
                            self.not_between_model_polynomial(model, middle, from, to)?
                        {
                            switch_conditions.push(not_between);
                        }
                    }

                    let switch_symbol = Arc::new(AndFunction::new(
                        next_gantt_symbol_id(),
                        &format!("switch_{}_{}_{}", ei, from, to),
                        switch_conditions,
                    ));
                    model.add_symbol(switch_symbol.clone())
                        .map_err(|e| GanttError::Calculation {
                            message: format!("Failed to register switch_{}_{}_{}: {:?}", ei, from, to, e),
                        })?;
                    let switch_idx = model.find_token(switch_symbol.result_variable().id())
                        .map(|token| token.solver_index)
                        .ok_or_else(|| GanttError::Calculation {
                            message: format!("switch_{}_{}_{} result variable not found", ei, from, to),
                        })?;
                    let flat_index = self.switch_flat_index(ei, from, to);
                    self.switch_model_indices[flat_index] = Some(switch_idx);
                    self.switch_symbols.push(switch_symbol.clone());

                    let time_gap = Self::linear_difference(
                        &task_time.estimate_start_time_symbols[to],
                        &task_time.estimate_end_time_symbols[from],
                    );
                    let masking_symbol = Arc::new(MaskingFunction::new(
                        next_gantt_symbol_id(),
                        &format!("switch_time_mask_{}_{}_{}", ei, from, to),
                        time_gap,
                        switch_symbol.result_variable().clone(),
                    ));
                    model.add_symbol(masking_symbol.clone())
                        .map_err(|e| GanttError::Calculation {
                            message: format!(
                                "Failed to register switch_time_mask_{}_{}_{}: {:?}",
                                ei, from, to, e,
                            ),
                        })?;
                    let mask_flat_index = self.switch_flat_index(ei, from, to);
                    self.switch_time_mask_symbols[mask_flat_index] = Some(masking_symbol);
                }
            }
        }

        for from in 0..self.task_count {
            for to in 0..self.task_count {
                let mut switch_indicator_terms = Vec::new();
                let mut switch_time_terms = Vec::new();
                for ei in 0..self.executor_count {
                    if let Some(switch_idx) = self.switch_model_index(ei, from, to) {
                        switch_indicator_terms.push((switch_idx, 1.0));
                    }
                    if let Some(mask_symbol) = self.switch_time_mask_symbol(ei, from, to) {
                        let mask_idx = model.find_token(mask_symbol.result_variable().id())
                            .map(|token| token.solver_index)
                            .ok_or_else(|| GanttError::Calculation {
                                message: format!(
                                    "switch_time_mask_{}_{}_{} result variable not found",
                                    ei, from, to,
                                ),
                            })?;
                        switch_time_terms.push(LinearMonomial::new(1.0, mask_idx));
                    }
                }
                if switch_time_terms.is_empty() {
                    continue;
                }
                model.add_le_constraint(
                    &switch_indicator_terms,
                    1.0,
                    &format!("switch_sum_{}_{}", from, to),
                ).map_err(|e| GanttError::Calculation {
                    message: format!("Failed to register switch_sum_{}_{}: {:?}", from, to, e),
                })?;

                let symbol = Arc::new(LinearExpressionSymbol::new(
                    next_gantt_symbol_id(),
                    &format!("switch_time_{}_{}", from, to),
                    switch_time_terms,
                    0.0,
                ));
                model.add_symbol(symbol.clone())
                    .map_err(|e| GanttError::Calculation {
                        message: format!("Failed to register switch_time_{}_{}: {:?}", from, to, e),
                    })?;
                let flat_index = self.switch_time_flat_index(from, to);
                self.switch_time_symbols[flat_index] = Some(symbol);
            }
        }

        Ok(())
    }

    /// 获取 switch 模型索引 / Get switch model index
    pub fn switch_model_index(
        &self,
        executor_index: usize,
        from_task_index: usize,
        to_task_index: usize,
    ) -> Option<usize> {
        self.switch_model_indices
            .get(self.switch_flat_index(executor_index, from_task_index, to_task_index))
            .copied()
            .flatten()
    }

    /// 获取 switch_time 符号 / Get switch-time symbol
    pub fn switch_time_symbol(
        &self,
        from_task_index: usize,
        to_task_index: usize,
    ) -> Option<&Arc<LinearExpressionSymbol<f64>>> {
        self.switch_time_symbols
            .get(self.switch_time_flat_index(from_task_index, to_task_index))
            .and_then(Option::as_ref)
    }

    /// 获取 front_of 符号 / Get front-of symbol
    pub fn front_of_symbol(
        &self,
        from_task_index: usize,
        to_task_index: usize,
    ) -> Option<&Arc<InequalityFunction<f64>>> {
        self.front_of_symbols
            .get(self.switch_time_flat_index(from_task_index, to_task_index))
            .and_then(Option::as_ref)
    }

    /// 获取 between_in 符号 / Get between-in symbol
    pub fn between_in_symbol(
        &self,
        middle_task_index: usize,
        from_task_index: usize,
        to_task_index: usize,
    ) -> Option<&Arc<AndFunction<f64>>> {
        self.between_in_symbols
            .get(self.between_in_flat_index(middle_task_index, from_task_index, to_task_index))
            .and_then(Option::as_ref)
    }

    /// 获取 executor 级 switch_time 遮罩符号 / Get executor-level switch-time masking symbol
    pub fn switch_time_mask_symbol(
        &self,
        executor_index: usize,
        from_task_index: usize,
        to_task_index: usize,
    ) -> Option<&Arc<MaskingFunction<f64>>> {
        self.switch_time_mask_symbols
            .get(self.switch_flat_index(executor_index, from_task_index, to_task_index))
            .and_then(Option::as_ref)
    }

    /// 获取 switch_time 多项式 / Get switch-time polynomial
    pub fn switch_time_polynomial(
        &self,
        from_task_index: usize,
        to_task_index: usize,
    ) -> Option<Linear<f64>> {
        self.switch_time_symbol(from_task_index, to_task_index)
            .map(|symbol| symbol.to_linear_polynomial())
    }

    /// 获取 switch_time 值 / Get switch-time value
    pub fn switch_time_value(
        &self,
        from_task_index: usize,
        to_task_index: usize,
        solution: &[f64],
    ) -> Option<f64> {
        let polynomial = self.switch_time_polynomial(from_task_index, to_task_index)?;
        let mut value = *polynomial.constant_term();
        for monomial in polynomial.monomials() {
            let term_value = solution.get(monomial.var_index()).copied()?;
            value += *monomial.coefficient() * term_value;
        }
        Some(value)
    }

    fn reset_registered_state(&mut self) {
        self.front_of_symbols.clear();
        self.between_in_symbols.clear();
        self.switch_symbols.clear();
        self.switch_time_mask_symbols =
            vec![None; self.executor_count * self.task_count * self.task_count];
        self.switch_model_indices =
            vec![None; self.executor_count * self.task_count * self.task_count];
        self.switch_time_symbols = vec![None; self.task_count * self.task_count];
    }

    fn switch_flat_index(
        &self,
        executor_index: usize,
        from_task_index: usize,
        to_task_index: usize,
    ) -> usize {
        executor_index * self.task_count * self.task_count
            + from_task_index * self.task_count
            + to_task_index
    }

    fn switch_time_flat_index(&self, from_task_index: usize, to_task_index: usize) -> usize {
        from_task_index * self.task_count + to_task_index
    }

    fn between_in_flat_index(
        &self,
        middle_task_index: usize,
        from_task_index: usize,
        to_task_index: usize,
    ) -> usize {
        middle_task_index * self.task_count * self.task_count
            + from_task_index * self.task_count
            + to_task_index
    }

    fn front_of_model_polynomial(
        &self,
        model: &MetaModel<f64>,
        from_task_index: usize,
        to_task_index: usize,
    ) -> GanttResult<Option<Linear<f64>>> {
        let Some(symbol) = self.front_of_symbol(from_task_index, to_task_index) else {
            return Ok(None);
        };
        let solver_index = model.find_token(symbol.result_variable().id())
            .map(|token| token.solver_index)
            .ok_or_else(|| GanttError::Calculation {
                message: format!(
                    "front_of_{}_{} result variable not found",
                    from_task_index, to_task_index,
                ),
            })?;
        Ok(Some(Linear::new(
            vec![LinearMonomial::new(1.0, solver_index)],
            0.0,
        )))
    }

    fn not_between_model_polynomial(
        &self,
        model: &MetaModel<f64>,
        middle_task_index: usize,
        from_task_index: usize,
        to_task_index: usize,
    ) -> GanttResult<Option<Linear<f64>>> {
        let Some(symbol) = self.between_in_symbol(middle_task_index, from_task_index, to_task_index) else {
            return Ok(None);
        };
        let solver_index = model.find_token(symbol.result_variable().id())
            .map(|token| token.solver_index)
            .ok_or_else(|| GanttError::Calculation {
                message: format!(
                    "between_in_{}_{}_{} result variable not found",
                    middle_task_index, from_task_index, to_task_index,
                ),
            })?;
        Ok(Some(Linear::new(
            vec![LinearMonomial::new(-1.0, solver_index)],
            1.0,
        )))
    }

    fn linear_difference(
        left: &Arc<LinearExpressionSymbol<f64>>,
        right: &Arc<LinearExpressionSymbol<f64>>,
    ) -> Linear<f64> {
        Self::linear_scaled_add(
            1.0,
            &[left.to_linear_polynomial()],
            -1.0,
            &right.to_linear_polynomial(),
        )
    }

    fn linear_scaled_add(
        first_scale: f64,
        first: &[Linear<f64>],
        second_scale: f64,
        second: &Linear<f64>,
    ) -> Linear<f64> {
        let mut monomials = Vec::new();
        let mut constant = 0.0;
        for linear in first {
            constant += first_scale * *linear.constant_term();
            for monomial in linear.monomials() {
                monomials.push(LinearMonomial::new(
                    first_scale * *monomial.coefficient(),
                    monomial.var_index(),
                ));
            }
        }
        constant += second_scale * *second.constant_term();
        for monomial in second.monomials() {
            monomials.push(LinearMonomial::new(
                second_scale * *monomial.coefficient(),
                monomial.var_index(),
            ));
        }
        Linear::new(monomials, constant)
    }

    fn dynamic_switch_big_m(task_time: &TaskTime) -> f64 {
        let max_scheduled_start = task_time.scheduled_starts
            .iter()
            .flatten()
            .copied()
            .fold(0.0_f64, |acc, value| acc.max(value.abs()));
        let max_duration = task_time.durations
            .iter()
            .copied()
            .fold(0.0_f64, |acc, value| acc.max(value.abs()));
        (max_scheduled_start + max_duration).max(1_000_000.0)
    }

    fn is_static_adjacent<T, E, A>(tasks: &[T], from: usize, to: usize) -> bool
    where
        E: ExecutorTrait,
        A: AssignmentPolicyTrait<E>,
        T: TaskTrait<E, A>,
    {
        let Some(from_time) = tasks[from].time() else {
            return false;
        };
        let Some(to_time) = tasks[to].time() else {
            return false;
        };
        if from_time.start >= to_time.start {
            return false;
        }
        !tasks.iter().enumerate().any(|(middle, task)| {
            if middle == from || middle == to {
                return false;
            }
            task.time()
                .map(|time| from_time.start < time.start && time.start < to_time.start)
                .unwrap_or(false)
        })
    }

    fn static_switch_time_value<T, E, A>(tasks: &[T], from: usize, to: usize) -> f64
    where
        E: ExecutorTrait,
        A: AssignmentPolicyTrait<E>,
        T: TaskTrait<E, A>,
    {
        let Some(from_time) = tasks[from].time() else {
            return 0.0;
        };
        let Some(to_time) = tasks[to].time() else {
            return 0.0;
        };
        let distance = to_time.start - from_time.end;
        distance.whole_seconds().max(0) as f64
    }

}

// ============================================================================
// Solution 类型 / Solution Types
// ============================================================================

/// 任务解摘要 / Task solution summary
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSolutionSummary {
    /// 已分配任务数 / Assigned task count
    pub assigned_task_count: u64,
    /// 已取消任务数 / Canceled task count
    pub canceled_task_count: u64,
    /// 总任务数 / Total task count
    pub total_task_count: u64,
}

/// 任务解 / Task solution
#[derive(Debug, Clone)]
pub struct TaskSolution<T> {
    /// 已分配任务 / Assigned tasks
    pub assigned_tasks: Vec<T>,
    /// 已取消任务 / Canceled tasks
    pub canceled_tasks: Vec<T>,
}

impl<T> TaskSolution<T> {
    /// 获取解摘要 / Get solution summary
    pub fn summary(&self) -> TaskSolutionSummary {
        let assigned = self.assigned_tasks.len() as u64;
        let canceled = self.canceled_tasks.len() as u64;
        TaskSolutionSummary {
            assigned_task_count: assigned,
            canceled_task_count: canceled,
            total_task_count: assigned + canceled,
        }
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task_compilation::adapter::{
        IndexedVariableArray1, IndexedVariableArray2,
        build_linear_expression_symbol,
    };
    use crate::domain::task_compilation::{
        TaskCompilationAggregation,
        TaskCompilationAggregationWithTime,
    };
    use ospf_rust_core::variable::Binary;

    #[test]
    fn test_compilation_variable_registration() {
        let mut model = MetaModel::<f64>::new("test_compilation");
        let task_indices: Vec<usize> = vec![0, 1];
        let executor_indices: Vec<usize> = vec![0, 1];

        let x: IndexedVariableArray2<usize, usize, Binary> =
            IndexedVariableArray2::new("x", &task_indices, &executor_indices, &mut model).unwrap();
        assert_eq!(x.len(), 4);

        let y: IndexedVariableArray1<usize, Binary> =
            IndexedVariableArray1::new("y", &task_indices, &mut model).unwrap();
        assert_eq!(y.len(), 2);

        let z: IndexedVariableArray1<usize, Binary> =
            IndexedVariableArray1::new("z", &executor_indices, &mut model).unwrap();
        assert_eq!(z.len(), 2);

        let mut terms = Vec::new();
        for ei in &executor_indices {
            if let Some(idx) = x.model_index(&0, ei) {
                terms.push((idx, 1.0));
            }
        }
        if let Some(idx) = y.model_index(&0) {
            terms.push((idx, 1.0));
        }
        let symbol = build_linear_expression_symbol("task_compilation_0", &terms, 0.0);
        model.add_symbol(symbol).unwrap();
    }

    #[test]
    fn test_task_time_register_with_slack() {
        let mut model = MetaModel::<f64>::new("test_task_time");
        let mut task_time = TaskTime::new(true, false, true, false);

        let n_tasks = 2;
        let scheduled_starts = vec![Some(10.0), None];  // task 0 有排程时间，task 1 没有
        let durations = vec![5.0, 3.0];

        task_time.register(n_tasks, scheduled_starts, durations, &mut model).unwrap();

        assert!(task_time.est.is_some());
        let est = task_time.est.as_ref().unwrap();
        assert_eq!(est.len(), 2);

        // 验证中间符号数量
        assert_eq!(task_time.estimate_start_time_symbols.len(), 2);
        assert_eq!(task_time.estimate_end_time_symbols.len(), 2);

        // task 0 有 delay 和 advance slack
        assert!(task_time.delay_time_slacks[0].is_some());
        assert!(task_time.advance_time_slacks[0].is_some());
        assert!(task_time.delay_time_model_indices[0].is_some());
        assert!(task_time.advance_time_model_indices[0].is_some());

        // task 1 没有 slack（无排程时间）
        assert!(task_time.delay_time_slacks[1].is_none());
        assert!(task_time.advance_time_slacks[1].is_none());
    }

    #[test]
    fn test_makespan_register() {
        let mut model = MetaModel::<f64>::new("test_makespan");

        // 先注册 TaskTime
        let mut task_time = TaskTime::new(false, false, false, false);
        let scheduled_starts = vec![None, None];
        let durations = vec![5.0, 3.0];
        task_time.register(2, scheduled_starts, durations, &mut model).unwrap();

        // 再注册 Makespan
        let mut makespan = Makespan::new(false);
        makespan.register(&task_time, &mut model).unwrap();

        assert!(makespan.makespan_model_index.is_some());
    }

    #[test]
    fn test_switch_register_disabled_compatibility_entry() {
        let mut model = MetaModel::<f64>::new("test_switch");
        let mut switch = Switch::new(false);
        switch.register(&mut model).unwrap();
    }

    #[test]
    fn test_task_solution_summary() {
        let solution = TaskSolution::<String> {
            assigned_tasks: vec!["t0".to_string(), "t1".to_string()],
            canceled_tasks: vec!["t2".to_string()],
        };
        let summary = solution.summary();
        assert_eq!(summary.assigned_task_count, 2);
        assert_eq!(summary.canceled_task_count, 1);
        assert_eq!(summary.total_task_count, 3);
    }

    // ========================================================================
    // 端到端集成测试 / End-to-end integration tests
    // ========================================================================

    /// 测试用的简单任务 / Simple task for testing
    #[derive(Debug, Clone)]
    struct TestTask {
        id: String,
        name: String,
    }

    impl TestTask {
        fn new(id: &str, name: &str) -> Self {
            Self { id: id.to_string(), name: name.to_string() }
        }
    }

    use crate::domain::task::{ExecutorTrait, AssignmentPolicyTrait, TaskTrait, BasicExecutor, BasicAssignmentPolicy};
    use crate::infrastructure::TimeRange;

    impl<E: ExecutorTrait, A: AssignmentPolicyTrait<E>> TaskTrait<E, A> for TestTask {
        fn id(&self) -> &str { &self.id }
        fn name(&self) -> &str { &self.name }
    }

    #[derive(Debug, Clone)]
    struct TimedTestTask {
        id: String,
        name: String,
        time: TimeRange,
    }

    impl TimedTestTask {
        fn new(id: &str, name: &str, time: TimeRange) -> Self {
            Self {
                id: id.to_string(),
                name: name.to_string(),
                time,
            }
        }
    }

    impl<E: ExecutorTrait, A: AssignmentPolicyTrait<E>> TaskTrait<E, A> for TimedTestTask {
        fn id(&self) -> &str { &self.id }
        fn name(&self) -> &str { &self.name }
        fn time(&self) -> Option<&TimeRange> { Some(&self.time) }
    }

    #[test]
    fn test_full_task_compilation_model_registration() {
        // 创建 2 个任务和 2 个执行者 / Create 2 tasks and 2 executors
        let tasks = vec![
            TestTask::new("t0", "Task 0"),
            TestTask::new("t1", "Task 1"),
        ];
        let executors = vec![
            BasicExecutor::new("e0", "Executor 0"),
            BasicExecutor::new("e1", "Executor 1"),
        ];

        let mut model = MetaModel::<f64>::new("test_full_compilation");

        // 创建并注册 TaskCompilationAggregationWithTime
        let mut aggregation: TaskCompilationAggregationWithTime<TestTask, BasicExecutor, BasicAssignmentPolicy<BasicExecutor>> =
            TaskCompilationAggregationWithTime::new(
                tasks,
                executors,
                true,   // task_cancel_enabled
                false,  // with_executor_leisure
                false,  // switch_enabled
                true,   // delay_enabled
                false,  // over_max_delay_enabled
                true,   // advance_enabled
                false,  // over_max_advance_enabled
                false,  // makespan_extra
            );

        let scheduled_starts = vec![Some(10.0), Some(20.0)];
        let durations = vec![5.0, 3.0];

        aggregation.register(scheduled_starts, durations, &mut model).unwrap();

        // 验证 Compilation 组件
        let compilation = &aggregation.compilation;
        assert!(compilation.x.is_some());
        assert!(compilation.y.is_some()); // task_cancel_enabled = true
        assert!(compilation.z.is_none()); // with_executor_leisure = false
        assert_eq!(compilation.x.as_ref().unwrap().len(), 4); // 2 tasks * 2 executors
        assert_eq!(compilation.y.as_ref().unwrap().len(), 2); // 2 tasks

        // 验证中间符号
        assert_eq!(compilation.task_assignment_symbols.len(), 2);
        assert_eq!(compilation.task_compilation_symbols.len(), 2);
        assert_eq!(compilation.executor_compilation_symbols.len(), 2);

        // 验证 TaskTime 组件
        let task_time = &aggregation.task_time;
        assert!(task_time.est.is_some());
        assert_eq!(task_time.estimate_start_time_symbols.len(), 2);
        assert_eq!(task_time.estimate_end_time_symbols.len(), 2);

        // task 0 和 task 1 都有排程时间，所以都有 delay/advance slack
        assert!(task_time.delay_time_slacks[0].is_some());
        assert!(task_time.delay_time_slacks[1].is_some());
        assert!(task_time.advance_time_slacks[0].is_some());
        assert!(task_time.advance_time_slacks[1].is_some());

        // 验证 Makespan 组件
        assert!(aggregation.makespan.makespan_model_index.is_some());

        // 注册约束和目标
        use crate::domain::task_compilation::service::limits::*;
        use ospf_rust_framework::model::pipeline::Pipeline;

        // 1. TaskCompilationConstraint — 使用 task_compilation_model_indices
        let tc_groups = compilation.task_compilation_model_indices();
        assert_eq!(tc_groups.len(), 2);
        for group in &tc_groups {
            assert!(!group.is_empty());
        }
        // 对每个 task，task_compilation[task] 的变量索引和应为 1
        // For each task, the sum of task_compilation[task] variable indices should equal 1
        for (ti, group) in tc_groups.iter().enumerate() {
            let indices: Vec<(usize, f64)> = group.iter().map(|&idx| (idx, 1.0)).collect();
            model.add_eq_constraint(&indices, 1.0, &format!("task_compilation_{}", ti)).unwrap();
        }

        // 2. ExecutorCompilationConstraint — 使用 executor_compilation_model_indices
        let ec_groups = compilation.executor_compilation_model_indices();
        assert_eq!(ec_groups.len(), 2);
        for (ei, group) in ec_groups.iter().enumerate() {
            let indices: Vec<(usize, f64)> = group.iter().map(|&idx| (idx, 1.0)).collect();
            model.add_eq_constraint(&indices, 1.0, &format!("executor_compilation_{}", ei)).unwrap();
        }

        // 3. MakespanMinimization
        if let Some(makespan_idx) = aggregation.makespan.makespan_model_index {
            let makespan_obj = MakespanMinimization::new(makespan_idx, 1.0);
            makespan_obj.register(&mut model);
            makespan_obj.invoke(&model).unwrap();
        }

        // 4. TaskDelayTimeMinimization
        let delay_cost_terms: Vec<(usize, f64)> = task_time.delay_time_model_indices.iter()
            .filter_map(|idx: &Option<usize>| idx.map(|i| (i, 1.0)))
            .collect();
        let delay_obj = TaskDelayTimeMinimization::new(delay_cost_terms);
        delay_obj.register(&mut model);
        delay_obj.invoke(&model).unwrap();

        // 5. TaskAdvanceTimeMinimization
        let advance_cost_terms: Vec<(usize, f64)> = task_time.advance_time_model_indices.iter()
            .filter_map(|idx: &Option<usize>| idx.map(|i| (i, 1.0)))
            .collect();
        let advance_obj = TaskAdvanceTimeMinimization::new(advance_cost_terms);
        advance_obj.register(&mut model);
        advance_obj.invoke(&model).unwrap();

        // 验证模型状态
        // 模型应该成功注册变量、中间符号、约束和目标
        // Model should successfully register variables, intermediate symbols, constraints, and objectives
        // Compilation 变量
        assert!(compilation.x.as_ref().unwrap().model_index(&0, &0).is_some());
        assert!(compilation.x.as_ref().unwrap().model_index(&1, &1).is_some());
    }

    #[test]
    fn test_switch_registers_static_adjacent_symbols() {
        use time::macros::datetime;

        let tasks = vec![
            TimedTestTask::new(
                "t0",
                "Task 0",
                TimeRange::new(
                    datetime!(2020-08-30 08:00 UTC),
                    datetime!(2020-08-30 09:00 UTC),
                ),
            ),
            TimedTestTask::new(
                "t1",
                "Task 1",
                TimeRange::new(
                    datetime!(2020-08-30 10:00 UTC),
                    datetime!(2020-08-30 11:00 UTC),
                ),
            ),
            TimedTestTask::new(
                "t2",
                "Task 2",
                TimeRange::new(
                    datetime!(2020-08-30 12:00 UTC),
                    datetime!(2020-08-30 13:00 UTC),
                ),
            ),
        ];
        let executors = vec![BasicExecutor::new("e0", "Executor 0")];
        let mut model = MetaModel::<f64>::new("test_switch_static");

        let mut aggregation: TaskCompilationAggregation<
            TimedTestTask,
            BasicExecutor,
            BasicAssignmentPolicy<BasicExecutor>,
        > = TaskCompilationAggregation::new(
            tasks,
            executors,
            false,
            false,
            true,
        );
        aggregation.register(&mut model).unwrap();

        assert!(aggregation.switch.switch_model_index(0, 0, 1).is_some());
        assert!(aggregation.switch.switch_model_index(0, 1, 2).is_some());
        assert!(aggregation.switch.switch_model_index(0, 0, 2).is_none());
        assert!(aggregation.switch.switch_time_symbol(0, 1).is_some());

        let x = aggregation.compilation.x.as_ref().unwrap();
        let mut solution = vec![0.0; model.tokens().len()];
        solution[x.model_index(&0, &0).unwrap()] = 1.0;
        solution[x.model_index(&1, &0).unwrap()] = 1.0;
        solution[x.model_index(&2, &0).unwrap()] = 0.0;
        model.set_solution_by_solver_order(&solution);

        let switch_idx = aggregation.switch.switch_model_index(0, 0, 1).unwrap();
        solution[switch_idx] = 1.0;
        assert_eq!(
            aggregation.switch.switch_time_value(0, 1, &solution),
            Some(3600.0)
        );
    }

    #[test]
    fn test_switch_registers_dynamic_task_time_symbols() {
        let tasks = vec![
            TestTask::new("t0", "Task 0"),
            TestTask::new("t1", "Task 1"),
            TestTask::new("t2", "Task 2"),
        ];
        let executors = vec![BasicExecutor::new("e0", "Executor 0")];
        let mut model = MetaModel::<f64>::new("test_switch_dynamic_task_time");

        let mut aggregation: TaskCompilationAggregationWithTime<
            TestTask,
            BasicExecutor,
            BasicAssignmentPolicy<BasicExecutor>,
        > = TaskCompilationAggregationWithTime::new(
            tasks,
            executors,
            false,
            false,
            true,
            false,
            false,
            false,
            false,
            false,
        );
        aggregation.register(
            vec![Some(0.0), Some(10.0), Some(20.0)],
            vec![5.0, 5.0, 5.0],
            &mut model,
        ).unwrap();

        assert!(aggregation.switch.front_of_symbol(0, 1).is_some());
        assert!(aggregation.switch.between_in_symbol(1, 0, 2).is_some());
        assert!(aggregation.switch.switch_model_index(0, 0, 1).is_some());
        assert!(aggregation.switch.switch_model_index(0, 0, 2).is_some());
        assert!(aggregation.switch.switch_time_mask_symbol(0, 0, 1).is_some());
        assert!(aggregation.switch.switch_time_symbol(0, 1).is_some());
        let mechanism = model.try_to_mechanism_model().unwrap();
        assert!(mechanism.as_basic().num_constraints() > 0);

        let x = aggregation.compilation.x.as_ref().unwrap();
        let est = aggregation.task_time.est.as_ref().unwrap();
        let switch_idx = aggregation.switch.switch_model_index(0, 0, 1).unwrap();
        let mask_idx = aggregation.switch
            .switch_time_mask_symbol(0, 0, 1)
            .and_then(|symbol| model.find_token(symbol.result_variable().id()))
            .map(|token| token.solver_index)
            .unwrap();
        let mut solution = vec![0.0; model.tokens().len()];
        solution[x.model_index(&0, &0).unwrap()] = 1.0;
        solution[x.model_index(&1, &0).unwrap()] = 1.0;
        solution[est.model_index(&0).unwrap()] = 0.0;
        solution[est.model_index(&1).unwrap()] = 10.0;
        solution[est.model_index(&2).unwrap()] = 20.0;
        solution[switch_idx] = 1.0;
        solution[mask_idx] = 5.0;

        assert_eq!(
            aggregation.switch.switch_time_value(0, 1, &solution),
            Some(5.0)
        );
    }

    #[test]
    fn test_task_compilation_aggregation_without_time() {
        // 不带时间的简单聚合 / Simple aggregation without time
        let tasks = vec![
            TestTask::new("t0", "Task 0"),
            TestTask::new("t1", "Task 1"),
        ];
        let executors = vec![
            BasicExecutor::new("e0", "Executor 0"),
        ];

        let mut model = MetaModel::<f64>::new("test_basic_aggregation");

        let mut aggregation: TaskCompilationAggregation<TestTask, BasicExecutor, BasicAssignmentPolicy<BasicExecutor>> =
            TaskCompilationAggregation::new(
                tasks,
                executors,
                false,  // task_cancel_enabled
                true,   // with_executor_leisure
                false,  // switch_enabled
            );

        aggregation.register(&mut model).unwrap();

        // 验证 Compilation
        let compilation = &aggregation.compilation;
        assert!(compilation.x.is_some());
        assert!(compilation.y.is_none()); // task_cancel_enabled = false
        assert!(compilation.z.is_some()); // with_executor_leisure = true
        assert_eq!(compilation.x.as_ref().unwrap().len(), 2); // 2 tasks * 1 executor
        assert_eq!(compilation.z.as_ref().unwrap().len(), 1); // 1 executor

        // 验证中间符号
        assert_eq!(compilation.task_assignment_symbols.len(), 2);
        assert_eq!(compilation.task_compilation_symbols.len(), 2);
        assert_eq!(compilation.executor_compilation_symbols.len(), 1);
    }

    #[test]
    fn test_over_max_delay_constraint_registration() {
        // 测试超最大延迟约束注册 / Test over-max delay constraint registration
        use crate::domain::task_compilation::service::limits::TaskOverMaxDelayTimeConstraint;
        use ospf_rust_framework::model::pipeline::Pipeline;

        let mut model = MetaModel::<f64>::new("test_over_max_delay");

        // 先注册 TaskTime 组件
        let mut task_time = TaskTime::new(true, true, false, false);
        task_time.register(3, vec![Some(10.0), Some(20.0), Some(30.0)], vec![5.0, 3.0, 7.0], &mut model).unwrap();

        // 只有部分任务有 max_delay
        let max_delay_values: Vec<Option<f64>> = vec![Some(5.0), None, Some(10.0)];

        let constraint = TaskOverMaxDelayTimeConstraint::new(
            &task_time.delay_time_model_indices,
            &max_delay_values,
        );
        // 应该只有 task 0 和 task 2 的约束
        assert_eq!(constraint.constraints.len(), 2);

        constraint.register(&mut model);
        constraint.invoke(&model).unwrap();
    }

    #[test]
    fn test_deadline_constraints_registration() {
        // 测试截止时间约束注册 / Test deadline constraint registration
        use crate::domain::task_compilation::service::limits::{
            TaskDelayLastEndTimeConstraint,
            TaskAdvanceEarliestEndTimeConstraint,
        };
        use ospf_rust_framework::model::pipeline::Pipeline;

        let mut model = MetaModel::<f64>::new("test_deadline");

        let mut task_time = TaskTime::new(false, false, false, false);
        task_time.register(2, vec![None, None], vec![5.0, 3.0], &mut model).unwrap();

        let est_indices = task_time.est_model_indices();

        // last_end_time: task 0 = 20.0, task 1 = None
        let last_end_times: Vec<Option<f64>> = vec![Some(20.0), None];
        let durations = vec![5.0, 3.0];

        let delay_constraint = TaskDelayLastEndTimeConstraint::new(
            &est_indices,
            &last_end_times,
            &durations,
        );
        // 只有 task 0 有 last_end_time，adjusted = 20.0 - 5.0 = 15.0
        assert_eq!(delay_constraint.constraints.len(), 1);
        assert_eq!(delay_constraint.constraints[0].1, 15.0);

        delay_constraint.register(&mut model);
        delay_constraint.invoke(&model).unwrap();

        // earliest_end_time: task 1 = 10.0, task 0 = None
        let earliest_end_times: Vec<Option<f64>> = vec![None, Some(10.0)];
        let advance_constraint = TaskAdvanceEarliestEndTimeConstraint::new(
            &est_indices,
            &earliest_end_times,
            &durations,
        );
        // 只有 task 1 有 earliest_end_time，adjusted = 10.0 - 3.0 = 7.0
        assert_eq!(advance_constraint.constraints.len(), 1);
        assert_eq!(advance_constraint.constraints[0].1, 7.0);

        advance_constraint.register(&mut model);
        advance_constraint.invoke(&model).unwrap();
    }

    #[test]
    fn test_executor_cost_and_leisure_minimization() {
        // 测试执行器成本和空闲最小化 / Test executor cost and leisure minimization
        use crate::domain::task_compilation::service::limits::{
            ExecutorCostMinimization,
            ExecutorLeisureMinimization,
        };
        use ospf_rust_framework::model::pipeline::Pipeline;

        let mut model = MetaModel::<f64>::new("test_executor_obj");

        // 先注册 Compilation 组件
        let tasks = vec![TestTask::new("t0", "Task 0")];
        let executors = vec![
            BasicExecutor::new("e0", "Executor 0"),
            BasicExecutor::new("e1", "Executor 1"),
        ];

        let mut compilation: Compilation<TestTask, BasicExecutor, BasicAssignmentPolicy<BasicExecutor>> =
            Compilation::new(tasks, executors, false, true);
        compilation.register(&mut model).unwrap();

        // ExecutorCostMinimization: 使用 executor_compilation 中间符号的第一个变量索引
        let ec_groups = compilation.executor_compilation_model_indices();
        let cost_terms: Vec<(usize, f64)> = ec_groups.iter()
            .enumerate()
            .map(|(ei, g)| (g[0], (ei + 1) as f64 * 10.0))
            .collect();

        let executor_cost = ExecutorCostMinimization::new(cost_terms);
        executor_cost.register(&mut model);
        executor_cost.invoke(&model).unwrap();

        // ExecutorLeisureMinimization: 使用 z 变量索引
        if let Some(ref z) = compilation.z {
            let leisure_indices: Vec<usize> = (0..z.len())
                .filter_map(|ei| z.model_index(&ei))
                .collect();
            let leisure_obj = ExecutorLeisureMinimization::new(leisure_indices);
            leisure_obj.register(&mut model);
            leisure_obj.invoke(&model).unwrap();
        }
    }
}
