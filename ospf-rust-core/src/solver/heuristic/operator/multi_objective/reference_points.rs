//! 参考点生成 / Reference point generation

/// NSGA-III 的参考点（Das–Dennis 构造）/ Reference points of NSGA-III (Das–Dennis construction)
///
/// 在 `m` 维单位单纯形上均匀取点：每个目标的坐标非负且和为 1。这些点定义「理想前沿应当朝哪些方向均匀铺开」，
/// 因此选择时按「哪个参考方向的成员最少」来补位。
/// Points are spread uniformly over the `m`-dimensional unit simplex: coordinates are non-negative and sum to one. These
/// points define "which directions the ideal front should cover", so selection fills the reference direction with the
/// fewest members.
#[derive(Debug, Clone, Copy, Default)]
pub struct ReferencePoints;

impl ReferencePoints {
    /// 生成 Das–Dennis 参考点 / Generate Das–Dennis reference points
    ///
    /// 参数非法时返回空列表（不 panic）。
    /// Returns an empty list for invalid arguments (never panics).
    pub fn das_dennis(objective_amount: usize, divisions: usize) -> Vec<Vec<f64>> {
        if objective_amount < 1 || divisions < 1 {
            return Vec::new();
        }

        let mut points = Vec::new();
        let mut current = Vec::with_capacity(objective_amount);

        fn generate(
            remaining: usize,
            objectives_left: usize,
            divisions: usize,
            current: &mut Vec<f64>,
            points: &mut Vec<Vec<f64>>,
        ) {
            if objectives_left == 1 {
                current.push(remaining as f64 / divisions as f64);
                points.push(current.clone());
                current.pop();
                return;
            }
            for share in 0..=remaining {
                current.push(share as f64 / divisions as f64);
                generate(remaining - share, objectives_left - 1, divisions, current, points);
                current.pop();
            }
        }

        generate(divisions, objective_amount, divisions, &mut current, &mut points);
        points
    }

    /// 计算个体到参考方向的垂直距离 / Perpendicular distance from a point to a reference direction
    ///
    /// 参考方向取自原点，因此垂直距离等于个体向量在参考方向上的正交分量长度。
    /// The reference direction starts at the origin, so the perpendicular distance equals the length of the component of the
    /// individual vector orthogonal to that direction.
    pub fn perpendicular_distance(objectives: &[f64], reference: &[f64]) -> f64 {
        let mut projection = 0.0;
        let mut reference_norm = 0.0;
        for index in 0..reference.len() {
            let (Some(objective), Some(direction)) = (objectives.get(index), reference.get(index))
            else {
                return f64::INFINITY;
            };
            projection += objective * direction;
            reference_norm += direction * direction;
        }
        if reference_norm <= 1.0e-12 {
            return f64::INFINITY;
        }

        let mut squared = 0.0;
        for index in 0..reference.len() {
            let component = objectives[index] - projection * reference[index] / reference_norm;
            squared += component * component;
        }
        squared.sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn das_dennis_point_count_follows_the_simplex_lattice() {
        assert_eq!(ReferencePoints::das_dennis(2, 4).len(), 5);
        assert_eq!(ReferencePoints::das_dennis(3, 4).len(), 15);
        assert!(ReferencePoints::das_dennis(0, 4).is_empty());
        assert!(ReferencePoints::das_dennis(3, 0).is_empty());
    }

    #[test]
    fn simplex_points_sum_to_one() {
        for point in ReferencePoints::das_dennis(3, 4) {
            let sum: f64 = point.iter().sum();
            assert!((sum - 1.0).abs() < 1.0e-12);
        }
    }

    #[test]
    fn perpendicular_distance_measures_orthogonal_component() {
        let distance = ReferencePoints::perpendicular_distance(&[1.0, 1.0], &[1.0, 0.0]);
        assert!((distance - 1.0).abs() < 1.0e-12);
        assert!(ReferencePoints::perpendicular_distance(&[1.0], &[0.0]).is_infinite());
    }
}
