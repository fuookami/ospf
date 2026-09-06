//! Framework 数值别名（Kotlin 对齐，零额外依赖）
//! Framework number aliases (Kotlin-aligned, zero extra deps)

use super::{
    FeasibleSolution, FeasibleSolutionV, LPResult, LPResultV, LinearDualSolution,
    LinearDualSolutionV, LinearFeasibleResult, LinearFeasibleResultV, LinearInfeasibleResult,
    LinearInfeasibleResultV, LinearSubResult, LinearSubResultV, QuadraticFeasibleResult,
    QuadraticFeasibleResultV, QuadraticInfeasibleResult, QuadraticInfeasibleResultV,
    QuadraticSubResult, QuadraticSubResultV,
};

/// 默认浮点别名 / Default float alias
pub type FrameworkFlt64 = f64;

/// framework 可行解（f64）/ Framework feasible solution (f64)
pub type FrameworkFeasibleSolution = FeasibleSolution;

/// framework 可行解（typed）/ Framework feasible solution (typed)
pub type FrameworkFeasibleSolutionV<V> = FeasibleSolutionV<V>;

/// framework LP 结果（f64）/ Framework LP result (f64)
pub type FrameworkLPResult = LPResult;

/// framework LP 结果（typed）/ Framework LP result (typed)
pub type FrameworkLPResultV<V> = LPResultV<V>;

/// framework 线性对偶解（f64）/ Framework linear dual solution (f64)
pub type FrameworkLinearDualSolution = LinearDualSolution;

/// framework 线性对偶解（typed）/ Framework linear dual solution (typed)
pub type FrameworkLinearDualSolutionV<V> = LinearDualSolutionV<V>;

/// framework 线性子问题结果（f64）/ Framework linear sub-problem result (f64)
pub type FrameworkLinearSubResult = LinearSubResult;

/// framework 线性子问题结果（typed）/ Framework linear sub-problem result (typed)
pub type FrameworkLinearSubResultV<V> = LinearSubResultV<V>;

/// framework 线性可行子结果（f64）/ Framework linear feasible sub-result (f64)
pub type FrameworkLinearFeasibleResult = LinearFeasibleResult;

/// framework 线性可行子结果（typed）/ Framework linear feasible sub-result (typed)
pub type FrameworkLinearFeasibleResultV<V> = LinearFeasibleResultV<V>;

/// framework 线性不可行子结果（f64）/ Framework linear infeasible sub-result (f64)
pub type FrameworkLinearInfeasibleResult = LinearInfeasibleResult;

/// framework 线性不可行子结果（typed）/ Framework linear infeasible sub-result (typed)
pub type FrameworkLinearInfeasibleResultV<V> = LinearInfeasibleResultV<V>;

/// framework 二次子问题结果（f64）/ Framework quadratic sub-problem result (f64)
pub type FrameworkQuadraticSubResult = QuadraticSubResult;

/// framework 二次子问题结果（typed）/ Framework quadratic sub-problem result (typed)
pub type FrameworkQuadraticSubResultV<V> = QuadraticSubResultV<V>;

/// framework 二次可行子结果（f64）/ Framework quadratic feasible sub-result (f64)
pub type FrameworkQuadraticFeasibleResult = QuadraticFeasibleResult;

/// framework 二次可行子结果（typed）/ Framework quadratic feasible sub-result (typed)
pub type FrameworkQuadraticFeasibleResultV<V> = QuadraticFeasibleResultV<V>;

/// framework 二次不可行子结果（f64）/ Framework quadratic infeasible sub-result (f64)
pub type FrameworkQuadraticInfeasibleResult = QuadraticInfeasibleResult;

/// framework 二次不可行子结果（typed）/ Framework quadratic infeasible sub-result (typed)
pub type FrameworkQuadraticInfeasibleResultV<V> = QuadraticInfeasibleResultV<V>;
