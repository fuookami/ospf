use crate::algebra::{Arithmetic, Precision};
use crate::operator::Abs;

pub struct Zero<T: Arithmetic + Abs<Output = T>> {
    pub(self) precision: T,
}

impl<T: Arithmetic + Abs<Output = T>> Zero<T> {
    pub fn new() -> Self
    where
        T: Precision,
    {
        Self::new_with(<Self as Precision>::DECIMAL_PRECISION)
    }

    pub fn new_with(precision: T) -> Self {
        Self {
            precision: precision.abs(),
        }
    }

    pub fn precision(&self) -> &T {
        &self.precision
    }
}

impl<T: Arithmetic + Abs<Output = T>> FnOnce<T> for Zero<T> {
    type Output = bool;

    extern "rust-call" fn call_once(self, args: T) -> Self::Output {
        return args.abs() <= self.precision;
    }
}

impl<T: Arithmetic + Abs<Output = T>> FnMut<T> for Zero<T> {
    extern "rust-call" fn call_mut(&mut self, args: T) -> Self::Output {
        return self.call_once(args);
    }
}

impl<T: Arithmetic + Abs<Output = T>> Fn<T> for Zero<T> {
    extern "rust-call" fn call(&mut self, args: T) -> Self::Output {
        return self.call_once(args);
    }
}

impl<T: Arithmetic + Abs<Output = T>> FnOnce<&T> for Zero<T> {
    type Output = bool;

    extern "rust-call" fn call_once(self, args: &T) -> Self::Output {
        return args.abs() <= self.precision;
    }
}

impl<T: Arithmetic + Abs<Output = T>> FnMut<&T> for Zero<T> {
    extern "rust-call" fn call_mut(&mut self, args: &T) -> Self::Output {
        return self.call_once(args);
    }
}

impl<T: Arithmetic + Abs<Output = T>> Fn<&T> for Zero<T> {
    extern "rust-call" fn call(&mut self, args: &T) -> Self::Output {
        return self.call_once(args);
    }
}
