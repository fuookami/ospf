use std::cmp::Ordering;
use std::ops::{Add, Sub};

use crate::algebra::concept::*;

use super::equal::*;
use super::{ComparisonOperator, ThreeWayComparisonOperator};

pub trait CompareOpr<T, Rhs = T>: ThreeWayComparisonOperator<T, Rhs> {
    fn precision(&self) -> Option<&T> {
        None
    }
}

impl<T, Rhs> FnOnce<(&T, &Rhs)> for &dyn CompareOpr<T, Rhs> {
    type Output = Option<Ordering>;

    extern "rust-call" fn call_once(self, (x, y): (&T, &Rhs)) -> Option<Ordering> {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnMut<(&T, &Rhs)> for &dyn CompareOpr<T, Rhs> {
    extern "rust-call" fn call_mut(&mut self, (x, y): (&T, &Rhs)) -> Option<Ordering> {
        self.cmp(x, y)
    }
}

impl<T, Rhs> Fn<(&T, &Rhs)> for &dyn CompareOpr<T, Rhs> {
    extern "rust-call" fn call(&self, (x, y): (&T, &Rhs)) -> Option<Ordering> {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnOnce<(&T, &Rhs)> for Box<dyn CompareOpr<T, Rhs>> {
    type Output = Option<Ordering>;

    extern "rust-call" fn call_once(self, (x, y): (&T, &Rhs)) -> Option<Ordering> {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnMut<(&T, &Rhs)> for Box<dyn CompareOpr<T, Rhs>> {
    extern "rust-call" fn call_mut(&mut self, (x, y): (&T, &Rhs)) -> Option<Ordering> {
        self.cmp(x, y)
    }
}

impl<T, Rhs> Fn<(&T, &Rhs)> for Box<dyn CompareOpr<T, Rhs>> {
    extern "rust-call" fn call(&self, (x, y): (&T, &Rhs)) -> Option<Ordering> {
        self.cmp(x, y)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CompareInt {}

impl CompareInt {
    fn new() -> Self {
        Self {}
    }
}

impl<T: PartialOrd<Rhs>, Rhs> ThreeWayComparisonOperator<T, Rhs> for CompareInt {
    fn cmp(&self, x: &T, y: &Rhs) -> Option<Ordering> {
        x.partial_cmp(y)
    }
}

impl<T: PartialOrd<Rhs>, Rhs> CompareOpr<T, Rhs> for CompareInt {
    default fn precision(&self) -> Option<&T> {
        None
    }
}

impl<T: SemiArithmetic + PartialOrd<Rhs>, Rhs> CompareOpr<T, Rhs> for CompareInt {
    fn precision(&self) -> Option<&T> {
        Some(T::ZERO)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CompareFlt<T> {
    pub(self) eq: EqualFlt<T>,
}

impl<T> From<T> for CompareFlt<T>
where
    EqualFlt<T>: From<T>,
{
    fn from(precision: T) -> Self {
        Self {
            eq: EqualFlt::from(precision),
        }
    }
}

impl<T> CompareFlt<T> {
    pub fn new() -> Self
    where
        T: Precision + Clone,
    {
        Self {
            eq: EqualFlt::new(),
        }
    }

    pub fn new_with(precision: T) -> Self
    where
        CompareFlt<T>: From<T>,
    {
        Self::from(precision)
    }
}

impl<T: PartialOrd<Rhs>, Rhs> ThreeWayComparisonOperator<T, Rhs> for CompareFlt<T>
where
    for<'a> &'a T: Add + Sub,
    for<'a> <&'a T as Add>::Output: PartialOrd<Rhs>,
    for<'a> <&'a T as Sub>::Output: PartialOrd<Rhs>,
{
    fn cmp(&self, x: &T, y: &Rhs) -> Option<Ordering> {
        if self.eq.cmp(x, y) {
            Some(Ordering::Equal)
        } else if x < y {
            Some(Ordering::Less)
        } else if x > y {
            Some(Ordering::Greater)
        } else {
            None
        }
    }
}

impl<T: PartialOrd<Rhs>, Rhs> CompareOpr<T, Rhs> for CompareFlt<T>
where
    for<'a> &'a T: Add + Sub,
    for<'a> <&'a T as Add>::Output: PartialOrd<Rhs>,
    for<'a> <&'a T as Sub>::Output: PartialOrd<Rhs>,
{
    fn precision(&self) -> Option<&T> {
        self.eq.precision()
    }
}

pub trait CompareOprBuilder<T, Rhs = T> {
    fn new() -> Box<dyn CompareOpr<T, Rhs>>;
    fn new_with(precision: T) -> Box<dyn CompareOpr<T, Rhs>>;
}

pub struct Compare<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: PartialOrd<Rhs>, Rhs> CompareOprBuilder<T, Rhs> for Compare<T> {
    default fn new() -> Box<dyn CompareOpr<T, Rhs>> {
        Box::new(CompareInt::new())
    }

    default fn new_with(precision: T) -> Box<dyn CompareOpr<T, Rhs>> {
        Box::new(CompareInt::new())
    }
}

impl<T: 'static + PartialOrd<Rhs>, Rhs> CompareOprBuilder<T, Rhs> for Compare<T>
where
    for<'a> &'a T: Add + Sub,
    for<'a> <&'a T as Add>::Output: PartialOrd<Rhs>,
    for<'a> <&'a T as Sub>::Output: PartialOrd<Rhs>,
{
    default fn new() -> Box<dyn CompareOpr<T, Rhs>> {
        Box::new(CompareInt::new())
    }

    default fn new_with(precision: T) -> Box<dyn CompareOpr<T, Rhs>> {
        Box::new(CompareFlt::new_with(precision))
    }
}

impl<T: Precision + Clone + PartialOrd<Rhs>, Rhs> CompareOprBuilder<T, Rhs> for Compare<T>
where
    for<'a> &'a T: Add + Sub,
    for<'a> <&'a T as Add>::Output: PartialOrd<Rhs>,
    for<'a> <&'a T as Sub>::Output: PartialOrd<Rhs>,
{
    fn new() -> Box<dyn CompareOpr<T, Rhs>> {
        Box::new(CompareFlt::new())
    }

    fn new_with(precision: T) -> Box<dyn CompareOpr<T, Rhs>> {
        Box::new(CompareFlt::new_with(precision))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmp_int() {
        let cmp = Compare::<i64>::new();
        assert_eq!(cmp(&0, &0), Some(Ordering::Equal));
        assert_eq!(cmp(&1, &2), Some(Ordering::Less));
        assert_eq!(cmp(&2, &1), Some(Ordering::Greater));
    }

    #[test]
    fn test_cmp_flt() {
        let cmp = Compare::<f64>::new();
        assert_eq!(cmp(&0.0, &0.0), Some(Ordering::Equal));
        assert_eq!(cmp(&0.0, &1e-6), Some(Ordering::Less));
        assert_eq!(cmp(&1e-6, &0.0), Some(Ordering::Greater));
        assert_eq!(cmp(&0.0, &1e-4), Some(Ordering::Less));
        assert_eq!(cmp(&1e-4, &0.0), Some(Ordering::Greater));

        let gr = Compare::<f64>::new_with(1e-5);
        assert_eq!(gr(&0.0, &0.0), Some(Ordering::Equal));
        assert_eq!(gr(&0.0, &1e-6), Some(Ordering::Equal));
        assert_eq!(gr(&1e-6, &0.0), Some(Ordering::Equal));
        assert_eq!(gr(&0.0, &1e-4), Some(Ordering::Less));
        assert_eq!(gr(&1e-4, &0.0), Some(Ordering::Greater));
    }
}
