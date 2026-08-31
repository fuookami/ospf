// MIT License
//
// Copyright (c) 2024 fuookami
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! BigDecimal 幂运算函数
//! BigDecimal power functions using Taylor series
//!
//! 使用泰勒级数实现高精度幂运算
//! High-precision power operations using Taylor series

use bigdecimal::{BigDecimal, Num, ToPrimitive};

/// 默认计算精度 (使用 f64 的有效数字位数)
/// Default computation precision (using f64's significant digits)
const DEFAULT_PRECISION: i64 = f64::DIGITS as i64;

// ============================================================================
// 自然对数 ln(x) / Natural logarithm ln(x)
// ============================================================================

/// 计算自然对数 ln(x) 使用泰勒级数 (指定精度)
/// Calculate natural logarithm ln(x) using Taylor series (with specified precision)
///
/// ln(x) = 2 * sum((x-1)/(x+1)^(2n+1) / (2n+1)) for n from 0 to infinity
/// 当 x 接近 1 时收敛很快
/// Converges quickly when x is close to 1
///
/// # Arguments / 参数
/// * `x` - 输入值，必须为正数 / Input value, must be positive
/// * `precision` - 计算精度（小数位数）/ Computation precision (decimal places)
///
/// # Panics / 恐慌
/// 当 x <= 0 时恐慌 / Panics when x <= 0
pub fn ln_with_precision(x: &BigDecimal, precision: i64) -> BigDecimal {
    if x <= &BigDecimal::from(0) {
        panic!("ln(x) is undefined for x <= 0 / ln(x) 对于 x <= 0 无定义");
    }

    let two = BigDecimal::from(2);
    let one = BigDecimal::from(1);

    // 使用二进制归一化: x = m * 2^k, where 0.5 <= m < 2
    // Use binary normalization: x = m * 2^k, where 0.5 <= m < 2
    // ln(x) = ln(m) + k * ln(2)
    // 这在计算机上更自然，收敛更快
    // This is more natural on computers and converges faster
    let mut k: i64 = 0;
    let mut m = x.clone();

    // 除以2直到 m < 2
    // Divide by 2 until m < 2
    while m >= two {
        m = &m / &two;
        k += 1;
    }

    // 乘以2直到 m >= 0.5
    // Multiply by 2 until m >= 0.5
    let half = BigDecimal::from(1) / &two;
    while m < half {
        m = &m * &two;
        k -= 1;
    }

    // 现在 0.5 <= m < 2，使用 ln(m) = -ln(1/m) 将范围转换到 m >= 1
    // Now 0.5 <= m < 2, use ln(m) = -ln(1/m) to convert range to m >= 1
    let (m_for_calc, k_adj): (BigDecimal, i64) = if m < one {
        (BigDecimal::from(1) / &m, k - 1)
    } else {
        (m.clone(), k)
    };

    // 使用 ln(x) = 2 * sum(((x-1)/(x+1))^(2n+1) / (2n+1))
    // Use ln(x) = 2 * sum(((x-1)/(x+1))^(2n+1) / (2n+1))
    let numerator = &m_for_calc - &one;
    let denominator = &m_for_calc + &one;
    let ratio = numerator / denominator;

    let mut result = BigDecimal::from(0);
    let mut ratio_power = ratio.clone();
    let epsilon = BigDecimal::from(1) / two.powi(precision);

    for n in 0..200 {
        let term = &ratio_power / BigDecimal::from(2 * n + 1);
        result = &result + &term;

        // ratio_power *= ratio^2
        ratio_power = &ratio_power * &ratio * &ratio;

        // 检查收敛
        // Check convergence
        if term.abs() < epsilon {
            break;
        }
    }

    let ln_m = if m < one {
        -&result * &two
    } else {
        &result * &two
    };

    // ln(2) 的近似值 - 使用二进制更自然
    // Approximate value of ln(2) - using binary is more natural
    let ln_2 = BigDecimal::from_str_radix("0.69314718055994530941723212145817656807550013436025525412068000949339362196969471560586332699641868754200148102057068573368552023575813055703267075163507596193072757082837143519030703862389167347112335011536449795523912047517268157493216541096101569466184254461532633879014781408314451197560944315676285767329862575680625582568274093918624067789326413284110912878384691724574370101253247770914855064668086503918521085790109245740893068578424100527965076689029359017877002969067160305633046998947773938294572061450067768597394649649213057625664986239351238645270485066045923968514931414058151271992685941765263424092916634592815560424534690084776692843290390288980434869200697480624521305555252835940863404030550689911456976364200787165733978769704126970401661791767446375303429300258480585755021868777534064982525042568804963047411855247926422246824585930548866084461005275668192911778576823204722375887738884959289422418950471878484392885760983534150848162676152711523041965008566408948180569252798619204629085020", 10).unwrap();

    // ln(x) = ln(m) + k_adj * ln(2)
    ln_m + BigDecimal::from(k_adj) * ln_2
}

