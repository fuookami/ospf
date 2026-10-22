//! 通过显式计费账本执行局部搜索与组合更新 / Local search and composite updates through an explicit metered ledger

use std::cmp::Ordering;
use crate::model::callback::Solution;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{EvaluatedSolution, HeuristicValue, LocalEvaluation, VariableModel, to_f64};
use crate::solver::heuristic::operator::mutation::Mutation;
use crate::solver::heuristic::operator::neighborhood::Neighborhood;
use super::{SolutionUpdate, clamp_at};

/// 把变异提升为更新阶段 / Lift mutation into an update stage
pub struct MutationUpdate<V: HeuristicValue> {
    mutation: Box<dyn Mutation<V>>,
    rate: f64,
}

impl<V: HeuristicValue> MutationUpdate<V> {
    /// 创建阶段，概率超出[0,1]时调用不产生候选 / Create a stage; calls produce no candidates for rates outside [0,1]
    pub fn new(mutation: Box<dyn Mutation<V>>, rate: f64) -> Self { Self { mutation, rate } }
}

impl<V: HeuristicValue> SolutionUpdate<V> for MutationUpdate<V> {
    fn update(
        &self,
        model: &dyn VariableModel<V>,
        iteration: &Iteration,
        parents: &[Solution<V>],
        entropy: &mut dyn Entropy,
    ) -> Vec<Solution<V>> {
        if !self.rate.is_finite() || !(0.0..=1.0).contains(&self.rate) { return Vec::new(); }
        parents.iter().map(|parent| self.mutation.mutate(model, iteration, parent, self.rate, entropy)).collect()
    }
}

/// 把单点邻域提升为种群动作 / Lift a single-point neighborhood into a population action
pub struct NeighborhoodUpdate<V: HeuristicValue> {
    neighborhood: Box<dyn Neighborhood<V>>,
}

impl<V: HeuristicValue> NeighborhoodUpdate<V> {
    /// 创建邻域阶段 / Create a neighborhood stage
    pub fn new(neighborhood: Box<dyn Neighborhood<V>>) -> Self { Self { neighborhood } }
}

impl<V: HeuristicValue> SolutionUpdate<V> for NeighborhoodUpdate<V> {
    fn update(
        &self,
        model: &dyn VariableModel<V>,
        iteration: &Iteration,
        parents: &[Solution<V>],
        entropy: &mut dyn Entropy,
    ) -> Vec<Solution<V>> {
        parents.iter().map(|parent| self.neighborhood.neighbor(model, iteration, parent, entropy)).collect()
    }
}

/// 仅反射具有有限上下界的维度 / Reflect only dimensions with finite lower and upper bounds
#[derive(Default)]
pub struct OppositionUpdate;

impl<V: HeuristicValue> SolutionUpdate<V> for OppositionUpdate {
    fn update(
        &self,
        model: &dyn VariableModel<V>,
        _iteration: &Iteration,
        parents: &[Solution<V>],
        _entropy: &mut dyn Entropy,
    ) -> Vec<Solution<V>> {
        let order = model.variable_order();
        parents.iter().map(|parent| {
            let mut result = parent.clone();
            for (index, id) in order.iter().enumerate() {
                if let (Some(lower), Some(upper), Some(value)) = (model.lower_bound(index), model.upper_bound(index), parent.get(id)) {
                    if lower.is_finite() && upper.is_finite() {
                        result.insert(*id, clamp_at(model, index, lower + (upper - to_f64(value))));
                    }
                }
            }
            result
        }).collect()
    }
}

/// 有界Hooke-Jeeves坐标与模式精调 / Bounded Hooke-Jeeves coordinate polling and pattern intensification
pub struct PatternSearchUpdate {
    /// 初始相对步长 / Initial relative step
    pub initial_step: f64,
    /// 无改善时收缩率 / Contraction without improvement
    pub contraction: f64,
    /// 每次调用最多轮数 / Maximum sweeps per invocation
    pub sweeps: usize,
}

