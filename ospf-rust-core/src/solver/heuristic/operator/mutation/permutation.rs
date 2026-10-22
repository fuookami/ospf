//! 排列变异算子 / Permutation mutation operators
//!
//! 排列变异只重排基因，不改变取值集合。散射变异需要打乱片段顺序：基线使用全局随机源 `shuffled()`，而本实现
//! 使用调用方注入的熵源做 Fisher–Yates 洗牌，从而与其它算子共享同一可复现随机流。
//! Permutation mutations only reorder genes and keep the value multiset. Scramble mutation shuffles a segment: the
//! baseline used the global `shuffled()` source, whereas this implementation runs Fisher–Yates on the injected entropy
//! so it shares the same reproducible stream as every other operator.

use crate::model::callback::Solution;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, VariableModel};
use crate::solver::heuristic::operator::update::{read_values, write_values};
use crate::solver::heuristic::operator::{should_fire, unit};

use super::Mutation;

/// 在解上取两个不同的随机下标 / Draw two distinct random indices on a solution
fn two_indices(size: usize, entropy: &mut dyn Entropy) -> (usize, usize) {
    let first = ((unit(entropy) * size as f64) as usize).min(size - 1);
    let mut second = ((unit(entropy) * size as f64) as usize).min(size - 1);
    if second == first {
        second = (first + 1) % size;
    }
    (first, second)
}

/// Fisher–Yates 洗牌 / Fisher–Yates shuffle
fn shuffle<V>(values: &mut [V], entropy: &mut dyn Entropy) {
    for index in (1..values.len()).rev() {
        let swap_with = ((unit(entropy) * (index + 1) as f64) as usize).min(index);
        values.swap(index, swap_with);
    }
}

/// 交换变异，交换两个基因 / Swap mutation exchanging two genes
#[derive(Debug, Clone, Copy, Default)]
pub struct SwapMutation;

impl SwapMutation {
    /// 创建交换变异 / Create a swap mutation
    pub fn new() -> Self {
        Self
    }
}

impl<V> Mutation<V> for SwapMutation
where
    V: HeuristicValue,
{
    fn mutate(
        &self,
        model: &dyn VariableModel<V>,
        _iteration: &Iteration,
        solution: &Solution<V>,
        mutation_rate: f64,
        entropy: &mut dyn Entropy,
    ) -> Solution<V> {
        let mut values = read_values(model, solution);
        if values.len() < 2 || !should_fire(entropy, mutation_rate) {
            return solution.clone();
        }

        let (first, second) = two_indices(values.len(), entropy);
        values.swap(first, second);
        write_values(model, values)
    }
}

/// 逆序变异，反转两个切点之间的片段 / Inversion mutation reversing the segment between two cuts
#[derive(Debug, Clone, Copy, Default)]
pub struct InversionMutation;

impl InversionMutation {
    /// 创建逆序变异 / Create an inversion mutation
    pub fn new() -> Self {
        Self
    }
}

impl<V> Mutation<V> for InversionMutation
where
    V: HeuristicValue,
{
    fn mutate(
        &self,
        model: &dyn VariableModel<V>,
        _iteration: &Iteration,
        solution: &Solution<V>,
        mutation_rate: f64,
        entropy: &mut dyn Entropy,
    ) -> Solution<V> {
        let mut values = read_values(model, solution);
        if values.len() < 2 || !should_fire(entropy, mutation_rate) {
            return solution.clone();
        }

        let (first, second) = two_indices(values.len(), entropy);
        let start = first.min(second);
        let end = first.max(second);
        values[start..end].reverse();
        write_values(model, values)
    }
}

/// 插入变异，取出一个基因并插入到另一个位置 / Insertion mutation removing one gene and reinserting it elsewhere
#[derive(Debug, Clone, Copy, Default)]
pub struct InsertionMutation;

impl InsertionMutation {
    /// 创建插入变异 / Create an insertion mutation
    pub fn new() -> Self {
        Self
    }
}

impl<V> Mutation<V> for InsertionMutation
where
    V: HeuristicValue,
{
    fn mutate(
        &self,
        model: &dyn VariableModel<V>,
        _iteration: &Iteration,
        solution: &Solution<V>,
        mutation_rate: f64,
        entropy: &mut dyn Entropy,
    ) -> Solution<V> {
        let mut values = read_values(model, solution);
        if values.len() < 2 || !should_fire(entropy, mutation_rate) {
            return solution.clone();
        }

        let (from, to) = two_indices(values.len(), entropy);
        let value = values.remove(from);
        let target = to.min(values.len());
        values.insert(target, value);
        write_values(model, values)
    }
}

