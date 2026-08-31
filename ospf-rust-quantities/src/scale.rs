
//! Scale - 比例尺
//! Scale - Unit scale for physical quantities
//!
//! 保持 base^exponent 形式的符号运算，避免精度损失
//! Maintains base^exponent form for symbolic computation, avoiding precision loss

use std::cmp::Ordering;
use std::ops::{Div, Mul};
use std::sync::OnceLock;
use bigdecimal::{BigDecimal, FromPrimitive};
use num_bigint::BigInt;
use num_rational::BigRational;
use once_cell::sync::Lazy;
use ospf_rust_math::operator::reciprocal::Reciprocal;
use ospf_rust_math::ordinary;

/// ScaleBase - 高精度数值基
/// ScaleBase - High-precision numeric base
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScaleBase {
    /// 任意精度十进制数
    /// Arbitrary precision decimal number
    Float(BigDecimal),

    /// 任意精度有理数
    /// Arbitrary precision rational number
    Rational(BigRational),
}

impl ScaleBase {
    /// 创建十进制基
    /// Create a decimal base
    pub fn float(value: BigDecimal) -> Self {
        ScaleBase::Float(value)
    }

    /// 创建有理数基
    /// Create a rational base
    pub fn rational(num: BigInt, den: BigInt) -> Self {
        ScaleBase::Rational(BigRational::new(num, den))
    }

    /// 转换为 BigDecimal
    /// Convert to BigDecimal
    pub fn to_big_decimal(&self) -> BigDecimal {
        match self {
            ScaleBase::Float(f) => f.clone(),
            ScaleBase::Rational(r) => {
                BigDecimal::from(r.numer().clone()) / BigDecimal::from(r.denom().clone())
            }
        }
    }
}

impl PartialOrd for ScaleBase {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScaleBase {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_big_decimal().cmp(&other.to_big_decimal())
    }
}

/// ScaleFactor - 比例因子: base^exponent
/// ScaleFactor - Scale factor: base^exponent
#[derive(Debug, Clone, PartialEq)]
pub struct ScaleFactor {
    /// 底数 / Base
    pub base: ScaleBase,
    /// 指数 / Exponent
    pub exponent: BigDecimal,
}

impl ScaleFactor {
    /// 创建新的比例因子
    /// Create a new scale factor
    pub fn new(base: ScaleBase, exponent: BigDecimal) -> Self {
        Self { base, exponent }
    }

    /// 计算值: base^exponent
    /// Calculate value: base^exponent
    pub fn value(&self) -> BigDecimal {
        match &self.base {
            ScaleBase::Float(f) => ordinary::pow(f, &self.exponent),
            ScaleBase::Rational(r) => {
                let f: BigDecimal =
                    BigDecimal::from(r.numer().clone()) / BigDecimal::from(r.denom().clone());
                ordinary::pow(&f, &self.exponent)
            }
        }
    }
}

impl Eq for ScaleFactor {}

/// 整理比例因子：合并同类项、排序
/// Tidy scale factors: merge like terms, sort
fn tidy(scales: &Vec<ScaleFactor>) -> Vec<ScaleFactor> {
    // 移除指数为0的项
    // Remove items with zero exponent
    let mut non_zero: Vec<ScaleFactor> = scales
        .iter()
        .filter(|f| f.exponent != BigDecimal::from(0))
        .cloned()
        .collect();

    // 排序
    // Sort
    non_zero.sort_by(|a, b| a.base.cmp(&b.base));

    non_zero
}

/// Scale - 比例尺
/// Scale - Unit scale
///
/// 保持符号形式的比例尺，支持精确运算
/// Scale maintaining symbolic form, supporting exact computation
///
/// # Example / 示例
/// ```
/// use ospf_rust_quantities::scale::{Scale, KILO, MILLI};
///
/// let product = KILO.clone() * MILLI.clone();  // 10^3 * 10^-3 = 1
/// ```
#[derive(Clone, Debug)]
pub struct Scale {
    /// 比例因子列表 / List of scale factors
    /// 最终值 = product of all factors / Final value = all factors multiplied
    scales: Vec<ScaleFactor>,
    /// 比例尺的值 / Value of the scale
    value: OnceLock<BigDecimal>,
}

