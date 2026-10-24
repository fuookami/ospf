//! 动作选择算子 / Action selection operators

use crate::solver::heuristic::entropy::{Entropy, unit};

/// 动作选择契约：从一组动作价值中挑一个动作 / Action selection contract picking one action from a set of action values
pub trait ActionSelector {
    /// 选择动作 / Select an action
    fn select(&self, values: &[f64], entropy: &mut dyn Entropy) -> usize;
}

/// 取价值最大的动作 / Take the action with the largest value
#[derive(Debug, Clone, Copy, Default)]
pub struct GreedyActionSelector;

fn argmax(values: &[f64]) -> usize {
    if values.is_empty() {
        return 0;
    }
    let mut best = 0;
    let mut best_value = values[0];
    for (index, value) in values.iter().copied().enumerate().skip(1) {
        if value > best_value {
            best_value = value;
            best = index;
        }
    }
    best
}

impl ActionSelector for GreedyActionSelector {
    fn select(&self, values: &[f64], _entropy: &mut dyn Entropy) -> usize {
        argmax(values)
    }
}

/// ε-贪心选择：以概率 ε 均匀随机探索，否则取价值最大的动作 / ε-greedy selection exploring uniformly with probability ε
#[derive(Debug, Clone, Copy)]
pub struct EpsilonGreedyActionSelector {
    /// 探索概率 / Exploration probability
    pub epsilon: f64,
}

impl EpsilonGreedyActionSelector {
    /// 创建 ε-贪心选择 / Create an ε-greedy selector
    pub fn new(epsilon: f64) -> Self {
        Self { epsilon }
    }
}

impl Default for EpsilonGreedyActionSelector {
    fn default() -> Self {
        Self::new(0.1)
    }
}

impl ActionSelector for EpsilonGreedyActionSelector {
    fn select(&self, values: &[f64], entropy: &mut dyn Entropy) -> usize {
        if values.is_empty() {
            return 0;
        }
        let epsilon = self.epsilon.clamp(0.0, 1.0);
        if unit(entropy) < epsilon {
            // 探索：再取一个独立随机数，避免用同一个值既决定是否探索又决定探索位置。
            // Exploration: draw an independent sample instead of reusing the decision roll.
            let pick = unit(entropy);
            return ((pick * values.len() as f64) as usize).min(values.len() - 1);
        }
        argmax(values)
    }
}

#[cfg(test)]
mod tests {
    use crate::solver::heuristic::entropy::SeededEntropy;

    use super::*;

    #[test]
    fn greedy_always_picks_the_argmax() {
        let mut entropy = SeededEntropy::new(1);
        assert_eq!(GreedyActionSelector.select(&[1.0, 3.0, 2.0], &mut entropy), 1);
    }

    #[test]
    fn zero_epsilon_behaves_greedily() {
        let mut entropy = SeededEntropy::new(1);
        let selector = EpsilonGreedyActionSelector::new(0.0);
        assert_eq!(selector.select(&[1.0, 3.0, 2.0], &mut entropy), 1);
    }

    #[test]
    fn full_epsilon_stays_in_range() {
        let mut entropy = SeededEntropy::new(5);
        let selector = EpsilonGreedyActionSelector::new(1.0);
        for _ in 0..32 {
            assert!(selector.select(&[0.0; 4], &mut entropy) < 4);
        }
    }
}
