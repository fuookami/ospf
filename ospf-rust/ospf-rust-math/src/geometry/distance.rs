//! 距离度量模块
//! Distance metric module
//!
//! 本模块定义了不同类型的距离度量：
//! This module defines different types of distance metrics:
//!
//! - [`Euclidean`] - 欧几里得距离（L2 范数）/ Euclidean distance (L2 norm)
//! - [`Manhattan`] - 曼哈顿距离（L1 范数）/ Manhattan distance (L1 norm)
//! - [`Chebyshev`] - 切比雪夫距离（L∞ 范数）/ Chebyshev distance (L∞ norm)
//! - [`Minkowski`] - 闵可夫斯基距离 / Minkowski distance

use crate::algebra::Field;
use num_traits::Float;

// ============================================================================
// Distance Trait - 距离度量 trait
// ============================================================================

/// Distance - 距离度量 trait
/// Distance - Distance metric trait
///
/// 定义两点之间的距离计算方法。
/// Defines the method for calculating distance between two points.
///
/// # 泛型参数 / Generic Parameters
/// - `S`: 标量类型，必须实现 `Field + Float`
/// - `S`: Scalar type, must implement `Field + Float`
pub trait Distance<S: Field + Float>: Clone {
    /// 计算两点之间的距离
    /// Calculate the distance between two points
    ///
    /// # 参数 / Parameters
    /// - `coords1`: 第一个点的坐标 / Coordinates of the first point
    /// - `coords2`: 第二个点的坐标 / Coordinates of the second point
    ///
    /// # 返回值 / Returns
    /// 两点之间的距离 / The distance between the two points
    fn distance(&self, coords1: &[S], coords2: &[S]) -> S;
}

// ============================================================================
// Euclidean Distance - 欧几里得距离
// ============================================================================

/// Euclidean - 欧几里得距离
/// Euclidean - Euclidean distance
///
/// L2 范数距离，是最常用的距离度量。
/// L2 norm distance, the most commonly used distance metric.
///
/// 公式 / Formula: `d(p, q) = √(Σᵢ(pᵢ - qᵢ)²)`
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::geometry::distance::{Distance, Euclidean};
///
/// let euclidean = Euclidean;
/// let p = [0.0_f64, 0.0];
/// let q = [3.0, 4.0];
/// let dist = euclidean.distance(&p, &q);
/// assert!((dist - 5.0).abs() < 1e-10);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Euclidean;

impl<S: Field + Float> Distance<S> for Euclidean {
    fn distance(&self, coords1: &[S], coords2: &[S]) -> S {
        coords1
            .iter()
            .zip(coords2.iter())
            .fold(S::zero(), |acc, (&a, &b)| {
                let diff = a - b;
                acc + diff * diff
            })
            .sqrt()
    }
}

// ============================================================================
// Manhattan Distance - 曼哈顿距离
// ============================================================================

/// Manhattan - 曼哈顿距离
/// Manhattan - Manhattan distance
///
/// L1 范数距离，也称为城市街区距离。
/// L1 norm distance, also known as city block distance.
///
/// 公式 / Formula: `d(p, q) = Σᵢ|pᵢ - qᵢ|`
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::geometry::distance::{Distance, Manhattan};
///
/// let manhattan = Manhattan;
/// let p = [0.0_f64, 0.0];
/// let q = [3.0, 4.0];
/// let dist = manhattan.distance(&p, &q);
/// assert!((dist - 7.0).abs() < 1e-10);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Manhattan;

impl<S: Field + Float> Distance<S> for Manhattan {
    fn distance(&self, coords1: &[S], coords2: &[S]) -> S {
        coords1
            .iter()
            .zip(coords2.iter())
            .fold(S::zero(), |acc, (&a, &b)| acc + (a - b).abs())
    }
}

// ============================================================================
// Chebyshev Distance - 切比雪夫距离
// ============================================================================

/// Chebyshev - 切比雪夫距离
/// Chebyshev - Chebyshev distance
///
/// L∞ 范数距离，取各坐标差的最大值。
/// L∞ norm distance, taking the maximum of coordinate differences.
///
/// 公式 / Formula: `d(p, q) = maxᵢ|pᵢ - qᵢ|`
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::geometry::distance::{Distance, Chebyshev};
///
/// let chebyshev = Chebyshev;
/// let p = [0.0_f64, 0.0];
/// let q = [3.0, 4.0];
/// let dist = chebyshev.distance(&p, &q);
/// assert!((dist - 4.0).abs() < 1e-10);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Chebyshev;

impl<S: Field + Float> Distance<S> for Chebyshev {
    fn distance(&self, coords1: &[S], coords2: &[S]) -> S {
        coords1
            .iter()
            .zip(coords2.iter())
            .fold(S::zero(), |acc, (&a, &b)| {
                let diff = (a - b).abs();
                if diff > acc { diff } else { acc }
            })
    }
}

// ============================================================================
// Minkowski Distance - 闵可夫斯基距离
// ============================================================================

