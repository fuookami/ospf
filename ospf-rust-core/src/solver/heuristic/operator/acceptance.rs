//! 接受判据算子 / Acceptance criterion operators

use super::compare_individuals;
use super::evolution::{GeometricSource, ParameterSource};
use crate::solver::heuristic::entropy::{Entropy, unit};
use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, ObjectiveModel, to_f64};

/// 接受判据契约 / Acceptance criterion contract
///
/// 决定候选解是否取代当前解。梯度下降、爬山、模拟退火、禁忌搜索的区别只在这一个算子。
/// Decides whether a candidate replaces the current solution. Hill climbing, simulated annealing, and tabu search differ
/// only in this operator.
pub trait Acceptance<O, V>
where
    V: HeuristicValue,
{
    /// 判断是否接受候选解 / Decide whether to accept the candidate
    fn accept(
        &mut self,
        model: &dyn ObjectiveModel<O, V>,
        iteration: &Iteration,
        current: &Individual<O, V>,
        candidate: &Individual<O, V>,
        entropy: &mut dyn Entropy,
    ) -> bool;
}

/// 严格改进才接受，等价于爬山法 / Accept only on strict improvement, i.e. hill climbing
#[derive(Debug, Clone, Copy, Default)]
pub struct BetterOnlyAcceptance;

impl<O, V> Acceptance<O, V> for BetterOnlyAcceptance
where
    V: HeuristicValue,
{
    fn accept(
        &mut self,
        model: &dyn ObjectiveModel<O, V>,
        _iteration: &Iteration,
        current: &Individual<O, V>,
        candidate: &Individual<O, V>,
        _entropy: &mut dyn Entropy,
    ) -> bool {
        compare_individuals(model, &candidate, &current) == std::cmp::Ordering::Less
    }
}

/// 不劣于当前解即接受，允许在平台上横向移动 / Accept anything not worse, allowing lateral moves across plateaus
#[derive(Debug, Clone, Copy, Default)]
pub struct NotWorseAcceptance;

impl<O, V> Acceptance<O, V> for NotWorseAcceptance
where
    V: HeuristicValue,
{
    fn accept(
        &mut self,
        model: &dyn ObjectiveModel<O, V>,
        _iteration: &Iteration,
        current: &Individual<O, V>,
        candidate: &Individual<O, V>,
        _entropy: &mut dyn Entropy,
    ) -> bool {
        compare_individuals(model, &candidate, &current)
            != std::cmp::Ordering::Greater
    }
}

/// 无条件接受，用于纯随机游走 / Accept unconditionally, used for pure random walks
#[derive(Debug, Clone, Copy, Default)]
pub struct AlwaysAcceptance;

impl<O, V> Acceptance<O, V> for AlwaysAcceptance
where
    V: HeuristicValue,
{
    fn accept(
        &mut self,
        model: &dyn ObjectiveModel<O, V>,
        _iteration: &Iteration,
        current: &Individual<O, V>,
        candidate: &Individual<O, V>,
        _entropy: &mut dyn Entropy,
    ) -> bool {
        model.compare_constraint_tiers(&candidate.solution, &current.solution) != std::cmp::Ordering::Greater
    }
}

/// Metropolis 接受判据，模拟退火的核心 / Metropolis acceptance criterion, the core of simulated annealing
///
/// 更优解无条件接受；更差的解以 `exp(-Δ/T)` 的概率接受，因此高温期能翻越局部最优、低温期趋于纯爬山。
/// Better solutions are accepted unconditionally; worse ones with probability `exp(-Δ/T)`, so a high temperature escapes
/// local optima while a low temperature degenerates into plain hill climbing.
pub struct MetropolisAcceptance {
    temperature: Box<dyn ParameterSource>,
}

impl MetropolisAcceptance {
    /// 以温度演化源创建 / Create from a temperature evolution source
    pub fn new(temperature: Box<dyn ParameterSource>) -> Self {
        Self { temperature }
    }
}

impl Default for MetropolisAcceptance {
    fn default() -> Self {
        Self::new(Box::new(GeometricSource::new(100.0, 0.95)))
    }
}

impl<V> Acceptance<V, V> for MetropolisAcceptance
where
    V: HeuristicValue,
{
    fn accept(
        &mut self,
        model: &dyn ObjectiveModel<V, V>,
        iteration: &Iteration,
        current: &Individual<V, V>,
        candidate: &Individual<V, V>,
        entropy: &mut dyn Entropy,
    ) -> bool {
        let tier = model.compare_constraint_tiers(&candidate.solution, &current.solution);
        if tier != std::cmp::Ordering::Equal { return tier == std::cmp::Ordering::Less; }
        if compare_individuals(model, &candidate, &current)
            == std::cmp::Ordering::Less
        {
            return true;
        }

        let candidate_value = to_f64(&candidate.fitness);
        let current_value = to_f64(&current.fitness);
        let temperature = self.temperature.value(iteration, entropy);
        if !temperature.is_finite() || temperature <= 1.0e-12 {
            return false;
        }

        let delta = candidate_value - current_value;
        if delta.is_nan() {
            return false;
        }
        if delta <= 0.0 {
            return true;
        }
        unit(entropy) < (-delta / temperature).exp()
    }
}
