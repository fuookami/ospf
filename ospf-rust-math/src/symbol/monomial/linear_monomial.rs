use std::fmt::{Display, Formatter};
use std::ops::Mul;
use super::monomial::{Monomial, MonomialSymbol};
use crate::symbol::Symbol;

pub struct LinearMonomialSymbol {
    pub symbol: Box<dyn Symbol>
}

impl Display for LinearMonomialSymbol {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self.symbol)
    }
}

impl MonomialSymbol for LinearMonomialSymbol {}

pub struct LinearMonomial<T> {
    pub coefficient: T,
    pub symbol: LinearMonomialSymbol
}

impl<T> Monomial<T> for LinearMonomial<T> {
    type Symbol = LinearMonomialSymbol;

    fn coefficient(&self) -> &T {
        &self.coefficient
    }

    fn symbol(&self) -> &Self::Symbol {
        &self.symbol
    }
}
