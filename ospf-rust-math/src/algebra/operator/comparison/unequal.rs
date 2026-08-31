use std::ops::{Add, Sub};

use num::CheckedMul;

use crate::algebra::concept::*;
use crate::algebra::operator::Abs;

use super::ComparisonOperator;

pub trait UnequalOpr<T, Rhs = T>: ComparisonOperator<T, Rhs> {
    fn precision(&self) -> Option<&T>;
}

impl<T, Rhs> FnOnce<(&T, &Rhs)> for &dyn UnequalOpr<T, Rhs> {
    type Output = bool;

    extern "rust-call" fn call_once(self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnMut<(&T, &Rhs)> for &dyn UnequalOpr<T, Rhs> {
    extern "rust-call" fn call_mut(&mut self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> Fn<(&T, &Rhs)> for &dyn UnequalOpr<T, Rhs> {
    extern "rust-call" fn call(&self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnOnce<(&T, &Rhs)> for Box<dyn UnequalOpr<T, Rhs>> {
    type Output = bool;

    extern "rust-call" fn call_once(self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnMut<(&T, &Rhs)> for Box<dyn UnequalOpr<T, Rhs>> {
    extern "rust-call" fn call_mut(&mut self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> Fn<(&T, &Rhs)> for Box<dyn UnequalOpr<T, Rhs>> {
    extern "rust-call" fn call(&self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct UnequalInt {}

impl UnequalInt {
    fn new() -> Self {
        Self {}
    }
}

impl<T: PartialEq<Rhs>, Rhs> ComparisonOperator<T, Rhs> for UnequalInt {
    fn cmp(&self, x: &T, y: &Rhs) -> bool {
        x != y
    }
}

impl<T: PartialEq<Rhs>, Rhs> UnequalOpr<T, Rhs> for UnequalInt {
    default fn precision(&self) -> Option<&T> {
        None
    }
}

impl<T: SemiArithmetic + PartialEq<Rhs>, Rhs> UnequalOpr<T, Rhs> for UnequalInt {
    fn precision(&self) -> Option<&T> {
        Some(T::ZERO)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct UnequalFlt<T> {
    pub(self) precision: T,
}

impl<T> From<T> for UnequalFlt<T> {
    default fn from(precision: T) -> Self {
        Self { precision }
    }
}

impl<T: Signed> From<&T> for UnequalFlt<T>
where
    for<'a> &'a T: Abs<Output = T>,
{
    fn from(precision: &T) -> Self {
        Self {
            precision: precision.abs(),
        }
    }
}

impl<T: Signed + Copy + Abs<Output = T>> From<T> for UnequalFlt<T> {
    fn from(precision: T) -> Self {
        Self {
            precision: precision.abs(),
        }
    }
}

impl<T> UnequalFlt<T> {
    pub fn new() -> Self
    where
        T: Precision + Clone,
    {
        Self {
            precision: <T as Precision>::DECIMAL_PRECISION.clone(),
        }
    }

    pub fn new_with(precision: T) -> Self
    where
        Self: From<T>,
    {
        Self::from(precision)
    }
}

impl<T: PartialOrd<Rhs>, Rhs> ComparisonOperator<T, Rhs> for UnequalFlt<T>
where
    for<'a> &'a T: Add + Sub,
    for<'a> <&'a T as Add>::Output: PartialOrd<Rhs>,
    for<'a> <&'a T as Sub>::Output: PartialOrd<Rhs>,
{
    fn cmp(&self, x: &T, y: &Rhs) -> bool {
        return &(x - &self.precision) > y || &(x + &self.precision) < y;
    }
}

impl<T: PartialOrd<Rhs>, Rhs> UnequalOpr<T, Rhs> for UnequalFlt<T>
where
    for<'a> &'a T: Add + Sub,
    for<'a> <&'a T as Add>::Output: PartialOrd<Rhs>,
    for<'a> <&'a T as Sub>::Output: PartialOrd<Rhs>,
{
    fn precision(&self) -> Option<&T> {
        Some(&self.precision)
    }
}

pub trait UnequalOprBuilder<T, Rhs = T> {
    fn new() -> Box<dyn UnequalOpr<T, Rhs>>;
    fn new_with(precision: T) -> Box<dyn UnequalOpr<T, Rhs>>;
}

pub struct Unequal<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: PartialEq<Rhs>, Rhs> UnequalOprBuilder<T, Rhs> for Unequal<T> {
    default fn new() -> Box<dyn UnequalOpr<T, Rhs>> {
        Box::new(UnequalInt::new())
    }

    default fn new_with(precision: T) -> Box<dyn UnequalOpr<T, Rhs>> {
        Box::new(UnequalInt::new())
    }
}

impl<T: 'static + PartialOrd<Rhs>, Rhs> UnequalOprBuilder<T, Rhs> for Unequal<T>
where
    for<'a> &'a T: Add + Sub,
    for<'a> <&'a T as Add>::Output: PartialOrd<Rhs>,
    for<'a> <&'a T as Sub>::Output: PartialOrd<Rhs>,
{
    default fn new() -> Box<dyn UnequalOpr<T, Rhs>> {
        Box::new(UnequalInt::new())
    }

    default fn new_with(precision: T) -> Box<dyn UnequalOpr<T, Rhs>> {
        Box::new(UnequalFlt::new_with(precision))
    }
}

impl<T: FloatingNumber + Clone + PartialOrd<Rhs>, Rhs> UnequalOprBuilder<T, Rhs> for Unequal<T>
where
    for<'a> &'a T: Add + Sub,
    for<'a> <&'a T as Add>::Output: PartialOrd<Rhs>,
    for<'a> <&'a T as Sub>::Output: PartialOrd<Rhs>,
{
    fn new() -> Box<dyn UnequalOpr<T, Rhs>> {
        Box::new(UnequalFlt::new())
    }

    fn new_with(precision: T) -> Box<dyn UnequalOpr<T, Rhs>> {
        Box::new(UnequalFlt::new_with(precision))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neq_int() {
        let neq = Unequal::<i64>::new();
        assert_eq!(neq(&0, &0), false);
        assert_eq!(neq(&1, &0), true);
    }

    #[test]
    fn test_neq_flt() {
        let neq = Unequal::<f64>::new();
        assert_eq!(neq(&0.0, &0.0), false);
        assert_eq!(neq(&1e-6, &0.0), true);

        let neq = Unequal::<f64>::new_with(1e-5);
        assert_eq!(neq(&0.0, &0.0), false);
        assert_eq!(neq(&1e-6, &0.0), false);
    }
}