impl Default for PatternSearchUpdate {
    fn default() -> Self { Self { initial_step: 0.1, contraction: 0.5, sweeps: 32 } }
}

impl<V: HeuristicValue> SolutionUpdate<V> for PatternSearchUpdate {
    fn update(
        &self,
        model: &dyn VariableModel<V>,
        _iteration: &Iteration,
        parents: &[Solution<V>],
        _entropy: &mut dyn Entropy,
    ) -> Vec<Solution<V>> {
        if !self.initial_step.is_finite() || self.initial_step <= 0.0
            || !self.contraction.is_finite() || !(0.0..1.0).contains(&self.contraction)
            || self.contraction == 0.0 || self.sweeps == 0 { return Vec::new(); }
        let Some((seed, ledger)) = parents.first().zip(model.local_evaluation()) else { return Vec::new(); };
        let Some(mut current) = EvaluatedSolution::evaluate(ledger, seed.clone()) else { return Vec::new(); };
        let order = model.variable_order();
        let mut mesh: Vec<_> = order.iter().enumerate().map(|(index, id)| {
            let scale = seed.get(id).map_or(1.0, |value| to_f64(value).abs().max(1.0));
            let width = model.upper_bound(index).unwrap_or(f64::NAN) - model.lower_bound(index).unwrap_or(f64::NAN);
            self.initial_step * if width.is_finite() && width > 0.0 { width.min(scale) } else { scale }
        }).collect();
        for _ in 0..self.sweeps {
            let base = current.clone();
            for (index, id) in order.iter().enumerate() {
                for sign in [1.0, -1.0] {
                    // 保留外层最终完整评估的空间，与源预算边界一致 / Reserve room for the outer final full evaluation, matching the source boundary.
                    if ledger.remaining() <= 2 { return vec![current.solution().clone()]; }
                    let Some(value) = current.solution().get(id) else { return Vec::new(); };
                    let mut trial = current.solution().clone();
                    trial.insert(*id, clamp_at(model, index, to_f64(value) + sign * mesh[index]));
                    if trial == *current.solution() { continue; }
                    if let Some(candidate) = EvaluatedSolution::evaluate(ledger, trial) {
                        if candidate.compare(&current) == Some(Ordering::Less) { current = candidate; }
                    }
                }
            }
            if current.solution() == base.solution() {
                mesh.iter_mut().for_each(|step| *step *= self.contraction);
            } else {
                let mut pattern = current.solution().clone();
                for (index, id) in order.iter().enumerate() {
                    let (Some(value), Some(previous)) = (current.solution().get(id), base.solution().get(id)) else { return Vec::new(); };
                    pattern.insert(*id, clamp_at(model, index, 2.0 * to_f64(value) - to_f64(previous)));
                }
                if pattern != *current.solution() {
                    if ledger.remaining() <= 2 { return vec![current.solution().clone()]; }
                    if let Some(candidate) = EvaluatedSolution::evaluate(ledger, pattern) {
                        if candidate.compare(&current) == Some(Ordering::Less) { current = candidate; }
                    }
                }
            }
        }
        vec![current.solution().clone()]
    }
}

fn evaluate_all<'m, V: HeuristicValue>(
    ledger: &'m dyn LocalEvaluation<V>,
    solutions: impl IntoIterator<Item = Solution<V>>,
) -> Vec<EvaluatedSolution<'m, V>> {
    solutions.into_iter().filter_map(|solution| EvaluatedSolution::evaluate(ledger, solution)).collect()
}

/// 每阶段重评估中间产物并按完整目标排名的流水线 / Pipeline reevaluating and ranking intermediates by complete objectives at every stage
pub struct PipelineCompositeUpdate<V: HeuristicValue> {
    stages: Vec<Box<dyn SolutionUpdate<V>>>,
}

impl<V: HeuristicValue> PipelineCompositeUpdate<V> {
    /// 按执行顺序设置阶段 / Set stages in execution order
    pub fn new(stages: Vec<Box<dyn SolutionUpdate<V>>>) -> Self { Self { stages } }
}