/// 计算自然对数 ln(x) 使用泰勒级数 (默认精度)
/// Calculate natural logarithm ln(x) using Taylor series (with default precision)
pub fn ln(x: &BigDecimal) -> BigDecimal {
    ln_with_precision(x, DEFAULT_PRECISION)
}

// ============================================================================
// 指数函数 e^x / Exponential function e^x
// ============================================================================

/// 计算指数函数 e^x 使用泰勒级数 (指定精度)
/// Calculate exponential function e^x using Taylor series (with specified precision)
///
/// e^x = sum(x^n / n!) for n from 0 to infinity
///
/// # Arguments / 参数
/// * `x` - 输入值 / Input value
/// * `precision` - 计算精度（小数位数）/ Computation precision (decimal places)
pub fn exp_with_precision(x: &BigDecimal, precision: i64) -> BigDecimal {
    let one = BigDecimal::from(1);
    let two = BigDecimal::from(2);

    // 对于大的正 x，使用 e^x = (e^(x/2))^2 避免大数
    // For large positive x, use e^x = (e^(x/2))^2 to avoid large numbers
    if x > &BigDecimal::from(10) {
        let half_x = x / &two;
        let result = exp_with_precision(&half_x, precision);
        return &result * &result;
    }

    // 对于大的负 x，使用 e^x = 1 / e^(-x)
    // For large negative x, use e^x = 1 / e^(-x)
    if x < &BigDecimal::from(-10) {
        return one / exp_with_precision(&-x, precision);
    }

    let mut result = one.clone();
    let mut term = one;
    let mut n: i64 = 1;
    let epsilon = BigDecimal::from(1) / two.powi(precision);

    loop {
        term = term * x / BigDecimal::from(n);
        result = &result + &term;

        // 检查收敛
        // Check convergence
        if term.abs() < epsilon {
            break;
        }

        n += 1;
        if n > 500 {
            break;
        }
    }

    result
}

/// 计算指数函数 e^x 使用泰勒级数 (默认精度)
/// Calculate exponential function e^x using Taylor series (with default precision)
pub fn exp(x: &BigDecimal) -> BigDecimal {
    exp_with_precision(x, DEFAULT_PRECISION)
}

// ============================================================================
// 幂函数 x^y / Power function x^y
// ============================================================================

/// 检查 BigDecimal 是否为整数
/// Check if a BigDecimal is an integer
///
/// # Arguments / 参数
/// * `x` - 要检查的值 / Value to check
///
/// # Returns / 返回
/// 如果是整数则返回 Some(整数值)，否则返回 None
/// Returns Some(integer value) if integer, otherwise None
fn to_integer(x: &BigDecimal) -> Option<i64> {
    // 检查小数位数是否为 0
    // Check if the number of decimal digits is 0
    // BigDecimal 的 digits() 返回有效数字，我们可以通过检查 x - floor(x) 来判断
    // BigDecimal's digits() returns significant digits, we can check x - floor(x)
    let x_f64 = x.to_f64()?;
    let fract = x_f64.fract();
    if fract.abs() > 1e-10 {
        return None;
    }
    x.to_i64()
}

