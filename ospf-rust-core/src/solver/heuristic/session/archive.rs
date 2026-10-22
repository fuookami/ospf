//! 会话档案 / Session archive
//!
//! 与 Kotlin 的 `Archive` 契约对应：单目标用「最优 N 个」，多目标用「非支配集 + 容量裁剪」。会话层原先只保留
//! 非支配去重、没有容量上限，本模块补齐容量语义。
//! Mirrors the Kotlin `Archive` contract: "best N" for single-objective and "non-dominated set plus capacity trimming" for
//! multi-objective. The session previously deduplicated non-dominated solutions but had no capacity bound; this module adds it.

use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::operator::multi_objective::CrowdingDistance;
use crate::solver::heuristic::session::protocol::SearchProblem;

/// 目标分量视图 / Objective component view
///
/// 通用 `O` 无法直接参与拥挤距离计算，因此需要一层「分量」抽象：标量视作单分量，目标向量直接给出各分量。
/// Generic `O` cannot feed the crowding-distance computation directly, so a component view is required: a scalar is a single
/// component and an objective vector yields its components.
pub trait ObjectiveComponents: Clone {
    /// 目标分量 / Objective components
    fn components(&self) -> Vec<f64>;
}

impl ObjectiveComponents for f64 {
    fn components(&self) -> Vec<f64> {
        vec![*self]
    }
}

impl ObjectiveComponents for Vec<f64> {
    fn components(&self) -> Vec<f64> {
        self.clone()
    }
}

/// 会话档案：单目标保留最优 N 个，多目标保留非支配集并裁剪到容量 / Session archive: keep the best N for single-objective, the non-dominated set trimmed to capacity for multi-objective
pub struct SessionArchive<O, V>
where
    O: ObjectiveComponents,
    V: HeuristicValue,
{
    capacity: usize,
    entries: Vec<Individual<O, V>>,
}