/// Minkowski - 闵可夫斯基距离
/// Minkowski - Minkowski distance
///
/// Lp 范数距离，是欧几里得距离和曼哈顿距离的推广。
/// Lp norm distance, a generalization of Euclidean and Manhattan distances.
///
/// 公式 / Formula: `d(p, q) = (Σᵢ|pᵢ - qᵢ|ᵖ)^(1/p)`
///
/// 当 p = 1 时为曼哈顿距离，p = 2 时为欧几里得距离，p → ∞ 时为切比雪夫距离。
/// When p = 1, it's Manhattan distance; p = 2, it's Euclidean distance; p → ∞, it's Chebyshev distance.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::geometry::distance::{Distance, Minkowski};
///
/// let minkowski = Minkowski::new(2.0_f64);
/// let p = [0.0_f64, 0.0];
/// let q = [3.0, 4.0];
/// let dist = minkowski.distance(&p, &q);
/// assert!((dist - 5.0).abs() < 1e-10);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Minkowski<S: Field + Float> {
    /// 范数参数 p / Norm parameter p
    pub p: S,
}

impl<S: Field + Float> Minkowski<S> {
    /// 创建新的闵可夫斯基距离度量
    /// Create a new Minkowski distance metric
    ///
    /// # 参数 / Parameters
    /// - `p`: 范数参数，必须大于 0 / Norm parameter, must be greater than 0
    pub fn new(p: S) -> Self {
        assert!(p > S::zero(), "p must be greater than 0");
        Self { p }
    }
}

impl<S: Field + Float> Distance<S> for Minkowski<S> {
    fn distance(&self, coords1: &[S], coords2: &[S]) -> S {
        let sum = coords1
            .iter()
            .zip(coords2.iter())
            .fold(S::zero(), |acc, (&a, &b)| {
                let diff = (a - b).abs();
                acc + diff.powf(self.p)
            });
        sum.powf(S::one() / self.p)
    }
}

impl<S: Field + Float + Default> Default for Minkowski<S> {
    fn default() -> Self {
        Self::new(S::one() + S::one()) // p = 2 (Euclidean)
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_euclidean_distance() {
        let euclidean = Euclidean;

        // 测试 2D / Test 2D
        let p = [0.0_f64, 0.0];
        let q = [3.0, 4.0];
        assert!((euclidean.distance(&p, &q) - 5.0).abs() < 1e-10);

        // 测试 3D / Test 3D
        let p3 = [0.0_f64, 0.0, 0.0];
        let q3 = [1.0, 2.0, 2.0];
        assert!((euclidean.distance(&p3, &q3) - 3.0).abs() < 1e-10);

        // 测试相同点 / Test same point
        assert!(euclidean.distance(&p, &p).abs() < 1e-10);
    }

    #[test]
    fn test_manhattan_distance() {
        let manhattan = Manhattan;

        let p = [0.0_f64, 0.0];
        let q = [3.0, 4.0];
        assert!((manhattan.distance(&p, &q) - 7.0).abs() < 1e-10);

        // 测试 3D / Test 3D
        let p3 = [0.0_f64, 0.0, 0.0];
        let q3 = [1.0, 2.0, 2.0];
        assert!((manhattan.distance(&p3, &q3) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_chebyshev_distance() {
        let chebyshev = Chebyshev;

        let p = [0.0_f64, 0.0];
        let q = [3.0, 4.0];
        assert!((chebyshev.distance(&p, &q) - 4.0).abs() < 1e-10);

        // 测试 3D / Test 3D
        let p3 = [0.0_f64, 0.0, 0.0];
        let q3 = [1.0, 5.0, 2.0];
        assert!((chebyshev.distance(&p3, &q3) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_minkowski_distance() {
        // p = 1 (Manhattan) / p = 1 (Manhattan)
        let minkowski1 = Minkowski::new(1.0_f64);
        let p = [0.0_f64, 0.0];
        let q = [3.0, 4.0];
        assert!((minkowski1.distance(&p, &q) - 7.0).abs() < 1e-10);

        // p = 2 (Euclidean) / p = 2 (Euclidean)
        let minkowski2 = Minkowski::new(2.0_f64);
        assert!((minkowski2.distance(&p, &q) - 5.0).abs() < 1e-10);

        // p = 3 / p = 3
        let minkowski3 = Minkowski::new(3.0_f64);
        let dist = minkowski3.distance(&p, &q);
        // (3^3 + 4^3)^(1/3) = (27 + 64)^(1/3) = 91^(1/3) ≈ 4.498
        assert!((dist - 4.497941445).abs() < 1e-6);
    }

    #[test]
    fn test_distance_trait() {
        fn compute_distance<D: Distance<f64>>(metric: &D, p: &[f64], q: &[f64]) -> f64 {
            metric.distance(p, q)
        }

        let p = [0.0_f64, 0.0];
        let q = [3.0, 4.0];

        assert!((compute_distance(&Euclidean, &p, &q) - 5.0).abs() < 1e-10);
        assert!((compute_distance(&Manhattan, &p, &q) - 7.0).abs() < 1e-10);
        assert!((compute_distance(&Chebyshev, &p, &q) - 4.0).abs() < 1e-10);
    }
}
