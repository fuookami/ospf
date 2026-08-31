use std::ops::{Add, Sub};

use crate::algebra::concept::*;
use crate::algebra::operator::Abs;

use super::ComparisonOperator;

pub trait LessEqualOpr<T, Rhs = T>: ComparisonOperator<T, Rhs> {
    fn precision(&self) -> Option<&T> {
        None
    }
}

impl<T, Rhs> FnOnce<(&T, &Rhs)> for &dyn LessEqualOpr<T, Rhs> {
    type Output = bool;

    extern "rust-call" fn call_once(self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnMut<(&T, &Rhs)> for &dyn LessEqualOpr<T, Rhs> {
    extern "rust-call" fn call_mut(&mut self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> Fn<(&T, &Rhs)> for &dyn LessEqualOpr<T, Rhs> {
    extern "rust-call" fn call(&self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnOnce<(&T, &Rhs)> for Box<dyn LessEqualOpr<T, Rhs>> {
    type Output = bool;

    extern "rust-call" fn call_once(self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnMut<(&T, &Rhs)> for Box<dyn LessEqualOpr<T, Rhs>> {
    extern "rust-call" fn call_mut(&mut self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> Fn<(&T, &Rhs)> for Box<dyn LessEqualOpr<T, Rhs>> {
    extern "rust-call" fn call(&self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LessEqualInt {}

impl LessEqualInt {
    pub fn new() -> Self {
        Self {}
    }
}

impl<T: PartialOrd<Rhs>, Rhs> ComparisonOperator<T, Rhs> for LessEqualInt {
    fn cmp(&self, x: &T, y: &Rhs) -> bool {
        x <= y
    }
}

impl<T: PartialOrd<Rhs>, Rhs> LessEqualOpr<T, Rhs> for LessEqualInt {
    default fn precision(&self) -> Option<&T> {
        None
    }
}

impl<T: SemiArithmetic + PartialOrd<Rhs>, Rhs> LessEqualOpr<T, Rhs> for LessEqualInt {
    fn precision(&self) -> Option<&T> {
        Some(T::ZERO)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LessEqualFlt<T> {
    pub(self) precision: T,
}

impl<T> From<T> for LessEqualFlt<T> {
    default fn from(precision: T) -> Self {
        Self { precision }
    }
}

impl<T: Signed> From<&T> for LessEqualFlt<T>
where
    for<'a> &'a T: Abs<Output = T>,
{
    fn from(precision: &T) -> Self {
        Self {
            precision: precision.abs(),
        }
    }
}

impl<T: Signed + Copy + Abs<Output = T>> From<T> for LessEqualFlt<T> {
    fn from(precision: T) -> Self {
        Self {
            precision: precision.abs(),
        }
    }
}

impl<T> LessEqualFlt<T> {
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

impl<T: PartialOrd<Rhs>, Rhs> ComparisonOperator<T, Rhs> for LessEqualFlt<T>
where
    for<'a> &'a T: Sub,
    for<'a> <&'a T as Sub>::Output: PartialOrd<Rhs>,
{
    fn cmp(&self, x: &T, y: &Rhs) -> bool {
        if x < y {
            true
        } else {
            &(x - &self.precision) <= y
        }
    }
}

impl<T: PartialOrd<Rhs>, Rhs> LessEqualOpr<T, Rhs> for LessEqualFlt<T>
where
    for<'a> &'a T: Sub,
    for<'a> <&'a T as Sub>::Output: PartialOrd<Rhs>,
{
    fn precision(&self) -> Option<&T> {
        Some(&self.precision)
    }
}

pub trait LessEqualOprBuilder<T, Rhs = T> {
    fn new() -> Box<dyn LessEqualOpr<T, Rhs>>;
    fn new_with(precision: T) -> Box<dyn LessEqualOpr<T, Rhs>>;
}

pub struct LessEqual {}

impl<T: PartialOrd<Rhs>, Rhs> LessEqualOprBuilder<T, Rhs> for LessEqual {
    default fn new() -> Box<dyn LessEqualOpr<T, Rhs>> {
        Box::new(LessEqualInt::new())
    }

    default fn new_with(precision: T) -> Box<dyn LessEqualOpr<T, Rhs>> {
        Box::new(LessEqualInt::new())
    }
}

impl<T: 'static + PartialOrd<Rhs>, Rhs> LessEqualOprBuilder<T, Rhs> for LessEqual
where
    for<'a> &'a T: Sub,
    for<'a> <&'a T as Sub>::Output: PartialOrd<Rhs>,
{
    default fn new() -> Box<dyn LessEqualOpr<T, Rhs>> {
        Box::new(LessEqualInt::new())
    }

    default fn new_with(precision: T) -> Box<dyn LessEqualOpr<T, Rhs>> {
        Box::new(LessEqualFlt::new_with(precision))
    }
}

impl<T: FloatingNumber + Clone + PartialOrd<Rhs>, Rhs> LessEqualOprBuilder<T, Rhs> for LessEqual
where
    for<'a> &'a T: Sub,
    for<'a> <&'a T as Sub>::Output: PartialOrd<Rhs>,
{
    fn new() -> Box<dyn LessEqualOpr<T, Rhs>> {
        Box::new(LessEqualFlt::new())
    }

    fn new_with(precision: T) -> Box<dyn LessEqualOpr<T, Rhs>> {
        Box::new(LessEqualFlt::new_with(precision))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leq_int() {
        let leq = LessEqual::new();
        assert_eq!(leq(&1, &2), true);
        assert_eq!(leq(&2, &1), false);
        assert_eq!(leq(&1, &1), true);
    }

    #[test]
    fn test_leq_flt() {
        let leq = LessEqual::new();
        assert_eq!(leq(&0.0, &0.0), true);
        assert_eq!(leq(&0.0, &1e-6), true);
        assert_eq!(leq(&1e-6, &0.0), false);
        assert_eq!(leq(&0.0, &1e-4), true);
        assert_eq!(leq(&1e-4, &0.0), false);

        let leq = LessEqual::new_with(1e-5);
        assert_eq!(leq(&0.0, &0.0), true);
        assert_eq!(leq(&0.0, &1e-6), true);
        assert_eq!(leq(&1e-6, &0.0), true);
        assert_eq!(leq(&0.0, &1e-4), true);
        assert_eq!(leq(&1e-4, &0.0), false);
    }
}
