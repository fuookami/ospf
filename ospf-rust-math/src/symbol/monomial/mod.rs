use std::cell::RefCell;
use crate::Symbol;

pub trait MonomialSymbol {

}

pub trait Monomial<T, V> {
    type Symbol: MonomialSymbol;

    fn coefficient(&self) -> T;
    fn symbol(&self) -> Self::Symbol;
}

struct LinearMonomialSymbol {
    symbol: RefCell<dyn Symbol>
}
