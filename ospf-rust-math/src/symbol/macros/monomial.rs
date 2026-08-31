//! 单项式构造宏 / Monomial construction macros

/// 构造线性单项式 / Construct linear monomial
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::linear_monomial;
/// use ospf_rust_math::symbols_test;
///
/// symbols_test!(x, y);
///
/// let m1 = linear_monomial!(2.0, x);  // LinearMonomial { coefficient: 2.0, symbol: x }
/// let m2 = linear_monomial!(1.0, y);  // LinearMonomial { coefficient: 1.0, symbol: y }
///
/// assert_eq!(m1.coefficient, 2.0);
/// ```
#[macro_export]
macro_rules! linear_monomial {
    ($coeff:expr, $symbol:expr) => {
        $crate::symbol::LinearMonomial::new($coeff, $symbol)
    };
}

/// 构造二次单项式 / Construct quadratic monomial
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::quadratic_monomial;
/// use ospf_rust_math::symbols_test;
///
/// symbols_test!(x, y);
///
/// // 二次项（两个符号）
/// let q1 = quadratic_monomial!(2.0, x, y);  // 2xy
///
/// assert_eq!(q1.coefficient, 2.0);
/// ```
#[macro_export]
macro_rules! quadratic_monomial {
    // 二次项：系数 + 两个符号
    ($coeff:expr, $s1:expr, $s2:expr) => {
        $crate::symbol::QuadraticMonomial::quadratic($coeff, $s1, $s2)
    };

    // 线性项：系数 + 单个符号
    ($coeff:expr, $symbol:expr,) => {
        $crate::symbol::QuadraticMonomial::linear($coeff, $symbol)
    };
}

/// 构造标准单项式 / Construct canonical monomial
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::canonical_monomial;
/// use ospf_rust_math::symbols_test;
/// use std::collections::HashMap;
///
/// symbols_test!(x, y);
///
/// // 使用 HashMap 指定幂次
/// let mut powers = HashMap::new();
/// powers.insert(x.clone(), 2);
/// powers.insert(y.clone(), 3);
/// let c1 = canonical_monomial!(1.0, powers);  // x^2 * y^3
///
/// assert_eq!(c1.coefficient, 1.0);
/// ```
#[macro_export]
macro_rules! canonical_monomial {
    ($coeff:expr, $powers:expr) => {
        $crate::symbol::CanonicalMonomial::new($coeff, $powers)
    };
}

/// 从符号和幂次列表构造标准单项式
/// Construct canonical monomial from symbol-power pairs
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::canonical_monomial_with_powers;
/// use ospf_rust_math::symbols_test;
///
/// symbols_test!(x, y);
///
/// // x^2 * y^3
/// let c = canonical_monomial_with_powers!(1.0, (x, 2), (y, 3));
///
/// assert_eq!(c.coefficient, 1.0);
/// ```
#[macro_export]
macro_rules! canonical_monomial_with_powers {
    ($coeff:expr, $(($symbol:expr, $power:expr)),* $(,)?) => {
        {
            let mut powers = std::collections::HashMap::new();
            $(
                powers.insert($symbol, $power);
            )*
            $crate::symbol::CanonicalMonomial::new($coeff, powers)
        }
    };
}
