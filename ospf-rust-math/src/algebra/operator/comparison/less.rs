use std::ops::{Add, Sub};

use crate::algebra::concept::*;
use crate::algebra::operator::Abs;

use super::ComparisonOperator;

pub trait LessOpr<T, Rhs = T>: ComparisonOperator<T, Rhs> {
    fn precision(&self) -> Option<&T> {
        None
    }
}

impl<T, Rhs> FnOnce<(&T, &Rhs)> for &dyn LessOpr<T, Rhs> {
    type Output = bool;

    extern "rust-call" fn call_once(self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnMut<(&T, &Rhs)> for &dyn LessOpr<T, Rhs> {
    extern "rust-call" fn call_mut(&mut self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> Fn<(&T, &Rhs)> for &dyn LessOpr<T, Rhs> {
    extern "rust-call" fn call(&self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnOnce<(&T, &Rhs)> for Box<dyn LessOpr<T, Rhs>> {
    type Output = bool;

    extern "rust-call" fn call_once(self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnMut<(&T, &Rhs)> for Box<dyn LessOpr<T, Rhs>> {
    extern "rust-call" fn call_mut(&mut self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> Fn<(&T, &Rhs)> for Box<dyn LessOpr<T, Rhs>> {
    extern "rust-call" fn call(&self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LessInt {}

impl LessInt {
    pub fn new() -> Self {
        Self {}
    }
}

impl<T: PartialOrd<Rhs>, Rhs> ComparisonOperator<T, Rhs> for LessInt {
    fn cmp(&self, x: &T, y: &Rhs) -> bool {
        x < y
    }
}

impl<T: PartialOrd<Rhs>, Rhs> LessOpr<T, Rhs> for LessInt {
    default fn precision(&self) -> Option<&T> {
        None
    }
}

impl<T: SemiArithmetic + PartialOrd<Rhs>, Rhs> LessOpr<T, Rhs> for LessInt {
    fn precision(&self) -> Option<&T> {
        Some(T::ZERO)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LessFlt<T> {
    pub(self) precision: T,
}

impl<T> From<T> for LessFlt<T> {
    default fn from(precision: T) -> Self {
        Self { precision }
    }
}

impl<T: Signed> From<&T> for LessFlt<T>
where
    for<'a> &'a T: Abs<Output = T>,
{
    fn from(precision: &T) -> Self {
        Self {
            precision: precision.abs(),
        }
    }
}

impl<T: Signed + Copy + Abs<Output = T>> From<T> for LessFlt<T> {
    fn from(precision: T) -> Self {
        Self {
            precision: precision.abs(),
        }
    }
}

impl<T> LessFlt<T> {
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

impl<T: PartialOrd<Rhs>, Rhs> ComparisonOperator<T, Rhs> for LessFlt<T>
where
    for<'a> &'a T: Add,
    for<'a> <&'a T as Add>::Output: PartialOrd<Rhs>,
{
    fn cmp(&self, x: &T, y: &Rhs) -> bool {
        if x > y {
            false
        } else {
            &(x + &self.precision) < y
        }
    }
}

impl<T: PartialOrd<Rhs>, Rhs> LessOpr<T, Rhs> for LessFlt<T>
where
    for<'a> &'a T: Add,
    for<'a> <&'a T as Add>::Output: PartialOrd<Rhs>,
{
    fn precision(&self) -> Option<&T> {
        Some(&self.precision)
    }
}

pub trait LessOprBuilder<T, Rhs = T> {
    fn new() -> Box<dyn LessOpr<T, Rhs>>;
    fn new_with(precision: T) -> Box<dyn LessOpr<T, Rhs>>;
}

pub struct Less<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: PartialOrd<Rhs>, Rhs> LessOprBuilder<T, Rhs> for Less<T> {
    default fn new() -> Box<dyn LessOpr<T, Rhs>> {
        Box::new(LessInt::new())
    }

    default fn new_with(precision: T) -> Box<dyn LessOpr<T, Rhs>> {
        Box::new(LessInt::new())
    }
}

impl<T: 'static + PartialOrd<Rhs>, Rhs> LessOprBuilder<T, Rhs> for Less<T>
where
    for<'a> &'a T: Add,
    for<'a> <&'a T as Add>::Output: PartialOrd<Rhs>,
{
    default fn new() -> Box<dyn LessOpr<T, Rhs>> {
        Box::new(LessInt::new())
    }

    default fn new_with(precision: T) -> Box<dyn LessOpr<T, Rhs>> {
        Box::new(LessFlt::new_with(precision))
    }
}

impl<T: FloatingNumber + Clone + PartialOrd<Rhs>, Rhs> LessOprBuilder<T, Rhs> for Less<T>
where
    for<'a> &'a T: Add,
    for<'a> <&'a T as Add>::Output: PartialOrd<Rhs>,
{
    fn new() -> Box<dyn LessOpr<T, Rhs>> {
        Box::new(LessFlt::new())
    }

    fn new_with(precision: T) -> Box<dyn LessOpr<T, Rhs>> {
        Box::new(LessFlt::new_with(precision))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ls_int() {
        let ls = Less::new();
        assert_eq!(ls(&1, &2), true);
        assert_eq!(ls(&2, &1), false);
        assert_eq!(ls(&1, &1), false);
    }

    #[test]
    fn test_ls_flt() {
        let ls = Less::new();
        assert_eq!(ls(&0.0, &0.0), false);
        assert_eq!(ls(&0.0, &1e-6), true);
        assert_eq!(ls(&1e-6, &0.0), false);
        assert_eq!(ls(&0.0, &1e-4), true);
        assert_eq!(ls(&1e-4, &0.0), false);

        let ls = Less::new_with(1e-5);
        assert_eq!(ls(&0.0, &0.0), false);
        assert_eq!(ls(&0.0, &1e-6), false);
        assert_eq!(ls(&1e-6, &0.0), false);
        assert_eq!(ls(&0.0, &1e-4), true);
        assert_eq!(ls(&1e-4, &0.0), false);
    }
}
