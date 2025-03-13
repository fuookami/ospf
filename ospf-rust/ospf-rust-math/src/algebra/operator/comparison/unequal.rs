use super::Zero;
use crate::algebra::{Arithmetic, Precision};
use crate::operator::Abs;
use std::ops::Sub;

pub struct Unequal<T: Arithmetic + Abs<Output = T>> {
    pub(self) zero_op: Zero<T>,
}

impl<T: Arithmetic + Abs<Output = T>> Unequal<T> {
    fn new() -> Self
    where
        T: Precision,
    {
        Self {
            zero_op: Zero::new(),
        }
    }

    fn new_with(precision: T) -> Self {
        Self {
            zero_op: Zero::new_with(precision),
        }
    }
}

impl<T: Arithmetic + Sub<Output = T> + Abs<Output = T>> FnOnce<(T, T)> for Unequal<T> {
    type Output = bool;

    extern "rust-call" fn call_once(self, args: (T, T)) -> Self::Output {
        !(self.zero_op)(args.0 - args.1)
    }
}

impl<T: Arithmetic + Sub<Output = T> + Abs<Output = T>> FnMut<(T, T)> for Unequal<T> {
    extern "rust-call" fn call_mut(&mut self, args: (T, T)) -> Self::Output {
        return self.call_once(args);
    }
}

impl<T: Arithmetic + Sub<Output = T> + Abs<Output = T>> Fn<(T, T)> for Unequal<T> {
    extern "rust-call" fn call(&mut self, args: (T, T)) -> Self::Output {
        return self.call_once(args);
    }
}

impl<'a, T: Arithmetic + Abs<Output = T>> FnOnce<(&'a T, &'a T)> for Unequal<T>
where
    &'a T: Sub<&'a T, Output = T>,
{
    type Output = bool;

    extern "rust-call" fn call_once(self, args: (&'a T, &'a T)) -> Self::Output {
        !(self.zero_op)(args.0 - args.1)
    }
}

impl<'a, T: Arithmetic + Abs<Output = T>> FnMut<(&'a T, &'a T)> for Unequal<T>
where
    &'a T: Sub<&'a T, Output = T>,
{
    extern "rust-call" fn call_mut(&mut self, args: (&'a T, &'a T)) -> Self::Output {
        return self.call_once(args);
    }
}

impl<'a, T: Arithmetic + Abs<Output = T>> Fn<(&'a T, &'a T)> for Unequal<T>
where
    &'a T: Sub<&'a T, Output = T>,
{
    extern "rust-call" fn call(&mut self, args: (&'a T, &'a T)) -> Self::Output {
        return self.call_once(args);
    }
}