impl Scale {
    /// 创建空比例尺 (值为1)
    /// Create empty scale (value = 1)
    pub fn new() -> Self {
        Self {
            scales: Vec::new(),
            value: OnceLock::new(),
        }
    }

    /// 从底数和指数创建比例尺
    /// Create scale from base and exponent
    pub fn from_base_exponent(base: ScaleBase, exponent: BigDecimal) -> Self {
        if exponent == BigDecimal::from(0) {
            Self::new()
        } else {
            Self {
                scales: vec![ScaleFactor::new(base, exponent)],
                value: OnceLock::new(),
            }
        }
    }

    /// 从整数创建比例尺: base^1
    /// Create scale from integer: base^1
    pub fn from_int(base: i64) -> Self {
        if base == 1 {
            Self::new()
        } else {
            Self::from_base_exponent(
                ScaleBase::float(BigDecimal::from(base)),
                BigDecimal::from(1),
            )
        }
    }

    /// 从浮点数创建比例尺
    /// Create scale from float
    pub fn from_f64(base: f64) -> Self {
        if base == 1.0 {
            Self::new()
        } else {
            Self::from_base_exponent(
                ScaleBase::float(BigDecimal::from_f64(base).expect("无法从浮点数创建 BigDecimal / Failed to create BigDecimal from f64")),
                BigDecimal::from(1),
            )
        }
    }

    /// 从 BigDecimal 创建比例尺
    /// Create scale from BigDecimal
    pub fn from_big_decimal(base: BigDecimal) -> Self {
        if base == BigDecimal::from(1) {
            Self::new()
        } else {
            Self::from_base_exponent(ScaleBase::float(base), BigDecimal::from(1))
        }
    }

    /// 计算比例尺的值
    /// Calculate the value of the scale
    pub fn value(&self) -> &BigDecimal {
        self.value.get_or_init(|| {
            self.scales
                .iter()
                .fold(BigDecimal::from(1), |acc, factor| acc * factor.value())
        })
    }

    /// 幂运算
    /// Power operation
    pub fn pow(&self, exponent: &BigDecimal) -> Scale {
        let value = ordinary::pow(&self.value(), exponent);
        Scale::from_big_decimal(value)
    }
}

