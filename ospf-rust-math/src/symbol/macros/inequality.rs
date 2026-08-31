//! 不等式构造宏 / Inequality construction macros
//!
//! 提供更接近数学语言的中缀语法构造不等式。
//! Provides infix syntax closer to mathematical language for constructing inequalities.

/// 构造线性不等式 / Construct linear inequality
///
/// 使用中缀语法构造线性不等式，更接近数学表达式。
/// Uses infix syntax to construct linear inequalities, closer to mathematical expressions.
///
/// # 语法 / Syntax
///
/// - `ineq!((lhs) <= rhs)` → 小于等于 (≤)
/// - `ineq!((lhs) >= rhs)` → 大于等于 (≥)
/// - `ineq!((lhs) < rhs)` → 小于 (<)
/// - `ineq!((lhs) > rhs)` → 大于 (>)
/// - `ineq!((lhs) == rhs)` → 等于 (=)
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::{ineq, lin, symbols_test};
/// use ospf_rust_math::symbol::LinearInequality;
///
/// symbols_test!(x, y);
///
/// // 线性不等式: 2x + 3y <= 5
/// let ineq1: LinearInequality<f64> = ineq!((lin!(2 * x, 3 * y)) <= 5.0);
///
/// // 线性不等式: x >= 0
/// let ineq2: LinearInequality<f64> = ineq!((lin!(x)) >= 0.0);
///
/// // 线性不等式: x + y < 10
/// let ineq3: LinearInequality<f64> = ineq!((lin!(x, y)) < 10.0);
///
/// // 线性不等式: x > 0
/// let ineq4: LinearInequality<f64> = ineq!((lin!(x)) > 0.0);
///
/// // 等式约束: x + y == 1
/// let ineq5: LinearInequality<f64> = ineq!((lin!(x, y)) == 1.0);
/// ```
#[macro_export]
macro_rules! ineq {
    // <= 小于等于
    (($lhs:expr) <= $rhs:expr) => {
        $crate::symbol::LinearInequality::new($lhs, $crate::symbol::Comparison::LessEqual, $rhs)
    };

    // >= 大于等于
    (($lhs:expr) >= $rhs:expr) => {
        $crate::symbol::LinearInequality::new($lhs, $crate::symbol::Comparison::GreaterEqual, $rhs)
    };

    // < 小于
    (($lhs:expr) < $rhs:expr) => {
        $crate::symbol::LinearInequality::new($lhs, $crate::symbol::Comparison::Less, $rhs)
    };

    // > 大于
    (($lhs:expr) > $rhs:expr) => {
        $crate::symbol::LinearInequality::new($lhs, $crate::symbol::Comparison::Greater, $rhs)
    };

    // == 等于
    (($lhs:expr) == $rhs:expr) => {
        $crate::symbol::LinearInequality::new($lhs, $crate::symbol::Comparison::Equal, $rhs)
    };
}

/// 构造二次不等式 / Construct quadratic inequality
///
/// 使用中缀语法构造二次不等式，更接近数学表达式。
/// Uses infix syntax to construct quadratic inequalities, closer to mathematical expressions.
///
/// # 语法 / Syntax
///
/// - `qineq!((lhs) <= rhs)` → 小于等于 (≤)
/// - `qineq!((lhs) >= rhs)` → 大于等于 (≥)
/// - `qineq!((lhs) < rhs)` → 小于 (<)
/// - `qineq!((lhs) > rhs)` → 大于 (>)
/// - `qineq!((lhs) == rhs)` → 等于 (=)
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::{qineq, quad, symbols_test};
/// use ospf_rust_math::symbol::QuadraticInequality;
///
/// symbols_test!(x, y);
///
/// // 二次不等式: x² <= 1
/// let ineq1: QuadraticInequality<f64> = qineq!((quad!(x ^ 2)) <= 1.0);
///
/// // 二次不等式: 2xy >= 0
/// let ineq2: QuadraticInequality<f64> = qineq!((quad!(2 * x * y)) >= 0.0);
///
/// // 二次不等式: x² < 10
/// let ineq3: QuadraticInequality<f64> = qineq!((quad!(x ^ 2)) < 10.0);
///
/// // 二次不等式: x² > 0
/// let ineq4: QuadraticInequality<f64> = qineq!((quad!(x ^ 2)) > 0.0);
///
/// // 二次等式: x² == 1
/// let ineq5: QuadraticInequality<f64> = qineq!((quad!(x ^ 2)) == 1.0);
/// ```
#[macro_export]
macro_rules! qineq {
    // <= 小于等于
    (($lhs:expr) <= $rhs:expr) => {
        $crate::symbol::QuadraticInequality::new($lhs, $crate::symbol::Comparison::LessEqual, $rhs)
    };

    // >= 大于等于
    (($lhs:expr) >= $rhs:expr) => {
        $crate::symbol::QuadraticInequality::new($lhs, $crate::symbol::Comparison::GreaterEqual, $rhs)
    };

    // < 小于
    (($lhs:expr) < $rhs:expr) => {
        $crate::symbol::QuadraticInequality::new($lhs, $crate::symbol::Comparison::Less, $rhs)
    };

    // > 大于
    (($lhs:expr) > $rhs:expr) => {
        $crate::symbol::QuadraticInequality::new($lhs, $crate::symbol::Comparison::Greater, $rhs)
    };

    // == 等于
    (($lhs:expr) == $rhs:expr) => {
        $crate::symbol::QuadraticInequality::new($lhs, $crate::symbol::Comparison::Equal, $rhs)
    };
}

