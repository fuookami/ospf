use std::ops::{Add, Sub};

use crate::algebra::concept::*;
use crate::algebra::operator::Abs;

use super::ComparisonOperator;

pub trait GreaterOpr<T, Rhs = T>: ComparisonOperator<T, Rhs> {
    fn precision(&self) -> Option<&T> {
        None
    }
}

impl<T, Rhs> FnOnce<(&T, &Rhs)> for &dyn GreaterOpr<T, Rhs> {
    type Output = bool;

    extern "rust-call" fn call_once(self, (x, y): (&T, &Rhs)) -> Self::Output {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnMut<(&T, &Rhs)> for &dyn GreaterOpr<T, Rhs> {
    extern "rust-call" fn call_mut(&mut self, (x, y): (&T, &Rhs)) -> Self::Output {
        self.cmp(x, y)
    }
}

impl<T, Rhs> Fn<(&T, &Rhs)> for &dyn GreaterOpr<T, Rhs> {
    extern "rust-call" fn call(&self, (x, y): (&T, &Rhs)) -> Self::Output {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnOnce<(&T, &Rhs)> for Box<dyn GreaterOpr<T, Rhs>> {
    type Output = bool;

    extern "rust-call" fn call_once(self, (x, y): (&T, &Rhs)) -> Self::Output {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnMut<(&T, &Rhs)> for Box<dyn GreaterOpr<T, Rhs>> {
    extern "rust-call" fn call_mut(&mut self, (x, y): (&T, &Rhs)) -> Self::Output {
        self.cmp(x, y)
    }
}

impl<T, Rhs> Fn<(&T, &Rhs)> for Box<dyn GreaterOpr<T, Rhs>> {
    extern "rust-call" fn call(&self, (x, y): (&T, &Rhs)) -> Self::Output {
        self.cmp(x, y)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GreaterInt {}

impl GreaterInt {
    pub fn new() -> Self {
        Self {}
    }
}

impl<T: PartialOrd<Rhs>, Rhs> ComparisonOperator<T, Rhs> for GreaterInt {
    fn cmp(&self, lhs: &T, rhs: &Rhs) -> bool {
        lhs > rhs
    }
}

impl<T: PartialOrd<Rhs>, Rhs> GreaterOpr<T, Rhs> for GreaterInt {
    default fn precision(&self) -> Option<&T> {
        None
    }
}

impl<T: SemiArithmetic + PartialOrd<Rhs>, Rhs> GreaterOpr<T, Rhs> for GreaterInt {
    fn precision(&self) -> Option<&T> {
        Some(T::ZERO)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GreaterFlt<T> {
    pub(self) precision: T,
}

impl<T> From<T> for GreaterFlt<T> {
    default fn from(precision: T) -> Self {
        Self { precision }
    }
}

impl<T: Signed> From<&T> for GreaterFlt<T>
where
    for<'a> &'a T: Abs<Output = T>,
{
    fn from(precision: &T) -> Self {
        Self {
            precision: precision.abs(),
        }
    }
}

impl<T: Signed + Copy + Abs<Output = T>> From<T> for GreaterFlt<T> {
    fn from(precision: T) -> Self {
        Self {
            precision: precision.abs(),
        }
    }
}

impl<T> GreaterFlt<T> {
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

impl<T: PartialOrd<Rhs>, Rhs> ComparisonOperator<T, Rhs> for GreaterFlt<T>
where
    for<'a> &'a T: Sub,
    for<'a> <&'a T as Sub>::Output: PartialOrd<Rhs>,
{
    fn cmp(&self, x: &T, y: &Rhs) -> bool {
        if x < y {
            false
        } else {
            &(x - &self.precision) > y
        }
    }
}

impl<T: PartialOrd<Rhs>, Rhs> GreaterOpr<T, Rhs> for GreaterFlt<T>
where
    for<'a> &'a T: Sub,
    for<'a> <&'a T as Sub>::Output: PartialOrd<Rhs>,
{
    fn precision(&self) -> Option<&T> {
        Some(&self.precision)
    }
}

pub trait GreaterOprBuilder<T, Rhs = T> {
    fn new() -> Box<dyn GreaterOpr<T, Rhs>>;
    fn new_with(precision: T) -> Box<dyn GreaterOpr<T, Rhs>>;
}

pub struct Greater {}

impl<T: PartialOrd<Rhs>, Rhs> GreaterOprBuilder<T, Rhs> for Greater {
    default fn new() -> Box<dyn GreaterOpr<T, Rhs>> {
        Box::new(GreaterInt::new())
    }

    default fn new_with(precision: T) -> Box<dyn GreaterOpr<T, Rhs>> {
        Box::new(GreaterInt::new())
    }
}

impl<T: 'static + PartialOrd<Rhs>, Rhs> GreaterOprBuilder<T, Rhs> for Greater
where
    for<'a> &'a T: Sub,
    for<'a> <&'a T as Sub>::Output: PartialOrd<Rhs>,
{
    default fn new() -> Box<dyn GreaterOpr<T, Rhs>> {
        Box::new(GreaterInt::new())
    }

    default fn new_with(precision: T) -> Box<dyn GreaterOpr<T, Rhs>> {
        Box::new(GreaterFlt::new_with(precision))
    }
}

impl<T: FloatingNumber + Clone + PartialOrd<Rhs>, Rhs> GreaterOprBuilder<T, Rhs> for Greater
where
    for<'a> &'a T: Sub,
    for<'a> <&'a T as Sub>::Output: PartialOrd<Rhs>,
{
    fn new() -> Box<dyn GreaterOpr<T, Rhs>> {
        Box::new(GreaterFlt::new())
    }

    fn new_with(precision: T) -> Box<dyn GreaterOpr<T, Rhs>>
    where
        GreaterFlt<T>: From<T>,
    {
        Box::new(GreaterFlt::new_with(precision))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gr_int() {
        let gr = Greater::new();
        assert_eq!(gr(&1, &2), false);
        assert_eq!(gr(&2, &1), true);
        assert_eq!(gr(&1, &1), false);
    }

    #[test]
    fn test_gr_flt() {
        let gr = Greater::new();
        assert_eq!(gr(&0.0, &0.0), false);
        assert_eq!(gr(&0.0, &1e-6), false);
        assert_eq!(gr(&1e-6, &0.0), true);
        assert_eq!(gr(&0.0, &1e-4), false);
        assert_eq!(gr(&1e-4, &0.0), true);

        let gr = Greater::new_with(1e-5);
        assert_eq!(gr(&0.0, &0.0), false);
        assert_eq!(gr(&0.0, &1e-6), false);
        assert_eq!(gr(&1e-6, &0.0), false);
        assert_eq!(gr(&0.0, &1e-4), false);
        assert_eq!(gr(&1e-4, &0.0), true);
    }
}
