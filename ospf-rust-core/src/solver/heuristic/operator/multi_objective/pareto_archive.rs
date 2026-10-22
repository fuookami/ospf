//! Pareto 档案 / Pareto archive

use super::dominance::Dominance;
use super::non_dominated_sort::CrowdingDistance;

/// 有界 Pareto 档案，保存当前已知的非支配解集 / Bounded Pareto archive holding the currently known non-dominated set
///
/// 与单目标档案不同：任何被支配的解都不值得保留，而互不支配的解都是合法的最终答案。超出容量时按拥挤距离从大到小
/// 裁剪，保留分布最稀疏的解，以维持前沿覆盖度。
/// Unlike the single-objective archive: any dominated solution is worthless, while every mutually non-dominated solution is
/// a legitimate final answer. When the capacity is exceeded, solutions are truncated from the largest crowding distance
/// downwards, keeping the sparsest solutions to preserve the spread of the front.
pub struct ParetoArchive<T>
where
    T: Clone,
{
    capacity: usize,
    objectives_of: Box<dyn Fn(&T) -> Vec<f64>>,
    dominance: Box<dyn Dominance>,
    members: Vec<T>,
}

impl<T> ParetoArchive<T>
where
    T: Clone,
{
    /// 创建 Pareto 档案 / Create a Pareto archive
    pub fn new(
        capacity: usize,
        objectives_of: Box<dyn Fn(&T) -> Vec<f64>>,
        dominance: Box<dyn Dominance>,
    ) -> Self {
        Self {
            capacity,
            objectives_of,
            dominance,
            members: Vec::new(),
        }
    }

    /// 档案内容 / Archive content
    pub fn items(&self) -> &[T] {
        &self.members
    }

    /// 档案大小 / Archive size
    pub fn size(&self) -> usize {
        self.members.len()
    }

    /// 清空档案 / Clear the archive
    pub fn clear(&mut self) {
        self.members.clear();
    }

    /// 提交一个候选 / Offer one candidate
    pub fn offer_one(&mut self, candidate: T) -> &[T] {
        self.offer(vec![candidate]);
        &self.members
    }

    /// 提交一批候选 / Offer a batch of candidates
    pub fn offer(&mut self, candidates: Vec<T>) {
        for candidate in candidates {
            let candidate_objectives = (self.objectives_of)(&candidate);

            // 已被档案中某个成员支配：直接丢弃。
            // Dominated by an archive member: discard outright.
            if self
                .members
                .iter()
                .any(|member| self.dominance.dominates(&(self.objectives_of)(member), &candidate_objectives))
            {
                continue;
            }
            // 目标向量完全相同的重复解：保留已有的一份。
            // Duplicate with an identical objective vector: keep the existing one.
            if self
                .members
                .iter()
                .any(|member| (self.objectives_of)(member) == candidate_objectives)
            {
                continue;
            }

            // 新解支配了档案成员：淘汰它们。
            // The newcomer dominates some members: evict them.
            let dominance = &self.dominance;
            let objectives_of = &self.objectives_of;
            self.members
                .retain(|member| !dominance.dominates(&candidate_objectives, &(objectives_of)(member)));
            self.members.push(candidate);
        }

        self.trim();
    }

    /// 按拥挤距离裁剪到容量 / Truncate to capacity by crowding distance
    fn trim(&mut self) {
        if self.members.len() <= self.capacity {
            return;
        }

        let objectives: Vec<Vec<f64>> = self
            .members
            .iter()
            .map(|member| (self.objectives_of)(member))
            .collect();
        let distances = CrowdingDistance::assign(&objectives);
        let mut retained: Vec<usize> = (0..self.members.len()).collect();
        retained.sort_by(|lhs, rhs| {
            distances[*rhs]
                .partial_cmp(&distances[*lhs])
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| lhs.cmp(rhs))
        });
        retained.truncate(self.capacity);

        let survivors: Vec<T> = retained
            .into_iter()
            .map(|index| self.members[index].clone())
            .collect();
        self.members = survivors;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::heuristic::operator::multi_objective::dominance::ParetoDominance;

    #[test]
    fn archive_keeps_only_non_dominated_members() {
        let mut archive = ParetoArchive::new(
            8,
            Box::new(|point: &Vec<f64>| point.clone()),
            Box::new(ParetoDominance::minimization(2)),
        );
        archive.offer(vec![vec![1.0, 3.0], vec![2.0, 2.0], vec![3.0, 1.0]]);
        archive.offer(vec![vec![2.0, 3.0]]);
        assert_eq!(archive.size(), 3);

        archive.offer(vec![vec![0.5, 0.5]]);
        assert_eq!(archive.size(), 1);
        assert_eq!(archive.items()[0], vec![0.5, 0.5]);
    }

    #[test]
    fn archive_trims_by_capacity() {
        let mut archive = ParetoArchive::new(
            2,
            Box::new(|point: &Vec<f64>| point.clone()),
            Box::new(ParetoDominance::minimization(2)),
        );
        archive.offer(vec![
            vec![1.0, 5.0],
            vec![2.0, 4.0],
            vec![3.0, 3.0],
            vec![4.0, 2.0],
            vec![5.0, 1.0],
        ]);
        assert_eq!(archive.size(), 2);
    }
}