impl<O, V> SessionArchive<O, V>
where
    O: ObjectiveComponents,
    V: HeuristicValue,
{
    /// 创建档案 / Create an archive
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            entries: Vec::new(),
        }
    }

    /// 容量 / Capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 档案内容 / Archive content
    pub fn items(&self) -> &[Individual<O, V>] {
        &self.entries
    }

    /// 档案大小 / Archive size
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空 / Whether the archive is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 清空档案 / Clear the archive
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// 直接替换内容（用于恢复快照）/ Replace the content directly (used when restoring a snapshot)
    pub fn replace(&mut self, entries: Vec<Individual<O, V>>) {
        self.entries = entries;
        self.trim_entries();
    }

    /// 提交候选并返回「最优是否被改善」/ Offer candidates and report whether the best improved
    ///
    /// 单目标：严格优于当前最优才算改善；多目标：任一候选未被历史成员支配即算改善（与 Kotlin
    /// `ParetoSearchArchive.update` 一致）。
    /// Single-objective: improvement means strictly better than the current best. Multi-objective: improvement means some
    /// candidate is not dominated by any previous member, matching Kotlin's `ParetoSearchArchive.update`.
    pub fn update<P>(&mut self, problem: &P, candidates: Vec<Individual<O, V>>) -> bool
    where
        P: SearchProblem<O, V>,
    {
        if problem.multi_objective() {
            self.update_multi(problem, candidates)
        } else {
            self.update_single(problem, candidates)
        }
    }

    fn update_single<P>(&mut self, problem: &P, candidates: Vec<Individual<O, V>>) -> bool
    where
        P: SearchProblem<O, V>,
    {
        let previous_best = self.entries.first().cloned();
        self.entries.extend(candidates);
        self.entries.sort_by(|lhs, rhs| {
            if problem.dominates(&lhs.fitness, &rhs.fitness) {
                std::cmp::Ordering::Less
            } else if problem.dominates(&rhs.fitness, &lhs.fitness) {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });
        self.entries.dedup_by(|lhs, rhs| {
            problem.equivalent(&lhs.fitness, &rhs.fitness)
        });
        self.entries.truncate(self.capacity);

        match (previous_best, self.entries.first()) {
            (None, Some(_)) => true,
            (Some(previous), Some(current)) => problem.dominates(&current.fitness, &previous.fitness),
            _ => false,
        }
    }

    fn update_multi<P>(&mut self, problem: &P, candidates: Vec<Individual<O, V>>) -> bool
    where
        P: SearchProblem<O, V>,
    {
        // 与 Kotlin `ParetoArchive.offer` 完全同构：
        // 1. 被档案中任一成员支配 → 直接丢弃；
        // 2. 与既有成员目标完全相同 → 保留已有的一份；
        // 3. 新解支配档案成员 → 淘汰它们，然后加入。
        //
        // 「是否改进」按 Kotlin `ParetoSearchArchive.update` 的定义：候选相对**本轮开始前**的档案既不被支配也不等价。
        // Structurally identical to Kotlin's `ParetoArchive.offer`; "improved" follows Kotlin's
        // `ParetoSearchArchive.update`: the candidate is neither dominated by nor equal to a member of the archive as it was
        // before this round.
        let previous: Vec<O> = self
            .entries
            .iter()
            .map(|entry| entry.fitness.clone())
            .collect();
        let mut improved = false;

        for candidate in candidates {
            // 已被档案成员支配：丢弃（实时检查，不能用本轮开始前的快照）。
            // Dominated by a member of the archive: discard. Checked live, not against the round-start snapshot.
            if self.entries.iter().any(|entry| {
                problem.dominates(&entry.fitness, &candidate.fitness)
            }) {
                continue;
            }
            // 目标相同的重复解：保留已有的一份。
            // Duplicate with an identical objective vector: keep the existing one.
            if self
                .entries
                .iter()
                .any(|entry| same_objective(&entry.fitness, &candidate.fitness))
            {
                continue;
            }

            // 只有真正进入档案、且相对「本轮开始前」既未被支配也未被淘汰，才算改进。
            // Only a candidate that actually enters the archive and was neither dominated by nor equal to a round-start
            // member counts as an improvement.
            if previous.is_empty()
                || previous.iter().any(|entry| {
                    !same_objective(entry, &candidate.fitness)
                        && !problem.dominates(entry, &candidate.fitness)
                })
            {
                improved = true;
            }

            // 新解支配档案成员：淘汰它们。
            // The newcomer dominates some members: evict them.
            self.entries
                .retain(|entry| !problem.dominates(&candidate.fitness, &entry.fitness));
            self.entries.push(candidate);
        }

        self.trim_entries();
        improved
    }

    /// 按容量裁剪：多目标按拥挤距离从大到小，单目标按目标值顺序 / Trim to capacity: crowding distance for multi-objective, objective order for single-objective
    fn trim_entries(&mut self) {
        if self.entries.len() <= self.capacity {
            return;
        }
        if self.entries.len() <= 2 || self.entries[0].fitness.components().len() <= 1 {
            self.entries.truncate(self.capacity);
            return;
        }

        let objectives: Vec<Vec<f64>> = self
            .entries
            .iter()
            .map(|entry| entry.fitness.components())
            .collect();
        let distances = CrowdingDistance::assign(&objectives);
        let mut retained: Vec<usize> = (0..self.entries.len()).collect();
        retained.sort_by(|lhs, rhs| {
            distances[*rhs]
                .partial_cmp(&distances[*lhs])
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| lhs.cmp(rhs))
        });
        retained.truncate(self.capacity);

        let survivors: Vec<Individual<O, V>> = retained
            .into_iter()
            .map(|index| self.entries[index].clone())
            .collect();
        self.entries = survivors;
    }
}

/// 目标是否完全相同 / Whether two objectives are exactly identical
///
/// 与 Kotlin `ParetoArchive` 的 `objectivesOf(it) == candidateObjectives` 一致：按分量严格相等，而不是「互不支配」。
/// 后者会把任何两个非支配点误判为重复，从而错误丢弃真实的前沿成员。
/// Matches Kotlin `ParetoArchive`'s `objectivesOf(it) == candidateObjectives`: component-wise exact equality rather than
/// "mutually non-dominated". The latter would treat any two non-dominated points as duplicates and wrongly drop real front
/// members.
fn same_objective<O: ObjectiveComponents>(lhs: &O, rhs: &O) -> bool {
    lhs.components() == rhs.components()
}

