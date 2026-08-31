use super::monomial_cell::MonomialCell;
use crate::core::frontend::expression::Expression;
use crate::core::frontend::token::{Evaluate, TokenValueType};
use ospf_rust_math::symbol::{Category, ExpressionRange};
use ospf_rust_math::{Bound, Bounded, RealNumber};
use std::fmt::Display;

pub trait MonomialSymbol<const category: Category>:
    ospf_rust_math::symbol::monomial::MonomialSymbol
{
    fn category(&self) -> Category;
    fn discrete(&self) -> bool;
}

pub trait Monomial<const category: Category, T: RealNumber + TokenValueType>:
    ospf_rust_math::symbol::monomial::Monomial + Expression<T> + Evaluate<T, ResultType = T>
{
    type Cell: MonomialCell<T>;
    type Symbol: MonomialSymbol<category>;

    fn set_name(&self, name: &str);

    fn cell<'a>(&'a self) -> impl Iterator<Item = &'a Self::Cell>
    where
        <Self as Monomial<category, T>>::Cell: 'a;

    fn cached(&self) -> bool;

    fn flush(&self) {
        self.flush_with(false);
    }

    fn flush_with(&self, forced: bool);
}

pub trait LinearMonomial<T: RealNumber + TokenValueType = f64>: Monomial<{ Category::Linear }, T> {}
pub trait QuadraticMonomial<T: RealNumber + TokenValueType = f64>: Monomial<{ Category::Quadratic }, T> {}
