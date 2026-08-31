//! 多项式构造宏 / Polynomial construction macros
//!
//! 提供更接近数学语言的语法构造多项式。
//! Provides syntax closer to mathematical language for constructing polynomials.

/// 线性多项式快速构造宏 / Quick linear polynomial construction macro
///
/// 使用简化语法快速构造线性多项式。
/// Uses simplified syntax to quickly construct linear polynomials.
///
/// # 语法 / Syntax
///
/// - `lin!()` → 零多项式
/// - `lin!(x)` → x（系数为 1）
/// - `lin!(c * x)` → cx（指定系数）
/// - `lin!(c * x, d * y)` → cx + dy（多项式）
/// - `lin!(c * x + k)` → cx + k（带常数）
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::{lin, symbols_test};
/// use ospf_rust_math::symbol::Linear;
///
/// symbols_test!(x, y, z);
///
/// // 单个符号
/// let p1: Linear<f64> = lin!(x);
///
/// // 带系数的单个符号
/// let p2: Linear<f64> = lin!(2 * x);
///
/// // 多项式（所有项都必须带系数）
/// let p3: Linear<f64> = lin!(2 * x, 3 * y);
///
/// // 减法：使用负系数
/// let p4: Linear<f64> = lin!(1 * x, -1 * y);  // x - y
/// ```
#[macro_export]
macro_rules! lin {
    // 空 → 零多项式
    () => {
        $crate::symbol::Linear::zero()
    };

    // 两个符号
    ($s1:ident, $s2:ident) => {
        $crate::symbol::Linear::new(
            vec![
                $crate::symbol::LinearMonomial::new(1.0f64, $s1.clone()),
                $crate::symbol::LinearMonomial::new(1.0f64, $s2.clone()),
            ],
            0.0f64,
        )
    };

    // 三个符号
    ($s1:ident, $s2:ident, $s3:ident) => {
        $crate::symbol::Linear::new(
            vec![
                $crate::symbol::LinearMonomial::new(1.0f64, $s1.clone()),
                $crate::symbol::LinearMonomial::new(1.0f64, $s2.clone()),
                $crate::symbol::LinearMonomial::new(1.0f64, $s3.clone()),
            ],
            0.0f64,
        )
    };

    // 带常数的单个符号
    ($s:ident + $const:expr) => {
        $crate::symbol::Linear::new(
            vec![$crate::symbol::LinearMonomial::new(1.0f64, $s.clone())],
            $const as f64,
        )
    };

    // 带常数的正系数 * 单个符号
    ($c:literal * $s:ident + $const:expr) => {
        $crate::symbol::Linear::new(
            vec![$crate::symbol::LinearMonomial::new($c as f64, $s.clone())],
            $const as f64,
        )
    };

    // 带常数的负系数 * 单个符号
    (- $c:literal * $s:ident + $const:expr) => {
        $crate::symbol::Linear::new(
            vec![$crate::symbol::LinearMonomial::new(
                -($c as f64),
                $s.clone(),
            )],
            $const as f64,
        )
    };

    // 系数 * 符号, 系数 * 符号
    ($c1:literal * $s1:ident, $c2:literal * $s2:ident) => {
        $crate::symbol::Linear::new(
            vec![
                $crate::symbol::LinearMonomial::new($c1 as f64, $s1.clone()),
                $crate::symbol::LinearMonomial::new($c2 as f64, $s2.clone()),
            ],
            0.0f64,
        )
    };

    // 系数 * 符号, -系数 * 符号
    ($c1:literal * $s1:ident, - $c2:literal * $s2:ident) => {
        $crate::symbol::Linear::new(
            vec![
                $crate::symbol::LinearMonomial::new($c1 as f64, $s1.clone()),
                $crate::symbol::LinearMonomial::new(-($c2 as f64), $s2.clone()),
            ],
            0.0f64,
        )
    };

    // -系数 * 符号, 系数 * 符号
    (- $c1:literal * $s1:ident, $c2:literal * $s2:ident) => {
        $crate::symbol::Linear::new(
            vec![
                $crate::symbol::LinearMonomial::new(-($c1 as f64), $s1.clone()),
                $crate::symbol::LinearMonomial::new($c2 as f64, $s2.clone()),
            ],
            0.0f64,
        )
    };

    // -系数 * 符号, -系数 * 符号
    (- $c1:literal * $s1:ident, - $c2:literal * $s2:ident) => {
        $crate::symbol::Linear::new(
            vec![
                $crate::symbol::LinearMonomial::new(-($c1 as f64), $s1.clone()),
                $crate::symbol::LinearMonomial::new(-($c2 as f64), $s2.clone()),
            ],
            0.0f64,
        )
    };

    // 单个符号（无系数）- 放在最后
    ($s:ident) => {
        $crate::symbol::Linear::new(
            vec![$crate::symbol::LinearMonomial::new(1.0f64, $s.clone())],
            0.0f64,
        )
    };

    // 正系数 * 单个符号 - 放在最后
    ($c:literal * $s:ident) => {
        $crate::symbol::Linear::new(
            vec![$crate::symbol::LinearMonomial::new($c as f64, $s.clone())],
            0.0f64,
        )
    };

    // 负系数 * 单个符号 - 放在最后
    (- $c:literal * $s:ident) => {
        $crate::symbol::Linear::new(
            vec![$crate::symbol::LinearMonomial::new(
                -($c as f64),
                $s.clone(),
            )],
            0.0f64,
        )
    };
}

