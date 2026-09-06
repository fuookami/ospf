/// MetaModel 求解器后端 / MetaModel solver backend
pub trait MetaModelSolverBackend: Debug + Send + Sync {
    /// backend 名称 / Backend name
    fn name(&self) -> &str;

    /// 求解 RMP LP / Solve RMP LP
    fn solve_rmp(
        &self,
        model: &MetaModel<f64>,
        diagnostics: &MetaModelExecutionDiagnostics,
    ) -> Result<MetaModelExecutorSolveResult, String>;

    /// 求解 final MILP / Solve final MILP
    fn solve_final(
        &self,
        model: &MetaModel<f64>,
        diagnostics: &MetaModelExecutionDiagnostics,
    ) -> Result<MetaModelExecutorSolveResult, String>;
}

/// 空操作 MetaModel 求解器后端 / No-op MetaModel solver backend
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

/// 列生成求解器 MetaModel 后端 / ColumnGenerationSolver MetaModel backend
///
/// 该 adapter 复用 framework 的 solver 入口，RMP 使用 LP 求解并读取 dual，
/// final 使用 MILP 求解并读取 primal solution。
/// This adapter reuses framework solver entries: RMP uses LP solving with duals,
/// while final uses MILP solving with primal solution extraction.
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

    /// 设置 RMP 求解参数 / Set RMP solve options
    pub fn with_rmp_options(mut self, options: FrameworkSolveOptions) -> Self {
        self.rmp_options = options;
        self
    }

    /// 设置 final MILP 求解参数 / Set final MILP solve options
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
            .map_err(|e| e.to_string())?;
        let result = self
            .solver
            .solve_lp_with_options(&triad_model, self.rmp_options.clone());
        let result = result.map_err(|e| e.to_string())?;
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
            .map_err(|e| e.to_string())?;
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