impl Default for Scale {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for Scale {
    fn eq(&self, other: &Self) -> bool {
        self.value() == other.value()
    }
}

impl Eq for Scale {}

// ============================================================================
// SI 前缀静态实例 / SI prefix static instances
// ============================================================================

/// atto: 10^-18
pub static ATTO: Lazy<Scale> = Lazy::new(|| {
    Scale::from_base_exponent(
        ScaleBase::float(BigDecimal::from(10)),
        BigDecimal::from(-18),
    )
});

/// femto: 10^-15
pub static FEMTO: Lazy<Scale> = Lazy::new(|| {
    Scale::from_base_exponent(
        ScaleBase::float(BigDecimal::from(10)),
        BigDecimal::from(-15),
    )
});

/// pico: 10^-12
pub static PICO: Lazy<Scale> = Lazy::new(|| {
    Scale::from_base_exponent(
        ScaleBase::float(BigDecimal::from(10)),
        BigDecimal::from(-12),
    )
});

/// nano: 10^-9
pub static NANO: Lazy<Scale> = Lazy::new(|| {
    Scale::from_base_exponent(ScaleBase::float(BigDecimal::from(10)), BigDecimal::from(-9))
});

/// micro: 10^-6
pub static MICRO: Lazy<Scale> = Lazy::new(|| {
    Scale::from_base_exponent(ScaleBase::float(BigDecimal::from(10)), BigDecimal::from(-6))
});

/// milli: 10^-3
pub static MILLI: Lazy<Scale> = Lazy::new(|| {
    Scale::from_base_exponent(ScaleBase::float(BigDecimal::from(10)), BigDecimal::from(-3))
});

/// centi: 10^-2
pub static CENTI: Lazy<Scale> = Lazy::new(|| {
    Scale::from_base_exponent(ScaleBase::float(BigDecimal::from(10)), BigDecimal::from(-2))
});

/// deci: 10^-1
pub static DECI: Lazy<Scale> = Lazy::new(|| {
    Scale::from_base_exponent(ScaleBase::float(BigDecimal::from(10)), BigDecimal::from(-1))
});

/// deca: 10^1
pub static DECA: Lazy<Scale> = Lazy::new(|| {
    Scale::from_base_exponent(ScaleBase::float(BigDecimal::from(10)), BigDecimal::from(1))
});

/// hecto: 10^2
pub static HECTO: Lazy<Scale> = Lazy::new(|| {
    Scale::from_base_exponent(ScaleBase::float(BigDecimal::from(10)), BigDecimal::from(2))
});

/// kilo: 10^3
pub static KILO: Lazy<Scale> = Lazy::new(|| {
    Scale::from_base_exponent(ScaleBase::float(BigDecimal::from(10)), BigDecimal::from(3))
});

/// mega: 10^6
pub static MEGA: Lazy<Scale> = Lazy::new(|| {
    Scale::from_base_exponent(ScaleBase::float(BigDecimal::from(10)), BigDecimal::from(6))
});

/// giga: 10^9
pub static GIGA: Lazy<Scale> = Lazy::new(|| {
    Scale::from_base_exponent(ScaleBase::float(BigDecimal::from(10)), BigDecimal::from(9))
});

/// tera: 10^12
pub static TERA: Lazy<Scale> = Lazy::new(|| {
    Scale::from_base_exponent(ScaleBase::float(BigDecimal::from(10)), BigDecimal::from(12))
});

/// peta: 10^15
pub static PETA: Lazy<Scale> = Lazy::new(|| {
    Scale::from_base_exponent(ScaleBase::float(BigDecimal::from(10)), BigDecimal::from(15))
});

/// exa: 10^18
pub static EXA: Lazy<Scale> = Lazy::new(|| {
    Scale::from_base_exponent(ScaleBase::float(BigDecimal::from(10)), BigDecimal::from(18))
});

// ============================================================================
// 进制 / Radix
// ============================================================================

/// 二进制 / Binary number
pub static BINARY: Lazy<Scale> = Lazy::new(|| Scale::from_int(2));

/// 八进制 / Octal number
pub static OCTAL: Lazy<Scale> = Lazy::new(|| Scale::from_int(8));

/// 六十进制数 / Sexagesimal number
pub static SEXAGESIMAL: Lazy<Scale> = Lazy::new(|| Scale::from_int(60));

// ============================================================================
// 运算符实现 / Operator implementations
// ============================================================================

impl Mul for Scale {
    type Output = Self;

    fn mul(mut self, rhs: Self) -> Self::Output {
        // 合并两个比例尺的因子
        // Merge factors from both scales
        for rhs_factor in rhs.scales {
            // 查找是否有相同底数的因子
            // Look for factor with same base
            if let Some(existing) = self.scales.iter_mut().find(|f| f.base == rhs_factor.base) {
                existing.exponent += rhs_factor.exponent;
            } else {
                self.scales.push(rhs_factor);
            }
        }
        Scale {
            scales: tidy(&self.scales),
            value: OnceLock::new(),
        }
    }
}

impl Mul<&Scale> for Scale {
    type Output = Self;

