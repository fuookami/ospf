//! 求解结果定义
//! Solver Output Definitions

use std::sync::Arc;
use std::time::Duration;
use crate::error::Result;
use crate::error::{CoreError, SolverError};
use crate::solver::iis::LinearIISModel;
use crate::solver::value::boundary::value_from_backend_f64;
use crate::solver::value::conversion_context::SolveValueConversionContext;
use crate::solver::value::{SolveValue, SolveValueConversionPolicy};

/// 求解状态 / Solver Status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverStatus {
    /// 最优 / Optimal
    Optimal,
    /// 可行（非最优） / Feasible (non-optimal)
    Feasible,
    /// 不可行 / Infeasible
    Infeasible,
    /// 不可行或无界 / Infeasible or unbounded
    InfeasibleOrUnbounded,
    /// 无界 / Unbounded
    Unbounded,
    /// 达到迭代上限 / Iteration limit
    IterationLimit,
    /// 达到时间上限 / Time limit
    TimeLimit,
    /// 数值错误 / Numeric error
    NumericError,
    /// 未开始 / Not started
    NotStarted,
    /// 求解中 / Solving
    Solving,
    /// 用户中断 / User interrupt
    UserInterrupt,
    /// 未知 / Unknown
    Unknown,
}

impl SolverStatus {
    /// 检查是否找到最优解 / Check if optimal solution found
    pub fn is_optimal(&self) -> bool {
        matches!(self, SolverStatus::Optimal)
    }

    /// 检查是否可行 / Check if feasible
    pub fn is_feasible(&self) -> bool {
        matches!(
            self,
            SolverStatus::Optimal
                | SolverStatus::Feasible
                | SolverStatus::IterationLimit
                | SolverStatus::TimeLimit
        )
    }

    /// 检查是否不可行 / Check if infeasible
    pub fn is_infeasible(&self) -> bool {
        matches!(
            self,
            SolverStatus::Infeasible | SolverStatus::InfeasibleOrUnbounded
        )
    }

    /// 检查是否无界 / Check if unbounded
    pub fn is_unbounded(&self) -> bool {
        matches!(self, SolverStatus::Unbounded)
    }
}

/// 求解结果 / Solver Output
#[derive(Debug, Clone)]
pub struct SolverOutput {
    /// 求解状态 / Solver status
    pub status: SolverStatus,
    /// 目标值 / Objective value
    pub objective_value: Option<f64>,
    /// 解向量 / Solution vector
    pub solution: Option<Vec<f64>>,
    /// 对偶解 / Dual solution
    pub dual_solution: Option<Vec<f64>>,
    /// 二次约束对偶解 / Quadratic-constraint dual solution
    pub quadratic_dual_solution: Option<Vec<f64>>,
    /// 求解时间 / Solve time
    pub solve_time: Duration,
    /// 迭代次数 / Iteration count
    pub iterations: Option<usize>,
    /// 节点数（MIP）/ Node count (MIP)
    pub node_count: Option<usize>,
    /// MIP Gap / MIP Gap
    pub mip_gap: Option<f64>,
    /// 最优下界（MIP）/ Best bound (MIP)
    pub best_bound: Option<f64>,
}

impl SolverOutput {
    /// 创建新的求解结果 / Create new solver output
    pub fn new(status: SolverStatus) -> Self {
        Self {
            status,
            objective_value: None,
            solution: None,
            dual_solution: None,
            quadratic_dual_solution: None,
            solve_time: Duration::ZERO,
            iterations: None,
            node_count: None,
            mip_gap: None,
            best_bound: None,
        }
    }

    /// 创建最优解结果 / Create optimal solution output
    pub fn optimal(objective_value: f64, solution: Vec<f64>) -> Self {
        Self {
            status: SolverStatus::Optimal,
            objective_value: Some(objective_value),
            solution: Some(solution),
            dual_solution: None,
            quadratic_dual_solution: None,
            solve_time: Duration::ZERO,
            iterations: None,
            node_count: None,
            mip_gap: None,
            best_bound: None,
        }
    }

    /// 创建不可行结果 / Create infeasible output
    pub fn infeasible() -> Self {
        Self::new(SolverStatus::Infeasible)
    }

    /// 创建无界结果 / Create unbounded output
    pub fn unbounded() -> Self {
        Self::new(SolverStatus::Unbounded)
    }

    /// 设置目标值 / Set objective value
    pub fn with_objective(mut self, value: f64) -> Self {
        self.objective_value = Some(value);
        self
    }

    /// 设置解向量 / Set solution
    pub fn with_solution(mut self, solution: Vec<f64>) -> Self {
        self.solution = Some(solution);
        self
    }

    /// 设置对偶解 / Set dual solution
    pub fn with_dual(mut self, dual: Vec<f64>) -> Self {
        self.dual_solution = Some(dual);
        self
    }