/// 二次多项式快速构造宏 / Quick quadratic polynomial construction macro
///
/// 使用简化语法快速构造二次多项式。
/// Uses simplified syntax to quickly construct quadratic polynomials.
///
/// # 语法 / Syntax
///
/// - `quad!()` → 零多项式
/// - `quad!(x * y)` → xy（两个符号相乘）
/// - `quad!(c * x * y)` → cxy（带系数）
/// - `quad!(x ^ 2)` → x²（平方）
/// - `quad!(c * x ^ 2)` → cx²（带系数的平方）
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::{quad, symbols_test};
/// use ospf_rust_math::symbol::Quadratic;
///
/// symbols_test!(x, y);
///
/// // 二次项 x * y
/// let q1: Quadratic<f64> = quad!(x * y);
///
/// // 带系数的二次项 2xy
/// let q2: Quadratic<f64> = quad!(2 * x * y);
///
/// // 平方项 x²
/// let q3: Quadratic<f64> = quad!(x ^ 2);
///
/// // 带系数的平方项 2x²
/// let q4: Quadratic<f64> = quad!(2 * x ^ 2);
/// ```
#[macro_export]
macro_rules! quad {
    // 空 → 零多项式
    () => {
        $crate::symbol::Quadratic::zero()
    };

    // 常数
    ($c:literal) => {
        $crate::symbol::Quadratic::from_constant($c as f64)
    };

    // c * x * y (系数 * 两个符号)
    ($c:literal * $s1:ident * $s2:ident) => {
        $crate::symbol::Quadratic::new(
            vec![$crate::symbol::QuadraticMonomial::quadratic(
                $c as f64,
                $s1.clone(),
                $s2.clone(),
            )],
            0.0f64,
        )
    };

    // 负系数 * x * y
    (- $c:literal * $s1:ident * $s2:ident) => {
        $crate::symbol::Quadratic::new(
            vec![$crate::symbol::QuadraticMonomial::quadratic(
                -($c as f64),
                $s1.clone(),
                $s2.clone(),
            )],
            0.0f64,
        )
    };

    // x * y (两个符号相乘)
    ($s1:ident * $s2:ident) => {
        $crate::symbol::Quadratic::new(
            vec![$crate::symbol::QuadraticMonomial::quadratic(
                1.0f64,
                $s1.clone(),
                $s2.clone(),
            )],
            0.0f64,
        )
    };

    // c * x^2 (系数 * 平方)
    ($c:literal * $s:ident ^ 2) => {
        $crate::symbol::Quadratic::new(
            vec![$crate::symbol::QuadraticMonomial::quadratic(
                $c as f64,
                $s.clone(),
                $s.clone(),
            )],
            0.0f64,
        )
    };

    // 负系数 * x^2
    (- $c:literal * $s:ident ^ 2) => {
        $crate::symbol::Quadratic::new(
            vec![$crate::symbol::QuadraticMonomial::quadratic(
                -($c as f64),
                $s.clone(),
                $s.clone(),
            )],
            0.0f64,
        )
    };

    // x^2 (平方)
    ($s:ident ^ 2) => {
        $crate::symbol::Quadratic::new(
            vec![$crate::symbol::QuadraticMonomial::quadratic(
                1.0f64,
                $s.clone(),
                $s.clone(),
            )],
            0.0f64,
        )
    };
}

/// 构造线性多项式（旧版）/ Construct linear polynomial (legacy)
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::symbol::Linear;
///
/// // 常数多项式
/// let l1 = Linear::from_constant(5.0);
///
/// assert_eq!(l1.constant, 5.0);
/// ```
#[macro_export]
macro_rules! linear {
    // 常数多项式
    ($constant:expr) => {
        $crate::symbol::Linear::from_constant($constant)
    };

    // 从单项式列表和常数构造
    ([$($coeff:expr, $symbol:expr),* $(,)?], $constant:expr) => {
        {
            let monomials = vec![
                $(
                    $crate::symbol::LinearMonomial::new($coeff, $symbol)
                ),*
            ];
            $crate::symbol::Linear::new(monomials, $constant)
        }
    };

    // 只有单项式列表（常数为零）
    ([$($coeff:expr, $symbol:expr),* $(,)?]) => {
        $crate::symbol::linear!([$($coeff, $symbol),*], <_ as num_traits::Zero>::zero())
    };
}

