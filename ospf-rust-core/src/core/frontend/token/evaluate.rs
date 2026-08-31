use super::token::TokenValueType;
use super::token_list::AbstractTokenList;
use super::token_table::AbstractTokenTable;
use ospf_rust_math::RealNumber;
use std::fmt::Display;
use std::ops::Mul;

pub trait Evaluate<T> {
    type ResultType = T;

    fn evaluate<V: TokenValueType, TL: AbstractTokenList<V>>(
        &self,
        token_list: &TL,
        zero_if_none: bool,
    ) -> Option<Self::ResultType>
    where
        for<'a> &'a T: Mul<&'a V, Output = T>;

    fn evaluate_with<V: TokenValueType, TL: AbstractTokenList<V>>(
        &self,
        solution: &[V],
        token_list: &TL,
        zero_if_none: bool,
    ) -> Option<Self::ResultType>
    where
        for<'a> &'a T: Mul<&'a V, Output = T>;

    fn evaluate_in<V: TokenValueType, TB: AbstractTokenTable<V>>(
        &self,
        token_table: &TB,
        zero_if_none: bool,
    ) -> Option<Self::ResultType>
    where
        for<'a> &'a T: Mul<&'a V, Output = T>,
    {
        self.evaluate(token_table.token_list(), zero_if_none)
    }

    fn evaluate_within<V: TokenValueType, TB: AbstractTokenTable<V>>(
        &self,
        solution: &[V],
        token_table: &TB,
        zero_if_none: bool,
    ) -> Option<Self::ResultType>
    where
        for<'a> &'a T: Mul<&'a V, Output = T>,
    {
        self.evaluate_with(solution, token_table.token_list(), zero_if_none)
    }
}
