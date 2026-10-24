//! 变异算子 / Mutation operators
//!
//! 变异是解更新之后的独立槽位：它对单个个体施加扰动，因此与「从一组父代产生一批解」的解更新契约分开。
//! Mutation is the slot after solution update: it perturbs one individual, so it is kept separate from the
//! solution-update contract that emits a batch from a parent group.

use crate::model::callback::Solution;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, VariableModel};

pub mod continuous;
pub mod no_mutation;
pub mod permutation;

pub use continuous::{
    BoundaryMutation, CauchyMutation, ChaoticMutation, GaussianMutation, LevyMutation,
    NonUniformMutation, PolynomialMutation, UniformMutation,
};
pub use no_mutation::NoMutation;
pub use permutation::{InsertionMutation, InversionMutation, ScrambleMutation, SwapMutation};

/// 变异算子契约 / Mutation contract
///
/// 熵源由调用方注入而非由实现持有，因此所有实现都是无状态单元，可安全共享。
/// The entropy source is injected by the caller instead of being held by an implementation, so every implementation is
/// a stateless unit that can be shared safely.
///
/// # 类型参数 / Type parameters
/// - `V`: 值类型 / Value type
pub trait Mutation<V>
where
    V: HeuristicValue,
{
    /// 对个体执行变异 / Mutate one individual
    ///
    /// @param model 变量模型 / Variable model
    /// @param iteration 当前迭代 / Current iteration
    /// @param solution 待变异的解 / Solution to mutate
    /// @param mutation_rate 逐基因变异率 / Per-gene mutation rate
    /// @param entropy 均匀分布熵源 / Uniform entropy source
    /// @return 变异后的新解 / Mutated solution
    fn mutate(
        &self,
        model: &dyn VariableModel<V>,
        iteration: &Iteration,
        solution: &Solution<V>,
        mutation_rate: f64,
        entropy: &mut dyn Entropy,
    ) -> Solution<V>;
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::collections::HashMap;
    use std::sync::Arc;

    use crate::model::callback::{AbstractCallBackModel, Solution};
    use crate::variable::{VariableId, new_standalone_id};

    /// 测试模型与变量标识 / Test model and variable identifiers
    pub(crate) struct TestModel {
        pub(crate) model: AbstractCallBackModel<f64>,
        pub(crate) x: VariableId,
        pub(crate) y: VariableId,
    }

    /// 构建两变量、统一上下界的测试模型 / Build a two-variable test model with shared bounds
    pub(crate) fn model_with_bounds(lower: f64, upper: f64) -> TestModel {
        let x = new_standalone_id();
        let y = new_standalone_id();
        let mut model = AbstractCallBackModel::<f64>::new("mutation_operator_test");
        model.register_variable_name(x, "x".to_string());
        model.register_variable_name(y, "y".to_string());
        model.set_variable_bounds_provider(Arc::new(move |_id| Some((Some(lower), Some(upper)))));
        TestModel { model, x, y }
    }

    /// 构造一个二变量解 / Build a two-variable solution
    pub(crate) fn solution(test: &TestModel, x: f64, y: f64) -> Solution<f64> {
        HashMap::from([(test.x, x), (test.y, y)])
    }

    /// 断言解维度正确且落在边界内 / Assert the solution has the right dimension and stays within bounds
    pub(crate) fn assert_dimension_and_bounds(solution: &Solution<f64>, lower: f64, upper: f64) {
        assert_eq!(solution.len(), 2);
        for value in solution.values() {
            assert!(value.is_finite(), "value must be finite: {value}");
            assert!(
                *value >= lower - 1.0e-9 && *value <= upper + 1.0e-9,
                "value {value} leaves [{lower}, {upper}]"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::solver::heuristic::entropy::SeededEntropy;
    use crate::solver::heuristic::iteration::Iteration;

    use super::continuous::GaussianMutation;
    use super::test_support::{model_with_bounds, solution};
    use super::{Mutation, NoMutation};

    #[test]
    fn boxed_mutations_dispatch_through_the_contract() {
        let test = model_with_bounds(0.0, 10.0);
        let input = solution(&test, 3.0, 6.0);
        let iteration = Iteration::new();

        let operators: Vec<Box<dyn Mutation<f64>>> = vec![
            Box::new(NoMutation::new()),
            Box::new(GaussianMutation::default()),
        ];

        for operator in operators {
            let output = operator.mutate(
                &test.model,
                &iteration,
                &input,
                0.5,
                &mut SeededEntropy::new(7),
            );
            assert_eq!(output.len(), 2);
        }
    }
}
