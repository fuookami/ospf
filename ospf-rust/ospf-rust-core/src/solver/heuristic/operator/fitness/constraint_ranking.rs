//! 已计费业务评估的独立约束排序 / Independent constraint ranking of already metered domain assessments

use std::cmp::Ordering;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::result::Result;
use crate::solver::heuristic::session::{ConstraintHandlingConfig, SearchRandom};
use crate::solver::heuristic::session::json::invalid_state;

/// 显式评估，不把容差内个体标为严格可行 / Explicit assessment without labeling tolerance-admitted individuals strictly feasible
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintAssessment<T> {
    /// 原始业务值 / Original domain value
    pub value: T,
    /// 显式最小化标量 / Explicit minimization scalar
    pub objective: f64,
    /// 归一化非负违约度，不是可行性证明 / Normalized nonnegative violation, not a feasibility proof
    pub violation: f64,
}

fn validate<T>(entries: &[ConstraintAssessment<T>]) -> Result<()> {
    if entries.iter().any(|entry| !entry.objective.is_finite() || !entry.violation.is_finite() || entry.violation < 0.0) {
        return Err(invalid_state("invalid constraint assessment / 无效约束评估"));
    }
    Ok(())
}

/// 收紧至零的独立 epsilon 排序器 / Independent epsilon ranker tightening to zero
#[derive(Debug, Clone, Copy)]
pub struct EpsilonConstraintEvaluator {
    /// 初始违约容差 / Initial violation tolerance
    pub initial_epsilon: f64,
    /// 收紧步数 / Tightening horizon
    pub horizon: u64,
    /// 收紧指数 / Tightening exponent
    pub exponent: f64,
}

impl EpsilonConstraintEvaluator {
    /// 对已评估候选稳定排序，不执行目标评估 / Stably rank already assessed candidates without evaluating objectives
    pub fn rank<T: Clone>(&self, entries: &[ConstraintAssessment<T>], step: u64) -> Result<Vec<ConstraintAssessment<T>>> {
        ConstraintHandlingConfig { initial_epsilon: Some(self.initial_epsilon), horizon: self.horizon,
            exponent: self.exponent, ..Default::default() }.validate()?;
        if step > i64::MAX as u64 { return Err(invalid_state("constraint step overflow / 约束步数溢出")); }
        validate(entries)?;
        let epsilon = self.initial_epsilon * libm::pow((1.0 - step as f64 / self.horizon as f64).clamp(0.0, 1.0), self.exponent);
        let mut ranked = entries.to_vec();
        ranked.sort_by(|a, b| {
            if a.violation <= epsilon && b.violation <= epsilon || a.violation == b.violation {
                a.objective.partial_cmp(&b.objective).unwrap_or(Ordering::Equal)
            } else { a.violation.partial_cmp(&b.violation).unwrap_or(Ordering::Equal) }
        });
        Ok(ranked)
    }
}

/// 有界冒泡随机排序，绝不充当随机比较器 / Bounded bubble stochastic ranking, never a random comparator
#[derive(Debug, Clone, Copy)]
pub struct StochasticRanking {
    /// 双不可行评估按目标比较的概率 / Probability of objective comparison for infeasible assessments
    pub objective_probability: f64,
}

impl Default for StochasticRanking {
    fn default() -> Self { Self { objective_probability: 0.45 } }
}

impl StochasticRanking {
    /// 使用调用方可恢复随机流排序已评估候选 / Rank already assessed candidates using a caller-owned restorable random stream
    pub fn rank<T: Clone>(
        &self,
        entries: &[ConstraintAssessment<T>],
        random: &mut SearchRandom,
    ) -> Result<Vec<ConstraintAssessment<T>>> {
        ConstraintHandlingConfig { objective_probability: self.objective_probability, ..Default::default() }.validate()?;
        validate(entries)?;
        let mut ranked = entries.to_vec();
        for _ in 0..ranked.len() {
            let mut swapped = false;
            for index in 0..ranked.len().saturating_sub(1) {
                let a = &ranked[index];
                let b = &ranked[index + 1];
                let objective = a.violation == 0.0 && b.violation == 0.0 || random.next_unit() < self.objective_probability;
                if if objective { a.objective > b.objective } else { a.violation > b.violation } {
                    ranked.swap(index, index + 1);
                    swapped = true;
                }
            }
            if !swapped { break; }
        }
        Ok(ranked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries() -> Vec<ConstraintAssessment<usize>> {
        [(0, 1.0, 2.0), (1, 3.0, 0.0), (2, 2.0, 1.0)].into_iter()
            .map(|(value, objective, violation)| ConstraintAssessment { value, objective, violation }).collect()
    }

    #[test]
    fn epsilon_ranker_tightens_without_changing_assessments() {
        let ranker = EpsilonConstraintEvaluator { initial_epsilon: 2.0, horizon: 2, exponent: 2.0 };
        let values = |entries: Vec<ConstraintAssessment<usize>>| entries.into_iter().map(|entry| entry.value).collect::<Vec<_>>();
        assert_eq!(values(ranker.rank(&entries(), 0).unwrap()), [0, 2, 1]);
        assert_eq!(values(ranker.rank(&entries(), 2).unwrap()), [1, 2, 0]);
        assert_eq!(ranker.rank(&entries(), 0).unwrap()[0].violation, 2.0);
    }

    #[test]
    fn stochastic_ranker_is_reproducible_and_rejects_bad_inputs_without_drawing() {
        let ranker = StochasticRanking::default();
        let mut random = SearchRandom::new(42);
        let mut restored = random.clone();
        assert_eq!(ranker.rank(&entries(), &mut random).unwrap(), ranker.rank(&entries(), &mut restored).unwrap());
        let state = random.state();
        let mut bad = entries(); bad[0].objective = f64::NAN;
        assert!(ranker.rank(&bad, &mut random).is_err());
        assert_eq!(random.state(), state);
        assert!(StochasticRanking { objective_probability: -1.0 }.rank(&entries(), &mut random).is_err());
    }
}
