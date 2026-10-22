//! 分形维数估计 / Fractal dimension estimation

use std::collections::HashSet;

use crate::geometry::Point2;

use super::escape_time::{FractalError, FractalResult};

/// 一维最小二乘斜率 / One-dimensional least-squares slope
fn least_squares_slope(xs: &[f64], ys: &[f64]) -> Option<f64> {
    if xs.len() < 2 || xs.len() != ys.len() {
        return None;
    }

    let count = xs.len() as f64;
    let mean_x = xs.iter().sum::<f64>() / count;
    let mean_y = ys.iter().sum::<f64>() / count;

    let mut numerator = 0.0;
    let mut denominator = 0.0;
    for index in 0..xs.len() {
        let dx = xs[index] - mean_x;
        numerator += dx * (ys[index] - mean_y);
        denominator += dx * dx;
    }

    if denominator <= 0.0 {
        None
    } else {
        Some(numerator / denominator)
    }
}

/// 用盒计数法估计二维点集的分形维数 / Estimate the fractal dimension of a 2D point set by box counting
///
/// 在归一化包围盒上按 `scales` 逐级划分网格，统计非空盒数量 `N(ε)`，由 `ln N(ε)` 对 `ln ε` 的斜率得到维数 D。
/// Divides the normalized bounding box into grids at each scale, counts the non-empty boxes `N(ε)`, and derives the
/// dimension D from the slope of `ln N(ε)` against `ln ε`.
pub fn box_counting_dimension(points: &[Point2<f64>]) -> FractalResult<f64> {
    box_counting_dimension_with(points, &[2, 4, 8, 16, 32])
}

/// 以指定网格划分数估计盒计数维数 / Estimate the box-counting dimension with explicit grid scales
pub fn box_counting_dimension_with(
    points: &[Point2<f64>],
    scales: &[usize],
) -> FractalResult<f64> {
    if points.len() < 2 {
        return Err(FractalError::new(
            "at least two points are required / 至少需要两个点",
        ));
    }
    if scales.len() < 2 || scales.iter().any(|scale| *scale < 2) {
        return Err(FractalError::new(
            "scales must contain at least two values >= 2 / 网格划分数需至少两个不小于 2 的值",
        ));
    }

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for point in points {
        min_x = min_x.min(point.x());
        max_x = max_x.max(point.x());
        min_y = min_y.min(point.y());
        max_y = max_y.max(point.y());
    }

    let width = max_x - min_x;
    let height = max_y - min_y;
    if width <= 0.0 && height <= 0.0 {
        return Err(FractalError::new(
            "points are degenerate / 点集退化，无法估计维数",
        ));
    }

    let mut log_epsilons = Vec::with_capacity(scales.len());
    let mut log_counts = Vec::with_capacity(scales.len());
    for scale in scales {
        let scale = *scale;
        let mut occupied: HashSet<usize> = HashSet::new();
        for point in points {
            let ix = if width <= 0.0 {
                0
            } else {
                (((point.x() - min_x) / width) * scale as f64) as usize
            };
            let iy = if height <= 0.0 {
                0
            } else {
                (((point.y() - min_y) / height) * scale as f64) as usize
            };
            occupied.insert(iy.min(scale - 1) * scale + ix.min(scale - 1));
        }
        if occupied.is_empty() {
            continue;
        }
        log_epsilons.push((1.0 / scale as f64).ln());
        log_counts.push((occupied.len() as f64).ln());
    }

    let slope = least_squares_slope(&log_epsilons, &log_counts).ok_or_else(|| {
        FractalError::new("not enough scales to fit the dimension / 网格划分数不足，无法拟合维数")
    })?;
    Ok(-slope)
}

/// 用相关维数（Grassberger–Procaccia）估计解集的分形维数 / Estimate the fractal dimension of a solution set by correlation dimension
///
/// 以成对距离的经验分布近似相关积分 `C(r)`，由 `ln C(r)` 对 `ln r` 的斜率得到维数 D。
/// Approximates the correlation integral `C(r)` from the empirical pairwise-distance distribution, and derives D from the
/// slope of `ln C(r)` against `ln r`.
pub fn correlation_dimension(solutions: &[Vec<f64>]) -> FractalResult<f64> {
    correlation_dimension_with(solutions, 12, 20_000)
}

