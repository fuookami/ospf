//! 任务编译服务与限制 / Task compilation services and limits
//!
//! 包含 SolutionAnalyzer 和限制 Pipeline 实现。
//! Contains SolutionAnalyzer and limit Pipeline implementations.

pub mod limits;

use crate::domain::task_compilation::model::Compilation;
use crate::domain::task_compilation::adapter::{extract_binary, extract_value};
use crate::domain::task::{AssignmentPolicyTrait, ExecutorTrait, TaskTrait};

// ============================================================================
// SolutionAnalyzer / 解分析器
// ============================================================================

/// 任务解分析器 / Task solution analyzer
///
/// 从求解器解向量中提取领域结果。
/// Extracts domain results from solver solution vector.
pub struct SolutionAnalyzer;

impl SolutionAnalyzer {
    /// 分析基本解（不含时间）/ Analyze basic solution (without time)
    ///
    /// 从 `compilation` 的 x/y 变量中提取任务分配和取消状态。
    /// Extracts task assignment and cancellation status from compilation's x/y variables.
    pub fn analyze<T, E, A>(
        compilation: &Compilation<T, E, A>,
        solution: &[f64],
    ) -> TaskAssignmentResult
    where
        E: ExecutorTrait,
        A: AssignmentPolicyTrait<E>,
        T: TaskTrait<E, A>,
    {
        let n_tasks = compilation.tasks.len();
        let n_executors = compilation.executors.len();

        let mut assigned = Vec::new();
        let mut canceled = Vec::new();

        if let Some(ref x) = compilation.x {
            for ti in 0..n_tasks {
                let mut found_executor = None;
                for ei in 0..n_executors {
                    if let Some(idx) = x.model_index(&ti, &ei) {
                        if extract_binary(solution, idx).unwrap_or(false) {
                            found_executor = Some(ei);
                            break;
                        }
                    }
                }

                if let Some(ei) = found_executor {
                    assigned.push(TaskAssignment { task_index: ti, executor_index: ei });
                } else if let Some(ref y) = compilation.y {
                    if let Some(idx) = y.model_index(&ti) {
                        if extract_binary(solution, idx).unwrap_or(false) {
                            canceled.push(ti);
                        }
                    }
                } else {
                    // 无取消变量且无分配 → 任务未分配
                    canceled.push(ti);
                }
            }
        }

        TaskAssignmentResult {
            assigned,
            canceled,
            n_tasks,
            n_executors,
        }
    }

    /// 分析带时间的解 / Analyze solution with time
    ///
    /// 在基本分析的基础上，从 `est` 变量中提取预估开始时间，
    /// 从 slack 变量中提取延迟和提前时间。
    ///
    /// In addition to basic analysis, extracts estimate start times from est variables,
    /// and delay/advance times from slack result variables.
    pub fn analyze_with_time<T, E, A>(
        compilation: &Compilation<T, E, A>,
        est_indices: &[Option<usize>],
        durations: &[f64],
        delay_time_indices: &[Option<usize>],
        advance_time_indices: &[Option<usize>],
        solution: &[f64],
    ) -> TaskAssignmentResultWithTime
    where
        E: ExecutorTrait,
        A: AssignmentPolicyTrait<E>,
        T: TaskTrait<E, A>,
    {
        let basic = Self::analyze(compilation, solution);

        let mut time_infos = Vec::new();
        for assignment in &basic.assigned {
            let ti = assignment.task_index;
            let est_value = est_indices.get(ti)
                .and_then(|idx| idx.and_then(|i| extract_value(solution, i)))
                .unwrap_or(0.0);
            let duration = durations.get(ti).copied().unwrap_or(0.0);

            let delay_time = delay_time_indices.get(ti)
                .and_then(|idx| idx.and_then(|i| extract_value(solution, i)))
                .unwrap_or(0.0);
            let advance_time = advance_time_indices.get(ti)
                .and_then(|idx| idx.and_then(|i| extract_value(solution, i)))
                .unwrap_or(0.0);

            time_infos.push(crate::domain::task_compilation::model::TaskTimeInfo {
                task_index: ti,
                estimate_start_time: est_value,
                estimate_end_time: est_value + duration,
                delay_time,
                advance_time,
            });
        }

        TaskAssignmentResultWithTime {
            basic,
            time_infos,
        }
    }
}

/// 任务分配结果 / Task assignment result
#[derive(Debug, Clone)]
pub struct TaskAssignmentResult {
    /// 已分配的任务 / Assigned tasks
    pub assigned: Vec<TaskAssignment>,
    /// 已取消的任务索引 / Canceled task indices
    pub canceled: Vec<usize>,
    /// 任务总数 / Total task count
    pub n_tasks: usize,
    /// 执行者总数 / Total executor count
    pub n_executors: usize,
}

/// 单个任务分配 / Single task assignment
#[derive(Debug, Clone)]
pub struct TaskAssignment {
    /// 任务索引 / Task index
    pub task_index: usize,
    /// 执行者索引 / Executor index
    pub executor_index: usize,
}

/// 带时间的任务分配结果 / Task assignment result with time
#[derive(Debug, Clone)]
pub struct TaskAssignmentResultWithTime {
    /// 基本分配结果 / Basic assignment result
    pub basic: TaskAssignmentResult,
    /// 时间信息 / Time information
    pub time_infos: Vec<crate::domain::task_compilation::model::TaskTimeInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task::{BasicExecutor, BasicAssignmentPolicy, ExecutorTrait, AssignmentPolicyTrait, TaskTrait};
    use crate::domain::task_compilation::model::Compilation;
    use ospf_rust_core::model::MetaModel;
    use ospf_rust_core::variable::Binary;

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

