use std::fmt::Display;

pub trait MonomialSymbol: Display {}

pub trait Monomial<T> {
    type Symbol: MonomialSymbol;

    fn coefficient(&self) -> &T;
    fn symbol(&self) -> &Self::Symbol;
}
