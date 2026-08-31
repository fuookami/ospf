use crate::symbol::monomial::Monomial;
use crate::symbol::Expression;
use std::ops::AddAssign;

pub trait Polynomial: Expression {
    type MonomialType: Monomial;

    fn monomials<'a>(&'a self) -> impl Iterator<Item = &'a Self::MonomialType>
    where
        <Self as Polynomial>::MonomialType: 'a;

    fn constant(&self) -> &<<Self as Polynomial>::MonomialType as Monomial>::ValueType;
}