/// 以显式半径数量与成对采样上限估计相关维数 / Estimate the correlation dimension with explicit radius count and pair sample limit
pub fn correlation_dimension_with(
    solutions: &[Vec<f64>],
    radius_amount: usize,
    pair_sample_limit: usize,
) -> FractalResult<f64> {
    if solutions.len() < 2 {
        return Err(FractalError::new(
            "at least two solutions are required / 至少需要两个解",
        ));
    }
    if radius_amount < 2 {
        return Err(FractalError::new(
            "radiusAmount must be at least 2 / 半径采样数量至少为 2",
        ));
    }
    if pair_sample_limit < 2 {
        return Err(FractalError::new(
            "pairSampleLimit must be at least 2 / 成对采样数量至少为 2",
        ));
    }

    let dimension = solutions[0].len();
    if dimension == 0 {
        return Err(FractalError::new(
            "solution vectors must be non-empty / 解向量不得为空",
        ));
    }
    if solutions.iter().any(|solution| solution.len() != dimension) {
        return Err(FractalError::new(
            "solution vectors must share the same dimension / 解向量必须维度一致",
        ));
    }

    let mut distances: Vec<f64> = Vec::new();
    'outer: for i in 0..solutions.len() {
        for j in (i + 1)..solutions.len() {
            let mut squared = 0.0;
            for k in 0..dimension {
                let difference = solutions[i][k] - solutions[j][k];
                squared += difference * difference;
            }
            distances.push(squared.sqrt());
            if distances.len() >= pair_sample_limit {
                break 'outer;
            }
        }
    }

    if distances.len() < 2 {
        return Err(FractalError::new(
            "not enough pairwise distances / 成对距离数量不足",
        ));
    }

    let maximum = distances.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if maximum <= 0.0 {
        return Err(FractalError::new(
            "all solutions are identical / 所有解完全相同，无法估计维数",
        ));
    }

    let mut log_radii = Vec::with_capacity(radius_amount);
    let mut log_correlations = Vec::with_capacity(radius_amount);
    let total = distances.len() as f64;
    for index in 0..radius_amount {
        let ratio =
            0.01 * (0.98 / 0.01_f64).powf(index as f64 / (radius_amount - 1) as f64);
        let radius = maximum * ratio;
        let correlated = distances.iter().filter(|value| **value < radius).count();
        if correlated == 0 {
            continue;
        }
        log_radii.push(radius.ln());
        log_correlations.push((correlated as f64 / total).ln());
    }

    least_squares_slope(&log_radii, &log_correlations).ok_or_else(|| {
        FractalError::new("not enough radii to fit the dimension / 半径采样不足，无法拟合维数")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_counting_dimension_of_a_line_is_close_to_one() {
        let points: Vec<Point2<f64>> = (0..64)
            .map(|index| Point2::new(index as f64 / 63.0, 0.0))
            .collect();
        let dimension = box_counting_dimension(&points).expect("dimension should be estimated");
        assert!((dimension - 1.0).abs() < 0.25, "dimension = {dimension}");
    }

    #[test]
    fn correlation_dimension_estimates_a_line() {
        let solutions: Vec<Vec<f64>> = (0..32)
            .map(|index| vec![index as f64 / 31.0, 0.0])
            .collect();
        let dimension =
            correlation_dimension(&solutions).expect("dimension should be estimated");
        assert!((dimension - 1.0).abs() < 0.25, "dimension = {dimension}");
    }

    #[test]
    fn degenerate_inputs_are_rejected() {
        assert!(box_counting_dimension(&[Point2::new(0.0, 0.0)]).is_err());
        let identical: Vec<Vec<f64>> = vec![vec![1.0, 2.0], vec![1.0, 2.0]];
        assert!(correlation_dimension(&identical).is_err());
    }
}
