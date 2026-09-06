//! 规范化操作
//! Normalization operations

use crate::operator::Exponent;
use crate::symbol::operation::combine::CombineTerms;
use crate::symbol::{
    Canonical, CanonicalInequality, Linear, LinearInequality, Quadratic, QuadraticInequality,
};
use num_traits::Zero;
use std::hash::Hash;
use std::ops::{Add, Sub};

/// 规范化 trait。
/// Normalization trait.
///
/// 对多项式，规范化表示合并同类项并移除零系数项。
/// For polynomials, normalization combines like terms and removes zero coefficients.
///
/// 对不等式，规范化表示将右侧常数并入左侧，并将右侧置零。
/// For inequalities, normalization moves the right-hand-side constant into the left-hand side
/// and sets the right-hand side to zero.
pub trait Normalize {
    /// 原地规范化。
    /// Normalize in place.
    fn normalize(&mut self);

    /// 返回规范化后的新实例。
    /// Return a normalized new instance.
    fn normalized(&self) -> Self
    where
        Self: Clone,
    {
        let mut result = self.clone();
        result.normalize();
        result
    }
}

/// 消耗值并返回规范化结果。
/// Consume a value and return its normalized result.
pub fn normalize<N: Normalize>(mut value: N) -> N {
    value.normalize();
    value
}

impl<T> Normalize for Linear<T>
where
    T: Clone + Zero + PartialEq + Add<Output = T>,
{
    fn normalize(&mut self) {
        self.combine_terms();
    }
}

impl<T> Normalize for Quadratic<T>
where
    T: Clone + Zero + PartialEq + Add<Output = T>,
{
    fn normalize(&mut self) {
        self.combine_terms();
    }
}

impl<T, E> Normalize for Canonical<T, E>
where
    T: Clone + Zero + PartialEq + Add<Output = T>,
    E: Exponent + Hash + Eq,
{
    fn normalize(&mut self) {
        self.combine_terms();
    }
}

impl<T> Normalize for LinearInequality<T>
where
    T: Clone + Zero + PartialEq + Add<Output = T> + Sub<Output = T>,
{
    fn normalize(&mut self) {
        self.lhs.constant = self.lhs.constant.clone() - self.rhs.clone();
        self.rhs = T::zero();
        self.lhs.combine_terms();
    }
}

impl<T> Normalize for QuadraticInequality<T>
where
    T: Clone + Zero + PartialEq + Add<Output = T> + Sub<Output = T>,
{
    fn normalize(&mut self) {
        self.lhs.constant = self.lhs.constant.clone() - self.rhs.clone();
        self.rhs = T::zero();
        self.lhs.combine_terms();
    }
}

impl<T, E> Normalize for CanonicalInequality<T, E>
where
    T: Clone + Zero + PartialEq + Add<Output = T> + Sub<Output = T>,
    E: Exponent + Hash + Eq,
{
    fn normalize(&mut self) {
        self.lhs.constant = self.lhs.constant.clone() - self.rhs.clone();
        self.rhs = T::zero();
        self.lhs.combine_terms();
    }
}

#[cfg(test)]
mod tests {
    use super::{Normalize, normalize};
    use crate::symbol::{
        Canonical, CanonicalInequality, CanonicalMonomial, Comparison, DynSymbol, Linear,
        LinearInequality, LinearMonomial, OwnedSymbol, Quadratic, QuadraticInequality,
        QuadraticMonomial, SymbolDynId,
    };
    use std::any::Any;
    use std::collections::HashMap;

    #[derive(Debug, Clone)]
    struct TestSymbol {
        id: usize,
        name: String,
    }

    impl std::fmt::Display for TestSymbol {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    fn normalizes_linear_polynomial_terms() {
        let x = make_symbol("x", 1);
        let poly = Linear::new(
            vec![
                LinearMonomial::new(1.0, x.clone()),
                LinearMonomial::new(2.0, x),
            ],
            0.0,
        );

        let normalized = poly.normalized();
        assert_eq!(normalized.monomials.len(), 1);
        assert_eq!(normalized.monomials[0].coefficient, 3.0);
    }

    #[test]
    fn normalizes_linear_inequality_rhs_to_zero() {
        let x = make_symbol("x", 1);
        let inequality = LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(2.0, x)], 3.0),
            Comparison::LessEqual,
            5.0,
        );

        let normalized = normalize(inequality);
        assert_eq!(normalized.comparison, Comparison::LessEqual);
        assert_eq!(normalized.rhs, 0.0);
        assert_eq!(normalized.lhs.constant, -2.0);
    }

    #[test]
    fn normalizes_quadratic_inequality_rhs_to_zero() {
        let x = make_symbol("x", 1);
        let inequality = QuadraticInequality::new(
            Quadratic::new(vec![QuadraticMonomial::linear(2.0, x)], 7.0),
            Comparison::GreaterEqual,
            5.0,
        );

        let normalized = normalize(inequality);
        assert_eq!(normalized.comparison, Comparison::GreaterEqual);
        assert_eq!(normalized.rhs, 0.0);
        assert_eq!(normalized.lhs.constant, 2.0);
    }

    #[test]
    fn normalizes_canonical_inequality_rhs_to_zero() {
        let x = make_symbol("x", 1);
        let mut powers = HashMap::new();
        powers.insert(x, 2);
        let inequality = CanonicalInequality::new(
            Canonical::new(vec![CanonicalMonomial::new(3.0, powers)], 1.0),
            Comparison::Equal,
            4.0,
        );

        let normalized = normalize(inequality);
        assert_eq!(normalized.comparison, Comparison::Equal);
        assert_eq!(normalized.rhs, 0.0);
        assert_eq!(normalized.lhs.constant, -3.0);
    }
}