/// 计算 x^y 使用泰勒级数: x^y = e^(y * ln(x)) (指定精度)
/// Calculate x^y using Taylor series: x^y = e^(y * ln(x)) (with specified precision)
///
/// # Arguments / 参数
/// * `x` - 底数 / Base
/// * `y` - 指数 / Exponent
/// * `precision` - 计算精度（小数位数）/ Computation precision (decimal places)
///
/// # Panics / 恐慌
/// 当 x <= 0 且 y 为非整数时恐慌 / Panics when x <= 0 and y is non-integer
pub fn pow_with_precision(x: &BigDecimal, y: &BigDecimal, precision: i64) -> BigDecimal {
    if y == &BigDecimal::from(0) {
        return BigDecimal::from(1);
    }

    if x == &BigDecimal::from(0) {
        return BigDecimal::from(0);
    }

    if x == &BigDecimal::from(1) {
        return BigDecimal::from(1);
    }

    // 检查 y 是否为整数
    // Check if y is an integer
    if let Some(y_int) = to_integer(y) {
        // 使用整数幂
        // Use integer power
        return if y_int >= 0 {
            x.powi(y_int)
        } else {
            BigDecimal::from(1) / x.powi(-y_int)
        };
    }

    // x^y = e^(y * ln(x))
    let ln_x = ln_with_precision(x, precision);
    let exponent = y * ln_x;
    exp_with_precision(&exponent, precision)
}

/// 计算 x^y 使用泰勒级数: x^y = e^(y * ln(x)) (默认精度)
/// Calculate x^y using Taylor series: x^y = e^(y * ln(x)) (with default precision)
pub fn pow(x: &BigDecimal, y: &BigDecimal) -> BigDecimal {
    pow_with_precision(x, y, DEFAULT_PRECISION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ln() {
        let one = BigDecimal::from(1);
        let two = BigDecimal::from(2);
        let e = BigDecimal::from_str_radix("2.718281828459045", 10).unwrap();

        assert!(ln(&one).abs() < BigDecimal::from_str_radix("0.0001", 10).unwrap());

        let ln_e = ln(&e);
        assert!((ln_e - &one).abs() < BigDecimal::from_str_radix("0.0001", 10).unwrap());

        let ln_2 = ln(&two);
        let expected = BigDecimal::from_str_radix("0.693", 10).unwrap();
        assert!((ln_2 - &expected).abs() < BigDecimal::from_str_radix("0.01", 10).unwrap());
    }

    #[test]
    fn test_exp() {
        let zero = BigDecimal::from(0);
        let one = BigDecimal::from(1);

        assert!((exp(&zero) - &one).abs() < BigDecimal::from_str_radix("0.0001", 10).unwrap());

        let e = exp(&one);
        let expected = BigDecimal::from_str_radix("2.718", 10).unwrap();
        assert!((e - &expected).abs() < BigDecimal::from_str_radix("0.01", 10).unwrap());
    }

    #[test]
    fn test_pow() {
        let two = BigDecimal::from(2);
        let three = BigDecimal::from(3);
        let half = BigDecimal::from_str_radix("0.5", 10).unwrap();

        let result = pow(&two, &three);
        assert!(
            (result - BigDecimal::from(8)).abs()
                < BigDecimal::from_str_radix("0.0001", 10).unwrap()
        );

        let sqrt2 = pow(&two, &half);
        let expected = BigDecimal::from_str_radix("1.414", 10).unwrap();
        assert!((&sqrt2 - &expected).abs() < BigDecimal::from_str_radix("0.01", 10).unwrap());
    }

    #[test]
    fn test_with_precision() {
        let two = BigDecimal::from(2);
        let half = BigDecimal::from_str_radix("0.5", 10).unwrap();

        // 高精度计算
        // High precision calculation
        let sqrt2_high = pow_with_precision(&two, &half, 50);

        // 低精度计算
        // Low precision calculation
        let sqrt2_low = pow_with_precision(&two, &half, 5);

        // 两者都应该接近 sqrt(2)，但高精度更准确
        // Both should be close to sqrt(2), but high precision is more accurate
        let expected = BigDecimal::from_str_radix("1.414", 10).unwrap();
        assert!((sqrt2_high - &expected).abs() < BigDecimal::from_str_radix("0.01", 10).unwrap());
        assert!((sqrt2_low - &expected).abs() < BigDecimal::from_str_radix("0.01", 10).unwrap());
    }
}
