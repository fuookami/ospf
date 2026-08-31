use crate::symbol::{Category, Expression};
use std::fmt::Display;

pub trait MonomialSymbol: Display {
    fn to_raw_string(&self) -> String {
        self.to_raw_string_with(true)
    }
    fn to_raw_string_with(&self, unfold: bool) -> String;
}

pub trait Monomial: Expression {
    type ValueType;
    type Symbol: MonomialSymbol;

    fn coefficient(&self) -> &Self::ValueType;
    fn symbol(&self) -> &Self::Symbol;
}
