//! 进阶交叉算子 / Advanced crossover operators
//!
//! SBX、差分交叉与排列交叉（OX / CX）。排列交叉只重排基因，不改变取值集合；SBX 与差分交叉按边界夹紧。
//! SBX, differential crossover, and permutation crossovers (OX / CX). Permutation crossovers only reorder genes and
//! keep the value multiset, while SBX and differential crossover clamp against the variable bounds.

use crate::model::callback::Solution;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, VariableModel, to_f64};
use crate::solver::heuristic::operator::{clamped_value, pick_two, unit};

use super::{SolutionUpdate, read_values, write_values};

/// 模拟二进制交叉（SBX），分布指数控制子代与父代的接近程度 / Simulated binary crossover (SBX) with a distribution index
#[derive(Debug, Clone, Copy)]
pub struct SimulatedBinaryCrossover {
    /// 分布指数 η，越大子代越接近父代 / Distribution index η; larger values keep children closer to the parents
    pub distribution_index: f64,
}

impl SimulatedBinaryCrossover {
    /// 创建 SBX / Create SBX
    pub fn new(distribution_index: f64) -> Self {
        Self {
            distribution_index,
        }
    }
}

impl Default for SimulatedBinaryCrossover {
    fn default() -> Self {
        Self::new(20.0)
    }
}

impl<V> SolutionUpdate<V> for SimulatedBinaryCrossover
where
    V: HeuristicValue,
{
    fn update(
        &self,
        model: &dyn VariableModel<V>,
        _iteration: &Iteration,
        parents: &[Solution<V>],
        entropy: &mut dyn Entropy,
    ) -> Vec<Solution<V>> {
        let Some((first, second)) = pick_two(parents) else {
            return Vec::new();
        };
        let left = read_values(model, first);
        let right = read_values(model, second);
        if left.len() != right.len() {
            return vec![first.clone(), second.clone()];
        }

        let eta = self.distribution_index.max(0.0);
        let exponent = 1.0 / (eta + 1.0);
        let mut first_child = Vec::with_capacity(left.len());
        let mut second_child = Vec::with_capacity(left.len());
        for index in 0..left.len() {
            let a = to_f64(&left[index]);
            let b = to_f64(&right[index]);
            let (lower, upper) = model.bounds_at(index);

            if (a - b).abs() < 1.0e-12 {
                first_child.push(clamped_value(a, lower, upper));
                second_child.push(clamped_value(b, lower, upper));
                continue;
            }

            let random = unit(entropy);
            let beta = if random <= 0.5 {
                (2.0 * random).powf(exponent)
            } else {
                (1.0 / (2.0 * (1.0 - random))).powf(exponent)
            };
            let child_a = 0.5 * ((1.0 + beta) * a + (1.0 - beta) * b);
            let child_b = 0.5 * ((1.0 - beta) * a + (1.0 + beta) * b);
            first_child.push(clamped_value(child_a, lower, upper));
            second_child.push(clamped_value(child_b, lower, upper));
        }
        vec![
            write_values(model, first_child),
            write_values(model, second_child),
        ]
    }
}

/// 差分交叉（DE/rand/1/bin），用种群差向量构造变异体再做二项式交叉 / Differential crossover (DE/rand/1/bin)
///
/// 父代约定为 `[target, a, b]`：变异体 = a + F × (b − c)，其中 c 缺省时复用 b。
/// Parents are expected as `[target, a, b]`: the mutant is a + F × (b − c), where c reuses b when absent.
#[derive(Debug, Clone, Copy)]
pub struct DifferentialCrossover {
    /// 缩放因子 F / Scaling factor F
    pub differential_weight: f64,
    /// 交叉概率 CR / Crossover rate CR
    pub cross_rate: f64,
}

impl DifferentialCrossover {
    /// 创建差分交叉 / Create a differential crossover
    pub fn new(differential_weight: f64, cross_rate: f64) -> Self {
        Self {
            differential_weight,
            cross_rate,
        }
    }
}

impl Default for DifferentialCrossover {
    fn default() -> Self {
        Self::new(0.5, 0.9)
    }
}

