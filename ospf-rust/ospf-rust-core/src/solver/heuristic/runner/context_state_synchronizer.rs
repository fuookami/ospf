//! 异构会话导入的类型化状态桥 / Typed state bridge for heterogeneous session imports

use std::cmp::Ordering;
use crate::model::callback::Solution;
use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::model::{HeuristicValue, to_f64};
use crate::solver::heuristic::result::{HeuristicError, Result};
use crate::solver::heuristic::session::SearchProblem;
use super::{ClpsoParticle, ParticleState, ShadeState};

/// 保留目的算法真实历史的已计费导入桥 / Bridge for already-metered imports preserving genuine destination history
pub struct ContextStateSynchronizer<'p, V: HeuristicValue> {
    problem: &'p dyn SearchProblem<V, V>,
    max_velocity: f64,
}

impl<'p, V: HeuristicValue> ContextStateSynchronizer<'p, V> {
    /// 绑定完整问题比较及正的速度夹紧值 / Bind complete problem comparison and a positive velocity clamp
    pub fn new(problem: &'p dyn SearchProblem<V, V>, max_velocity: f64) -> Result<Self> {
        if !max_velocity.is_finite() || max_velocity <= 0.0 { return Err(invalid("invalid bridge velocity limit")); }
        Ok(Self { problem, max_velocity })
    }

    fn valid_solution(&self, solution: &Solution<V>) -> bool {
        self.problem.valid_solution(solution) && solution.values().all(|value| to_f64(value).is_finite())
    }

    fn validate(&self, incoming: &[Individual<V, V>]) -> Result<()> {
        if incoming.iter().any(|value| !self.valid_solution(&value.solution)
            || !self.problem.valid_objective(&value.fitness) || !self.problem.admissible(&value.solution)) {
            return Err(invalid("invalid bridge imports"));
        }
        Ok(())
    }

    /// 按显式规范位置计算夹紧位移，不执行目标 / Compute clamped displacement in explicit canonical order without objective calls
    pub fn velocity(&self, previous: &Solution<V>, current: &Solution<V>) -> Result<Vec<f64>> {
        if !self.valid_solution(previous) || !self.valid_solution(current) { return Err(invalid("invalid bridge positions")); }
        Ok(self.problem.variable_order().iter().map(|id| (to_f64(&current[id]) - to_f64(&previous[id]))
            .clamp(-self.max_velocity, self.max_velocity)).collect())
    }

    /// 从最近的原目的粒子热启动速度，替换最差粒子并保留真实历史最优 / Warm-start from the nearest original destination particle, replace worst particles, and retain genuine personal best
    pub fn particles(&self, state: &[ParticleState<V>], incoming: &[Individual<V, V>]) -> Result<Vec<ParticleState<V>>> {
        self.validate(incoming)?;
        if state.iter().any(|particle| !self.valid_solution(&particle.individual.solution)) { return Err(invalid("invalid destination particles")); }
        if incoming.is_empty() || state.is_empty() { return Ok(state.to_vec()); }
        let order = self.problem.variable_order();
        let mut result = state.to_vec();
        result.sort_by(|a, b| self.problem.compare_individuals(&a.best, &b.best));
        for (index, candidate) in incoming.iter().take(state.len()).enumerate() {
            let distance = |particle: &ParticleState<V>| order.iter().map(|id| {
                let delta = to_f64(&candidate.solution[id]) - to_f64(&particle.individual.solution[id]); delta * delta
            }).sum::<f64>();
            let source = state.iter().skip(1).fold(&state[0], |nearest, particle|
                if distance(particle) < distance(nearest) { particle } else { nearest });
            result[state.len() - 1 - index] = ParticleState { individual: candidate.clone(),
                velocity: self.velocity(&source.individual.solution, &candidate.solution)?,
                best: if self.problem.compare_individuals(&source.best, candidate) == Ordering::Less { source.best.clone() } else { candidate.clone() } };
        }
        Ok(result)
    }

    /// 重建合法CLPSO示范索引，保留同步后的速度及历史最优 / Rebuild valid CLPSO exemplar indices while preserving synchronized velocities and personal best
    pub fn clpso(&self, state: &[ClpsoParticle<V>], incoming: &[Individual<V, V>]) -> Result<Vec<ClpsoParticle<V>>> {
        if incoming.is_empty() { return Ok(state.to_vec()); }
        let particles: Vec<_> = state.iter().map(|particle| ParticleState {
            individual: particle.individual.clone(), velocity: particle.velocity.clone(), best: particle.best.clone(),
        }).collect();
        self.particles(&particles, incoming).map(|particles| particles.into_iter().enumerate().map(|(index, particle)| ClpsoParticle {
            exemplars: vec![index; particle.individual.solution.len()], stagnation: 0,
            individual: particle.individual, velocity: particle.velocity, best: particle.best,
        }).collect())
    }

    /// 去重排序种群并归档淘汰成员，不伪造SHADE成功参数样本 / Deduplicate and rank the population, archive displaced members, and never fabricate SHADE success samples
    pub fn shade(&self, state: &ShadeState<V>, incoming: &[Individual<V, V>]) -> Result<ShadeState<V>> {
        self.validate(incoming)?;
        let mut candidates: Vec<Individual<V, V>> = Vec::new();
        for value in state.population.iter().chain(incoming) {
            if !candidates.iter().any(|old| old.solution == value.solution) { candidates.push(value.clone()); }
        }
        let population: Vec<_> = self.problem.rank_indices(&candidates).into_iter().take(state.population.len())
            .map(|index| candidates[index].clone()).collect();
        let mut archive = Vec::new();
        for solution in state.archive.iter().chain(state.population.iter()
            .filter(|old| !population.iter().any(|value| value.solution == old.solution)).map(|old| &old.solution)) {
            if !archive.contains(solution) { archive.push(solution.clone()); }
        }
        let keep_from = archive.len().saturating_sub(state.population.len());
        let mut result = state.clone(); result.population = population; result.archive = archive.into_iter().skip(keep_from).collect();
        Ok(result)
    }
}

fn invalid(message: &str) -> HeuristicError { HeuristicError::InvalidArgument(message.into()) }

#[cfg(test)]
mod tests;
