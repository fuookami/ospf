//! 并行组合线性求解器
//! Parallel Combinatorial Linear Solver
//!
//! 本模块提供并行执行的组合线性求解器。
//! This module provides parallel-executing combinatorial linear solvers.

use super::{
    FeasibleSolution, FrameworkSolveOptions, ObjectiveCategory, ParallelCombinatorialMode,
};
use ospf_rust_core::error::{CoreError, Result, SolverError};
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::intermediate::LinearTriadModel;
use std::sync::Arc;

/// 线性求解器 trait / Linear Solver Trait
///
/// 定义线性求解器的接口。
/// Defines interface for linear solvers.
#[cfg_attr(feature = "async", async_trait::async_trait)]
pub trait LinearSolver: Send + Sync {
    /// 获取求解器名称 / Get solver name
    fn name(&self) -> &str;

    /// 求解线性模型 / Solve linear model
    #[cfg(feature = "async")]
    async fn solve(&self, model: &LinearTriadModel) -> Result<FeasibleSolution>;

    /// 使用参数对象求解线性模型 / Solve linear model with options object
    #[cfg(feature = "async")]
    fn solve_with_options<'a>(
        &'a self,
        model: &'a LinearTriadModel,
        _options: FrameworkSolveOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>>
    {
        Box::pin(async move { self.solve(model).await })
    }

    /// 求解线性模型（同步）/ Solve linear model (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve(&self, model: &LinearTriadModel) -> Result<FeasibleSolution>;

    /// 使用参数对象求解线性模型（同步）/ Solve linear model with options object (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_with_options(
        &self,
        model: &LinearTriadModel,
        _options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution> {
        self.solve(model)
    }

    /// 求解线性模型并返回多个解（参数对象）/
    /// Solve linear model and return multiple solutions with options object
    #[cfg(feature = "async")]
    fn solve_multi_with_options<'a>(
        &'a self,
        model: &'a LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<(FeasibleSolution, Vec<Vec<f64>>)>> + Send + 'a,
        >,
    > {
        Box::pin(async move {
            let result = self.solve_with_options(model, options).await?;
            Ok((result.clone(), vec![result.solution]))
        })
    }

    /// 求解线性模型并返回多个解（参数对象，同步）/
    /// Solve linear model and return multiple solutions with options object (synchronous)
    #[cfg(not(feature = "async"))]
    fn solve_multi_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<(FeasibleSolution, Vec<Vec<f64>>)> {
        let result = self.solve_with_options(model, options)?;
        Ok((result.clone(), vec![result.solution]))
    }
}

/// 线性求解器 MetaModel 扩展入口 / MetaModel extension entry for linear solvers
///
/// 推荐应用层入口：`solve_meta` 与 `solve_meta_with_options`。
/// Recommended application-facing entries are `solve_meta` and `solve_meta_with_options`.
#[cfg(feature = "async")]
pub trait LinearMetaModelSolverExt: LinearSolver {
    /// 简化入口：求解 MetaModel / Simplified entry: solve MetaModel
    fn solve_meta<'a, V>(
        &'a self,
        meta_model: &'a MetaModel<V>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>>
    where
        V: ospf_rust_core::solver::SolveValue,
    {
        self.solve_meta_with_options(meta_model, FrameworkSolveOptions::default())
    }

    /// 简化入口：求解 MetaModel（参数对象）/
    /// Simplified entry: solve MetaModel with options object
    fn solve_meta_with_options<'a, V>(
        &'a self,
        meta_model: &'a MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>>
    where
        V: ospf_rust_core::solver::SolveValue,
    {
        let mechanism_model = match meta_model.try_to_mechanism_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let mechanism_model = match ospf_rust_core::solver::convert_mechanism_model_to_f64(
            &mechanism_model,
            options.value_conversion_policy,
        ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let triad_model = match mechanism_model.try_into_linear_triad_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        ) {
            Ok(model) => model,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        Box::pin(async move { self.solve_with_options(&triad_model, options).await })
    }
}

#[cfg(feature = "async")]
impl<T> LinearMetaModelSolverExt for T where T: LinearSolver + ?Sized {}

/// 线性求解器 MetaModel 扩展入口 / MetaModel extension entry for linear solvers
///
/// 推荐应用层入口：`solve_meta` 与 `solve_meta_with_options`。
/// Recommended application-facing entries are `solve_meta` and `solve_meta_with_options`.
#[cfg(not(feature = "async"))]
pub trait LinearMetaModelSolverExt: LinearSolver {
    /// 简化入口：求解 MetaModel / Simplified entry: solve MetaModel
    fn solve_meta<V>(&self, meta_model: &MetaModel<V>) -> Result<FeasibleSolution>
    where
        V: ospf_rust_core::solver::SolveValue,
    {
        self.solve_meta_with_options(meta_model, FrameworkSolveOptions::default())
    }