    impl<E: ExecutorTrait, A: AssignmentPolicyTrait<E>> TaskTrait<E, A> for TestTask {
        fn id(&self) -> &str { &self.id }
        fn name(&self) -> &str { &self.name }
    }

    #[test]
    fn test_solution_analyzer_basic() {
        let tasks = vec![
            TestTask::new("t0", "Task 0"),
            TestTask::new("t1", "Task 1"),
        ];
        let executors = vec![
            BasicExecutor::new("e0", "Executor 0"),
            BasicExecutor::new("e1", "Executor 1"),
        ];

        let mut model = MetaModel::<f64>::new("test_analyzer");

        let mut compilation: Compilation<TestTask, BasicExecutor, BasicAssignmentPolicy<BasicExecutor>> =
            Compilation::new(tasks, executors, true, false);
        compilation.register(&mut model).unwrap();

        // 构造模拟解：t0 分配给 e0，t1 取消
        // Build mock solution: t0 assigned to e0, t1 canceled
        let n_vars = model.register_auto_variable::<Binary>("padding").unwrap() + 1;
        let mut solution = vec![0.0; n_vars];

        // 设置 x[0,0] = 1.0 (t0 -> e0)
        if let Some(ref x) = compilation.x {
            if let Some(idx) = x.model_index(&0, &0) {
                solution[idx] = 1.0;
            }
        }
        // 设置 y[1] = 1.0 (t1 canceled)
        if let Some(ref y) = compilation.y {
            if let Some(idx) = y.model_index(&1) {
                solution[idx] = 1.0;
            }
        }

        let result = SolutionAnalyzer::analyze(&compilation, &solution);
        assert_eq!(result.assigned.len(), 1);
        assert_eq!(result.assigned[0].task_index, 0);
        assert_eq!(result.assigned[0].executor_index, 0);
        assert_eq!(result.canceled.len(), 1);
        assert_eq!(result.canceled[0], 1);
        assert_eq!(result.n_tasks, 2);
        assert_eq!(result.n_executors, 2);
    }

    #[test]
    fn test_solution_analyzer_with_time() {
        let tasks = vec![
            TestTask::new("t0", "Task 0"),
        ];
        let executors = vec![
            BasicExecutor::new("e0", "Executor 0"),
        ];

        let mut model = MetaModel::<f64>::new("test_analyzer_time");

        let mut compilation: Compilation<TestTask, BasicExecutor, BasicAssignmentPolicy<BasicExecutor>> =
            Compilation::new(tasks, executors, false, false);
        compilation.register(&mut model).unwrap();

        // 注册 est 变量
        let mut task_time = crate::domain::task_compilation::model::TaskTime::new(true, false, true, false);
        task_time.register(1, vec![Some(10.0)], vec![5.0], &mut model).unwrap();

        // 获取各变量的模型索引
        let est_indices = task_time.est_model_indices();
        let delay_indices = task_time.delay_time_model_indices.clone();
        let advance_indices = task_time.advance_time_model_indices.clone();

        // 确定解向量大小：收集所有模型索引的最大值
        let mut max_idx = 0;
        if let Some(ref x) = compilation.x {
            for ti in 0..1 {
                for ei in 0..1 {
                    if let Some(idx) = x.model_index(&ti, &ei) {
                        max_idx = max_idx.max(idx);
                    }
                }
            }
        }
        if let Some(Some(idx)) = est_indices.first() {
            max_idx = max_idx.max(*idx);
        }
        if let Some(Some(idx)) = delay_indices.first() {
            max_idx = max_idx.max(*idx);
        }
        if let Some(Some(idx)) = advance_indices.first() {
            max_idx = max_idx.max(*idx);
        }
        // SlackFunction 和 MinMaxFunction 会注册额外的辅助变量
        // SlackFunction and MinMaxFunction register additional auxiliary variables
        // 需要足够的空间容纳所有变量 / Need enough space for all variables
        let n_vars = max_idx + 50;
        let mut solution = vec![0.0; n_vars];

        // 设置 x[0,0] = 1.0
        let x_idx = compilation.x.as_ref().and_then(|x| x.model_index(&0, &0));
        if let Some(idx) = x_idx {
            solution[idx] = 1.0;
        }

        // 设置 est[0] = 12.0 (延迟 2.0)
        if let Some(Some(idx)) = est_indices.first() {
            solution[*idx] = 12.0;
        }

        // 设置 delay_time slack result = 2.0
        if let Some(Some(idx)) = delay_indices.first() {
            solution[*idx] = 2.0;
        }

        // 设置 advance_time slack result = 0.0
        if let Some(Some(idx)) = advance_indices.first() {
            solution[*idx] = 0.0;
        }

        let result = SolutionAnalyzer::analyze_with_time(
            &compilation,
            &est_indices,
            &[5.0],
            &delay_indices,
            &advance_indices,
            &solution,
        );

        assert_eq!(result.basic.assigned.len(), 1, "Should have 1 assigned task, got {}", result.basic.assigned.len());
        assert_eq!(result.time_infos.len(), 1);
        let info = &result.time_infos[0];
        assert_eq!(info.task_index, 0);
        assert!((info.estimate_start_time - 12.0).abs() < 1e-6);
        assert!((info.estimate_end_time - 17.0).abs() < 1e-6);
        assert!((info.delay_time - 2.0).abs() < 1e-6);
        assert!((info.advance_time - 0.0).abs() < 1e-6);
    }
}