impl<V> SolutionUpdate<V> for DifferentialCrossover
where
    V: HeuristicValue,
{
    fn update(
        &self,
        model: &dyn VariableModel<V>,
        _iteration: &Iteration,
        parents: &[Solution<V>],
        entropy: &mut dyn Entropy,
    ) -> Vec<Solution<V>> {
        if parents.len() < 2 {
            return parents.to_vec();
        }

        let target = read_values(model, &parents[0]);
        let base = read_values(model, &parents[1]);
        let first_difference = read_values(model, &parents[2 % parents.len()]);
        let second_difference = read_values(model, &parents[3 % parents.len()]);
        if base.len() != first_difference.len()
            || base.len() != second_difference.len()
            || base.len() != target.len()
        {
            return vec![parents[0].clone()];
        }

        let weight = self.differential_weight;
        let rate = self.cross_rate;
        let mut child = Vec::with_capacity(target.len());
        for index in 0..target.len() {
            let (lower, upper) = model.bounds_at(index);
            let mutant = to_f64(&base[index])
                + weight * (to_f64(&first_difference[index]) - to_f64(&second_difference[index]));
            let value = if unit(entropy) < rate {
                mutant
            } else {
                to_f64(&target[index])
            };
            child.push(clamped_value(value, lower, upper));
        }
        vec![write_values(model, child)]
    }
}

/// 单向顺序交叉 / One-directional order crossover
fn order_crossover_once<V: HeuristicValue>(
    segment_source: &[V],
    order_source: &[V],
    entropy: &mut dyn Entropy,
) -> Vec<V> {
    let size = segment_source.len();
    let left = ((unit(entropy) * size as f64) as usize).min(size - 1);
    let right = ((unit(entropy) * size as f64) as usize).min(size - 1);
    let start = left.min(right);
    let end = if start == left.max(right) {
        (start + 1).min(size)
    } else {
        left.max(right)
    };

    let mut child = order_source.to_vec();
    let mut used: Vec<V> = Vec::new();
    for index in start..end {
        child[index] = segment_source[index];
        if !used.contains(&segment_source[index]) {
            used.push(segment_source[index]);
        }
    }

    let free_positions: Vec<usize> = (0..size)
        .filter(|index| *index < start || *index >= end)
        .collect();
    let ordered_values: Vec<V> = (0..size)
        .map(|offset| order_source[(end + offset) % size])
        .filter(|value| !used.contains(value))
        .collect();
    let amount = free_positions.len().min(ordered_values.len());
    for index in 0..amount {
        child[free_positions[index]] = ordered_values[index];
    }
    child
}

/// 顺序交叉（OX），保留一个父代的片段，其余位置按另一个父代的相对顺序填充 / Order crossover (OX)
#[derive(Debug, Clone, Copy, Default)]
pub struct OrderCrossover;

impl OrderCrossover {
    /// 创建顺序交叉 / Create an order crossover
    pub fn new() -> Self {
        Self
    }
}

impl<V> SolutionUpdate<V> for OrderCrossover
where
    V: HeuristicValue,
{
    fn update(
        &self,
        model: &dyn VariableModel<V>,
        _iteration: &Iteration,
        parents: &[Solution<V>],
        entropy: &mut dyn Entropy,
    ) -> Vec<Solution<V>> {
        let Some((first, second)) = pick_two(parents) else {
            return Vec::new();
        };
        let left = read_values(model, first);
        let right = read_values(model, second);
        if left.len() != right.len() || left.is_empty() {
            return vec![first.clone(), second.clone()];
        }

        let first_child = order_crossover_once(&left, &right, entropy);
        let second_child = order_crossover_once(&right, &left, entropy);
        vec![
            write_values(model, first_child),
            write_values(model, second_child),
        ]
    }
}

/// 单向循环交叉 / One-directional cycle crossover
fn cycle_crossover_once<V: HeuristicValue>(cycle_source: &[V], other_source: &[V]) -> Vec<V> {
    let size = cycle_source.len();
    let mut child = other_source.to_vec();
    let mut assigned = vec![false; size];
    let mut index = 0;
    let mut guard = 0;
    while index < size && !assigned[index] && guard <= size {
        assigned[index] = true;
        child[index] = cycle_source[index];
        let next = other_source
            .iter()
            .position(|value| *value == cycle_source[index]);
        match next {
            Some(next) => {
                index = next;
                guard += 1;
            }
            None => break,
        }
    }
    child
}

/// 循环交叉（CX），沿父代映射形成的循环逐个赋值 / Cycle crossover (CX) assigning genes along the cycles formed by the parents
#[derive(Debug, Clone, Copy, Default)]
pub struct CycleCrossover;