    /// 设置二次约束对偶解 / Set quadratic-constraint dual solution
    pub fn with_quadratic_dual(mut self, dual: Vec<f64>) -> Self {
        self.quadratic_dual_solution = Some(dual);
        self
    }

    /// 设置求解时间 / Set solve time
    pub fn with_time(mut self, time: Duration) -> Self {
        self.solve_time = time;
        self
    }

    /// 设置迭代次数 / Set iteration count
    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = Some(iterations);
        self
    }

    /// 检查是否有解 / Check if has solution
    pub fn has_solution(&self) -> bool {
        self.solution.is_some()
    }

    /// 获取解向量引用 / Get solution reference
    pub fn get_solution(&self) -> Option<&[f64]> {
        self.solution.as_deref()
    }

    /// 转换为可行 typed 输出 / Convert into feasible typed output
    pub fn try_into_feasible_typed<V>(
        self,
        policy: SolveValueConversionPolicy,
    ) -> Result<FeasibleSolverOutput<V>>
    where
        V: SolveValue,
    {
        if !self.status.is_feasible() {
            return Err(CoreError::Solver(SolverError::SolveFailed(format!(
                "status {:?} is not feasible",
                self.status
            ))));
        }
        let solution = self
            .solution
            .ok_or(CoreError::Solver(SolverError::NoSolution))?;
        let convert_vector = |values: Vec<f64>, field_name: &str| -> Result<Vec<V>> {
            values
                .into_iter()
                .enumerate()
                .map(|(index, value)| {
                    let context =
                        SolveValueConversionContext::new(format!("{}[{}]", field_name, index));
                    value_from_backend_f64(value, policy, &context)
                })
                .collect::<Result<Vec<V>>>()
        };
        let typed_solution = convert_vector(solution, "solution")?;
        Ok(FeasibleSolverOutput {
            status: self.status,
            objective_value: self
                .objective_value
                .map(|value| {
                    let context = SolveValueConversionContext::new("objective_value");
                    value_from_backend_f64(value, policy, &context)
                })
                .transpose()?,
            solution: typed_solution,
            dual_solution: self
                .dual_solution
                .map(|values| convert_vector(values, "dual_solution"))
                .transpose()?,
            quadratic_dual_solution: self
                .quadratic_dual_solution
                .map(|values| convert_vector(values, "quadratic_dual_solution"))
                .transpose()?,
            solve_time: self.solve_time,
            iterations: self.iterations,
            node_count: self.node_count,
            mip_gap: self.mip_gap,
            best_bound: self
                .best_bound
                .map(|value| {
                    let context = SolveValueConversionContext::new("best_bound");
                    value_from_backend_f64(value, policy, &context)
                })
                .transpose()?,
        })
    }

    /// 转换为线性不可行输出 / Convert into linear infeasible output
    pub fn try_into_linear_infeasible(self) -> Result<LinearInfeasibleSolverOutput> {
        if !self.status.is_infeasible() {
            return Err(CoreError::Solver(SolverError::SolveFailed(format!(
                "status {:?} is not infeasible",
                self.status
            ))));
        }
        Ok(LinearInfeasibleSolverOutput {
            status: self.status,
            solve_time: self.solve_time,
            iterations: self.iterations,
            node_count: self.node_count,
            mip_gap: self.mip_gap,
            best_bound: self.best_bound,
        })
    }

    /// 转换为二次不可行输出 / Convert into quadratic infeasible output
    pub fn try_into_quadratic_infeasible(self) -> Result<QuadraticInfeasibleSolverOutput> {
        if !self.status.is_infeasible() {
            return Err(CoreError::Solver(SolverError::SolveFailed(format!(
                "status {:?} is not infeasible",
                self.status
            ))));
        }
        Ok(QuadraticInfeasibleSolverOutput {
            status: self.status,
            solve_time: self.solve_time,
            iterations: self.iterations,
            node_count: self.node_count,
            mip_gap: self.mip_gap,
            best_bound: self.best_bound,
        })
    }
}

/// 可行 typed 输出 / Feasible typed solver output
#[derive(Debug, Clone)]
pub struct FeasibleSolverOutput<V>
where
    V: SolveValue,
{
    /// 求解状态 / Solver status
    pub status: SolverStatus,
    /// typed 目标值 / Typed objective value
    pub objective_value: Option<V>,
    /// typed 解向量 / Typed solution vector
    pub solution: Vec<V>,
    /// typed 对偶解 / Typed dual solution
    pub dual_solution: Option<Vec<V>>,
    /// typed 二次约束对偶解 / Typed quadratic-constraint dual solution
    pub quadratic_dual_solution: Option<Vec<V>>,
    /// 求解时间 / Solve time
    pub solve_time: Duration,
    /// 迭代次数 / Iteration count
    pub iterations: Option<usize>,
    /// 节点数（MIP）/ Node count (MIP)
    pub node_count: Option<usize>,
    /// MIP Gap / MIP gap
    pub mip_gap: Option<f64>,
    /// typed 最优下界（MIP）/ Typed best bound (MIP)
    pub best_bound: Option<V>,
}

