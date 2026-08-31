//! 符号因式分解。
//! Symbolic factorization.

use std::ops::AddAssign;
use num_traits::{Float, Zero};
use crate::symbol::{OwnedSymbol, Quadratic, QuadraticMonomial};

/// 一元二次多项式系数。
/// Coefficients of a univariate quadratic polynomial.
///
/// 表示形式为 `a*x^2 + b*x + c`。
/// Represents `a*x^2 + b*x + c`.
#[derive(Clone, Debug, PartialEq)]
pub struct QuadraticCoefficients<T> {
    /// 二次项系数。
    /// Quadratic coefficient.
    pub a: T,
    /// 一次项系数。
    /// Linear coefficient.
    pub b: T,
    /// 常数项。
    /// Constant term.
    pub c: T,
    /// 变量符号。
    /// Variable symbol.
    pub symbol: OwnedSymbol,
}

/// 多项式实根。
/// Real roots of a polynomial.
#[derive(Clone, Debug, PartialEq)]
pub struct PolynomialRoots<T> {
    /// 实根列表。
    /// List of real roots.
    pub roots: Vec<T>,
    /// 判别式。
    /// Discriminant.
    pub discriminant: T,
    /// 是否存在于实数域。
    /// Whether the roots are in the real domain.
    pub is_real: bool,
}

/// 一元二次实根的兼容命名。
/// Compatibility name for univariate quadratic real roots.
pub type QuadraticRoots<T> = PolynomialRoots<T>;

/// 一次因式 `(x - root)`。
/// Linear factor `(x - root)`.
#[derive(Clone, Debug, PartialEq)]
pub struct LinearFactor<T> {
    /// 根。
    /// Root.
    pub root: T,
}

impl<T> LinearFactor<T> {
    /// 创建一次因式。
    /// Create a linear factor.
    pub fn new(root: T) -> Self {
        Self { root }
    }
}

/// 一元二次多项式因式分解结果。
/// Factorization result of a univariate quadratic polynomial.
///
/// 表示形式为 `leading_coefficient * (x - r1) * (x - r2)`。
/// Represents `leading_coefficient * (x - r1) * (x - r2)`.
#[derive(Clone, Debug, PartialEq)]
pub struct QuadraticFactorization<T> {
    /// 首项系数。
    /// Leading coefficient.
    pub leading_coefficient: T,
    /// 一次因式列表。
    /// List of linear factors.
    pub factors: Vec<LinearFactor<T>>,
    /// 变量符号。
    /// Variable symbol.
    pub symbol: OwnedSymbol,
}

impl<T> QuadraticFactorization<T>
where
    T: Clone
        + Zero
        + std::ops::Neg<Output = T>
        + std::ops::Add<Output = T>
        + std::ops::Mul<Output = T>,
{
    /// 将因式分解结果展开回二次多项式。
    /// Expand the factorization result back to a quadratic polynomial.
    pub fn expand(&self) -> Quadratic<T> {
        match self.factors.len() {
            0 => Quadratic::new(Vec::new(), self.leading_coefficient.clone()),
            1 => {
                let root = self.factors[0].root.clone();
                let constant = -(self.leading_coefficient.clone() * root);
                Quadratic::new(
                    vec![QuadraticMonomial::linear(
                        self.leading_coefficient.clone(),
                        self.symbol.clone(),
                    )],
                    constant,
                )
            }
            2 => {
                let r1 = self.factors[0].root.clone();
                let r2 = self.factors[1].root.clone();
                let linear_coefficient =
                    -(self.leading_coefficient.clone() * (r1.clone() + r2.clone()));
                let constant = self.leading_coefficient.clone() * r1 * r2;

                Quadratic::new(
                    vec![
                        QuadraticMonomial::quadratic(
                            self.leading_coefficient.clone(),
                            self.symbol.clone(),
                            self.symbol.clone(),
                        ),
                        QuadraticMonomial::linear(linear_coefficient, self.symbol.clone()),
                    ],
                    constant,
                )
            }
            _ => Quadratic::zero(),
        }
    }
}

/// 提取一元二次多项式系数。
/// Extract univariate quadratic polynomial coefficients.
pub trait ExtractUnivariateCoefficients<T> {
    /// 提取 `a*x^2 + b*x + c` 中的 `(a, b, c, x)`。
    /// Extract `(a, b, c, x)` from `a*x^2 + b*x + c`.
    fn extract_univariate_coefficients(&self) -> Option<QuadraticCoefficients<T>>;
}

