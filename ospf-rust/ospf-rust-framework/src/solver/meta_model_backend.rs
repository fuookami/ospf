//! MetaModel 求解器后端抽象 / MetaModel solver backend abstraction
//!
//! framework 领域 crate（BPP2D/BPP3D/CSP1D/CSP2D 等）的列生成执行器需要同一个可插拔求解边界：
//! RMP 走 LP 并读取对偶解，final 走 MILP 并读取原始解。
//! 把该边界放在 `ospf-rust-framework` 中，使领域 crate 无需互相依赖即可共享后端适配器。
//! Domain framework crates (BPP2D/BPP3D/CSP1D/CSP2D) need one pluggable solving boundary for their
//! column-generation executors: the RMP solves an LP and reads duals, the final solves a MILP and reads
//! the primal solution. Hosting that boundary in `ospf-rust-framework` lets domain crates share the
//! backend adapter without depending on each other.

use std::collections::HashMap;
use std::fmt::Debug;

use ospf_rust_core::model::MetaModel;

use super::column_generation_solver::ColumnGenerationSolver;
use super::framework_solve_options::FrameworkSolveOptions;

// ============================================================================
// MetaModelExecutorSolveResult - 执行器求解结果 / Executor solve result
// ============================================================================

/// MetaModel executor 求解结果 / MetaModel executor solve result
///
/// 只携带执行器与后端之间交换的最小信息，便于不同领域 crate 共用同一后端边界。
/// Carries only the information exchanged between an executor and a backend so that different
/// domain crates can share one backend boundary.
#[derive(Debug, Clone, Default)]
pub struct MetaModelExecutorSolveResult {
    /// 目标值 / Objective
    pub objective: Option<f64>,
    /// 原始解向量 / Primal solution vector
    pub primal_solution: Vec<f64>,
    /// 对偶解向量 / Dual solution vector
    pub dual_solution: Vec<f64>,
    /// 扩展影子价格 / Additional typed shadow prices
    pub additional_shadow_prices: HashMap<String, f64>,
    /// 附加信息 / Additional information
    pub info: HashMap<String, String>,
}

// ============================================================================
// MetaModelExecutionDiagnostics - 执行诊断 / Execution diagnostics
// ============================================================================

/// MetaModel 执行诊断 / MetaModel execution diagnostics
///
/// 由执行器在建模完成后构造，作为求解后端的可观测输入；后端可以忽略，也可以写入自身日志。
/// Built by an executor after modelling completes and passed to the backend as observable input;
/// a backend may ignore it or forward it to its own logging.
#[derive(Debug, Clone, Default)]
pub struct MetaModelExecutionDiagnostics {
    /// 模型名称 / Model name
    pub model_name: String,
    /// 变量数量 / Variable count
    pub variable_count: usize,
    /// 约束数量 / Constraint count
    pub constraint_count: usize,
    /// 注册的需求数量 / Registered demand count
    pub demand_count: usize,
    /// 注册的列数量 / Registered column count
    pub column_count: usize,
    /// 注册的箱或板材数量 / Registered bin or sheet count
    pub container_count: usize,
}

impl MetaModelExecutionDiagnostics {
    /// 从模型创建诊断 / Create diagnostics from a model
    ///
    /// `column_count` 与 `container_count` 由领域执行器按自身语义解释（层数/箱数、图案数/板材数）。
    /// `column_count` and `container_count` are interpreted by each domain executor
    /// (layer count / bin count, pattern count / sheet count).
    pub fn from_model(
        model_name: impl Into<String>,
        model: &MetaModel<f64>,
        demand_count: usize,
        column_count: usize,
        container_count: usize,
    ) -> Self {
        Self {
            model_name: model_name.into(),
            variable_count: model.num_tokens(),
            constraint_count: model.num_constraints(),
            demand_count,
            column_count,
            container_count,
        }
    }

    /// 写入 info map / Write into an info map
    pub fn write_info(&self, info: &mut HashMap<String, String>) {
        info.insert("model_name".to_string(), self.model_name.clone());
        info.insert("variable_count".to_string(), self.variable_count.to_string());
        info.insert("constraint_count".to_string(), self.constraint_count.to_string());
        info.insert("demand_count".to_string(), self.demand_count.to_string());
        info.insert("column_count".to_string(), self.column_count.to_string());
        info.insert("container_count".to_string(), self.container_count.to_string());
    }
}

// ============================================================================
// MetaModelSolverBackend - 求解后端 / Solver backend
// ============================================================================

/// MetaModel 求解器后端 / MetaModel solver backend
///
/// 列生成执行器把已注册的 `MetaModel` 交给后端求解；后端决定 solver、选项与结果提取口径。
/// Column-generation executors hand a registered `MetaModel` to a backend, which decides the solver,
/// the options, and how results are extracted.
pub trait MetaModelSolverBackend: Debug + Send + Sync {
    /// backend 名称 / Backend name
    fn name(&self) -> &str;

    /// 求解 RMP LP / Solve the RMP as an LP
    fn solve_rmp(
        &self,
        model: &MetaModel<f64>,
        diagnostics: &MetaModelExecutionDiagnostics,
    ) -> Result<MetaModelExecutorSolveResult, String>;

