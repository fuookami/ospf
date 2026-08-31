use std::ops::Add;

use crate::algebra::concept::*;
use crate::algebra::operator::Abs;

use super::ComparisonOperator;

pub trait GreaterEqualOpr<T, Rhs = T>: ComparisonOperator<T, Rhs> {
    fn precision(&self) -> Option<&T> {
        None
    }
}

impl<T, Rhs> FnOnce<(&T, &Rhs)> for &dyn GreaterEqualOpr<T, Rhs> {
    type Output = bool;

    extern "rust-call" fn call_once(self, (x, y): (&T, &Rhs)) -> Self::Output {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnMut<(&T, &Rhs)> for &dyn GreaterEqualOpr<T, Rhs> {
    extern "rust-call" fn call_mut(&mut self, (x, y): (&T, &Rhs)) -> Self::Output {
        self.cmp(x, y)
    }
}

impl<T, Rhs> Fn<(&T, &Rhs)> for &dyn GreaterEqualOpr<T, Rhs> {
    extern "rust-call" fn call(&self, (x, y): (&T, &Rhs)) -> Self::Output {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnOnce<(&T, &Rhs)> for Box<dyn GreaterEqualOpr<T, Rhs>> {
    type Output = bool;

    extern "rust-call" fn call_once(self, (x, y): (&T, &Rhs)) -> Self::Output {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnMut<(&T, &Rhs)> for Box<dyn GreaterEqualOpr<T, Rhs>> {
    extern "rust-call" fn call_mut(&mut self, (x, y): (&T, &Rhs)) -> Self::Output {
        self.cmp(x, y)
    }
}

impl<T, Rhs> Fn<(&T, &Rhs)> for Box<dyn GreaterEqualOpr<T, Rhs>> {
    extern "rust-call" fn call(&self, (x, y): (&T, &Rhs)) -> Self::Output {
        self.cmp(x, y)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GreaterEqualInt {}

impl GreaterEqualInt {
    pub fn new() -> Self {
        Self {}
    }
}

impl<T: PartialOrd<Rhs>, Rhs> ComparisonOperator<T, Rhs> for GreaterEqualInt {
    fn cmp(&self, x: &T, y: &Rhs) -> bool {
        x >= y
    }
}

impl<T: PartialOrd<Rhs>, Rhs> GreaterEqualOpr<T, Rhs> for GreaterEqualInt {
    default fn precision(&self) -> Option<&T> {
        None
    }
}

impl<T: SemiArithmetic + PartialOrd<Rhs>, Rhs> GreaterEqualOpr<T, Rhs> for GreaterEqualInt {
    fn precision(&self) -> Option<&T> {
        Some(T::ZERO)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GreaterEqualFlt<T> {
    pub(self) precision: T,
}

impl<T> From<T> for GreaterEqualFlt<T> {
    default fn from(precision: T) -> Self {
        Self { precision }
    }
}

impl<T: Signed> From<&T> for GreaterEqualFlt<T>
where
    for<'a> &'a T: Abs<Output = T>,
{
    fn from(precision: &T) -> Self {
        Self {
            precision: precision.abs(),
        }
    }
}

impl<T: Signed + Copy + Abs<Output = T>> From<T> for GreaterEqualFlt<T> {
    fn from(precision: T) -> Self {
        Self {
            precision: precision.abs(),
        }
    }
}

impl<T> GreaterEqualFlt<T> {
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

impl<T: PartialOrd<Rhs>, Rhs> ComparisonOperator<T, Rhs> for GreaterEqualFlt<T>
where
    for<'a> &'a T: Add,
    for<'a> <&'a T as Add>::Output: PartialOrd<Rhs>,
{
    fn cmp(&self, x: &T, y: &Rhs) -> bool {
        if x > y {
            true
        } else {
            &(x + &self.precision) >= y
        }
    }
}

impl<T: PartialOrd<Rhs>, Rhs> GreaterEqualOpr<T, Rhs> for GreaterEqualFlt<T>
where
    for<'a> &'a T: Add,
    for<'a> <&'a T as Add>::Output: PartialOrd<Rhs>,
{
    fn precision(&self) -> Option<&T> {
        Some(&self.precision)
    }
}

pub trait GreaterEqualOprBuilder<T, Rhs = T> {
    fn new() -> Box<dyn GreaterEqualOpr<T, Rhs>>;
    fn new_with(precision: T) -> Box<dyn GreaterEqualOpr<T, Rhs>>;
}

pub struct GreaterEqual {}

impl<T: PartialOrd<Rhs>, Rhs> GreaterEqualOprBuilder<T, Rhs> for GreaterEqual {
    default fn new() -> Box<dyn GreaterEqualOpr<T, Rhs>> {
        Box::new(GreaterEqualInt::new())
    }

    default fn new_with(precision: T) -> Box<dyn GreaterEqualOpr<T, Rhs>> {
        Box::new(GreaterEqualInt::new())
    }
}

impl<T: 'static + PartialOrd<Rhs>, Rhs> GreaterEqualOprBuilder<T, Rhs> for GreaterEqual
where
    for<'a> &'a T: Add,
    for<'a> <&'a T as Add>::Output: PartialOrd<Rhs>,
{
    default fn new() -> Box<dyn GreaterEqualOpr<T, Rhs>> {
        Box::new(GreaterEqualInt::new())
    }

    default fn new_with(precision: T) -> Box<dyn GreaterEqualOpr<T, Rhs>> {
        Box::new(GreaterEqualFlt::new_with(precision))
    }
}

impl<T: FloatingNumber + Clone + PartialOrd<Rhs>, Rhs> GreaterEqualOprBuilder<T, Rhs>
    for GreaterEqual
where
    for<'a> &'a T: Add,
    for<'a> <&'a T as Add>::Output: PartialOrd<Rhs>,
{
    fn new() -> Box<dyn GreaterEqualOpr<T, Rhs>> {
        Box::new(GreaterEqualFlt::new())
    }

    fn new_with(precision: T) -> Box<dyn GreaterEqualOpr<T, Rhs>> {
        Box::new(GreaterEqualFlt::new_with(precision))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geq_int() {
        let geq = GreaterEqual::new();
        assert_eq!(geq(&1, &2), false);
        assert_eq!(geq(&2, &1), true);
        assert_eq!(geq(&1, &1), true);
    }

    #[test]
    fn test_geq_flt() {
        let geq = GreaterEqual::new();
        assert_eq!(geq(&0.0, &0.0), true);
        assert_eq!(geq(&0.0, &1e-6), false);
        assert_eq!(geq(&1e-6, &0.0), true);
        assert_eq!(geq(&0.0, &1e-4), false);
        assert_eq!(geq(&1e-4, &0.0), true);

        let geq = GreaterEqual::new_with(1e-5);
        assert_eq!(geq(&0.0, &0.0), true);
        assert_eq!(geq(&0.0, &1e-6), true);
        assert_eq!(geq(&1e-6, &0.0), true);
        assert_eq!(geq(&0.0, &1e-4), false);
        assert_eq!(geq(&1e-4, &0.0), true);
    }
}
