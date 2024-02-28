use super::*;
use crate::algebra::concept::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BoundSide {
    Lower,
    Upper
}

pub struct BoundStc<T: Arithmetic, I: IntervalStc = Closed> {
    value: ValueWrapper<T>,
    side: BoundSide,
    _marker: std::marker::PhantomData<I>,
}

impl<T: Arithmetic, I: IntervalStc> Clone for BoundStc<T, I>
where
    ValueWrapper<T>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            side: self.side,
            _marker: std::marker::PhantomData::<I> {}
        }
    }
}

impl<T: Arithmetic, I: IntervalStc> Copy for BoundStc<T, I> where ValueWrapper<T>: Copy {}

impl<T: Arithmetic, I: IntervalStc> std::fmt::Display for BoundStc<T, I> where ValueWrapper<T>: std::fmt::Display {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.side {
            Lower => write!(f, "{}{}", I::to_lb_sign(), self.value),
            Upper => write!(f, "{}{}", self.value, I::to_ub_sign())
        }
    }
}

impl<T: Arithmetic, I: IntervalStc> std::fmt::Debug for BoundStc<T, I> where ValueWrapper<T>: std::fmt::Display {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.side {
            Lower => write!(f, "{}{}", I::to_lb_sign(), self.value),
            Upper => write!(f, "{}{}", self.value, I::to_ub_sign())
        }
    }
}

pub struct Bound<T: Arithmetic> {
    pub value: ValueWrapper<T>,
    pub interval: Interval,
    pub side: BoundSide
}

impl<T: Arithmetic> Clone for Bound<T>
where
    ValueWrapper<T>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            interval: self.interval,
            side: self.side
        }
    }
}

impl<T: Arithmetic> Copy for Bound<T> where ValueWrapper<T>: Copy {}

impl<T: Arithmetic> std::fmt::Display for Bound<T> where ValueWrapper<T>: std::fmt::Display {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.side {
            Lower => write!(f, "{}{}", self.interval.to_lb_sign(), self.value),
            Upper => write!(f, "{}{}", self.value, self.interval.to_ub_sign())
        }
    }
}

impl<T: Arithmetic> std::fmt::Debug for Bound<T> where ValueWrapper<T>: std::fmt::Display {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.side {
            Lower => write!(f, "{}{}", self.interval.to_lb_sign(), self.value),
            Upper => write!(f, "{}{}", self.value, self.interval.to_ub_sign())
        }
    }
}