/// 构造二次多项式（旧版）/ Construct quadratic polynomial (legacy)
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::symbol::Quadratic;
///
/// // 常数多项式
/// let q1 = Quadratic::from_constant(5.0);
///
/// assert_eq!(q1.constant, 5.0);
/// ```
#[macro_export]
macro_rules! quadratic {
    // 常数多项式
    ($constant:expr) => {
        $crate::symbol::Quadratic::from_constant($constant)
    };

    // 从单项式列表和常数构造
    ([$($coeff:expr, $s1:expr, $s2:expr),* $(,)?], $constant:expr) => {
        {
            let monomials = vec![
                $(
                    $crate::symbol::QuadraticMonomial::new($coeff, $s1, $s2)
                ),*
            ];
            $crate::symbol::Quadratic::new(monomials, $constant)
        }
    };

    // 只有单项式列表（常数为零）
    ([$($coeff:expr, $s1:expr, $s2:expr),* $(,)?]) => {
        $crate::symbol::quadratic!([$($coeff, $s1, $s2),*], <_ as num_traits::Zero>::zero())
    };
}

/// 构造标准多项式 / Construct canonical polynomial
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::symbol::Canonical;
///
/// // 常数多项式
/// let c1: Canonical<f64, i32> = Canonical::from_constant(5.0);
///
/// assert_eq!(c1.constant, 5.0);
/// ```
#[macro_export]
macro_rules! canonical {
    // 常数多项式
    ($constant:expr) => {
        $crate::symbol::Canonical::from_constant($constant)
    };

    // 从单项式列表和常数构造
    ([$($coeff:expr, $powers:expr),* $(,)?], $constant:expr) => {
        {
            let monomials = vec![
                $(
                    $crate::symbol::CanonicalMonomial::new($coeff, $powers)
                ),*
            ];
            $crate::symbol::Canonical::new(monomials, $constant)
        }
    };

    // 只有单项式列表（常数为零）
    ([$($coeff:expr, $powers:expr),* $(,)?]) => {
        $crate::symbol::canonical!([$($coeff, $powers),*], <_ as num_traits::Zero>::zero())
    };
}

#[cfg(test)]
mod tests {
    use crate::symbol::{Linear, Quadratic};
    use crate::{lin, quad, symbols_test};

    #[test]
    fn test_lin_basic() {
        symbols_test!(x, y);

        let p1: Linear<f64> = lin!(x);
        assert_eq!(p1.monomials.len(), 1);
        assert_eq!(p1.monomials[0].coefficient, 1.0);

        let p2: Linear<f64> = lin!(2 * x);
        assert_eq!(p2.monomials[0].coefficient, 2.0);
    }

    #[test]
    fn test_lin_multiple() {
        symbols_test!(x, y);

        let p: Linear<f64> = lin!(2 * x, 3 * y);
        assert_eq!(p.monomials.len(), 2);
        assert_eq!(p.monomials[0].coefficient, 2.0);
        assert_eq!(p.monomials[1].coefficient, 3.0);
    }

    #[test]
    fn test_lin_with_constant() {
        symbols_test!(x, y);

        let p: Linear<f64> = lin!(2 * x + 1);
        assert_eq!(p.monomials.len(), 1);
        assert_eq!(p.constant, 1.0);
    }

    #[test]
    fn test_lin_symbols_only() {
        symbols_test!(x, y);

        let p: Linear<f64> = lin!(x, y);
        assert_eq!(p.monomials.len(), 2);
        assert_eq!(p.monomials[0].coefficient, 1.0);
        assert_eq!(p.monomials[1].coefficient, 1.0);
    }

    #[test]
    fn test_lin_subtraction() {
        symbols_test!(x, y);

        let p: Linear<f64> = lin!(1 * x, -1 * y);
        assert_eq!(p.monomials.len(), 2);
        assert_eq!(p.monomials[0].coefficient, 1.0);
        assert_eq!(p.monomials[1].coefficient, -1.0);
    }

    #[test]
    fn test_quad_basic() {
        symbols_test!(x, y);

        let q1: Quadratic<f64> = quad!(x * y);
        assert_eq!(q1.monomials.len(), 1);

        let q2: Quadratic<f64> = quad!(2 * x * y);
        assert_eq!(q2.monomials[0].coefficient, 2.0);

        let q3: Quadratic<f64> = quad!(x ^ 2);
        assert_eq!(q3.monomials[0].coefficient, 1.0);

        let q4: Quadratic<f64> = quad!(2 * x ^ 2);
        assert_eq!(q4.monomials[0].coefficient, 2.0);
    }
}
