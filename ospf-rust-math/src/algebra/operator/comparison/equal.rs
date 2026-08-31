use std::ops::{Add, Sub};

use crate::algebra::concept::*;
use crate::algebra::operator::Abs;

use super::ComparisonOperator;

pub trait EqualOpr<T, Rhs = T>: ComparisonOperator<T, Rhs> {
    fn precision(&self) -> Option<&T> {
        None
    }
}

impl<T, Rhs> FnOnce<(&T, &Rhs)> for &dyn EqualOpr<T, Rhs> {
    type Output = bool;

    extern "rust-call" fn call_once(self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnMut<(&T, &Rhs)> for &dyn EqualOpr<T, Rhs> {
    extern "rust-call" fn call_mut(&mut self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> Fn<(&T, &Rhs)> for &dyn EqualOpr<T, Rhs> {
    extern "rust-call" fn call(&self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnOnce<(&T, &Rhs)> for Box<dyn EqualOpr<T, Rhs>> {
    type Output = bool;

    extern "rust-call" fn call_once(self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnMut<(&T, &Rhs)> for Box<dyn EqualOpr<T, Rhs>> {
    extern "rust-call" fn call_mut(&mut self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> Fn<(&T, &Rhs)> for Box<dyn EqualOpr<T, Rhs>> {
    extern "rust-call" fn call(&self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EqualInt {}

impl EqualInt {
    fn new() -> Self {
        Self {}
    }
}

impl<T: PartialEq<Rhs>, Rhs> ComparisonOperator<T, Rhs> for EqualInt {
    fn cmp(&self, x: &T, y: &Rhs) -> bool {
        x == y
    }
}

impl<T: PartialEq<Rhs>, Rhs> EqualOpr<T, Rhs> for EqualInt {
    default fn precision(&self) -> Option<&T> {
        None
    }
}

impl<T: SemiArithmetic + PartialEq<Rhs>, Rhs> EqualOpr<T, Rhs> for EqualInt {
    fn precision(&self) -> Option<&T> {
        Some(T::ZERO)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EqualFlt<T> {
    pub(self) precision: T,
}

impl<T> From<T> for EqualFlt<T> {
    default fn from(precision: T) -> Self {
        Self { precision }
    }
}

impl<T: Signed> From<&T> for EqualFlt<T>
where
    for<'a> &'a T: Abs<Output = T>,
{
    fn from(precision: &T) -> Self {
        Self {
            precision: precision.abs(),
        }
    }
}

impl<T: Signed + Copy + Abs<Output = T>> From<T> for EqualFlt<T> {
    fn from(precision: T) -> Self {
        Self {
            precision: precision.abs(),
        }
    }
}

impl<T> EqualFlt<T> {
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

impl<T: PartialOrd<Rhs>, Rhs> ComparisonOperator<T, Rhs> for EqualFlt<T>
where
    for<'a> &'a T: Add + Sub,
    for<'a> <&'a T as Add>::Output: PartialOrd<Rhs>,
    for<'a> <&'a T as Sub>::Output: PartialOrd<Rhs>,
{
    fn cmp(&self, x: &T, y: &Rhs) -> bool {
        return &(x - &self.precision) <= y && &(x + &self.precision) >= y;
    }
}

impl<T: PartialOrd<Rhs>, Rhs> EqualOpr<T, Rhs> for EqualFlt<T>
where
    for<'a> &'a T: Add + Sub,
    for<'a> <&'a T as Add>::Output: PartialOrd<Rhs>,
    for<'a> <&'a T as Sub>::Output: PartialOrd<Rhs>,
{
    fn precision(&self) -> Option<&T> {
        Some(&self.precision)
    }
}

pub trait EqualOprBuilder<T, Rhs = T> {
    fn new() -> Box<dyn EqualOpr<T, Rhs>>;
    fn new_with(precision: T) -> Box<dyn EqualOpr<T, Rhs>>;
}

pub struct Equal<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: PartialEq<Rhs>, Rhs> EqualOprBuilder<T, Rhs> for Equal<T> {
    default fn new() -> Box<dyn EqualOpr<T, Rhs>> {
        Box::new(EqualInt::new())
    }

    default fn new_with(precision: T) -> Box<dyn EqualOpr<T, Rhs>> {
        Box::new(EqualInt::new())
    }
}

impl<T: 'static + PartialOrd<Rhs>, Rhs> EqualOprBuilder<T, Rhs> for Equal<T>
where
    for<'a> &'a T: Add + Sub,
    for<'a> <&'a T as Add>::Output: PartialOrd<Rhs>,
    for<'a> <&'a T as Sub>::Output: PartialOrd<Rhs>,
{
    default fn new() -> Box<dyn EqualOpr<T, Rhs>> {
        Box::new(EqualInt::new())
    }

    default fn new_with(precision: T) -> Box<dyn EqualOpr<T, Rhs>> {
        Box::new(EqualFlt::new_with(precision))
    }
}

impl<T: FloatingNumber + Clone + PartialOrd<Rhs>, Rhs> EqualOprBuilder<T, Rhs> for Equal<T>
where
    for<'a> &'a T: Add + Sub,
    for<'a> <&'a T as Add>::Output: PartialOrd<Rhs>,
    for<'a> <&'a T as Sub>::Output: PartialOrd<Rhs>,
{
    fn new() -> Box<dyn EqualOpr<T, Rhs>> {
        Box::new(EqualFlt::new())
    }

    fn new_with(precision: T) -> Box<dyn EqualOpr<T, Rhs>> {
        Box::new(EqualFlt::new_with(precision))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eq_int() {
        let eq = Equal::<i64>::new();
        assert_eq!(eq(&0, &0), true);
        assert_eq!(eq(&1, &0), false);
    }

    #[test]
    fn test_eq_flt() {
        let eq = Equal::<f64>::new();
        assert_eq!(eq(&0.0, &0.0), true);
        assert_eq!(eq(&1e-6, &0.0), false);

        let eq = Equal::<f64>::new_with(1e-5);
        assert_eq!(eq(&0.0, &0.0), true);
        assert_eq!(eq(&1e-6, &0.0), true);
    }
}
