//! 非支配排序与拥挤距离 / Non-dominated sorting and crowding distance

use super::dominance::Dominance;

/// 快速非支配排序（NSGA-II 的分层步骤）/ Fast non-dominated sorting (the layering step of NSGA-II)
///
/// 每一层内部的解互不支配，层号越小越优；第 1 层就是整个候选集的 Pareto 前沿。
/// Solutions within a front are mutually non-dominated and a smaller front index is better; front 1 is the Pareto front of
/// the whole candidate set.
#[derive(Debug, Clone, Copy, Default)]
pub struct FastNonDominatedSort;

impl FastNonDominatedSort {
    /// 对候选集分层 / Layer the candidate set
    pub fn sort<T: Clone>(
        items: &[T],
        objectives_of: impl Fn(&T) -> Vec<f64>,
        dominance: &dyn Dominance,
    ) -> Vec<Vec<T>> {
        let size = items.len();
        if size == 0 {
            return Vec::new();
        }

        let objectives: Vec<Vec<f64>> = items.iter().map(&objectives_of).collect();
        let mut dominated: Vec<Vec<usize>> = vec![Vec::new(); size];
        let mut dominated_amount = vec![0_usize; size];
        let mut current: Vec<usize> = Vec::new();

        for left in 0..size {
            for right in 0..size {
                if left == right {
                    continue;
                }
                if dominance.dominates(&objectives[left], &objectives[right]) {
                    dominated[left].push(right);
                } else if dominance.dominates(&objectives[right], &objectives[left]) {
                    dominated_amount[left] += 1;
                }
            }
            if dominated_amount[left] == 0 {
                current.push(left);
            }
        }

        let mut visited = vec![false; size];
        let mut fronts: Vec<Vec<usize>> = Vec::new();
        while !current.is_empty() {
            for index in &current {
                visited[*index] = true;
            }
            fronts.push(current.clone());

            let mut next = Vec::new();
            for index in &current {
                for dominated_index in &dominated[*index] {
                    if visited[*dominated_index] {
                        continue;
                    }
                    dominated_amount[*dominated_index] =
                        dominated_amount[*dominated_index].saturating_sub(1);
                    if dominated_amount[*dominated_index] == 0 {
                        next.push(*dominated_index);
                    }
                }
            }
            current = next;
        }

        fronts
            .into_iter()
            .map(|front| front.into_iter().map(|index| items[index].clone()).collect())
            .collect()
    }
}

/// 拥挤距离（NSGA-II 的多样性度量）/ Crowding distance (the diversity measure of NSGA-II)
///
/// 同一层内，每个解在每一维上取左右邻居的归一化间隔之和；边界解取正无穷以强制保留。
/// Within a front, each solution accumulates the normalised gap to its neighbours on every objective; boundary solutions
/// get positive infinity so that they are always retained.
#[derive(Debug, Clone, Copy, Default)]
pub struct CrowdingDistance;

impl CrowdingDistance {
    /// 计算拥挤距离 / Compute crowding distances
    pub fn assign(objectives: &[Vec<f64>]) -> Vec<f64> {
        let size = objectives.len();
        if size == 0 {
            return Vec::new();
        }
        if size <= 2 {
            return vec![f64::INFINITY; size];
        }

        let mut distance = vec![0.0; size];
        let dimension = objectives.first().map(|row| row.len()).unwrap_or(0);
        for index in 0..dimension {
            let mut order: Vec<usize> = (0..size).collect();
            order.sort_by(|lhs, rhs| {
                objectives[*lhs][index]
                    .partial_cmp(&objectives[*rhs][index])
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let minimum = objectives[order[0]][index];
            let maximum = objectives[order[size - 1]][index];
            distance[order[0]] = f64::INFINITY;
            distance[order[size - 1]] = f64::INFINITY;

            let span = maximum - minimum;
            if span <= 0.0 {
                continue;
            }
            for position in 1..size - 1 {
                let next = objectives[order[position + 1]][index];
                let previous = objectives[order[position - 1]][index];
                distance[order[position]] += (next - previous) / span;
                if !objectives[order[position]][index].is_finite() {
                    distance[order[position]] = f64::INFINITY;
                }
            }
        }
        distance
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::heuristic::operator::multi_objective::dominance::ParetoDominance;

    #[test]
    fn fronts_layer_a_two_objective_population() {
        let points = vec![
            vec![1.0, 4.0],
            vec![2.0, 3.0],
            vec![3.0, 2.0],
            vec![4.0, 1.0],
            vec![5.0, 5.0],
        ];
        let fronts = FastNonDominatedSort::sort(
            &points,
            |point| point.clone(),
            &ParetoDominance::minimization(2),
        );
        assert_eq!(fronts.len(), 2);
        assert_eq!(fronts[0].len(), 4);
        assert_eq!(fronts[1], vec![vec![5.0, 5.0]]);
    }

    #[test]
    fn crowding_distance_marks_boundaries_infinite() {
        let distances = CrowdingDistance::assign(&[
            vec![0.0, 2.0],
            vec![1.0, 1.0],
            vec![2.0, 0.0],
        ]);
        assert!(distances[0].is_infinite());
        assert!(distances[2].is_infinite());
        assert!(distances[1].is_finite());
    }
}
