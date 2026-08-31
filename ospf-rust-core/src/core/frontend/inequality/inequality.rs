use crate::core::frontend::expression::Expression;
use crate::core::frontend::monomial::Monomial;
use crate::core::frontend::polynomial::Polynomial;
use crate::core::frontend::token::{
    AbstractTokenList, AbstractTokenTable, Evaluate, TokenValueType,
};
use ospf_rust_math::{Category, RealNumber};
use std::fmt::Display;
use std::ops::Mul;

pub trait Inequality<const category: Category, T: RealNumber + TokenValueType>:
    ospf_rust_math::symbol::inequality::Inequality + Evaluate<T, ResultType = bool>
{
    type PolynomialType: Polynomial<category, T>;

    fn normalize(&self) -> Self;

    fn cell<'a>(
        &'a self,
    ) -> impl Iterator<Item=&'a <<<Self as Inequality<category, T>>::PolynomialType as Polynomial<category, T>>::MonomialType as Monomial<category, T>>::Cell>
    where
        <<<Self as Inequality<category, T>>::PolynomialType as Polynomial<category, T>>::MonomialType as Monomial<category, T>>::Cell: 'a;

    fn cached(&self) -> bool;

    fn flush(&self) {
        self.flush_with(false)
    }
    fn flush_with(&self, forced: bool);

    fn is_true<V: TokenValueType, TL: AbstractTokenList<V>>(
        &self,
        token_list: &TL,
        zero_if_none: bool,
    ) -> Option<bool>
    where
        for<'a> &'a T: Mul<&'a V, Output = T>,
    {
        self.evaluate(token_list, zero_if_none)
    }

    fn is_true_with<V: TokenValueType, TL: AbstractTokenList<V>>(
        &self,
        solution: &[V],
        token_list: &TL,
        zero_if_none: bool,
    ) -> Option<bool>
    where
        for<'a> &'a T: Mul<&'a V, Output = T>,
    {
        self.evaluate_with(solution, token_list, zero_if_none)
    }

    fn is_true_in<V: TokenValueType, TB: AbstractTokenTable<V>>(
        &self,
        token_table: &TB,
        zero_if_none: bool,
    ) -> Option<bool>
    where
        for<'a> &'a T: Mul<&'a V, Output = T>,
    {
        self.evaluate_in(token_table, zero_if_none)
    }

    fn is_true_within<V: TokenValueType, TB: AbstractTokenTable<V>>(
        &self,
        solution: &[V],
        token_table: &TB,
        zero_if_none: bool,
    ) -> Option<bool>
    where
        for<'a> &'a T: Mul<&'a V, Output = T>,
    {
        self.evaluate_within(solution, token_table, zero_if_none)
    }
}
