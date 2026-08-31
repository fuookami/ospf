use std::cmp::Ordering;
use std::ops::{Add, Div, Mul, Neg, Sub};
use std::time::{Duration, Instant};

use chrono::NaiveDateTime;

use crate::SemiArithmetic;

use super::interval::{Closed, Interval, IntervalType};
use super::value_wrapper::ValueWrapper;
use super::IllegalArgumentError;

#[derive(Clone, Copy)]
pub struct Bound<T> {
    pub value: ValueWrapper<T>,
    pub interval: Interval,
}

impl<T, U> From<&Bound<U>> for Bound<T>
where
    ValueWrapper<T>: for<'a> From<&'a ValueWrapper<U>>,
{
    fn from(bound: &Bound<U>) -> Self {
        Self {
            value: ValueWrapper::from(&bound.value),
            interval: bound.interval,
        }
    }
}

impl<T, U> PartialEq<U> for Bound<T>
where
    ValueWrapper<T>: PartialEq<U>,
{
    fn eq(&self, other: &U) -> bool {
        self.value == *other && self.interval == Interval::Closed
    }
}

impl<T, U> PartialEq<Bound<U>> for Bound<T>
where
    ValueWrapper<T>: PartialEq<ValueWrapper<U>>,
{
    fn eq(&self, other: &Bound<U>) -> bool {
        self.value == other.value && self.interval == other.interval
    }
}

macro_rules! bound_template {
    ($type:ident, $rhs:ident) => {
        impl Add<$rhs> for Bound<$type> {
            type Output = Result<Bound<<$type as Add<$rhs>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: $rhs) -> Self::Output {
                let value = (self.value + rhs)?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }

        impl<'a> Add<&'a $rhs> for Bound<$type> {
            type Output = Result<Bound<<$type as Add<$rhs>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'a $rhs) -> Self::Output {
                let value = self.value.add(rhs.clone())?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }

        impl<'a> Add<$rhs> for &'a Bound<$type> {
            type Output = Result<Bound<<$type as Add<$rhs>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: $rhs) -> Self::Output {
                let value = self.value.add(rhs)?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }

        impl<'a, 'b> Add<&'a $rhs> for &'b Bound<$type> {
            type Output = Result<Bound<<$type as Add<$rhs>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'a $rhs) -> Self::Output {
                let value = self.value.add(rhs.clone())?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }

        impl Sub<$rhs> for Bound<$type> {
            type Output = Result<Bound<<$type as Sub<$rhs>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: $rhs) -> Self::Output {
                let value = self.value.sub(rhs)?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }

        impl<'a> Sub<&'a $rhs> for Bound<$type> {
            type Output = Result<Bound<<$type as Sub<$rhs>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'a $rhs) -> Self::Output {
                let value = self.value.sub(rhs.clone())?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }

        impl<'a> Sub<$rhs> for &'a Bound<$type> {
            type Output = Result<Bound<<$type as Sub<$rhs>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: $rhs) -> Self::Output {
                let value = self.value.sub(rhs)?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }

        impl<'a, 'b> Sub<&'a $rhs> for &'b Bound<$type> {
            type Output = Result<Bound<<$type as Sub<$rhs>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'a $rhs) -> Self::Output {
                let value = self.value.sub(rhs.clone())?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }
    };
}
bound_template!(Instant, Duration);
bound_template!(NaiveDateTime, Duration);