/// 散射变异，打乱一段随机片段内的基因顺序 / Scramble mutation shuffling the genes inside a random segment
#[derive(Debug, Clone, Copy)]
pub struct ScrambleMutation {
    /// 片段长度占比 / Segment length as a ratio of the solution length
    pub segment_ratio: f64,
}

impl ScrambleMutation {
    /// 创建散射变异 / Create a scramble mutation
    pub fn new(segment_ratio: f64) -> Self {
        Self { segment_ratio }
    }
}

impl Default for ScrambleMutation {
    fn default() -> Self {
        Self::new(0.2)
    }
}

impl<V> Mutation<V> for ScrambleMutation
where
    V: HeuristicValue,
{
    fn mutate(
        &self,
        model: &dyn VariableModel<V>,
        _iteration: &Iteration,
        solution: &Solution<V>,
        mutation_rate: f64,
        entropy: &mut dyn Entropy,
    ) -> Solution<V> {
        let mut values = read_values(model, solution);
        if values.len() < 2 || !should_fire(entropy, mutation_rate) {
            return solution.clone();
        }

        let size = values.len();
        let (first, second) = two_indices(size, entropy);
        let start = first.min(second);
        let mut end = first.max(second);
        let segment_length = ((size as f64 * self.segment_ratio) as usize).max(2);
        let minimum_end = (start + segment_length).min(size);
        end = end.max(minimum_end).min(size);
        shuffle(&mut values[start..end], entropy);
        write_values(model, values)
    }
}

#[cfg(test)]
mod tests {
    use crate::solver::heuristic::entropy::SeededEntropy;
    use crate::solver::heuristic::iteration::Iteration;

    use super::super::test_support::{model_with_bounds, solution};
    use super::super::Mutation;
    use super::{InsertionMutation, InversionMutation, ScrambleMutation, SwapMutation};

    fn sorted_values(solution: &crate::model::callback::Solution<f64>) -> Vec<f64> {
        let mut values: Vec<f64> = solution.values().copied().collect();
        values.sort_by(f64::total_cmp);
        values
    }

    #[test]
    fn zero_mutation_rate_returns_the_input_for_every_permutation_operator() {
        let test = model_with_bounds(0.0, 10.0);
        let input = solution(&test, 4.0, 7.0);
        let iteration = Iteration::new();

        let operators: Vec<Box<dyn Mutation<f64>>> = vec![
            Box::new(SwapMutation::new()),
            Box::new(InversionMutation::new()),
            Box::new(InsertionMutation::new()),
            Box::new(ScrambleMutation::default()),
        ];

        for operator in operators {
            let output = operator.mutate(
                &test.model,
                &iteration,
                &input,
                0.0,
                &mut SeededEntropy::new(211),
            );
            assert_eq!(output, input);
        }
    }

    #[test]
    fn permutation_mutations_preserve_the_value_multiset() {
        let test = model_with_bounds(0.0, 10.0);
        let input = solution(&test, 4.0, 7.0);
        let expected = sorted_values(&input);
        let iteration = Iteration::new();

        let operators: Vec<Box<dyn Mutation<f64>>> = vec![
            Box::new(SwapMutation::new()),
            Box::new(InversionMutation::new()),
            Box::new(InsertionMutation::new()),
            Box::new(ScrambleMutation::new(1.0)),
        ];

        for operator in operators {
            let output = operator.mutate(
                &test.model,
                &iteration,
                &input,
                1.0,
                &mut SeededEntropy::new(221),
            );
            assert_eq!(sorted_values(&output), expected);
        }
    }

    #[test]
    fn permutation_mutations_are_reproducible_for_the_same_seed() {
        let test = model_with_bounds(0.0, 10.0);
        let input = solution(&test, 4.0, 7.0);
        let iteration = Iteration::new();
        let operator = ScrambleMutation::new(1.0);

        let first = operator.mutate(
            &test.model,
            &iteration,
            &input,
            1.0,
            &mut SeededEntropy::new(231),
        );
        let second = operator.mutate(
            &test.model,
            &iteration,
            &input,
            1.0,
            &mut SeededEntropy::new(231),
        );
        assert_eq!(first, second);
    }
}
