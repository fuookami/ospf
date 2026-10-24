//! 档案与工作集分离 / Separate archives and working sets

use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::operator::multi_objective::CrowdingDistance;
use crate::solver::heuristic::session::{SearchProblem, archive::ObjectiveComponents};
use crate::solver::hyper_heuristic::components::RecoverableComponent;

mod adaptive;

pub use adaptive::{AutoAcceptance, LateAcceptance, RecordToRecordAcceptance};
pub(crate) use adaptive::builtin_acceptance;

/// 接受策略，仅影响工作集，不回滚算法原生状态 / Acceptance policy affecting working sets without rolling back native state
pub trait AcceptancePolicy<O: Clone, V: HeuristicValue>: RecoverableComponent {
    /// 不执行目标调用的接受能力预检 / Preflight acceptance capabilities without objective calls
    fn validate_problem(&self, _problem: &dyn SearchProblem<O, V>) -> crate::solver::heuristic::result::Result<()> { Ok(()) }
    /// 是否接收有效候选 / Whether to accept a valid candidate
    fn accept(
        &mut self,
        problem: &dyn SearchProblem<O, V>,
        working: &[Individual<O, V>],
        candidate: &Individual<O, V>,
    ) -> bool;
}

/// 仅接受未被工作集支配的候选 / Accept candidates not dominated by the working set
pub struct NotWorseAcceptance;

impl RecoverableComponent for NotWorseAcceptance {
    fn stateless(&self) -> bool { true }
}

/// 接受所有有效工作集候选 / Accept every valid working-set candidate
pub struct AlwaysAcceptance;

impl RecoverableComponent for AlwaysAcceptance {
    fn stateless(&self) -> bool { true }
}

impl<O: Clone, V: HeuristicValue> AcceptancePolicy<O, V> for AlwaysAcceptance {
    fn accept(
        &mut self,
        _: &dyn SearchProblem<O, V>,
        _: &[Individual<O, V>],
        _: &Individual<O, V>,
    ) -> bool { true }
}

impl<O: Clone, V: HeuristicValue> AcceptancePolicy<O, V> for NotWorseAcceptance {
    fn accept(
        &mut self,
        problem: &dyn SearchProblem<O, V>,
        working: &[Individual<O, V>],
        candidate: &Individual<O, V>,
    ) -> bool {
        !working.iter().any(|value| problem.dominates(&value.fitness, &candidate.fitness))
    }
}

/// 协调器独占的有效档案与已接受工作集 / Valid archive and accepted working set owned by the coordinator
pub struct SearchMemory<O, V> {
    archive: Vec<Individual<O, V>>,
    working: Vec<Individual<O, V>>,
    capacity: usize,
    working_capacity: usize,
    crowding_truncation: bool,
}

impl<O: ObjectiveComponents, V: HeuristicValue> SearchMemory<O, V> {
    /// 创建有界记忆 / Create bounded memory
    pub fn new(capacity: usize, working_capacity: usize) -> Self {
        Self { archive: Vec::new(), working: Vec::new(), capacity, working_capacity, crowding_truncation: true }
    }

    // 局部会话按到达顺序保留前缀，全局多目标记忆才使用拥挤距离。
    // Local sessions retain the arrival-order prefix; only global multiobjective memory uses crowding distance.
    pub(crate) fn local(capacity: usize, working_capacity: usize) -> Self {
        Self { crowding_truncation: false, ..Self::new(capacity, working_capacity) }
    }

    /// 有效档案 / Valid archive
    pub fn archive(&self) -> &[Individual<O, V>] { &self.archive }
    /// 已接受工作集 / Accepted working set
    pub fn working(&self) -> &[Individual<O, V>] { &self.working }

    pub(crate) fn restore(
        &mut self,
        problem: &dyn SearchProblem<O, V>,
        archive: Vec<Individual<O, V>>,
        working: Vec<Individual<O, V>>,
    ) -> crate::solver::heuristic::result::Result<()> {
        if archive.len() > self.capacity || working.len() > self.working_capacity
            || archive.iter().chain(&working).any(|value| !problem.valid_solution(&value.solution)
                || !problem.valid_objective(&value.fitness) || !problem.admissible(&value.solution) || !problem.feasible(&value.solution)) {
            return Err(super::invalid("invalid checkpoint memory"));
        }
        self.archive = archive;
        self.working = working;
        Ok(())
    }