macro_rules! signed_bound_template {
    ($($type:ident)*) => ($(
        impl Neg for Bound<$type> {
            type Output = Result<Bound<<$type as Neg>::Output>, IllegalArgumentError>;

            fn neg(self) -> Self::Output {
                let value = self.value.neg()?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }

        impl<'a> Neg for &'a Bound<$type> {
            type Output = Result<Bound<<$type as Neg>::Output>, IllegalArgumentError>;

            fn neg(self) -> Self::Output {
                let value = self.value.neg()?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }
    )*)
}
signed_bound_template! { i8 i16 i32 i64 i128 isize f32 f64 }

macro_rules! real_number_bound_template {
    ($($type:ident)*) => ($(
        impl Add<$type> for Bound<$type> {
            type Output = Result<Bound<<$type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: $type) -> Self::Output {
                let value = self.value.add(rhs)?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }

        impl<'a> Add<&'a $type> for Bound<$type> {
            type Output = Result<Bound<<$type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'a $type) -> Self::Output {
                let value = self.value.add(rhs.clone())?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }

        impl<'a> Add<$type> for &'a Bound<$type> {
            type Output = Result<Bound<<$type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: $type) -> Self::Output {
                let value = self.value.add(rhs)?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }

        impl<'a, 'b> Add<&'a $type> for &'b Bound<$type> {
            type Output = Result<Bound<<$type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'a $type) -> Self::Output {
                let value = self.value.add(rhs.clone())?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }

        impl Add<Bound<$type>> for Bound<$type> {
            type Output = Result<Bound<<$type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: Bound<$type>) -> Self::Output {
                let value = self.value.add(rhs.value)?;
                Ok(Bound {
                    value,
                    interval: self.interval.intersect(&rhs.interval),
                })
            }
        }

        impl<'a> Add<&'a Bound<$type>> for Bound<$type> {
            type Output = Result<Bound<<$type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'a Bound<$type>) -> Self::Output {
                let value = self.value.add(rhs.value.clone())?;
                Ok(Bound {
                    value,
                    interval: self.interval.intersect(&rhs.interval),
                })
            }
        }

        impl<'a> Add<Bound<$type>> for &'a Bound<$type> {
            type Output = Result<Bound<<$type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: Bound<$type>) -> Self::Output {
                let value = self.value.add(rhs.value)?;
                Ok(Bound {
                    value,
                    interval: self.interval.intersect(&rhs.interval),
                })
            }
        }

        impl<'a, 'b> Add<&'a Bound<$type>> for &'b Bound<$type> {
            type Output = Result<Bound<<$type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'a Bound<$type>) -> Self::Output {
                let value = self.value.add(rhs.value.clone())?;
                Ok(Bound {
                    value,
                    interval: self.interval.intersect(&rhs.interval),
                })
            }
        }

        impl Sub<$type> for Bound<$type> {
            type Output = Result<Bound<<$type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: $type) -> Self::Output {
                let value = self.value.sub(rhs)?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }

        impl<'a> Sub<&'a $type> for Bound<$type> {
            type Output = Result<Bound<<$type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'a $type) -> Self::Output {
                let value = self.value.sub(rhs.clone())?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }

        impl<'a> Sub<$type> for &'a Bound<$type> {
            type Output = Result<Bound<<$type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: $type) -> Self::Output {
                let value = self.value.sub(rhs)?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }

        impl<'a, 'b> Sub<&'a $type> for &'b Bound<$type> {
            type Output = Result<Bound<<$type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'a $type) -> Self::Output {
                let value = self.value.sub(rhs.clone())?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }

        impl Sub<Bound<$type>> for Bound<$type> {
            type Output = Result<Bound<<$type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: Bound<$type>) -> Self::Output {
                let value = self.value.sub(rhs.value)?;
                Ok(Bound {
                    value,
                    interval: self.interval.intersect(&rhs.interval),
                })
            }
        }

        impl<'a> Sub<&'a Bound<$type>> for Bound<$type> {
            type Output = Result<Bound<<$type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'a Bound<$type>) -> Self::Output {
                let value = self.value.sub(rhs.value.clone())?;
                Ok(Bound {
                    value,
                    interval: self.interval.intersect(&rhs.interval),
                })
            }
        }

        impl<'a> Sub<Bound<$type>> for &'a Bound<$type> {
            type Output = Result<Bound<<$type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: Bound<$type>) -> Self::Output {
                let value = self.value.sub(rhs.value)?;
                Ok(Bound {
                    value,
                    interval: self.interval.intersect(&rhs.interval),
                })
            }
        }

        impl<'a, 'b> Sub<&'a Bound<$type>> for &'b Bound<$type> {
            type Output = Result<Bound<<$type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'a Bound<$type>) -> Self::Output {
                let value = self.value.sub(rhs.value.clone())?;
                Ok(Bound {
                    value,
                    interval: self.interval.intersect(&rhs.interval),
                })
            }
        }

        impl Mul<$type> for Bound<$type> {
            type Output = Result<Bound<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: $type) -> Self::Output {
                let value = self.value.mul(rhs)?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }

        impl<'a> Mul<&'a $type> for Bound<$type> {
            type Output = Result<Bound<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: &'a $type) -> Self::Output {
                let value = self.value.mul(rhs.clone())?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }

        impl<'a> Mul<$type> for &'a Bound<$type> {
            type Output = Result<Bound<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: $type) -> Self::Output {
                let value = self.value.mul(rhs)?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }

        impl<'a, 'b> Mul<&'a $type> for &'b Bound<$type> {
            type Output = Result<Bound<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: &'a $type) -> Self::Output {
                let value = self.value.mul(rhs.clone())?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }

        impl Mul<Bound<$type>> for Bound<$type> {
            type Output = Result<Bound<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: Bound<$type>) -> Self::Output {
                let value = self.value.mul(rhs.value)?;
                Ok(Bound {
                    value,
                    interval: self.interval.intersect(&rhs.interval),
                })
            }
        }

        impl<'a> Mul<&'a Bound<$type>> for Bound<$type> {
            type Output = Result<Bound<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: &'a Bound<$type>) -> Self::Output {
                let value = self.value.mul(rhs.value.clone())?;
                Ok(Bound {
                    value,
                    interval: self.interval.intersect(&rhs.interval),
                })
            }
        }

        impl<'a> Mul<Bound<$type>> for &'a Bound<$type> {
            type Output = Result<Bound<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: Bound<$type>) -> Self::Output {
                let value = self.value.mul(rhs.value)?;
                Ok(Bound {
                    value,
                    interval: self.interval.intersect(&rhs.interval),
                })
            }
        }

        impl<'a, 'b> Mul<&'a Bound<$type>> for &'b Bound<$type> {
            type Output = Result<Bound<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: &'a Bound<$type>) -> Self::Output {
                let value = self.value.mul(rhs.value.clone())?;
                Ok(Bound {
                    value,
                    interval: self.interval.intersect(&rhs.interval),
                })
            }
        }

        impl Div<$type> for Bound<$type> {
            type Output = Result<Bound<<$type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: $type) -> Self::Output {
                let value = self.value.div(rhs)?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }

        impl<'a> Div<&'a $type> for Bound<$type> {
            type Output = Result<Bound<<$type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: &'a $type) -> Self::Output {
                let value = self.value.div(rhs.clone())?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }

        impl<'a> Div<$type> for &'a Bound<$type> {
            type Output = Result<Bound<<$type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: $type) -> Self::Output {
                let value = self.value.div(rhs)?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }

        impl<'a, 'b> Div<&'a $type> for &'b Bound<$type> {
            type Output = Result<Bound<<$type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: &'a $type) -> Self::Output {
                let value = self.value.div(rhs.clone())?;
                Ok(Bound {
                    value,
                    interval: self.interval,
                })
            }
        }

        impl Div<Bound<$type>> for Bound<$type> {
            type Output = Result<Bound<<$type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: Bound<$type>) -> Self::Output {
                let value = (self.value / rhs.value)?;
                Ok(Bound {
                    value,
                    interval: self.interval.intersect(&rhs.interval),
                })
            }
        }

        impl<'a> Div<&'a Bound<$type>> for Bound<$type> {
            type Output = Result<Bound<<$type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: &'a Bound<$type>) -> Self::Output {
                let value = (self.value / rhs.value)?;
                Ok(Bound {
                    value,
                    interval: self.interval.intersect(&rhs.interval),
                })
            }
        }

        impl<'a> Div<Bound<$type>> for &'a Bound<$type> {
            type Output = Result<Bound<<$type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: Bound<$type>) -> Self::Output {
                let value = (self.value / rhs.value)?;
                Ok(Bound {
                    value,
                    interval: self.interval.intersect(&rhs.interval),
                })
            }
        }

        impl<'a, 'b> Div<&'a Bound<$type>> for &'b Bound<$type> {
            type Output = Result<Bound<<$type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: &'a Bound<$type>) -> Self::Output {
                let value = (self.value / rhs.value)?;
                Ok(Bound {
                    value,
                    interval: self.interval.intersect(&rhs.interval),
                })
            }
        }
    )*)
}
real_number_bound_template! { u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize f32 f64 }