impl<T> ExtractUnivariateCoefficients<T> for Quadratic<T>
where
    T: Clone + Zero + AddAssign<T>,
{
    fn extract_univariate_coefficients(&self) -> Option<QuadraticCoefficients<T>> {
        let mut symbol: Option<OwnedSymbol> = None;
        let mut a = T::zero();
        let mut b = T::zero();

        for monomial in &self.monomials {
            if let Some(current_symbol) = &symbol {
                if monomial.symbol1 != *current_symbol {
                    return None;
                }
            } else {
                symbol = Some(monomial.symbol1.clone());
            }

            match &monomial.symbol2 {
                Some(symbol2) => {
                    if *symbol2 != monomial.symbol1 {
                        return None;
                    }
                    a += monomial.coefficient.clone();
                }
                None => {
                    b += monomial.coefficient.clone();
                }
            }
        }

        symbol.map(|symbol| QuadraticCoefficients {
            a,
            b,
            c: self.constant.clone(),
            symbol,
        })
    }
}

/// 解一元二次方程。
/// Solve a univariate quadratic equation.
pub fn solve_quadratic<T>(coefficients: QuadraticCoefficients<T>) -> PolynomialRoots<T>
where
    T: Float,
{
    let a = coefficients.a;
    let b = coefficients.b;
    let c = coefficients.c;
    let zero = T::zero();

    if a == zero {
        if b == zero {
            return PolynomialRoots {
                roots: Vec::new(),
                discriminant: zero,
                is_real: true,
            };
        }

        return PolynomialRoots {
            roots: vec![-c / b],
            discriminant: zero,
            is_real: true,
        };
    }

    let two = T::one() + T::one();
    let four = two + two;
    let discriminant = b * b - four * a * c;

    if discriminant < zero {
        PolynomialRoots {
            roots: Vec::new(),
            discriminant,
            is_real: false,
        }
    } else if discriminant == zero {
        PolynomialRoots {
            roots: vec![-b / (two * a)],
            discriminant,
            is_real: true,
        }
    } else {
        let sqrt_discriminant = discriminant.sqrt();
        let two_a = two * a;

        PolynomialRoots {
            roots: vec![
                (-b + sqrt_discriminant) / two_a,
                (-b - sqrt_discriminant) / two_a,
            ],
            discriminant,
            is_real: true,
        }
    }
}

/// 一元二次多项式求解 trait。
/// Trait for solving univariate quadratic polynomials.
pub trait SolveQuadratic<T> {
    /// 求解多项式实根。
    /// Solve real roots of the polynomial.
    fn solve(&self) -> Option<PolynomialRoots<T>>;
}

impl<T> SolveQuadratic<T> for Quadratic<T>
where
    T: Float + AddAssign<T>,
{
    fn solve(&self) -> Option<PolynomialRoots<T>> {
        if self.monomials.is_empty() {
            return Some(PolynomialRoots {
                roots: Vec::new(),
                discriminant: T::zero(),
                is_real: true,
            });
        }

        self.extract_univariate_coefficients().map(solve_quadratic)
    }
}

/// 一元二次多项式因式分解。
/// Factorize a univariate quadratic polynomial.
pub fn factorize_quadratic<T>(
    coefficients: QuadraticCoefficients<T>,
) -> Option<QuadraticFactorization<T>>
where
    T: Float,
{
    let a = coefficients.a;
    let b = coefficients.b;
    let c = coefficients.c;
    let symbol = coefficients.symbol;
    let zero = T::zero();

    if a == zero {
        if b == zero {
            return None;
        }

        return Some(QuadraticFactorization {
            leading_coefficient: b,
            factors: vec![LinearFactor::new(-c / b)],
            symbol,
        });
    }

    let roots = solve_quadratic(QuadraticCoefficients {
        a,
        b,
        c,
        symbol: symbol.clone(),
    });
    if !roots.is_real || roots.roots.is_empty() {
        return None;
    }

    Some(QuadraticFactorization {
        leading_coefficient: a,
        factors: roots.roots.into_iter().map(LinearFactor::new).collect(),
        symbol,
    })
}

/// 一元二次多项式因式分解 trait。
/// Trait for factorizing univariate quadratic polynomials.
pub trait FactorizeQuadratic<T> {
    /// 对多项式做实数域因式分解。
    /// Factorize the polynomial over the real domain.
    fn factorize(&self) -> Option<QuadraticFactorization<T>>;
}