impl<O, V> Default for SessionArchive<O, V>
where
    O: ObjectiveComponents,
    V: HeuristicValue,
{
    fn default() -> Self {
        Self::new(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::callback::Solution;
    use crate::variable::{VariableId, new_standalone_id};

    /// 标量问题，`dominates` 为严格小于 / Scalar problem whose `dominates` is strict less-than
    struct Scalar {
        x: VariableId,
    }

    impl SearchProblem<f64, f64> for Scalar {
        fn id(&self) -> &str {
            "scalar"
        }
        fn fingerprint(&self) -> &str {
            "scalar-v1"
        }
        fn variable_order(&self) -> Vec<VariableId> {
            vec![self.x]
        }
        fn initial_solutions(&self, _amount: usize) -> Vec<Solution<f64>> {
            Vec::new()
        }
        fn objective(&self, solution: &Solution<f64>) -> Option<f64> {
            solution.get(&self.x).copied()
        }
        fn dominates(&self, lhs: &f64, rhs: &f64) -> bool {
            lhs < rhs
        }
    }

    /// 双目标问题 / Bi-objective problem
    struct BiObjective {
        x: VariableId,
    }

    impl SearchProblem<Vec<f64>, f64> for BiObjective {
        fn id(&self) -> &str {
            "bi"
        }
        fn fingerprint(&self) -> &str {
            "bi-v1"
        }
        fn variable_order(&self) -> Vec<VariableId> {
            vec![self.x]
        }
        fn initial_solutions(&self, _amount: usize) -> Vec<Solution<f64>> {
            Vec::new()
        }
        fn objective(&self, solution: &Solution<f64>) -> Option<Vec<f64>> {
            let value = solution.get(&self.x).copied()?;
            Some(vec![value, value])
        }
        fn dominates(&self, lhs: &Vec<f64>, rhs: &Vec<f64>) -> bool {
            let no_worse = lhs[0] <= rhs[0] && lhs[1] <= rhs[1];
            let better = lhs[0] < rhs[0] || lhs[1] < rhs[1];
            no_worse && better
        }
        fn multi_objective(&self) -> bool {
            true
        }
    }

    /// 权衡问题：目标 (x, 1-x)，任意两点互不支配 / Trade-off problem: objectives (x, 1-x) where any two points are mutually non-dominated
    struct TradeOff {
        x: VariableId,
    }

    impl SearchProblem<Vec<f64>, f64> for TradeOff {
        fn id(&self) -> &str {
            "trade-off"
        }
        fn fingerprint(&self) -> &str {
            "trade-off-v1"
        }
        fn variable_order(&self) -> Vec<VariableId> {
            vec![self.x]
        }
        fn initial_solutions(&self, _amount: usize) -> Vec<Solution<f64>> {
            Vec::new()
        }
        fn objective(&self, solution: &Solution<f64>) -> Option<Vec<f64>> {
            let value = solution.get(&self.x).copied()?;
            Some(vec![value, 1.0 - value])
        }
        fn dominates(&self, lhs: &Vec<f64>, rhs: &Vec<f64>) -> bool {
            let no_worse = lhs[0] <= rhs[0] && lhs[1] <= rhs[1];
            let better = lhs[0] < rhs[0] || lhs[1] < rhs[1];
            no_worse && better
        }
        fn multi_objective(&self) -> bool {
            true
        }
    }

    fn individual(x: VariableId, value: f64, fitness: f64) -> Individual<f64, f64> {
        Individual::new(Solution::from([(x, value)]), fitness)
    }

    #[test]
    fn scalar_archive_keeps_best_and_trims_to_capacity() {
        let x = new_standalone_id();
        let problem = Scalar { x };
        let mut archive = SessionArchive::new(2);

        assert!(archive.update(&problem, vec![individual(x, 3.0, 3.0), individual(x, 1.0, 1.0)]));
        assert_eq!(archive.len(), 2);
        assert_eq!(archive.items()[0].fitness, 1.0);

        // 更优解进入，且被裁剪到容量 / A better solution enters and the archive is trimmed to capacity
        assert!(archive.update(&problem, vec![individual(x, 0.5, 0.5)]));
        assert_eq!(archive.len(), 2);
        assert_eq!(archive.items()[0].fitness, 0.5);

        // 更差解不改变最优，也不报改进 / A worse solution neither changes the best nor reports improvement
        assert!(!archive.update(&problem, vec![individual(x, 9.0, 9.0)]));
        assert_eq!(archive.items()[0].fitness, 0.5);
    }

    #[test]
    fn scalar_archive_reports_no_improvement_for_equal_objective() {
        let x = new_standalone_id();
        let problem = Scalar { x };
        let mut archive = SessionArchive::new(1);
        assert!(archive.update(&problem, vec![individual(x, 2.0, 2.0)]));
        assert!(!archive.update(&problem, vec![individual(x, 2.0, 2.0)]));
        assert_eq!(archive.len(), 1);
    }

    #[test]
    fn multi_objective_archive_drops_dominated_and_respects_capacity() {
        let x = new_standalone_id();
        let problem = BiObjective { x };
        let mut archive = SessionArchive::new(2);

        // 目标 (x, x)：x 小者支配 x 大者，因此档案只保留最小的一个，容量不是瓶颈。
        // Objectives (x, x): a smaller x dominates a larger one, so only the smallest survives and capacity is not binding.
        let bi = |value: f64| Individual::new(Solution::from([(x, value)]), vec![value, value]);
        assert!(archive.update(&problem, vec![bi(3.0), bi(1.0), bi(2.0)]));
        assert_eq!(archive.len(), 1);
        assert_eq!(archive.items()[0].fitness.components(), vec![1.0, 1.0]);

        // 更差候选既不进入也不报改进 / A worse candidate neither enters nor reports improvement
        assert!(!archive.update(&problem, vec![bi(4.0)]));
        assert_eq!(archive.len(), 1);
    }

    #[test]
    fn multi_objective_archive_evicts_dominated_members() {
        let x = new_standalone_id();
        let problem = BiObjective { x };
        let mut archive = SessionArchive::new(8);
        let bi = |value: f64| Individual::new(Solution::from([(x, value)]), vec![value, value]);

        archive.update(&problem, vec![bi(2.0), bi(3.0)]);
        // 两个候选互不支配（2,2 与 3,3 可比，后者被前者支配），因此只剩 2。
        // The two candidates are comparable, so only x=2 survives.
        assert_eq!(archive.len(), 1);

        // 更优候选淘汰旧成员 / A better candidate evicts the old one
        assert!(archive.update(&problem, vec![bi(1.0)]));
        assert_eq!(archive.len(), 1);
        assert_eq!(archive.items()[0].fitness.components(), vec![1.0, 1.0]);
    }

    #[test]
    fn multi_objective_archive_reports_improvement_only_for_new_members() {
        let x = new_standalone_id();
        let problem = BiObjective { x };
        let mut archive = SessionArchive::new(8);
        let bi = |value: f64| Individual::new(Solution::from([(x, value)]), vec![value, value]);

        assert!(archive.update(&problem, vec![bi(1.0)]));
        // 完全相同目标不再算改进 / An identical objective is not an improvement
        assert!(!archive.update(&problem, vec![bi(1.0)]));
        // 被支配的解不算改进 / A dominated solution is not an improvement
        assert!(!archive.update(&problem, vec![bi(2.0)]));
        // 更优解算改进 / A better solution is an improvement
        assert!(archive.update(&problem, vec![bi(0.5)]));
        assert_eq!(archive.len(), 1);
    }

    #[test]
    fn multi_objective_archive_keeps_mutually_non_dominated_members() {
        let x = new_standalone_id();
        let problem = TradeOff { x };
        let mut archive = SessionArchive::new(2);
        let trade_off = |value: f64| {
            Individual::new(Solution::from([(x, value)]), vec![value, 1.0 - value])
        };

        // 目标 (x, 1-x)：任意两点互不支配，容量 2 触发按拥挤距离裁剪。
        // Objectives (x, 1-x): any two points are mutually non-dominated, so capacity 2 trims by crowding distance.
        assert!(archive.update(&problem, vec![trade_off(1.0), trade_off(2.0), trade_off(3.0)]));
        assert_eq!(archive.len(), 2);
        // 边界解拥挤距离为正无穷，必须被保留 / Boundary solutions have infinite crowding distance and must survive
        let values: Vec<f64> = archive
            .items()
            .iter()
            .map(|item| item.solution.get(&x).copied().unwrap_or_default())
            .collect();
        assert!(values.contains(&1.0));
        assert!(values.contains(&3.0));
    }

    #[test]
    fn replace_restores_and_trims() {
        let x = new_standalone_id();
        let mut archive = SessionArchive::new(2);
        let entries: Vec<Individual<Vec<f64>, f64>> = (1..=5)
            .map(|value| {
                let value = value as f64;
                Individual::new(Solution::from([(x, value)]), vec![value, 1.0 - value])
            })
            .collect();
        archive.replace(entries);
        assert_eq!(archive.len(), 2);
    }
}