    /// 简化入口：求解 MetaModel（参数对象）/
    /// Simplified entry: solve MetaModel with options object
    fn solve_meta_with_options<V>(
        &self,
        meta_model: &MetaModel<V>,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution>
    where
        V: ospf_rust_core::solver::SolveValue,
    {
        let mechanism_model = meta_model.try_to_mechanism_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        )?;
        let mechanism_model = ospf_rust_core::solver::convert_mechanism_model_to_f64(
            &mechanism_model,
            options.value_conversion_policy,
        )?;
        let triad_model = mechanism_model.try_into_linear_triad_model_with_status_callback(
            options.model_building_status_callback.as_ref(),
        )?;
        self.solve_with_options(&triad_model, options)
    }
}

#[cfg(not(feature = "async"))]
impl<T> LinearMetaModelSolverExt for T where T: LinearSolver + ?Sized {}

/// 并行组合线性求解器 / Parallel Combinatorial Linear Solver
///
/// 组合多个线性求解器并行执行，根据模式选择结果。
/// Combines multiple linear solvers to execute in parallel, selecting result based on mode.
pub struct ParallelCombinatorialLinearSolver {
    /// 求解器列表 / Solver list
    solvers: Vec<Arc<dyn LinearSolver>>,
    /// 组合模式 / Combinatorial mode
    mode: ParallelCombinatorialMode,
    /// 名称缓存 / Cached name
    name: String,
}

impl ParallelCombinatorialLinearSolver {
    /// 创建新的并行组合线性求解器 / Create new parallel combinatorial linear solver
    pub fn new(solvers: Vec<Arc<dyn LinearSolver>>, mode: ParallelCombinatorialMode) -> Self {
        let names: Vec<&str> = solvers.iter().map(|s| s.name()).collect();
        let name = format!("ParallelCombinatorial({})", names.join(","));
        Self {
            solvers,
            mode,
            name,
        }
    }

    /// 使用默认模式创建 / Create with default mode
    pub fn with_solvers(solvers: Vec<Arc<dyn LinearSolver>>) -> Self {
        Self::new(solvers, ParallelCombinatorialMode::default())
    }

    /// 获取求解器名称 / Get solver name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取模式 / Get mode
    pub fn mode(&self) -> ParallelCombinatorialMode {
        self.mode
    }