/// 构造标准不等式 / Construct canonical inequality
///
/// 使用中缀语法构造标准不等式。
/// Uses infix syntax to construct canonical inequalities.
///
/// # 语法 / Syntax
///
/// - `cineq!((lhs) <= rhs)` → 小于等于 (≤)
/// - `cineq!((lhs) >= rhs)` → 大于等于 (≥)
/// - `cineq!((lhs) < rhs)` → 小于 (<)
/// - `cineq!((lhs) > rhs)` → 大于 (>)
/// - `cineq!((lhs) == rhs)` → 等于 (=)
#[macro_export]
macro_rules! cineq {
    // <= 小于等于
    (($lhs:expr) <= $rhs:expr) => {
        $crate::symbol::CanonicalInequality::new($lhs, $crate::symbol::Comparison::LessEqual, $rhs)
    };

    // >= 大于等于
    (($lhs:expr) >= $rhs:expr) => {
        $crate::symbol::CanonicalInequality::new($lhs, $crate::symbol::Comparison::GreaterEqual, $rhs)
    };

    // < 小于
    (($lhs:expr) < $rhs:expr) => {
        $crate::symbol::CanonicalInequality::new($lhs, $crate::symbol::Comparison::Less, $rhs)
    };

    // > 大于
    (($lhs:expr) > $rhs:expr) => {
        $crate::symbol::CanonicalInequality::new($lhs, $crate::symbol::Comparison::Greater, $rhs)
    };

    // == 等于
    (($lhs:expr) == $rhs:expr) => {
        $crate::symbol::CanonicalInequality::new($lhs, $crate::symbol::Comparison::Equal, $rhs)
    };
}

/// 约束集合构造宏 / Constraint set construction macro
///
/// 方便地定义多个约束。
/// Conveniently define multiple constraints.
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::{constraints, ineq, lin, symbols_test};
/// use ospf_rust_math::symbol::LinearInequality;
///
/// symbols_test!(x, y);
///
/// // 定义多个约束
/// let cons = constraints![
///     ineq!((lin!(2 * x, 1 * y)) <= 5.0),
///     ineq!((lin!(1 * x, 2 * y)) <= 4.0),
///     ineq!((lin!(x, y)) >= 1.0),
/// ];
///
/// assert_eq!(cons.len(), 3);
/// ```
#[macro_export]
macro_rules! constraints {
    [$($ineq:expr),* $(,)?] => {
        vec![$($ineq),*]
    };
}

#[cfg(test)]
mod tests {
    use crate::{ineq, qineq, constraints, lin, quad, symbols_test};
    use crate::symbol::{LinearInequality, QuadraticInequality};

    #[test]
    fn test_linear_inequalities() {
        symbols_test!(x, y);

        // <= 小于等于
        let ineq1: LinearInequality<f64> = ineq!((lin!(2 * x, 1 * y)) <= 5.0);
        assert_eq!(ineq1.rhs, 5.0);

        // >= 大于等于
        let ineq2: LinearInequality<f64> = ineq!((lin!(2 * x + 1)) >= 0.0);
        assert_eq!(ineq2.rhs, 0.0);

        // < 小于
        let ineq3: LinearInequality<f64> = ineq!((lin!(x, y)) < 10.0);
        assert_eq!(ineq3.rhs, 10.0);

        // > 大于
        let ineq4: LinearInequality<f64> = ineq!((lin!(x)) > 0.0);
        assert_eq!(ineq4.rhs, 0.0);

        // == 等于
        let ineq5: LinearInequality<f64> = ineq!((lin!(x, y)) == 1.0);
        assert_eq!(ineq5.rhs, 1.0);
    }

    #[test]
    fn test_quadratic_inequalities() {
        symbols_test!(x, y);

        // <= 小于等于
        let ineq1: QuadraticInequality<f64> = qineq!((quad!(x ^ 2)) <= 1.0);
        assert_eq!(ineq1.rhs, 1.0);

        // >= 大于等于
        let ineq2: QuadraticInequality<f64> = qineq!((quad!(2 * x * y)) >= 0.0);
        assert_eq!(ineq2.rhs, 0.0);

        // < 小于
        let ineq3: QuadraticInequality<f64> = qineq!((quad!(x ^ 2)) < 10.0);
        assert_eq!(ineq3.rhs, 10.0);

        // > 大于
        let ineq4: QuadraticInequality<f64> = qineq!((quad!(x * y)) > 0.0);
        assert_eq!(ineq4.rhs, 0.0);

        // == 等于
        let ineq5: QuadraticInequality<f64> = qineq!((quad!(x ^ 2)) == 1.0);
        assert_eq!(ineq5.rhs, 1.0);
    }

    #[test]
    fn test_constraints() {
        symbols_test!(x, y);

        let cons = constraints![
            ineq!((lin!(2 * x, 1 * y)) <= 5.0),
            ineq!((lin!(1 * x, 2 * y)) <= 4.0),
            ineq!((lin!(x, y)) >= 1.0),
        ];

        assert_eq!(cons.len(), 3);
    }
}