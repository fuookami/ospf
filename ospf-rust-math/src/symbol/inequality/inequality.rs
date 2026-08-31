use super::sign::SignType;
use crate::symbol::polynomial::Polynomial;
use crate::symbol::Expression;

pub trait Inequality: Expression<ResultType = bool> {
    type PolynomialType: Polynomial;

    fn lhs(&self) -> &Self::PolynomialType;
    fn rhs(&self) -> &Self::PolynomialType;
    fn sign(&self) -> SignType;

    fn reverse(&self) -> Self;
}
