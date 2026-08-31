use super::monomial::{Monomial, MonomialSymbol};
use crate::concept::RealNumber;
use crate::symbol::{Symbol, CompositeSymbol, Expression, SymbolIdentifier};
use std::cell::Cell;
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;
use std::ops::Mul;

pub enum LinearMonomialSymbol<T> {
    Pure(Box<dyn Symbol>),
    Composite(Box<dyn CompositeSymbol<ResultType = T>>)
}

impl<T> Display for LinearMonomialSymbol<T> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            LinearMonomialSymbol::Pure(s) => write!(f, "{}", s),
            LinearMonomialSymbol::Composite(s) => write!(f, "{}", s),
        }
    }
}

impl<T> MonomialSymbol for LinearMonomialSymbol<T> {
    fn to_raw_string_with(&self, unfold: bool) -> String {
        match self {
            LinearMonomialSymbol::Pure(s) => s.display_name().unwrap_or(s.name()).to_string(),
            LinearMonomialSymbol::Composite(s) => s.to_raw_string_with(unfold),
        }
    }
}

pub struct LinearMonomial<T> {
    pub coefficient: T,
    pub symbol: LinearMonomialSymbol<T>,
}

impl<T: Display> Display for LinearMonomial<T> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{} * {}", self.coefficient, self.symbol)
    }
}

impl<T: Display> Expression for LinearMonomial<T> {
    type ResultType = T;

    fn to_raw_string_with(&self, unfold: bool) -> String {
        format!(
            "{} * {}",
            self.coefficient,
            self.symbol.to_raw_string_with(unfold)
        )
    }
}

impl<T: Display> Monomial for LinearMonomial<T> {
    type ValueType = T;
    type Symbol = LinearMonomialSymbol<T>;

    fn coefficient(&self) -> &T {
        &self.coefficient
    }

    fn symbol(&self) -> &Self::Symbol {
        &self.symbol
    }
}