/// 线性不可行输出 / Linear infeasible solver output
#[derive(Debug, Clone)]
pub struct LinearInfeasibleSolverOutput {
    /// 求解状态 / Solver status
    pub status: SolverStatus,
    /// 求解时间 / Solve time
    pub solve_time: Duration,
    /// 迭代次数 / Iteration count
    pub iterations: Option<usize>,
    /// 节点数（MIP）/ Node count (MIP)
    pub node_count: Option<usize>,
    /// MIP Gap / MIP gap
    pub mip_gap: Option<f64>,
    /// 最优下界（MIP）/ Best bound (MIP)
    pub best_bound: Option<f64>,
}

/// 二次不可行输出 / Quadratic infeasible solver output
#[derive(Debug, Clone)]
pub struct QuadraticInfeasibleSolverOutput {
    /// 求解状态 / Solver status
    pub status: SolverStatus,
    /// 求解时间 / Solve time
    pub solve_time: Duration,
    /// 迭代次数 / Iteration count
    pub iterations: Option<usize>,
    /// 节点数（MIP）/ Node count (MIP)
    pub node_count: Option<usize>,
    /// MIP Gap / MIP gap
    pub mip_gap: Option<f64>,
    /// 最优下界（MIP）/ Best bound (MIP)
    pub best_bound: Option<f64>,
}

/// 带 IIS 的求解输出 / Solver output with IIS
#[derive(Debug, Clone)]
pub struct SolverOutputWithIIS<IIS = LinearIISModel> {
    /// 原始求解输出 / Raw solver output
    pub output: SolverOutput,
    /// IIS 结果（仅不可行时）/ IIS result (only when infeasible)
    pub iis: Option<IIS>,
}

/// 求解状态快照 / Solving status snapshot
#[derive(Debug, Clone)]
pub struct SolvingStatus {
    /// 求解器名称 / Solver name
    pub solver: String,
    /// 求解状态 / Solver status
    pub status: SolverStatus,
    /// 当前目标值 / Current objective value
    pub objective_value: Option<f64>,
    /// 最优下界 / Best bound
    pub best_bound: Option<f64>,
    /// MIP Gap / MIP gap
    pub mip_gap: Option<f64>,
    /// 迭代次数 / Iteration count
    pub iterations: Option<usize>,
    /// 节点数 / Node count
    pub node_count: Option<usize>,
    /// 累计耗时 / Elapsed time
    pub solve_time: Duration,
}

impl SolvingStatus {
    /// 创建“求解中”状态 / Build "solving" status
    pub fn solving(solver: impl Into<String>) -> Self {
        Self {
            solver: solver.into(),
            status: SolverStatus::Solving,
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
            solve_time: Duration::ZERO,
        }
    }

    /// 从输出构造状态 / Build status from solver output
    pub fn from_output(solver: impl Into<String>, output: &SolverOutput) -> Self {
        Self {
            solver: solver.into(),
            status: output.status,
            objective_value: output.objective_value,
            best_bound: output.best_bound,
            mip_gap: output.mip_gap,
            iterations: output.iterations,
            node_count: output.node_count,
            solve_time: output.solve_time,
        }
    }
}

/// 求解状态回调 / Solving status callback
pub type SolvingStatusCallback = Arc<dyn Fn(&SolvingStatus) -> Result<()> + Send + Sync>;

#[cfg(test)]
mod tests {
    use super::*;
    use bigdecimal::BigDecimal;
    use std::str::FromStr;

    #[test]
    fn feasible_output_can_convert_to_typed() {
        let raw = SolverOutput::optimal(1.25, vec![2.5, 3.5]).with_dual(vec![0.5, -0.5]);
        let typed = raw
            .try_into_feasible_typed::<BigDecimal>(SolveValueConversionPolicy::AllowRounding)
            .expect("typed conversion should succeed");
        assert_eq!(
            typed.objective_value,
            Some(BigDecimal::from_str("1.25").expect("build decimal"))
        );
        assert_eq!(typed.solution.len(), 2);
        assert_eq!(typed.dual_solution.as_ref().map(|v| v.len()), Some(2));
    }

    #[test]
    fn infeasible_output_can_convert_to_linear_infeasible() {
        let raw = SolverOutput::infeasible().with_time(Duration::from_millis(7));
        let converted = raw
            .try_into_linear_infeasible()
            .expect("infeasible conversion should succeed");
        assert!(converted.status.is_infeasible());
        assert_eq!(converted.solve_time, Duration::from_millis(7));
    }
}
