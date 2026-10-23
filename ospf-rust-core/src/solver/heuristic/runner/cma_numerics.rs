//! CMA 数值与高斯采样协议 / CMA numerical and Gaussian sampling contracts

use crate::solver::heuristic::entropy::{Entropy, unit};
use crate::solver::heuristic::operator::linear::{SymmetricEigen, symmetric_eigen};

/// CMA 数值模式；显式协议不改变历史默认行为 / CMA numerical mode; explicit contracts preserve historical defaults
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CmaNumerics {
    /// 平台原生 Box-Muller 与特征分解 / Platform-native Box-Muller and eigendecomposition
    #[default]
    Native,
    /// 极坐标高斯、fdlibm 与固定 Jacobi 顺序 / Polar Gaussian, fdlibm and fixed Jacobi order
    PolarFdlibmV1,
}

impl CmaNumerics {
    pub(crate) fn sum(self, values: impl Iterator<Item = f64>) -> f64 {
        match self {
            Self::Native => values.sum(),
            Self::PolarFdlibmV1 => values.fold(0.0, |total, value| total + value),
        }
    }

    pub(crate) fn ln(self, value: f64) -> f64 {
        match self { Self::Native => value.ln(), Self::PolarFdlibmV1 => super::fdlibm_log::log(value) }
    }

    pub(crate) fn sqrt(self, value: f64) -> f64 {
        match self { Self::Native => value.sqrt(), Self::PolarFdlibmV1 => libm::sqrt(value) }
    }

    pub(crate) fn exp(self, value: f64) -> f64 {
        match self { Self::Native => value.exp(), Self::PolarFdlibmV1 => libm::exp(value) }
    }

    pub(crate) fn pow(self, base: f64, exponent: f64) -> f64 {
        match self { Self::Native => base.powf(exponent), Self::PolarFdlibmV1 => libm::pow(base, exponent) }
    }

    pub(crate) fn gaussian(self, entropy: &mut dyn Entropy) -> f64 {
        if self == Self::Native {
            let first = unit(entropy).max(1.0e-12);
            let second = unit(entropy);
            return (-2.0 * first.ln()).sqrt() * (2.0 * std::f64::consts::PI * second).cos();
        }
        // 每次仅使用极坐标对的第一分量，不缓存第二分量，检查点无需隐藏采样状态。
        // Use only the first polar component without caching its partner, leaving no hidden checkpoint state.
        loop {
            let x = 2.0 * unit(entropy) - 1.0;
            let y = 2.0 * unit(entropy) - 1.0;
            let radius = x * x + y * y;
            if radius > 0.0 && radius < 1.0 {
                return x * self.sqrt((-2.0 * self.ln(radius)) / radius);
            }
        }
    }