    /// 求解 final MILP / Solve the final MILP
    fn solve_final(
        &self,
        model: &MetaModel<f64>,
        diagnostics: &MetaModelExecutionDiagnostics,
    ) -> Result<MetaModelExecutorSolveResult, String>;
}

// ============================================================================
// NoopMetaModelSolverBackend - 空操作后端 / No-op backend
// ============================================================================

/// 空操作 MetaModel 求解器后端 / No-op MetaModel solver backend
///
/// 返回空结果，供不求解的装配自检与契约测试使用。
/// Returns an empty result, for assembly self-checks and contract tests that do not actually solve.
#[derive(Debug, Clone, Default)]
pub struct NoopMetaModelSolverBackend;

impl MetaModelSolverBackend for NoopMetaModelSolverBackend {
    fn name(&self) -> &str {
        "noop"
    }

    fn solve_rmp(
        &self,
        _model: &MetaModel<f64>,
        _diagnostics: &MetaModelExecutionDiagnostics,
    ) -> Result<MetaModelExecutorSolveResult, String> {
        Ok(MetaModelExecutorSolveResult::default())
    }

    fn solve_final(
        &self,
        _model: &MetaModel<f64>,
        _diagnostics: &MetaModelExecutionDiagnostics,
    ) -> Result<MetaModelExecutorSolveResult, String> {
        Ok(MetaModelExecutorSolveResult::default())
    }
}

// ============================================================================
// ColumnGenerationSolverMetaModelBackend - 列生成求解器适配 / ColumnGenerationSolver adapter
// ============================================================================

/// 列生成求解器 MetaModel 后端 / ColumnGenerationSolver MetaModel backend
///
/// 该 adapter 复用 framework 的 solver 入口：RMP 使用 LP 求解并读取 dual，
/// final 使用 MILP 求解并读取 primal solution。
/// This adapter reuses the framework solver entries: the RMP solves an LP and reads duals,
/// while the final solves a MILP and reads the primal solution.
#[cfg(not(feature = "async"))]
#[derive(Clone)]
pub struct ColumnGenerationSolverMetaModelBackend<S>
where
    S: ColumnGenerationSolver,
{
    /// 求解器 / Solver
    pub solver: S,
    /// RMP 求解参数 / RMP solve options
    pub rmp_options: FrameworkSolveOptions,
    /// final MILP 求解参数 / Final MILP solve options
    pub final_options: FrameworkSolveOptions,
}

#[cfg(not(feature = "async"))]
impl<S> Debug for ColumnGenerationSolverMetaModelBackend<S>
where
    S: ColumnGenerationSolver,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ColumnGenerationSolverMetaModelBackend")
            .field("solver", &self.solver.name())
            .finish()
    }
}

#[cfg(not(feature = "async"))]
impl<S> ColumnGenerationSolverMetaModelBackend<S>
where
    S: ColumnGenerationSolver,
{
    /// 使用默认参数创建 / Create with default options
    pub fn new(solver: S) -> Self {
        Self {
            solver,
            rmp_options: FrameworkSolveOptions::new(),
            final_options: FrameworkSolveOptions::new(),
        }
    }

    /// 设置 RMP 求解参数 / Set the RMP solve options
    pub fn with_rmp_options(mut self, options: FrameworkSolveOptions) -> Self {
        self.rmp_options = options;
        self
    }

    /// 设置 final MILP 求解参数 / Set the final MILP solve options
    pub fn with_final_options(mut self, options: FrameworkSolveOptions) -> Self {
        self.final_options = options;
        self
    }
}

#[cfg(not(feature = "async"))]
impl<S> MetaModelSolverBackend for ColumnGenerationSolverMetaModelBackend<S>
where
    S: ColumnGenerationSolver + Debug,
{
    fn name(&self) -> &str {
        self.solver.name()
    }

    fn solve_rmp(
        &self,
        model: &MetaModel<f64>,
        _diagnostics: &MetaModelExecutionDiagnostics,
    ) -> Result<MetaModelExecutorSolveResult, String> {
        let triad_model = model
            .try_to_linear_triad_model()
            .map_err(|error| error.to_string())?;
        let result = self
            .solver
            .solve_lp_with_options(&triad_model, self.rmp_options.clone())
            .map_err(|error| error.to_string())?;
        Ok(MetaModelExecutorSolveResult {
            objective: Some(result.result.obj),
            primal_solution: result.result.solution,
            dual_solution: result.dual_solution.constraints,
            additional_shadow_prices: HashMap::new(),
            info: HashMap::from([
                ("backend_kind".to_string(), "column_generation_solver".to_string()),
                ("backend_phase".to_string(), "rmp".to_string()),
            ]),
        })
    }

    fn solve_final(
        &self,
        model: &MetaModel<f64>,
        _diagnostics: &MetaModelExecutionDiagnostics,
    ) -> Result<MetaModelExecutorSolveResult, String> {
        let result = self
            .solver
            .solve_with_options(model, self.final_options.clone())
            .map_err(|error| error.to_string())?;
        Ok(MetaModelExecutorSolveResult {
            objective: Some(result.obj),
            primal_solution: result.solution,
            dual_solution: Vec::new(),
            additional_shadow_prices: HashMap::new(),
            info: HashMap::from([
                ("backend_kind".to_string(), "column_generation_solver".to_string()),
                ("backend_phase".to_string(), "final".to_string()),
            ]),
        })
    }
}
