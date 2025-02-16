use super::*;
use crate::algebra::concept::*;
use crate::algebra::operator::*;
use std::ops::{Add, Div, Mul, Sub};

pub(self) fn empty<T: Arithmetic>(lb: &T, ub: &T, lb_interval: Interval, ub_interval: Interval) -> bool {
    !(lb_interval.lb_op())(lb, ub) || !(ub_interval.ub_op())(lb, ub)
}

pub struct ValueRange<T: Arithmetic> {
    pub lb: Option<Bound<T>>,
    pub ub: Option<Bound<T>>,
}

impl<T: Arithmetic> ValueRange<T> {
    pub fn new() -> Self {
        Self {
            lb: Option::Some(Bound {
                value: ValueWrapper::NegInf,
                interval: Interval::Closed,
                side: BoundSide::Lower,
            }),
            ub: Option::Some(Bound {
                value: ValueWrapper::Inf,
                interval: Interval::Closed,
                side: BoundSide::Upper,
            }),
        }
    }

    pub fn new_with(lb: T, ub: T, lb_interval: Interval, ub_interval: Interval) -> Self {
        assert!(!empty(&lb, &ub, lb_interval, ub_interval));
        Self {
            lb: Option::Some(Bound {
                value: ValueWrapper::Value(lb),
                interval: lb_interval,
                side: BoundSide::Lower,
            }),
            ub: Option::Some(Bound {
                value: ValueWrapper::Value(ub),
                interval: ub_interval,
                side: BoundSide::Upper,
            }),
        }
    }

    fn empty(&self) -> bool {
        !self.lb.is_none() && !self.ub.is_none()
    }

    fn fixed(&self) -> bool
    where
        T: Precision,
    {
        if let (Some(lower_bound), Some(upper_bound)) = (&self.lb, &self.ub) {
            lower_bound.interval == Interval::Closed
                && upper_bound.interval == Interval::Closed
                && if let (ValueWrapper::Value(lower_value), ValueWrapper::Value(upper_value)) =
                    (&lower_bound.value, &upper_bound.value)
                {
                    let eq_op = Equal::new();
                    eq_op(lower_value, upper_value)
                } else {
                    false
                }
        } else {
            false
        }
    }
}

impl<T: Arithmetic> std::fmt::Display for ValueRange<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        
    }
}

impl<'a, T: Arithmetic> Add<&'a T> for &'a ValueRange<T>
where
    &'a T: Add<&'a T, Output = T>,
{
    type Output = Result<ValueRange<T>, IllegalArgumentError>;

    fn add(self, rhs: &'a T) -> Self::Output {
        if let (Some(lb), Some(ub)) = (&self.lb, &self.ub) {
            if let (Ok(lb_value), Ok(ub_value)) = (&lb.value + rhs, &ub.value + rhs) {
                return Ok(ValueRange {
                    lb: Some(Bound {
                        value: lb_value,
                        interval: lb.interval,
                        side: BoundSide::Lower,
                    }),
                    ub: Some(Bound {
                        value: ub_value,
                        interval: ub.interval,
                        side: BoundSide::Lower,
                    }),
                });
            }
        }
        Err(IllegalArgumentError {
            msg: format!("Invalid add between {} and {}", self, rhs),
        })
    }
}

impl<'a, T: RealNumber> Add<&'a T> for &'a ValueRange<T>
where
    &'a T: Add<&'a T, Output = T>,
{
    type Output = Result<ValueRange<T>, IllegalArgumentError>;

    fn add(self, rhs: &'a T) -> Self::Output {}
}

impl<'a, T: Arithmetic> Add<&'a ValueRange<T>> for &'a ValueRange<T>
where
    &'a T: Add<&'a T, Output = T>,
{
    type Output = Result<ValueRange<T>, IllegalArgumentError>;

    fn add(self, rhs: &'a T) -> Self::Output {}
}

impl<'a, T: Arithmetic> Sub<&'a T> for &'a ValueRange<T>
where
    &'a T: Sub<&'a T, Output = T>,
{
    type Output = Result<ValueRange<T>, IllegalArgumentError>;

    fn sub(self, rhs: &'a T) -> Self::Output {}
}

impl<'a, T: RealNumber> Sub<&'a T> for &'a ValueRange<T>
where
    &'a T: Sub<&'a T, Output = T>,
{
    type Output = Result<ValueRange<T>, IllegalArgumentError>;

    fn sub(self, rhs: &'a T) -> Self::Output {}
}

impl<'a, T: Arithmetic> Sub<&'a ValueRange<T>> for &'a ValueRange<T>
where
    &'a T: Sub<&'a T, Output = T>,
{
    type Output = Result<ValueRange<T>, IllegalArgumentError>;

    fn sub(self, rhs: &'a T) -> Self::Output {}
}
