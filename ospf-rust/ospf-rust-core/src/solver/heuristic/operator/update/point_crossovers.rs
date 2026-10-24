//! 经典交叉算子 / Classic crossover operators
//!
//! 点交叉、均匀交叉与算术/混合交叉都在规范顺序上按位置交换基因，因此与解的 `HashMap` 表示解耦。
//! Point, uniform, arithmetic and blend crossovers exchange genes by position on the canonical order, which keeps them
//! independent of the `HashMap` solution representation.

use crate::model::callback::Solution;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, VariableModel, to_f64};
use crate::solver::heuristic::operator::{clamped_value, pick_two, unit};

use super::{SolutionUpdate, read_values, write_values};

/// 单点交叉，随机选一个切点后交换尾部 / Single-point crossover swapping the tails after one random cut
#[derive(Debug, Clone, Copy, Default)]
pub struct SinglePointCrossover;

impl SinglePointCrossover {
    /// 创建单点交叉 / Create a single-point crossover
    pub fn new() -> Self {
        Self
    }
}

impl<V> SolutionUpdate<V> for SinglePointCrossover
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
        let mut left = read_values(model, first);
        let mut right = read_values(model, second);
        if left.len() != right.len() || left.len() < 2 {
            return vec![first.clone(), second.clone()];
        }

        let size = left.len();
        let offset = ((unit(entropy) * (size - 1) as f64) as usize).min(size - 2);
        let cut = 1 + offset;
        for index in cut..size {
            std::mem::swap(&mut left[index], &mut right[index]);
        }
        vec![write_values(model, left), write_values(model, right)]
    }
}

/// 两点交叉，交换两个切点之间的片段 / Two-point crossover swapping the segment between two cuts
#[derive(Debug, Clone, Copy, Default)]
pub struct TwoPointCrossover;

impl TwoPointCrossover {
    /// 创建两点交叉 / Create a two-point crossover
    pub fn new() -> Self {
        Self
    }
}

impl<V> SolutionUpdate<V> for TwoPointCrossover
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
        let mut left = read_values(model, first);
        let mut right = read_values(model, second);
        if left.len() != right.len() || left.len() < 2 {
            return vec![first.clone(), second.clone()];
        }

        let size = left.len();
        let left_cut = ((unit(entropy) * size as f64) as usize).min(size - 1);
        let right_cut = ((unit(entropy) * size as f64) as usize).min(size - 1);
        let start = left_cut.min(right_cut);
        let end = if start == left_cut.max(right_cut) {
            (start + 1).min(size - 1)
        } else {
            left_cut.max(right_cut)
        };
        for index in start..end {
            std::mem::swap(&mut left[index], &mut right[index]);
        }
        vec![write_values(model, left), write_values(model, right)]
    }
}

/// 多点交叉，随机选若干切点后交错交换片段 / Multi-point crossover swapping alternating segments split by several cuts
#[derive(Debug, Clone, Copy)]
pub struct MultiPointCrossover {
    /// 切点数量 / Number of cuts
    pub cut_amount: usize,
}

impl MultiPointCrossover {
    /// 创建多点交叉 / Create a multi-point crossover
    pub fn new(cut_amount: usize) -> Self {
        Self { cut_amount }
    }
}

impl Default for MultiPointCrossover {
    fn default() -> Self {
        Self::new(3)
    }
}

impl<V> SolutionUpdate<V> for MultiPointCrossover
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
        let mut left = read_values(model, first);
        let mut right = read_values(model, second);
        if left.len() != right.len() || left.len() < 2 {
            return vec![first.clone(), second.clone()];
        }

        let size = left.len();
        let amount = self.cut_amount.clamp(1, size - 1);
        let mut cuts: Vec<usize> = Vec::with_capacity(amount);
        let mut attempts = 0;
        while cuts.len() < amount && attempts < amount * 8 {
            let cut = ((unit(entropy) * size as f64) as usize).clamp(1, size - 1);
            if !cuts.contains(&cut) {
                cuts.push(cut);
            }
            attempts += 1;
        }
        cuts.sort_unstable();

        let mut swapping = false;
        let mut cursor = 0;
        for cut in cuts {
            if swapping {
                for index in cursor..cut {
                    std::mem::swap(&mut left[index], &mut right[index]);
                }
            }
            swapping = !swapping;
            cursor = cut;
        }
        if swapping {
            for index in cursor..size {
                std::mem::swap(&mut left[index], &mut right[index]);
            }
        }
        vec![write_values(model, left), write_values(model, right)]
    }
}

