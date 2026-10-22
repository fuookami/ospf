//! 空变异算子 / No-op mutation

use crate::model::callback::Solution;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, VariableModel};

use super::Mutation;

/// 空变异，原样返回解 / No-op mutation returning the solution unchanged
///
/// 用于本身不含变异的算法（GWO、SCA、MVO 等），使它们可以复用种群骨架而不被强加一次扰动。
/// Used by algorithms that have no mutation step of their own (GWO, SCA, MVO, …) so they can reuse the population
/// skeleton without being forced through a perturbation.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoMutation;

impl NoMutation {
    /// 创建空变异 / Create a no-op mutation
    pub fn new() -> Self {
        Self
    }
}

impl<V> Mutation<V> for NoMutation
where
    V: HeuristicValue,
{
    fn mutate(
        &self,
        _model: &dyn VariableModel<V>,
        _iteration: &Iteration,
        solution: &Solution<V>,
        _mutation_rate: f64,
        _entropy: &mut dyn Entropy,
    ) -> Solution<V> {
        solution.clone()
    }
}

#[cfg(test)]
mod tests {
    use crate::solver::heuristic::entropy::SeededEntropy;
    use crate::solver::heuristic::iteration::Iteration;

    use super::super::test_support::{model_with_bounds, solution};
    use super::super::Mutation;
    use super::NoMutation;

    #[test]
    fn no_mutation_returns_the_input_unchanged() {
        let test = model_with_bounds(0.0, 10.0);
        let input = solution(&test, 2.0, 7.0);

        let output = NoMutation::new().mutate(
            &test.model,
            &Iteration::new(),
            &input,
            0.5,
            &mut SeededEntropy::new(1),
        );

        assert_eq!(output, input);
    }
}
