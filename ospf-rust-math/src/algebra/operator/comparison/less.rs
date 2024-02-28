use crate::algebra::{Arithmetic, Precision};
use crate::operator::{Abs, Neg};
use std::ops::Sub;

pub struct Less<T: Arithmetic + Abs<Output = T> + Neg<Output = T>> {
    pub(self) precision: T,
    pub(self) neg_precision: T,
}

impl<T: Arithmetic + Abs<Output = T> + Neg<Output = T>> Less<T> {
    pub fn new() -> Self
    where
        T: Precision,
    {
        Self::new_with(<Self as Precision>::DECIMAL_PRECISION)
    }

    pub fn new_with(precision: T) -> Self {
        let actual_precision = precision.abs();
        let neg_precision = actual_precision.neg();
        Self {
            precision: actual_precision,
            neg_precision: neg_precision,
        }
    }

    pub fn precision(&self) -> &T {
        &self.precision
    }
}

impl<T: Arithmetic + Sub<Output = T> + Abs<Output = T> + Neg<Output = T>> FnOnce<(T, T)>
    for Less<T>
{
    type Output = bool;

    extern "rust-call" fn call_once(self, args: (T, T)) -> Self::Output {
        (args.0 - args.1) < self.neg_precision
    }
}

impl<T: Arithmetic + Sub<Output = T> + Abs<Output = T> + Neg<Output = T>> FnMut<(T, T)>
    for Less<T>
{
    extern "rust-call" fn call_mut(&mut self, args: (T, T)) -> Self::Output {
        return self.call_once(args);
    }
}

impl<T: Arithmetic + Sub<Output = T> + Abs<Output = T> + Neg<Output = T>> Fn<(T, T)> for Less<T> {
    extern "rust-call" fn call(&mut self, args: (T, T)) -> Self::Output {
        return self.call_once(args);
    }
}

impl<'a, T: Arithmetic + Abs<Output = T> + Neg<Output = T>> FnOnce<(&'a T, &'a T)> for Less<T>
where
    &'a T: Sub<&'a T, Output = T>,
{
    type Output = bool;

    extern "rust-call" fn call_once(self, args: (&'a T, &'a T)) -> Self::Output {
        (args.0 - args.1) < self.neg_precision
    }
}

impl<'a, T: Arithmetic + Abs<Output = T> + Neg<Output = T>> FnMut<(&'a T, &'a T)> for Less<T>
where
    &'a T: Sub<&'a T, Output = T>,
{
    extern "rust-call" fn call_mut(&mut self, args: (&'a T, &'a T)) -> Self::Output {
        return self.call_once(args);
    }
}

impl<'a, T: Arithmetic + Abs<Output = T> + Neg<Output = T>> Fn<(&'a T, &'a T)> for Less<T>
where
    &'a T: Sub<&'a T, Output = T>,
{
    extern "rust-call" fn call(&mut self, args: (&'a T, &'a T)) -> Self::Output {
        return self.call_once(args);
    }
}
