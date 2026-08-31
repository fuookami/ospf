use super::sign::SignType;
use super::super::polynomial::Polynomial;

pub trait Inequality<T> {
    type PolynomialType: Polynomial<T>;

    fn lhs(&self) -> &Self::PolynomialType;
    fn rhs(&self) -> &Self::PolynomialType;
    fn sign(&self) -> SignType;
}