    /// 选择最优解 / Select best solution
    fn select_best(
        solutions: Vec<FeasibleSolution>,
        objective_category: ObjectiveCategory,
    ) -> Option<FeasibleSolution> {
        if solutions.is_empty() {
            return None;
        }

        match objective_category {
            ObjectiveCategory::Minimum => solutions.into_iter().min_by(|a, b| {
                a.obj
                    .partial_cmp(&b.obj)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            ObjectiveCategory::Maximum => solutions.into_iter().max_by(|a, b| {
                a.obj
                    .partial_cmp(&b.obj)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
        }
    }

    fn objective_category(model: &LinearTriadModel) -> ObjectiveCategory {
        match model.objective_category {
            ospf_rust_core::model::ObjectiveCategory::Minimum => ObjectiveCategory::Minimum,
            ospf_rust_core::model::ObjectiveCategory::Maximum => ObjectiveCategory::Maximum,
        }
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl LinearSolver for ParallelCombinatorialLinearSolver {
    fn name(&self) -> &str {
        &self.name
    }

    async fn solve(&self, model: &LinearTriadModel) -> Result<FeasibleSolution> {
        use tokio::task::JoinSet;

        match self.mode {
            ParallelCombinatorialMode::First => {
                let mut tasks: JoinSet<Result<FeasibleSolution>> = JoinSet::new();

                for solver in &self.solvers {
                    let solver = Arc::clone(solver);
                    let model = model.clone();
                    tasks.spawn(async move { solver.solve(&model).await });
                }

                while let Some(result) = tasks.join_next().await {
                    match result {
                        Ok(Ok(solution)) => {
                            tasks.shutdown().await;
                            return Ok(solution);
                        }
                        Ok(Err(e)) => {
                            log::warn!("Solver failed: {}", e);
                        }
                        Err(e) => {
                            log::warn!("Task panicked: {}", e);
                        }
                    }
                }

                Err(CoreError::Solver(SolverError::NotAvailable(
                    "No solver valid".into(),
                )))
            }

            ParallelCombinatorialMode::Best => {
                let mut tasks: JoinSet<Result<FeasibleSolution>> = JoinSet::new();

                for solver in &self.solvers {
                    let solver = Arc::clone(solver);
                    let model = model.clone();
                    tasks.spawn(async move { solver.solve(&model).await });
                }

                let mut solutions = Vec::new();
                while let Some(result) = tasks.join_next().await {
                    match result {
                        Ok(Ok(solution)) => {
                            log::info!("Solver found a solution");
                            solutions.push(solution);
                        }
                        Ok(Err(e)) => {
                            log::warn!("Solver failed: {}", e);
                        }
                        Err(e) => {
                            log::warn!("Task panicked: {}", e);
                        }
                    }
                }

                Self::select_best(solutions, Self::objective_category(model)).ok_or_else(|| {
                    CoreError::Solver(SolverError::NotAvailable("No solver valid".into()))
                })
            }
        }
    }

    fn solve_multi_with_options<'a>(
        &'a self,
        model: &'a LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<(FeasibleSolution, Vec<Vec<f64>>)>> + Send + 'a,
        >,
    > {
        Box::pin(async move {
            let result = LinearSolver::solve_with_options(self, model, options).await?;
            Ok((result.clone(), vec![result.solution]))
        })
    }
}

#[cfg(not(feature = "async"))]
impl LinearSolver for ParallelCombinatorialLinearSolver {
    fn name(&self) -> &str {
        &self.name
    }

    fn solve(&self, model: &LinearTriadModel) -> Result<FeasibleSolution> {
        use std::thread;

        match self.mode {
            ParallelCombinatorialMode::First => {
                let handles: Vec<_> = self
                    .solvers
                    .iter()
                    .map(|solver| {
                        let solver = Arc::clone(solver);
                        let model = model.clone();
                        thread::spawn(move || solver.solve(&model))
                    })
                    .collect();

                for handle in handles {
                    match handle.join() {
                        Ok(Ok(solution)) => return Ok(solution),
                        Ok(Err(e)) => log::warn!("Solver failed: {}", e),
                        Err(_) => log::warn!("Thread panicked"),
                    }
                }

                Err(CoreError::Solver(SolverError::NotAvailable(
                    "No solver valid".into(),
                )))
            }

            ParallelCombinatorialMode::Best => {
                let handles: Vec<_> = self
                    .solvers
                    .iter()
                    .map(|solver| {
                        let solver = Arc::clone(solver);
                        let model = model.clone();
                        thread::spawn(move || solver.solve(&model))
                    })
                    .collect();

                let mut solutions = Vec::new();
                for handle in handles {
                    match handle.join() {
                        Ok(Ok(solution)) => {
                            log::info!("Solver found a solution");
                            solutions.push(solution);
                        }
                        Ok(Err(e)) => log::warn!("Solver failed: {}", e),
                        Err(_) => log::warn!("Thread panicked"),
                    }
                }

                Self::select_best(solutions, Self::objective_category(model)).ok_or_else(|| {
                    CoreError::Solver(SolverError::NotAvailable("No solver valid".into()))
                })
            }
        }
    }

    fn solve_multi_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<(FeasibleSolution, Vec<Vec<f64>>)> {
        let result = LinearSolver::solve_with_options(self, model, options)?;
        Ok((result.clone(), vec![result.solution]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "async"))]
    use ospf_rust_core::error::{CoreError, SolverError};
    use ospf_rust_core::model::MetaModel;
    #[cfg(not(feature = "async"))]
    use ospf_rust_core::model::{
        ConstraintRelation, ModelBuildingStage, ModelBuildingStatusCallback,
    };
    #[cfg(not(feature = "async"))]
    use ospf_rust_core::variable::ContinuousVariableItem;
    #[cfg(not(feature = "async"))]
    use std::sync::{Arc, Mutex};

    struct MockSolver {
        name: String,
        result: f64,
    }

    #[cfg_attr(feature = "async", async_trait::async_trait)]
    impl LinearSolver for MockSolver {
        fn name(&self) -> &str {
            &self.name
        }

        #[cfg(feature = "async")]
        async fn solve(&self, _model: &LinearTriadModel) -> Result<FeasibleSolution> {
            Ok(FeasibleSolution::new(self.result, vec![self.result]))
        }

        #[cfg(not(feature = "async"))]
        fn solve(&self, _model: &LinearTriadModel) -> Result<FeasibleSolution> {
            Ok(FeasibleSolution::new(self.result, vec![self.result]))
        }

        #[cfg(feature = "async")]
        fn solve_multi_with_options<'a>(
            &'a self,
            model: &'a LinearTriadModel,
            options: FrameworkSolveOptions,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<(FeasibleSolution, Vec<Vec<f64>>)>>
                    + Send
                    + 'a,
            >,
        > {
            Box::pin(async move {
                let result = LinearSolver::solve_with_options(self, model, options).await?;
                Ok((result.clone(), vec![result.solution]))
            })
        }

        #[cfg(not(feature = "async"))]
        fn solve_multi_with_options(
            &self,
            model: &LinearTriadModel,
            options: FrameworkSolveOptions,
        ) -> Result<(FeasibleSolution, Vec<Vec<f64>>)> {
            let result = LinearSolver::solve_with_options(self, model, options)?;
            Ok((result.clone(), vec![result.solution]))
        }
    }

    #[test]
    fn test_parallel_combinatorial_solver() {
        let solvers: Vec<Arc<dyn LinearSolver>> = vec![
            Arc::new(MockSolver {
                name: "solver1".to_string(),
                result: 1.0,
            }),
            Arc::new(MockSolver {
                name: "solver2".to_string(),
                result: 2.0,
            }),
        ];

        let solver = ParallelCombinatorialLinearSolver::with_solvers(solvers);
        assert!(solver.name().contains("solver1"));
        assert!(solver.name().contains("solver2"));
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_parallel_combinatorial_solver_supports_meta_model_shortcut() {
        let solvers: Vec<Arc<dyn LinearSolver>> = vec![Arc::new(MockSolver {
            name: "solver_meta".to_string(),
            result: 7.0,
        })];
        let solver = ParallelCombinatorialLinearSolver::with_solvers(solvers);
        let mut meta_model = MetaModel::<f64>::new("parallel_linear_meta_shortcut");
        let x = ContinuousVariableItem::auto("parallel_linear_meta_shortcut_x");
        let x_index = meta_model.register_variable(x).unwrap();
        meta_model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                1.0,
                "parallel_linear_meta_shortcut_c",
            )
            .unwrap();

        let result = solver
            .solve_meta(&meta_model)
            .expect("parallel linear combinatorial solver meta shortcut should succeed");
        assert!((result.obj - 7.0).abs() <= 1e-9);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_best_mode_respects_maximum_objective_category() {
        let solvers: Vec<Arc<dyn LinearSolver>> = vec![
            Arc::new(MockSolver {
                name: "solver1".to_string(),
                result: 1.0,
            }),
            Arc::new(MockSolver {
                name: "solver2".to_string(),
                result: 2.0,
            }),
        ];
        let solver =
            ParallelCombinatorialLinearSolver::new(solvers, ParallelCombinatorialMode::Best);
        let mut model = LinearTriadModel::new("max_linear");
        model.objective_category = ospf_rust_core::model::ObjectiveCategory::Maximum;

        let result = solver
            .solve(&model)
            .expect("best mode should return one feasible solution");
        assert!((result.obj - 2.0).abs() <= 1e-9);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_meta_model_shortcut_builds_and_solves() {
        let solver = MockSolver {
            name: "meta_shortcut_solver".to_string(),
            result: 3.0,
        };
        let mut meta_model = MetaModel::<f64>::new("linear_meta_shortcut");
        let x = ContinuousVariableItem::auto("linear_meta_shortcut_x");
        let x_index = meta_model.register_variable(x).unwrap();
        meta_model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                1.0,
                "linear_meta_shortcut_c",
            )
            .unwrap();

        let stages = Arc::new(Mutex::new(Vec::new()));
        let stages_for_callback = stages.clone();
        let callback: ModelBuildingStatusCallback = Arc::new(move |status| {
            stages_for_callback.lock().unwrap().push(status.stage);
            Ok(())
        });

        let options = FrameworkSolveOptions::new().with_building_callback(Some(callback));
        let result = solver
            .solve_meta_with_options(&meta_model, options)
            .expect("meta-model shortcut solve should succeed");
        assert!((result.obj - 3.0).abs() <= 1e-9);

        let stages = stages.lock().unwrap();
        assert!(
            stages
                .iter()
                .any(|stage| *stage == ModelBuildingStage::RegisterTokens)
        );
        assert!(
            stages
                .iter()
                .any(|stage| *stage == ModelBuildingStage::FlattenLinearModel)
        );
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_meta_model_simplified_alias_builds_and_solves() {
        let solver = MockSolver {
            name: "meta_shortcut_solver_alias".to_string(),
            result: 3.5,
        };
        let mut meta_model = MetaModel::<f64>::new("linear_meta_shortcut_alias");
        let x = ContinuousVariableItem::auto("linear_meta_shortcut_alias_x");
        let x_index = meta_model.register_variable(x).unwrap();
        meta_model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                1.0,
                "linear_meta_shortcut_alias_c",
            )
            .unwrap();

        let result = solver
            .solve_meta(&meta_model)
            .expect("meta-model simplified alias solve should succeed");
        assert!((result.obj - 3.5).abs() <= 1e-9);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_meta_model_shortcut_rejects_non_finite_value_in_strict_mode() {
        let solver = MockSolver {
            name: "meta_shortcut_solver_non_finite".to_string(),
            result: 3.0,
        };
        let mut meta_model = MetaModel::<f64>::new("linear_meta_shortcut_non_finite");
        let x = ContinuousVariableItem::auto("linear_meta_shortcut_non_finite_x");
        let x_index = meta_model.register_variable(x).unwrap();
        meta_model
            .add_linear_constraint(
                &[(x_index, f64::NAN)],
                ConstraintRelation::LessEqual,
                1.0,
                "linear_meta_shortcut_non_finite_c",
            )
            .unwrap();

        let options = FrameworkSolveOptions::new().with_value_conversion_policy(
            ospf_rust_core::solver::SolveValueConversionPolicy::Strict,
        );
        let error = solver
            .solve_meta_with_options(&meta_model, options)
            .expect_err("strict mode should reject non-finite conversion");
        assert!(matches!(
            error,
            CoreError::Solver(SolverError::NonFinite(_))
        ));
    }

    #[cfg(feature = "async")]
    fn assert_send<T: Send>(_: &T) {}

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_meta_model_shortcut_returns_send_future() {
        let solver = MockSolver {
            name: "meta_shortcut_async_solver".to_string(),
            result: 4.0,
        };
        let meta_model = MetaModel::<f64>::new("linear_meta_shortcut_async");

        let future = solver.solve_meta(&meta_model);
        assert_send(&future);
        let result = future
            .await
            .expect("meta-model shortcut async solve should succeed");
        assert!((result.obj - 4.0).abs() <= 1e-9);
    }
}