/// 均匀交叉，逐基因独立抛硬币决定是否交换 / Uniform crossover deciding every gene independently with a coin flip
#[derive(Debug, Clone, Copy)]
pub struct UniformCrossover {
    /// 交换概率 / Exchange probability
    pub mixing_ratio: f64,
}

impl UniformCrossover {
    /// 创建均匀交叉 / Create a uniform crossover
    pub fn new(mixing_ratio: f64) -> Self {
        Self { mixing_ratio }
    }
}

impl Default for UniformCrossover {
    fn default() -> Self {
        Self::new(0.5)
    }
}

impl<V> SolutionUpdate<V> for UniformCrossover
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
        let mut left = read_values(model, first);
        let mut right = read_values(model, second);
        if left.len() != right.len() {
            return vec![first.clone(), second.clone()];
        }

        let ratio = self.mixing_ratio;
        for index in 0..left.len() {
            if unit(entropy) < ratio {
                std::mem::swap(&mut left[index], &mut right[index]);
            }
        }
        vec![write_values(model, left), write_values(model, right)]
    }
}

/// 算术交叉，两个子代是父代的互补加权平均 / Arithmetic crossover whose children are complementary weighted averages
#[derive(Debug, Clone, Copy, Default)]
pub struct ArithmeticCrossover;

impl ArithmeticCrossover {
    /// 创建算术交叉 / Create an arithmetic crossover
    pub fn new() -> Self {
        Self
    }
}

impl<V> SolutionUpdate<V> for ArithmeticCrossover
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

        let alpha = unit(entropy);
        let mut first_child = Vec::with_capacity(left.len());
        let mut second_child = Vec::with_capacity(left.len());
        for index in 0..left.len() {
            let a = to_f64(&left[index]);
            let b = to_f64(&right[index]);
            let (lower, upper) = model.bounds_at(index);
            first_child.push(clamped_value(alpha * a + (1.0 - alpha) * b, lower, upper));
            second_child.push(clamped_value(alpha * b + (1.0 - alpha) * a, lower, upper));
        }
        vec![
            write_values(model, first_child),
            write_values(model, second_child),
        ]
    }
}

/// 混合交叉（BLX-α），在父代张成的扩展区间内均匀采样 / Blend crossover (BLX-α) sampling inside the expanded parent interval
#[derive(Debug, Clone, Copy)]
pub struct BlendCrossover {
    /// 扩展系数 α / Expansion coefficient α
    pub expansion: f64,
}

impl BlendCrossover {
    /// 创建混合交叉 / Create a blend crossover
    pub fn new(expansion: f64) -> Self {
        Self { expansion }
    }
}

impl Default for BlendCrossover {
    fn default() -> Self {
        Self::new(0.5)
    }
}

