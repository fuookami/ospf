//! 求解器 Trait 定义
//! Solver Trait Definitions

use super::{SolverOutput, SolvingStatus};
use crate::error::Result;
use crate::model::intermediate::{LinearTriadModel, QuadraticTetradModel};

/// 求解器能力 / Solver Capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverCapability {
    /// 线性规划 / Linear Programming
    Linear,
    /// 混合整数线性规划 / Mixed Integer Linear Programming
    Mip,
    /// 二次规划 / Quadratic Programming
    Quadratic,
    /// 混合整数二次规划 / Mixed Integer Quadratic Programming
    MIQP,
    /// 二阶锥规划 / Second Order Cone Programming
    SOCP,
    /// 原生 indicator 约束支持 / Native indicator-constraint support
    NativeIndicator,
    /// 原生 SOS1 约束支持 / Native SOS1-constraint support
    NativeSOS1,
}

/// 求解器通用信息 Trait / Solver common info trait
pub trait SolverInfo: Send + Sync {
    /// 获取求解器名称 / Get solver name
    fn name(&self) -> &str;

    /// 获取求解器能力 / Get solver capabilities
    fn capabilities(&self) -> Vec<SolverCapability>;

    /// 检查是否支持某种能力 / Check if capability is supported
    fn supports(&self, capability: SolverCapability) -> bool {
        self.capabilities().contains(&capability)
    }
}

/// 线性模型求解 trait / Linear-model solver trait
pub trait LinearSolver: SolverInfo {
    /// 求解线性模型 / Solve linear model
    fn solve_linear(&self, model: &LinearTriadModel) -> Result<SolverOutput>;

    /// 求解线性模型并尝试返回解池（可选）/ Solve linear model and optionally return solution pool
    ///
    /// 默认实现返回 `None`，表示求解器未提供原生多解支持。
    /// Default returns `None`, meaning no native multi-solution support.
    fn solve_linear_with_solution_pool(
        &self,
        _model: &LinearTriadModel,
        _solution_amount: usize,
    ) -> Result<Option<(SolverOutput, Vec<Vec<f64>>)>> {
        Ok(None)
    }

    /// 求解线性模型（参数对象）/ Solve linear model with options object
    fn solve_linear_with_options(
        &self,
        model: &LinearTriadModel,
        options: &super::SolveOptions<'_>,
    ) -> Result<SolverOutput> {
        if let Some(callback) = options.solving_status_callback {
            callback(&SolvingStatus::solving(self.name()))?;
        }
        let output = self.solve_linear(model)?;
        if let Some(callback) = options.solving_status_callback {
            let status = SolvingStatus::from_output(self.name(), &output);
            callback(&status)?;
        }
        Ok(output)
    }
}

/// 二次模型求解 trait / Quadratic-model solver trait
pub trait QuadraticSolver: SolverInfo {
    /// 求解二次模型 / Solve quadratic model
    fn solve_quadratic(&self, model: &QuadraticTetradModel) -> Result<SolverOutput>;

    /// 求解二次模型并尝试返回解池（可选）/ Solve quadratic model and optionally return solution pool
    ///
    /// 默认实现返回 `None`，表示求解器未提供原生多解支持。
    /// Default returns `None`, meaning no native multi-solution support.
    fn solve_quadratic_with_solution_pool(
        &self,
        _model: &QuadraticTetradModel,
        _solution_amount: usize,
    ) -> Result<Option<(SolverOutput, Vec<Vec<f64>>)>> {
        Ok(None)
    }

    /// 求解二次模型（参数对象）/ Solve quadratic model with options object
    fn solve_quadratic_with_options(
        &self,
        model: &QuadraticTetradModel,
        options: &super::SolveOptions<'_>,
    ) -> Result<SolverOutput> {
        if let Some(callback) = options.solving_status_callback {
            callback(&SolvingStatus::solving(self.name()))?;
        }
        let output = self.solve_quadratic(model)?;
        if let Some(callback) = options.solving_status_callback {
            let status = SolvingStatus::from_output(self.name(), &output);
            callback(&status)?;
        }
        Ok(output)
    }
}