impl CycleCrossover {
    /// 创建循环交叉 / Create a cycle crossover
    pub fn new() -> Self {
        Self
    }
}

impl<V> SolutionUpdate<V> for CycleCrossover
where
    V: HeuristicValue,
{
    fn update(
        &self,
        model: &dyn VariableModel<V>,
        _iteration: &Iteration,
        parents: &[Solution<V>],
        _entropy: &mut dyn Entropy,
    ) -> Vec<Solution<V>> {
        let Some((first, second)) = pick_two(parents) else {
            return Vec::new();
        };
        let left = read_values(model, first);
        let right = read_values(model, second);
        if left.len() != right.len() || left.is_empty() {
            return vec![first.clone(), second.clone()];
        }

        vec![
            write_values(model, cycle_crossover_once(&left, &right)),
            write_values(model, cycle_crossover_once(&right, &left)),
        ]
    }
}

#[cfg(test)]
mod tests {
    use crate::solver::heuristic::entropy::{FnEntropy, SeededEntropy};
    use crate::solver::heuristic::iteration::Iteration;

    use super::super::test_support::{assert_dimension_and_bounds, model_with_bounds, solution};
    use super::super::SolutionUpdate;
    use super::{
        CycleCrossover, DifferentialCrossover, OrderCrossover, SimulatedBinaryCrossover,
    };

    fn setup() -> (
        super::super::test_support::TestModel,
        Vec<crate::model::callback::Solution<f64>>,
    ) {
        let test = model_with_bounds(0.0, 10.0);
        let parents = vec![
            solution(&test, 1.0, 6.0),
            solution(&test, 5.0, 2.0),
            solution(&test, 9.0, 4.0),
            solution(&test, 3.0, 8.0),
        ];
        (test, parents)
    }

    fn sorted_values(solutions: &[crate::model::callback::Solution<f64>]) -> Vec<f64> {
        let mut values: Vec<f64> = solutions
            .iter()
            .flat_map(|solution| solution.values().copied())
            .collect();
        values.sort_by(f64::total_cmp);
        values
    }

    #[test]
    fn sbx_keeps_dimension_and_bounds() {
        let (test, parents) = setup();
        let children = SimulatedBinaryCrossover::new(20.0).update(
            &test.model,
            &Iteration::new(),
            &parents,
            &mut SeededEntropy::new(31),
        );

        assert_eq!(children.len(), 2);
        assert_dimension_and_bounds(&children, 0.0, 10.0);
    }

    #[test]
    fn differential_crossover_is_reproducible_for_the_same_seed() {
        let (test, parents) = setup();
        let iteration = Iteration::new();
        let operator = DifferentialCrossover::new(0.5, 0.9);

        let first = operator.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(41),
        );
        let second = operator.update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(41),
        );

        assert_eq!(first, second);
        assert_eq!(first.len(), 1);
        assert_dimension_and_bounds(&first, 0.0, 10.0);
    }

    #[test]
    fn differential_crossover_reuses_the_first_parent_when_it_is_alone() {
        let (test, parents) = setup();
        let single = vec![parents[0].clone()];

        let children = DifferentialCrossover::default().update(
            &test.model,
            &Iteration::new(),
            &single,
            &mut SeededEntropy::new(1),
        );
        assert_eq!(children, single);
    }

    #[test]
    fn permutation_crossovers_preserve_the_value_multiset() {
        let test = model_with_bounds(0.0, 10.0);
        // 排列编码下两个父代必须是同一取值集合的排列。
        // Under permutation encoding both parents must be permutations of the same value set.
        let parents = vec![solution(&test, 1.0, 2.0), solution(&test, 2.0, 1.0)];
        let expected = sorted_values(&parents);

        let order_children = OrderCrossover::new().update(
            &test.model,
            &Iteration::new(),
            &parents,
            &mut FnEntropy::new(|| 0.4),
        );
        assert_eq!(order_children.len(), 2);
        assert_eq!(sorted_values(&order_children), expected);

        let cycle_children = CycleCrossover::new().update(
            &test.model,
            &Iteration::new(),
            &parents,
            &mut FnEntropy::new(|| 0.4),
        );
        assert_eq!(cycle_children.len(), 2);
        assert_eq!(sorted_values(&cycle_children), expected);
    }
}