    pub(crate) fn eigen(self, matrix: &[Vec<f64>]) -> Option<SymmetricEigen> {
        if self == Self::Native { return symmetric_eigen(matrix); }
        let n = matrix.len();
        if n == 0 || matrix.iter().any(|row| row.len() != n || row.iter().any(|x| !x.is_finite())) { return None; }
        let mut a = matrix.to_vec();
        let mut vectors: Vec<Vec<f64>> = (0..n).map(|row|
            (0..n).map(|column| if row == column { 1.0 } else { 0.0 }).collect()).collect();
        for _ in 0..60 {
            let mut off_diagonal = 0.0;
            for (index, row) in a.iter().enumerate() {
                for value in row.iter().skip(index + 1) { off_diagonal += value * value; }
            }
            if off_diagonal <= 1.0e-12 { break; }
            for first in 0..n {
                for second in first + 1..n {
                    let pair = a[first][second];
                    if pair.abs() <= 1.0e-12 { continue; }
                    let theta = (a[second][second] - a[first][first]) / (2.0 * pair);
                    let sign = if theta >= 0.0 { 1.0 } else { -1.0 };
                    let t = sign / (theta.abs() + self.sqrt(theta * theta + 1.0));
                    let cosine = 1.0 / self.sqrt(t * t + 1.0);
                    let sine = t * cosine;
                    for row in &mut a {
                        let left = row[first]; let right = row[second];
                        row[first] = cosine * left - sine * right;
                        row[second] = sine * left + cosine * right;
                    }
                    let (before_second, from_second) = a.split_at_mut(second);
                    for (left, right) in before_second[first].iter_mut().zip(&mut from_second[0]) {
                        let upper = *left; let lower = *right;
                        *left = cosine * upper - sine * lower;
                        *right = sine * upper + cosine * lower;
                    }
                    for row in &mut vectors {
                        let left = row[first]; let right = row[second];
                        row[first] = cosine * left - sine * right;
                        row[second] = sine * left + cosine * right;
                    }
                }
            }
        }
        let values: Vec<_> = (0..n).map(|i| a[i][i]).collect();
        if values.iter().any(|x| !x.is_finite()) { return None; }
        Some(SymmetricEigen { values, vectors: (0..n).map(|column| (0..n).map(|row| vectors[row][column]).collect()).collect() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::heuristic::entropy::FnEntropy;

    #[test]
    fn fdlibm_primitives_match_shared_binary64_vectors() {
        let mode = CmaNumerics::PolarFdlibmV1;
        for (input, bits) in [(f64::from_bits(1), 0xc0874385446d71c3), (1e-12, 0xc03ba18a998fffa0),
            (0.125, 0xc000a2b23f3bab73), (0.75, 0xbfd269621134db92), (1.0, 0), (4.0, 0x3ff62e42fefa39ef)] {
            assert_eq!(mode.ln(input).to_bits(), bits);
        }
        for (input, bits) in [(-745.0, 1), (-27.6, 0x3d72257eee500edc), (-0.1, 0x3fecf46d99d52b3a),
            (0.0, 0x3ff0000000000000), (0.1, 0x3ff1aec7b35a00d4), (27.6, 0x426c3700dcfb6769), (709.0, 0x7fdd422d2be5dc9b)] {
            assert_eq!(mode.exp(input).to_bits(), bits);
        }
        for (input, bits) in [(f64::from_bits(1), 0x1e60000000000000), (1e-20, 0x3ddb7cdfd9d7bdbb), (0.75, 0x3febb67ae8584caa)] {
            assert_eq!(mode.sqrt(input).to_bits(), bits);
        }
        for (exponent, bits) in [(2.0, 0x3fe47ae147ae147c), (100.0, 0x3debff2ee48e055c), (1_000_000.0, 0)] {
            assert_eq!(mode.pow(0.8, exponent).to_bits(), bits);
        }
    }

    #[test]
    fn shared_new_component_transcendentals_keep_strictmath_rounding() {
        let mode = CmaNumerics::PolarFdlibmV1;
        assert_eq!(mode.ln(9170.0).to_bits(), 0x40223f54a1c504c2);
        assert_eq!(mode.pow(547.0 / 10001.0, 1.0 / 3.0 + 0.01).to_bits(), 0x3fd799126e159c7e);
        assert_eq!(mode.sqrt(mode.ln(200.0) / 246.0).to_bits(), 0x3fc2c8f6547a82c9);
    }

    #[test]
    fn polar_gaussian_rejects_zero_and_outer_pairs_without_hidden_cache() {
        let draws = [0.5, 0.5, 0.0, 0.0, 0.75, 0.625, 0.75, 0.625];
        let mut used = 0;
        let mut entropy = FnEntropy::new(|| { let value = draws[used]; used += 1; value });
        for _ in 0..2 { assert_eq!(CmaNumerics::PolarFdlibmV1.gaussian(&mut entropy).to_bits(), 0x3ff5d3c33f332f3d); }
        assert_eq!(used, 8);
    }

    #[test]
    fn logarithm_preserves_original_fdlibm_grouping_at_observed_polar_inputs() {
        for (input, expected) in [(0x3fed03d5192d1533, 0xbfb9118bde7c4c17), (0x3fe90a282aa45379, 0xbfcf65261fdd34cd), (0x3fe4263db49013ff, 0xbfdd9a9f74bbacfa)] {
            assert_eq!(CmaNumerics::PolarFdlibmV1.ln(f64::from_bits(input)).to_bits(), expected);
        }
    }

    #[test]
    fn shared_jacobi_returns_column_vectors_without_native_resorting() {
        let eigen = CmaNumerics::PolarFdlibmV1.eigen(&[vec![1.0, 0.0], vec![0.0, 3.0]]).unwrap();
        assert_eq!(eigen.values, vec![1.0, 3.0]);
        assert_eq!(eigen.vectors, vec![vec![1.0, 0.0], vec![0.0, 1.0]]);
        let rotated = CmaNumerics::PolarFdlibmV1.eigen(&[vec![2.0, 1.0], vec![1.0, 2.0]]).unwrap();
        assert_eq!(rotated.vectors[0][0].to_bits(), rotated.vectors[1][1].to_bits());
        assert!(CmaNumerics::PolarFdlibmV1.eigen(&[vec![f64::NAN]]).is_none());
    }
}