    fn mul(mut self, rhs: &Scale) -> Self::Output {
        for rhs_factor in &rhs.scales {
            // 查找是否有相同底数的因子
            // Look for factor with same base
            if let Some(existing) = self.scales.iter_mut().find(|f| f.base == rhs_factor.base) {
                existing.exponent += &rhs_factor.exponent;
            } else {
                self.scales.push(rhs_factor.clone());
            }
        }
        Scale {
            scales: tidy(&self.scales),
            value: OnceLock::new(),
        }
    }
}

impl Mul for &Scale {
    type Output = Scale;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut result = self.scales.clone();
        for rhs_factor in &rhs.scales {
            // 查找是否有相同底数的因子
            // Look for factor with same base
            let base_to_remove = {
                if let Some(existing) = result.iter_mut().find(|f| f.base == rhs_factor.base) {
                    existing.exponent += &rhs_factor.exponent;
                    if existing.exponent == BigDecimal::from(0) {
                        Some(existing.base.clone())
                    } else {
                        None
                    }
                } else {
                    result.push(rhs_factor.clone());
                    None
                }
            };
            if let Some(base) = base_to_remove {
                result.retain(|f| f.base != base);
            }
        }
        Scale {
            scales: result,
            value: OnceLock::new(),
        }
    }
}

impl Div for Scale {
    type Output = Self;

    fn div(mut self, rhs: Self) -> Self::Output {
        // 将 rhs 的指数取反后合并
        // Negate rhs exponents and merge
        for rhs_factor in rhs.scales {
            if let Some(existing) = self.scales.iter_mut().find(|f| f.base == rhs_factor.base) {
                existing.exponent -= rhs_factor.exponent;
            } else {
                self.scales
                    .push(ScaleFactor::new(rhs_factor.base, -rhs_factor.exponent));
            }
        }
        Scale {
            scales: tidy(&self.scales),
            value: OnceLock::new(),
        }
    }
}

impl Div<&Scale> for Scale {
    type Output = Self;

    fn div(mut self, rhs: &Scale) -> Self::Output {
        for rhs_factor in &rhs.scales {
            if let Some(existing) = self.scales.iter_mut().find(|f| f.base == rhs_factor.base) {
                existing.exponent -= &rhs_factor.exponent;
            } else {
                self.scales.push(ScaleFactor::new(
                    rhs_factor.base.clone(),
                    -&rhs_factor.exponent,
                ));
            }
        }
        Scale {
            scales: tidy(&self.scales),
            value: OnceLock::new(),
        }
    }
}

impl Div for &Scale {
    type Output = Scale;

    fn div(self, rhs: Self) -> Self::Output {
        let mut result = self.scales.clone();
        for rhs_factor in &rhs.scales {
            // 查找是否有相同底数的因子
            // Look for factor with same base
            let base_to_remove = {
                if let Some(existing) = result.iter_mut().find(|f| f.base == rhs_factor.base) {
                    existing.exponent += -&rhs_factor.exponent;
                    if existing.exponent == BigDecimal::from(0) {
                        Some(existing.base.clone())
                    } else {
                        None
                    }
                } else {
                    result.push(ScaleFactor::new(
                        rhs_factor.base.clone(),
                        -&rhs_factor.exponent,
                    ));
                    None
                }
            };
            if let Some(base) = base_to_remove {
                result.retain(|f| f.base != base);
            }
        }
        Scale {
            scales: result,
            value: OnceLock::new(),
        }
    }
}

impl Mul<i64> for Scale {
    type Output = Self;

    fn mul(mut self, rhs: i64) -> Self::Output {
        for factor in self.scales.iter_mut() {
            factor.exponent *= rhs;
        }
        Scale {
            scales: self.scales,
            value: OnceLock::new(),
        }
    }
}

impl Mul<i64> for &Scale {
    type Output = Scale;

    fn mul(self, rhs: i64) -> Self::Output {
        Scale {
            scales: self
                .scales
                .iter()
                .map(|f| ScaleFactor {
                    base: f.base.clone(),
                    exponent: &f.exponent * BigDecimal::from(rhs),
                })
                .collect(),
            value: OnceLock::new(),
        }
    }
}

impl Div<i64> for Scale {
    type Output = Self;

    fn div(mut self, rhs: i64) -> Self::Output {
        for factor in self.scales.iter_mut() {
            factor.exponent -= rhs;
        }
        Scale {
            scales: self.scales,
            value: OnceLock::new(),
        }
    }
}

impl Div<i64> for &Scale {
    type Output = Scale;