impl<T> FactorizeQuadratic<T> for Quadratic<T>
where
    T: Float + AddAssign<T>,
{
    fn factorize(&self) -> Option<QuadraticFactorization<T>> {
        self.extract_univariate_coefficients()
            .and_then(factorize_quadratic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::{DynSymbol, SymbolDynId};
    use std::any::Any;
    use std::fmt::{Display, Formatter, Result};

    #[derive(Debug, Clone)]
    struct TestSymbol {
        id: usize,
        name: String,
    }

    impl Display for TestSymbol {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            write!(f, "{}", self.name)
        }
    }

    impl DynSymbol for TestSymbol {
        fn name(&self) -> &str {
            &self.name
        }

        fn display_name(&self) -> &str {
            &self.name
        }

        fn dyn_id(&self) -> SymbolDynId<'_> {
            SymbolDynId::standalone(self.id)
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    fn make_symbol(name: &str, id: usize) -> OwnedSymbol {
        OwnedSymbol::new(TestSymbol {
            id,
            name: name.to_string(),
        })
    }

    #[test]
    fn extracts_univariate_coefficients() {
        let x = make_symbol("x", 1);
        let poly = Quadratic::new(
            vec![
                QuadraticMonomial::quadratic(2.0, x.clone(), x.clone()),
                QuadraticMonomial::linear(3.0, x.clone()),
                QuadraticMonomial::quadratic(4.0, x.clone(), x.clone()),
            ],
            5.0,
        );

        let coefficients = poly.extract_univariate_coefficients().unwrap();
        assert_eq!(coefficients.a, 6.0);
        assert_eq!(coefficients.b, 3.0);
        assert_eq!(coefficients.c, 5.0);
        assert_eq!(coefficients.symbol, x);
    }

    #[test]
    fn rejects_multivariate_quadratic() {
        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);
        let poly = Quadratic::new(vec![QuadraticMonomial::quadratic(2.0, x, y)], 0.0);

        assert!(poly.extract_univariate_coefficients().is_none());
    }

    #[test]
    fn solves_two_real_roots() {
        let x = make_symbol("x", 1);
        let coefficients = QuadraticCoefficients {
            a: 1.0,
            b: -3.0,
            c: 2.0,
            symbol: x,
        };

        let roots = solve_quadratic(coefficients);
        assert!(roots.is_real);
        assert_eq!(roots.discriminant, 1.0);
        assert_eq!(roots.roots, vec![2.0, 1.0]);
    }

    #[test]
    fn solves_repeated_root() {
        let x = make_symbol("x", 1);
        let roots = solve_quadratic(QuadraticCoefficients {
            a: 1.0,
            b: -2.0,
            c: 1.0,
            symbol: x,
        });

        assert!(roots.is_real);
        assert_eq!(roots.roots, vec![1.0]);
    }

    #[test]
    fn rejects_no_real_roots() {
        let x = make_symbol("x", 1);
        let factorization = factorize_quadratic(QuadraticCoefficients {
            a: 1.0,
            b: 0.0,
            c: 1.0,
            symbol: x,
        });

        assert!(factorization.is_none());
    }

    #[test]
    fn factorizes_linear_degenerate_case() {
        let x = make_symbol("x", 1);
        let factorization = factorize_quadratic(QuadraticCoefficients {
            a: 0.0,
            b: 2.0,
            c: -4.0,
            symbol: x.clone(),
        })
        .unwrap();

        assert_eq!(factorization.leading_coefficient, 2.0);
        assert_eq!(factorization.factors, vec![LinearFactor::new(2.0)]);
        assert_eq!(factorization.symbol, x);
        assert_eq!(factorization.expand().constant, -4.0);
    }

    #[test]
    fn factorizes_and_expands_quadratic() {
        let x = make_symbol("x", 1);
        let poly = Quadratic::new(
            vec![
                QuadraticMonomial::quadratic(1.0, x.clone(), x.clone()),
                QuadraticMonomial::linear(-3.0, x.clone()),
            ],
            2.0,
        );

        let factorization = poly.factorize().unwrap();
        let expanded = factorization.expand();

        assert_eq!(factorization.leading_coefficient, 1.0);
        assert_eq!(
            factorization.factors,
            vec![LinearFactor::new(2.0), LinearFactor::new(1.0)]
        );
        assert_eq!(expanded.monomials.len(), 2);
        assert_eq!(expanded.constant, 2.0);
    }

    #[test]
    fn constant_polynomial_cannot_be_factorized() {
        let poly = Quadratic::new(Vec::<QuadraticMonomial<f64>>::new(), 2.0);

        assert!(poly.factorize().is_none());
    }

    #[test]
    fn solves_constant_polynomial_as_empty_roots() {
        let poly = Quadratic::new(Vec::<QuadraticMonomial<f64>>::new(), 2.0);

        let roots = poly.solve().unwrap();
        assert!(roots.is_real);
        assert!(roots.roots.is_empty());
        assert_eq!(roots.discriminant, 0.0);
    }
}
