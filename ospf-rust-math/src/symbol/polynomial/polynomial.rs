use super::super::monomial::Monomial;

pub trait Polynomial<T> {
    type MonomialType: Monomial<T>;

    fn monomials<'a>(&'a self) -> impl Iterator<Item=&'a Self::MonomialType> where <Self as Polynomial<T>>::MonomialType: 'a;
}