impl<V> SolutionUpdate<V> for BlendCrossover
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

        let mut first_child = Vec::with_capacity(left.len());
        let mut second_child = Vec::with_capacity(left.len());
        for index in 0..left.len() {
            let a = to_f64(&left[index]);
            let b = to_f64(&right[index]);
            let spread = (a - b).abs() * self.expansion;
            let low = a.min(b) - spread;
            let high = a.max(b) + spread;
            let (lower, upper) = model.bounds_at(index);
            first_child.push(clamped_value(low + unit(entropy) * (high - low), lower, upper));
            second_child.push(clamped_value(low + unit(entropy) * (high - low), lower, upper));
        }
        vec![
            write_values(model, first_child),
            write_values(model, second_child),
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
        ArithmeticCrossover, BlendCrossover, MultiPointCrossover, SinglePointCrossover,
        TwoPointCrossover, UniformCrossover,
    };

    fn parents() -> (
        super::super::test_support::TestModel,
        Vec<crate::model::callback::Solution<f64>>,
    ) {
        let test = model_with_bounds(0.0, 10.0);
        let parents = vec![solution(&test, 1.0, 2.0), solution(&test, 7.0, 8.0)];
        (test, parents)
    }

    #[test]
    fn point_crossovers_keep_dimension_and_bounds() {
        let (test, parents) = parents();
        let iteration = Iteration::new();

        let mut entropy = SeededEntropy::new(21);
        let children = SinglePointCrossover::new().update(
            &test.model,
            &iteration,
            &parents,
            &mut entropy,
        );
        assert_eq!(children.len(), 2);
        assert_dimension_and_bounds(&children, 0.0, 10.0);

        let mut entropy = SeededEntropy::new(22);
        let children = TwoPointCrossover::new().update(
            &test.model,
            &iteration,
            &parents,
            &mut entropy,
        );
        assert_eq!(children.len(), 2);
        assert_dimension_and_bounds(&children, 0.0, 10.0);

        let mut entropy = SeededEntropy::new(23);
        let children = MultiPointCrossover::new(2).update(
            &test.model,
            &iteration,
            &parents,
            &mut entropy,
        );
        assert_eq!(children.len(), 2);
        assert_dimension_and_bounds(&children, 0.0, 10.0);

        let mut entropy = SeededEntropy::new(24);
        let children = UniformCrossover::new(0.5).update(
            &test.model,
            &iteration,
            &parents,
            &mut entropy,
        );
        assert_eq!(children.len(), 2);
        assert_dimension_and_bounds(&children, 0.0, 10.0);

        let mut entropy = SeededEntropy::new(25);
        let children = ArithmeticCrossover::new().update(
            &test.model,
            &iteration,
            &parents,
            &mut entropy,
        );
        assert_eq!(children.len(), 2);
        assert_dimension_and_bounds(&children, 0.0, 10.0);

        let mut entropy = SeededEntropy::new(26);
        let children = BlendCrossover::new(0.5).update(
            &test.model,
            &iteration,
            &parents,
            &mut entropy,
        );
        assert_eq!(children.len(), 2);
        assert_dimension_and_bounds(&children, 0.0, 10.0);
    }

    #[test]
    fn single_point_crossover_is_reproducible_for_the_same_seed() {
        let (test, parents) = parents();
        let iteration = Iteration::new();

        let first = SinglePointCrossover::new().update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(77),
        );
        let second = SinglePointCrossover::new().update(
            &test.model,
            &iteration,
            &parents,
            &mut SeededEntropy::new(77),
        );
        assert_eq!(first, second);
    }

    #[test]
    fn arithmetic_crossover_averages_parents_at_half_alpha() {
        let (test, parents) = parents();
        let children = ArithmeticCrossover::new().update(
            &test.model,
            &Iteration::new(),
            &parents,
            &mut FnEntropy::new(|| 0.5),
        );

        let mut values: Vec<f64> = children
            .iter()
            .flat_map(|child| child.values().copied())
            .collect();
        values.sort_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap_or(std::cmp::Ordering::Equal));
        assert_eq!(values, vec![4.0, 4.0, 5.0, 5.0]);
    }

    #[test]
    fn uniform_crossover_with_full_ratio_swaps_every_gene() {
        let (test, parents) = parents();
        let children = UniformCrossover::new(1.0).update(
            &test.model,
            &Iteration::new(),
            &parents,
            &mut FnEntropy::new(|| 0.0),
        );

        let first = &children[0];
        let second = &children[1];
        assert_eq!(first.get(&test.x).copied(), Some(7.0));
        assert_eq!(first.get(&test.y).copied(), Some(8.0));
        assert_eq!(second.get(&test.x).copied(), Some(1.0));
        assert_eq!(second.get(&test.y).copied(), Some(2.0));
    }
}
