//! 支配关系 / Dominance relations

/// 支配关系契约，多目标优化的最底层判据 / Dominance contract, the bottom-most predicate of multi-objective optimization
///
/// 单目标优化用全序比较任意两解；多目标优化只有偏序，而偏序的全部内容就是「谁支配谁」。
/// Single-objective optimization compares any two solutions with a total order; multi-objective optimization only has a
/// partial order, whose entire content is "who dominates whom".
pub trait Dominance {
    /// 判断左目标向量是否支配右目标向量 / Decide whether the left objective vector dominates the right one
    fn dominates(&self, lhs: &[f64], rhs: &[f64]) -> bool;
}

/// Pareto 支配：所有目标都不差，且至少一个目标严格更好 / Pareto dominance: no objective is worse and at least one is strictly better
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParetoDominance {
    /// 每个目标的优化方向，`true` 表示最小化 / Optimization direction per objective, `true` means minimisation
    pub directions: Vec<bool>,
}

impl ParetoDominance {
    /// 创建 Pareto 支配 / Create a Pareto dominance relation
    pub fn new(directions: Vec<bool>) -> Self {
        Self { directions }
    }

    /// 全部最小化 / Minimise every objective
    pub fn minimization(objective_amount: usize) -> Self {
        Self::new(vec![true; objective_amount])
    }
}

impl Dominance for ParetoDominance {
    fn dominates(&self, lhs: &[f64], rhs: &[f64]) -> bool {
        let mut strictly_better = false;
        for index in 0..self.directions.len() {
            let (Some(left), Some(right)) = (lhs.get(index), rhs.get(index)) else {
                return false;
            };
            let (better, worse) = if self.directions[index] {
                (*left < *right, *left > *right)
            } else {
                (*left > *right, *left < *right)
            };
            if worse {
                return false;
            }
            if better {
                strictly_better = true;
            }
        }
        strictly_better
    }
}

/// 全最小化的 Pareto 支配，是 [`ParetoDominance`] 的便捷形态 / Pareto dominance minimising every objective
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinimizationDominance {
    delegate: ParetoDominance,
}

impl MinimizationDominance {
    /// 创建全最小化支配 / Create an all-minimisation dominance relation
    pub fn new(objective_amount: usize) -> Self {
        Self {
            delegate: ParetoDominance::minimization(objective_amount),
        }
    }
}

impl Dominance for MinimizationDominance {
    fn dominates(&self, lhs: &[f64], rhs: &[f64]) -> bool {
        self.delegate.dominates(lhs, rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pareto_dominance_requires_no_worse_and_one_better() {
        let dominance = ParetoDominance::minimization(2);
        assert!(dominance.dominates(&[1.0, 2.0], &[2.0, 3.0]));
        assert!(!dominance.dominates(&[1.0, 2.0], &[1.0, 2.0]));
        assert!(!dominance.dominates(&[1.0, 4.0], &[2.0, 3.0]));
        assert!(!dominance.dominates(&[2.0, 3.0], &[1.0, 2.0]));
    }

    #[test]
    fn maximisation_flips_the_direction() {
        let dominance = ParetoDominance::new(vec![false]);
        assert!(dominance.dominates(&[3.0], &[2.0]));
        assert!(!dominance.dominates(&[2.0], &[3.0]));
    }
}