    fn div(self, rhs: i64) -> Self::Output {
        Scale {
            scales: self
                .scales
                .iter()
                .map(|f| ScaleFactor {
                    base: f.base.clone(),
                    exponent: &f.exponent - BigDecimal::from(rhs),
                })
                .collect(),
            value: OnceLock::new(),
        }
    }
}

impl Mul<f64> for Scale {
    type Output = Self;

    fn mul(mut self, rhs: f64) -> Self::Output {
        for factor in self.scales.iter_mut() {
            factor.exponent += BigDecimal::from_f64(rhs).expect("无法从浮点数创建 BigDecimal / Failed to create BigDecimal from f64")
        }
        Scale {
            scales: self.scales,
            value: OnceLock::new(),
        }
    }
}

impl Mul<f64> for &Scale {
    type Output = Scale;

    fn mul(self, rhs: f64) -> Self::Output {
        Scale {
            scales: self
                .scales
                .iter()
                .map(|f| ScaleFactor {
                    base: f.base.clone(),
                    exponent: &f.exponent + BigDecimal::from_f64(rhs).expect("无法从浮点数创建 BigDecimal / Failed to create BigDecimal from f64"),
                })
                .collect(),
            value: OnceLock::new(),
        }
    }
}

impl Div<f64> for Scale {
    type Output = Self;

    fn div(mut self, rhs: f64) -> Self::Output {
        for factor in self.scales.iter_mut() {
            factor.exponent -= BigDecimal::from_f64(rhs).expect("无法从浮点数创建 BigDecimal / Failed to create BigDecimal from f64");
        }
        Scale {
            scales: self.scales,
            value: OnceLock::new(),
        }
    }
}

impl Div<f64> for &Scale {
    type Output = Scale;

    fn div(self, rhs: f64) -> Self::Output {
        Scale {
            scales: self
                .scales
                .iter()
                .map(|f| ScaleFactor {
                    base: f.base.clone(),
                    exponent: &f.exponent - BigDecimal::from_f64(rhs).expect("无法从浮点数创建 BigDecimal / Failed to create BigDecimal from f64"),
                })
                .collect(),
            value: OnceLock::new(),
        }
    }
}

impl Reciprocal for Scale {
    type Output = Self;

    fn reciprocal(self) -> Self {
        Scale {
            scales: self
                .scales
                .iter()
                .map(|f| ScaleFactor {
                    base: f.base.clone(),
                    exponent: -&f.exponent,
                })
                .collect(),
            value: OnceLock::new(),
        }
    }
}

impl Reciprocal for &Scale {
    type Output = Scale;

    fn reciprocal(self) -> Scale {
        Scale {
            scales: self
                .scales
                .iter()
                .map(|f| ScaleFactor {
                    base: f.base.clone(),
                    exponent: -&f.exponent,
                })
                .collect(),
            value: OnceLock::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bigdecimal::Num;

    #[test]
    fn test_scale_creation() {
        let one = Scale::new();
        assert_eq!(one.value(), &BigDecimal::from(1));

        assert!(
            (KILO.value() - BigDecimal::from(1000)).abs()
                < BigDecimal::from_str_radix("0.0001", 10).unwrap()
        );
    }

    #[test]
    fn test_scale_multiplication() {
        let product = KILO.clone() * MILLI.clone();
        assert!(
            (product.value() - BigDecimal::from(1)).abs()
                < BigDecimal::from_str_radix("0.0001", 10).unwrap()
        );
    }

    #[test]
    fn test_scale_division() {
        let result = KILO.clone() / KILO.clone();
        println!("{:?}", result.value());
        assert!(
            (result.value() - BigDecimal::from(1)).abs()
                < BigDecimal::from_str_radix("0.0001", 10).unwrap()
        );
    }

    #[test]
    fn test_scale_with_float() {
        let inch = Scale::from_f64(2.54);
        let val = inch.value();
        assert!(val > &BigDecimal::from(2) && val < &BigDecimal::from(3));
    }
}