/// 组合求解器 trait / Combined solver trait
pub trait Solver: LinearSolver + QuadraticSolver {}

impl<T> Solver for T where T: LinearSolver + QuadraticSolver + ?Sized {}

/// 可配置求解器 Trait / Configurable Solver Trait
pub trait ConfigurableSolver: SolverInfo {
    /// 配置类型 / Configuration type
    type Config;

    /// 获取配置 / Get configuration
    fn config(&self) -> &Self::Config;

    /// 设置配置 / Set configuration
    fn set_config(&mut self, config: Self::Config);
}

/// 异步求解器 Trait / Async Solver Trait
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait AsyncSolver: Send + Sync {
    /// 获取求解器名称 / Get solver name
    fn name(&self) -> &str;

    /// 异步求解线性模型 / Solve linear model asynchronously
    async fn solve_linear(&self, model: &LinearTriadModel) -> Result<SolverOutput>;

    /// 异步求解二次模型 / Solve quadratic model asynchronously
    async fn solve_quadratic(&self, model: &QuadraticTetradModel) -> Result<SolverOutput>;
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::model::intermediate::{BasicLinearTriadModel, BasicQuadraticTetradModel};
    use crate::solver::SolverStatus;

    #[derive(Debug)]
    struct DummySolver;

    impl SolverInfo for DummySolver {
        fn name(&self) -> &str {
            "dummy_status_solver"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::Linear, SolverCapability::Quadratic]
        }
    }

    impl LinearSolver for DummySolver {
        fn solve_linear(&self, _model: &LinearTriadModel) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(1.0, vec![2.0]))
        }
    }

    impl QuadraticSolver for DummySolver {
        fn solve_quadratic(&self, _model: &QuadraticTetradModel) -> Result<SolverOutput> {
            Ok(SolverOutput::new(SolverStatus::Optimal).with_solution(vec![3.0]))
        }
    }

    #[test]
    fn solve_linear_with_options_reports_solving_then_final() {
        let solver = DummySolver;
        let model = LinearTriadModel::from_basic(BasicLinearTriadModel::new("cb_linear"));

        let statuses = Arc::new(Mutex::new(Vec::new()));
        let statuses_for_callback = statuses.clone();
        let callback: super::super::SolvingStatusCallback = Arc::new(move |status| {
            statuses_for_callback.lock().unwrap().push(status.status);
            Ok(())
        });

        let options = super::super::SolveOptions::new().with_solving_callback(Some(&callback));
        let output = solver
            .solve_linear_with_options(&model, &options)
            .expect("linear solve with callback should succeed");
        assert!(output.status.is_optimal());

        let statuses = statuses.lock().unwrap();
        assert_eq!(statuses.len(), 2);
        assert_eq!(statuses[0], SolverStatus::Solving);
        assert_eq!(statuses[1], SolverStatus::Optimal);
    }

    #[test]
    fn solve_quadratic_with_options_reports_solving_then_final() {
        let solver = DummySolver;
        let model =
            QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::new("cb_quadratic"));

        let statuses = Arc::new(Mutex::new(Vec::new()));
        let statuses_for_callback = statuses.clone();
        let callback: super::super::SolvingStatusCallback = Arc::new(move |status| {
            statuses_for_callback.lock().unwrap().push(status.status);
            Ok(())
        });

        let options = super::super::SolveOptions::new().with_solving_callback(Some(&callback));
        let output = solver
            .solve_quadratic_with_options(&model, &options)
            .expect("quadratic solve with callback should succeed");
        assert!(output.status.is_optimal());

        let statuses = statuses.lock().unwrap();
        assert_eq!(statuses.len(), 2);
        assert_eq!(statuses[0], SolverStatus::Solving);
        assert_eq!(statuses[1], SolverStatus::Optimal);
    }
}
