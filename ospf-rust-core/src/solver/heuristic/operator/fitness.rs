//! 适应度评估算子 / Fitness evaluation operators

use crate::model::callback::Solution;

use super::super::individual::Individual;
use super::super::model::{HeuristicValue, ObjectiveModel, to_f64};

/// 适应度评估算子契约 / Fitness evaluator contract
///
/// 把解转成个体：计算目标值并拒绝不可比较的结果。所有入口都必须经过「非有限值过滤」，否则 NaN/±∞ 会
/// 传染整个种群并破坏排序。
/// Turns a solution into an individual: evaluates the objective and rejects incomparable results. Every entry point must
/// pass through the non-finite filter, otherwise NaN/±∞ would infect the population and break sorting.
pub trait FitnessEvaluator<V>
where
    V: HeuristicValue,
{
    /// 评估一个解 / Evaluate a solution
    fn evaluate(
        &self,
        model: &dyn ObjectiveModel<V, V>,
        solution: &Solution<V>,
    ) -> Option<Individual<V, V>>;
}

/// 生成个体并在入口处过滤非有限目标值 / Build an individual and reject non-finite objectives at the entry point
pub fn to_individual<V>(solution: Solution<V>, fitness: V) -> Option<Individual<V, V>>
where
    V: HeuristicValue,
{
    if !to_f64(&fitness).is_finite() {
        return None;
    }
    Some(Individual::new(solution, fitness))
}

/// 仅使用目标评估器 / Objective-only evaluation
#[derive(Debug, Clone, Copy, Default)]
pub struct ObjectiveFitnessEvaluator;

impl<V> FitnessEvaluator<V> for ObjectiveFitnessEvaluator
where
    V: HeuristicValue,
{
    fn evaluate(
        &self,
        model: &dyn ObjectiveModel<V, V>,
        solution: &Solution<V>,
    ) -> Option<Individual<V, V>> {
        let fitness = model.objective(solution)?;
        to_individual(solution.clone(), fitness)
    }
}

/// 目标缺失时回退到默认目标值 / Fall back to the default objective when evaluation misses
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultOnMissFitnessEvaluator;

impl<V> FitnessEvaluator<V> for DefaultOnMissFitnessEvaluator
where
    V: HeuristicValue,
{
    fn evaluate(
        &self,
        model: &dyn ObjectiveModel<V, V>,
        solution: &Solution<V>,
    ) -> Option<Individual<V, V>> {
        let fitness = model
            .objective(solution)
            .or_else(|| model.default_objective())?;
        to_individual(solution.clone(), fitness)
    }
}

/// 约束感知评估：不可行解直接拒绝 / Constraint-aware evaluation rejecting infeasible solutions
#[derive(Debug, Clone, Copy, Default)]
pub struct ConstraintAwareFitnessEvaluator;

impl<V> FitnessEvaluator<V> for ConstraintAwareFitnessEvaluator
where
    V: HeuristicValue,
{
    fn evaluate(
        &self,
        model: &dyn ObjectiveModel<V, V>,
        solution: &Solution<V>,
    ) -> Option<Individual<V, V>> {
        if model.constraint_satisfied(solution) == Some(false) {
            return None;
        }
        let fitness = model
            .objective(solution)
            .or_else(|| model.default_objective())?;
        to_individual(solution.clone(), fitness)
    }
}