impl<V: HeuristicValue> SolutionUpdate<V> for PipelineCompositeUpdate<V> {
    fn update(
        &self,
        model: &dyn VariableModel<V>,
        iteration: &Iteration,
        parents: &[Solution<V>],
        entropy: &mut dyn Entropy,
    ) -> Vec<Solution<V>> {
        let Some(ledger) = model.local_evaluation() else { return Vec::new(); };
        let mut current = evaluate_all(ledger, parents.iter().cloned());
        for stage in &self.stages {
            if current.is_empty() { break; }
            let seeds: Vec<_> = current.iter().map(|entry| entry.solution().clone()).collect();
            let next = evaluate_all(ledger, stage.update(model, iteration, &seeds, entropy));
            let Some(ranking) = EvaluatedSolution::rank(&next) else { return Vec::new(); };
            current = ranking.into_iter().map(|index| next[index].clone()).collect();
        }
        current.into_iter().map(|entry| entry.solution().clone()).collect()
    }
}

/// 严格改善后回到首邻域的有限VND / Finite VND restarting at the first neighborhood after strict improvement
pub struct VNDCompositeUpdate<V: HeuristicValue> {
    neighborhoods: Vec<Box<dyn SolutionUpdate<V>>>,
    max_trials: usize,
}

impl<V: HeuristicValue> VNDCompositeUpdate<V> {
    /// 设置有序邻域和最大尝试数 / Set ordered neighborhoods and maximum trials
    pub fn new(neighborhoods: Vec<Box<dyn SolutionUpdate<V>>>, max_trials: usize) -> Self { Self { neighborhoods, max_trials } }
}

impl<V: HeuristicValue> SolutionUpdate<V> for VNDCompositeUpdate<V> {
    fn update(
        &self,
        model: &dyn VariableModel<V>,
        iteration: &Iteration,
        parents: &[Solution<V>],
        entropy: &mut dyn Entropy,
    ) -> Vec<Solution<V>> {
        let Some((seed, ledger)) = parents.first().zip(model.local_evaluation()) else { return Vec::new(); };
        let Some(mut current) = EvaluatedSolution::evaluate(ledger, seed.clone()) else { return Vec::new(); };
        let mut index = 0;
        for _ in 0..self.max_trials {
            if index >= self.neighborhoods.len() { break; }
            let candidates = evaluate_all(ledger, self.neighborhoods[index].update(model, iteration, &[current.solution().clone()], entropy));
            let Some(ranking) = EvaluatedSolution::rank(&candidates) else { return Vec::new(); };
            if let Some(candidate) = ranking.first().map(|&index| &candidates[index]) {
                if candidate.compare(&current) == Some(Ordering::Less) {
                    current = candidate.clone(); index = 0; continue;
                }
            }
            index += 1;
        }
        vec![current.solution().clone()]
    }
}

/// 仅精调已排序Top-K，保留其余父代 / Intensify only ranked Top-K, retaining the remaining parents
pub struct EliteLocalSearchWrapper<V: HeuristicValue> {
    local_search: Box<dyn SolutionUpdate<V>>,
    amount: usize,
}

impl<V: HeuristicValue> EliteLocalSearchWrapper<V> {
    /// 设置精调动作及精英数量 / Set the intensification action and elite count
    pub fn new(local_search: Box<dyn SolutionUpdate<V>>, amount: usize) -> Self { Self { local_search, amount } }
}

impl<V: HeuristicValue> SolutionUpdate<V> for EliteLocalSearchWrapper<V> {
    fn update(
        &self,
        model: &dyn VariableModel<V>,
        iteration: &Iteration,
        parents: &[Solution<V>],
        entropy: &mut dyn Entropy,
    ) -> Vec<Solution<V>> {
        parents.iter().take(self.amount).flat_map(|parent| self.local_search.update(model, iteration, &[parent.clone()], entropy))
            .chain(parents.iter().skip(self.amount).cloned()).collect()
    }
}

#[cfg(test)]
mod tests;