    /// 相对派发时参考档案计算贡献 / Compute contribution against the dispatch-time reference archive
    pub fn contribution(
        &self,
        problem: &dyn SearchProblem<O, V>,
        reference: &[Individual<O, V>],
        candidates: &[Individual<O, V>],
    ) -> (bool, f64) {
        let merged = self.merged(problem, reference, candidates);
        let novel = if problem.multi_objective() {
            merged.iter().filter(|candidate| !reference.iter().any(|old|
                problem.dominates(&old.fitness, &candidate.fitness) || problem.equivalent(&old.fitness, &candidate.fitness))).count()
        } else {
            usize::from(merged.first().is_some_and(|candidate|
                reference.first().is_none_or(|old| problem.dominates(&candidate.fitness, &old.fitness))))
        };
        (novel > 0, novel as f64 / candidates.len().max(1) as f64)
    }

    fn merged(
        &self,
        problem: &dyn SearchProblem<O, V>,
        reference: &[Individual<O, V>],
        candidates: &[Individual<O, V>],
    ) -> Vec<Individual<O, V>> {
        let mut all = Vec::new();
        for value in reference.iter().chain(candidates) {
            if problem.valid_solution(&value.solution) && problem.valid_objective(&value.fitness)
                && problem.admissible(&value.solution) && problem.feasible(&value.solution)
                && !all.iter().any(|old: &Individual<O, V>| old.solution == value.solution) {
                all.push(value.clone());
            }
        }
        let mut merged = Vec::new();
        for value in &all {
            if problem.multi_objective() && (all.iter().any(|old| problem.dominates(&old.fitness, &value.fitness))
                || merged.iter().any(|old: &Individual<O, V>| problem.equivalent(&old.fitness, &value.fitness))) { continue; }
            merged.push(value.clone());
        }
        if problem.multi_objective() && merged.len() > self.capacity {
            if !self.crowding_truncation {
                merged.truncate(self.capacity);
                return merged;
            }
            let distances = CrowdingDistance::assign(&merged.iter().map(|value| value.fitness.components()).collect::<Vec<_>>());
            let mut order: Vec<_> = (0..merged.len()).collect();
            order.sort_by(|lhs, rhs| distances[*rhs].total_cmp(&distances[*lhs]).then(lhs.cmp(rhs)));
            order.truncate(self.capacity);
            order.sort_unstable();
            merged = order.into_iter().map(|index| merged[index].clone()).collect();
        } else if !problem.multi_objective() {
            merged.sort_by(|lhs, rhs| {
                if problem.dominates(&lhs.fitness, &rhs.fitness) { std::cmp::Ordering::Less }
                else if problem.dominates(&rhs.fitness, &lhs.fitness) { std::cmp::Ordering::Greater }
                else { std::cmp::Ordering::Equal }
            });
            merged.truncate(self.capacity);
        }
        merged
    }

    /// 归并有效候选，不触发目标重评估 / Merge valid candidates without re-evaluating objectives
    pub fn offer(
        &mut self,
        problem: &dyn SearchProblem<O, V>,
        candidates: Vec<Individual<O, V>>,
        acceptance: &mut dyn AcceptancePolicy<O, V>,
    ) -> bool {
        let valid: Vec<_> = candidates.into_iter().filter(|value| problem.valid_solution(&value.solution)
            && problem.valid_objective(&value.fitness) && problem.admissible(&value.solution) && problem.feasible(&value.solution)).collect();
        let improved = self.contribution(problem, &self.archive, &valid).0;
        self.archive = self.merged(problem, &self.archive, &valid);
        for candidate in &valid {
            if acceptance.accept(problem, &self.working, candidate) {
                self.working.retain(|old| old.solution != candidate.solution);
                self.working.insert(0, candidate.clone());
                self.working.truncate(self.working_capacity);
            }
        }
        improved
    }
}
