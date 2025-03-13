use crate::algebra::{Arithmetic, Precision};
use crate::operator::{Abs, Neg};
use std::ops::Sub;

pub struct Greater<T: Arithmetic + Abs<Output = T> + Neg<Output = T>> {
    pub(self) precision: T,
}

impl<T: Arithmetic + Abs<Output = T> + Neg<Output = T>> Greater<T> {
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

impl<T: Arithmetic + Sub<Output = T> + Abs<Output = T> + Neg<Output = T>> FnOnce<(T, T)>
    for Greater<T>
{
    type Output = bool;

    extern "rust-call" fn call_once(self, args: (T, T)) -> Self::Output {
        (args.0 - args.1) > self.precision
    }
}

impl<T: Arithmetic + Sub<Output = T> + Abs<Output = T> + Neg<Output = T>> FnMut<(T, T)>
    for Greater<T>
{
    extern "rust-call" fn call_mut(&mut self, args: (T, T)) -> Self::Output {
        return self.call_once(args);
    }
}

impl<T: Arithmetic + Sub<Output = T> + Abs<Output = T> + Neg<Output = T>> Fn<(T, T)>
    for Greater<T>
{
    extern "rust-call" fn call(&mut self, args: (T, T)) -> Self::Output {
        return self.call_once(args);
    }
}

impl<'a, T: Arithmetic + Abs<Output = T> + Neg<Output = T>> FnOnce<(&'a T, &'a T)> for Greater<T>
where
    &'a T: Sub<&'a T, Output = T>,
{
    type Output = bool;

    extern "rust-call" fn call_once(self, args: (&'a T, &'a T)) -> Self::Output {
        (args.0 - args.1) > self.precision
    }
}

impl<'a, T: Arithmetic + Abs<Output = T> + Neg<Output = T>> FnMut<(&'a T, &'a T)> for Greater<T>
where
    &'a T: Sub<&'a T, Output = T>,
{
    extern "rust-call" fn call_mut(&mut self, args: (&'a T, &'a T)) -> Self::Output {
        return self.call_once(args);
    }
}

impl<'a, T: Arithmetic + Abs<Output = T> + Neg<Output = T>> Fn<(&'a T, &'a T)> for Greater<T>
where
    &'a T: Sub<&'a T, Output = T>,
{
    extern "rust-call" fn call(&mut self, args: (&'a T, &'a T)) -> Self::Output {
        return self.call_once(args);
    }
}
