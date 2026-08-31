use std::ops::{Add, Div, Mul, Neg, Sub};
use std::time::{Duration, Instant};

use chrono::NaiveDateTime;

use super::error::IllegalArgumentError;
use super::interval::Interval;
use super::value_wrapper::ValueWrapper;

#[non_exhaustive]
#[derive(Clone, Copy)]
pub struct Bound<T> {
    pub value: ValueWrapper<T>,
    pub interval: Interval,
}

impl<T> Bound<T> {
    pub fn new(value: ValueWrapper<T>, interval: Interval) -> Self {
        match value {
            ValueWrapper::Value(_) => Self { value, interval },
            _ => Self {
                value,
                interval: Interval::Open,
            },
        }
    }
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
        self.value.eq(other) && self.interval == Interval::Closed
    }
}

impl<T, U> PartialEq<Bound<U>> for Bound<T>
where
    ValueWrapper<T>: PartialEq<ValueWrapper<U>>,
{
    fn eq(&self, other: &Bound<U>) -> bool {
        self.value.eq(&other.value) && self.interval == other.interval
    }
}

macro_rules! bound_template {
    ($type:ident, $rhs:ident) => {
        impl Add<$rhs> for Bound<$type> {
            type Output = Result<Bound<<$type as Add<$rhs>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: $rhs) -> Self::Output {
                let value = (self.value + rhs)?;
                Ok(Bound::new(value, self.interval))
            }
        }

        impl<'a> Add<&'a $rhs> for Bound<$type> {
            type Output = Result<Bound<<$type as Add<$rhs>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'a $rhs) -> Self::Output {
                let value = self.value.add(rhs)?;
                Ok(Bound::new(value, self.interval))
            }
        }

        impl<'a> Add<$rhs> for &'a Bound<$type> {
            type Output = Result<Bound<<$type as Add<$rhs>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: $rhs) -> Self::Output {
                let value = self.value.add(rhs)?;
                Ok(Bound::new(value, self.interval))
            }
        }

        impl<'a, 'b> Add<&'a $rhs> for &'b Bound<$type> {
            type Output = Result<Bound<<$type as Add<$rhs>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'a $rhs) -> Self::Output {
                let value = self.value.add(rhs)?;
                Ok(Bound::new(value, self.interval))
            }
        }

        impl Sub<$rhs> for Bound<$type> {
            type Output = Result<Bound<<$type as Sub<$rhs>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: $rhs) -> Self::Output {
                let value = self.value.sub(rhs)?;
                Ok(Bound::new(value, self.interval))
            }
        }

        impl<'a> Sub<&'a $rhs> for Bound<$type> {
            type Output = Result<Bound<<$type as Sub<$rhs>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'a $rhs) -> Self::Output {
                let value = self.value.sub(rhs)?;
                Ok(Bound::new(value, self.interval))
            }
        }

        impl<'a> Sub<$rhs> for &'a Bound<$type> {
            type Output = Result<Bound<<$type as Sub<$rhs>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: $rhs) -> Self::Output {
                let value = self.value.sub(rhs)?;
                Ok(Bound::new(value, self.interval))
            }
        }

        impl<'a, 'b> Sub<&'a $rhs> for &'b Bound<$type> {
            type Output = Result<Bound<<$type as Sub<$rhs>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'a $rhs) -> Self::Output {
                let value = self.value.sub(rhs)?;
                Ok(Bound::new(value, self.interval))
            }
        }
    };
}
bound_template!(Instant, Duration);
bound_template!(NaiveDateTime, Duration);
bound_template!(Duration, Duration);

macro_rules! signed_bound_template {
    ($($type:ident)*) => ($(
        impl Neg for Bound<$type> {
            type Output = Result<Bound<<$type as Neg>::Output>, IllegalArgumentError>;

            fn neg(self) -> Self::Output {
                let value = self.value.neg()?;
                Ok(Bound::new(value, self.interval))
            }
        }

        impl<'a> Neg for &'a Bound<$type> {
            type Output = Result<Bound<<$type as Neg>::Output>, IllegalArgumentError>;

            fn neg(self) -> Self::Output {
                let value = self.value.neg()?;
                Ok(Bound::new(value, self.interval))
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
                Ok(Bound::new(value, self.interval))
            }
        }

        impl<'a> Add<&'a $type> for Bound<$type> {
            type Output = Result<Bound<<$type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'a $type) -> Self::Output {
                let value = self.value.add(rhs)?;
                Ok(Bound::new(value, self.interval))
            }
        }

        impl<'a> Add<$type> for &'a Bound<$type> {
            type Output = Result<Bound<<$type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: $type) -> Self::Output {
                let value = self.value.add(rhs)?;
                Ok(Bound::new(value, self.interval))
            }
        }

        impl<'a, 'b> Add<&'a $type> for &'b Bound<$type> {
            type Output = Result<Bound<<$type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'a $type) -> Self::Output {
                let value = self.value.add(rhs)?;
                Ok(Bound::new(value, self.interval))
            }
        }

        impl Add<Bound<$type>> for $type {
            type Output = Result<Bound<<$type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: Bound<$type>) -> Self::Output {
                let value = self.add(rhs.value)?;
                Ok(Bound::new(value, rhs.interval))
            }
        }

        impl<'a> Add<Bound<$type>> for &'a $type {
            type Output = Result<Bound<<&'a $type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: Bound<$type>) -> Self::Output {
                let value = self.add(rhs.value)?;
                Ok(Bound::new(value, rhs.interval))
            }
        }

        impl<'a> Add<&'a Bound<$type>> for $type {
            type Output = Result<Bound<<$type as Add<&'a $type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'a Bound<$type>) -> Self::Output {
                let value = self.add(rhs.value)?;
                Ok(Bound::new(value, rhs.interval))
            }
        }

        impl<'a, 'b> Add<&'b Bound<$type>> for &'a $type {
            type Output = Result<Bound<<&'a $type as Add<&'b $type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'b Bound<$type>) -> Self::Output {
                let value = self.add(rhs.value)?;
                Ok(Bound::new(value, rhs.interval))
            }
        }

        impl Add<Bound<$type>> for Bound<$type> {
            type Output = Result<Bound<<$type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: Bound<$type>) -> Self::Output {
                let value = self.value.add(rhs.value)?;
                Ok(Bound::new(value, self.interval.intersect(&rhs.interval)))
            }
        }

        impl<'a> Add<&'a Bound<$type>> for Bound<$type> {
            type Output = Result<Bound<<$type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'a Bound<$type>) -> Self::Output {
                let value = self.value.add(rhs.value)?;
                Ok(Bound::new(value, self.interval.intersect(&rhs.interval)))
            }
        }

        impl<'a> Add<Bound<$type>> for &'a Bound<$type> {
            type Output = Result<Bound<<$type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: Bound<$type>) -> Self::Output {
                let value = self.value.add(rhs.value)?;
                Ok(Bound::new(value, self.interval.intersect(&rhs.interval)))
            }
        }

        impl<'a, 'b> Add<&'a Bound<$type>> for &'b Bound<$type> {
            type Output = Result<Bound<<$type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'a Bound<$type>) -> Self::Output {
                let value = self.value.add(rhs.value)?;
                Ok(Bound::new(value, self.interval.intersect(&rhs.interval)))
            }
        }

        impl Sub<$type> for Bound<$type> {
            type Output = Result<Bound<<$type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: $type) -> Self::Output {
                let value = self.value.sub(rhs)?;
                Ok(Bound::new(value, self.interval))
            }
        }

        impl<'a> Sub<&'a $type> for Bound<$type> {
            type Output = Result<Bound<<$type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'a $type) -> Self::Output {
                let value = self.value.sub(rhs)?;
                Ok(Bound::new(value, self.interval))
            }
        }

        impl<'a> Sub<$type> for &'a Bound<$type> {
            type Output = Result<Bound<<$type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: $type) -> Self::Output {
                let value = self.value.sub(rhs)?;
                Ok(Bound::new(value, self.interval))
            }
        }

        impl<'a, 'b> Sub<&'a $type> for &'b Bound<$type> {
            type Output = Result<Bound<<$type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'a $type) -> Self::Output {
                let value = self.value.sub(rhs)?;
                Ok(Bound::new(value, self.interval))
            }
        }

        impl Sub<Bound<$type>> for $type {
            type Output = Result<Bound<<$type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: Bound<$type>) -> Self::Output {
                let value = self.sub(rhs.value)?;
                Ok(Bound::new(value, rhs.interval))
            }
        }

        impl<'a> Sub<Bound<$type>> for &'a $type {
            type Output = Result<Bound<<$type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: Bound<$type>) -> Self::Output {
                let value = self.sub(rhs.value)?;
                Ok(Bound::new(value, rhs.interval))
            }
        }

        impl<'a> Sub<&'a Bound<$type>> for $type {
            type Output = Result<Bound<<$type as Sub<&'a $type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'a Bound<$type>) -> Self::Output {
                let value = self.sub(rhs.value)?;
                Ok(Bound::new(value, rhs.interval))
            }
        }

        impl<'a, 'b> Sub<&'b Bound<$type>> for &'a $type {
            type Output = Result<Bound<<$type as Sub<&'b $type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'b Bound<$type>) -> Self::Output {
                let value = self.sub(rhs.value)?;
                Ok(Bound::new(value, rhs.interval))
            }
        }

        impl Sub<Bound<$type>> for Bound<$type> {
            type Output = Result<Bound<<$type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: Bound<$type>) -> Self::Output {
                let value = self.value.sub(rhs.value)?;
                Ok(Bound::new(value, self.interval.intersect(&rhs.interval)))
            }
        }

        impl<'a> Sub<&'a Bound<$type>> for Bound<$type> {
            type Output = Result<Bound<<$type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'a Bound<$type>) -> Self::Output {
                let value = self.value.sub(rhs.value)?;
                Ok(Bound::new(value, self.interval.intersect(&rhs.interval)))
            }
        }

        impl<'a> Sub<Bound<$type>> for &'a Bound<$type> {
            type Output = Result<Bound<<$type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: Bound<$type>) -> Self::Output {
                let value = self.value.sub(rhs.value)?;
                Ok(Bound::new(value, self.interval.intersect(&rhs.interval)))
            }
        }

        impl<'a, 'b> Sub<&'a Bound<$type>> for &'b Bound<$type> {
            type Output = Result<Bound<<$type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'a Bound<$type>) -> Self::Output {
                let value = self.value.sub(rhs.value)?;
                Ok(Bound::new(value, self.interval.intersect(&rhs.interval)))
            }
        }

        impl Mul<$type> for Bound<$type> {
            type Output = Result<Bound<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: $type) -> Self::Output {
                let value = self.value.mul(rhs)?;
                Ok(Bound::new(value, self.interval))
            }
        }

        impl<'a> Mul<&'a $type> for Bound<$type> {
            type Output = Result<Bound<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: &'a $type) -> Self::Output {
                let value = self.value.mul(rhs)?;
                Ok(Bound::new(value, self.interval))
            }
        }

        impl<'a> Mul<$type> for &'a Bound<$type> {
            type Output = Result<Bound<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: $type) -> Self::Output {
                let value = self.value.mul(rhs)?;
                Ok(Bound::new(value, self.interval))
            }
        }

        impl<'a, 'b> Mul<&'a $type> for &'b Bound<$type> {
            type Output = Result<Bound<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: &'a $type) -> Self::Output {
                let value = self.value.mul(rhs)?;
                Ok(Bound::new(value, self.interval))
            }
        }

        impl Mul<Bound<$type>> for $type {
            type Output = Result<Bound<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: Bound<$type>) -> Self::Output {
                let value = self.mul(rhs.value)?;
                Ok(Bound::new(value, rhs.interval))
            }
        }

        impl<'a> Mul<Bound<$type>> for &'a $type {
            type Output = Result<Bound<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: Bound<$type>) -> Self::Output {
                let value = self.mul(rhs.value)?;
                Ok(Bound::new(value, rhs.interval))
            }
        }

        impl<'a> Mul<&'a Bound<$type>> for $type {
            type Output = Result<Bound<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: &'a Bound<$type>) -> Self::Output {
                let value = self.mul(rhs.value)?;
                Ok(Bound::new(value, rhs.interval))
            }
        }

        impl<'a, 'b> Mul<&'b Bound<$type>> for &'a $type {
            type Output = Result<Bound<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: &'b Bound<$type>) -> Self::Output {
                let value = self.mul(rhs.value)?;
                Ok(Bound::new(value, rhs.interval))
            }
        }

        impl Mul<Bound<$type>> for Bound<$type> {
            type Output = Result<Bound<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: Bound<$type>) -> Self::Output {
                let value = self.value.mul(rhs.value)?;
                Ok(Bound::new(value, self.interval.intersect(&rhs.interval)))
            }
        }

        impl<'a> Mul<&'a Bound<$type>> for Bound<$type> {
            type Output = Result<Bound<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: &'a Bound<$type>) -> Self::Output {
                let value = self.value.mul(rhs.value)?;
                Ok(Bound::new(value, self.interval.intersect(&rhs.interval)))
            }
        }

        impl<'a> Mul<Bound<$type>> for &'a Bound<$type> {
            type Output = Result<Bound<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: Bound<$type>) -> Self::Output {
                let value = self.value.mul(rhs.value)?;
                Ok(Bound::new(value, self.interval.intersect(&rhs.interval)))
            }
        }

        impl<'a, 'b> Mul<&'a Bound<$type>> for &'b Bound<$type> {
            type Output = Result<Bound<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: &'a Bound<$type>) -> Self::Output {
                let value = self.value.mul(rhs.value)?;
                Ok(Bound::new(value, self.interval.intersect(&rhs.interval)))
            }
        }

        impl Div<$type> for Bound<$type> {
            type Output = Result<Bound<<$type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: $type) -> Self::Output {
                let value = self.value.div(rhs)?;
                Ok(Bound::new(value, self.interval))
            }
        }

        impl<'a> Div<&'a $type> for Bound<$type> {
            type Output = Result<Bound<<$type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: &'a $type) -> Self::Output {
                let value = self.value.div(rhs)?;
                Ok(Bound::new(value, self.interval))
            }
        }

        impl<'a> Div<$type> for &'a Bound<$type> {
            type Output = Result<Bound<<$type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: $type) -> Self::Output {
                let value = self.value.div(rhs)?;
                Ok(Bound::new(value, self.interval))
            }
        }

        impl<'a, 'b> Div<&'a $type> for &'b Bound<$type> {
            type Output = Result<Bound<<$type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: &'a $type) -> Self::Output {
                let value = self.value.div(rhs)?;
                Ok(Bound::new(value, self.interval))
            }
        }

        impl Div<Bound<$type>> for $type {
            type Output = Result<Bound<<$type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: Bound<$type>) -> Self::Output {
                let value = (self / rhs.value)?;
                Ok(Bound::new(value, rhs.interval))
            }
        }

        impl<'a> Div<Bound<$type>> for &'a $type {
            type Output = Result<Bound<<$type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: Bound<$type>) -> Self::Output {
                let value = (self / rhs.value)?;
                Ok(Bound::new(value, rhs.interval))
            }
        }

        impl<'a> Div<&'a Bound<$type>> for $type {
            type Output = Result<Bound<<$type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: &'a Bound<$type>) -> Self::Output {
                let value = (self / rhs.value)?;
                Ok(Bound::new(value, rhs.interval))
            }
        }

        impl<'a, 'b> Div<&'b Bound<$type>> for &'a $type {
            type Output = Result<Bound<<$type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: &'b Bound<$type>) -> Self::Output {
                let value = (self / rhs.value)?;
                Ok(Bound::new(value, rhs.interval))
            }
        }

        impl Div<Bound<$type>> for Bound<$type> {
            type Output = Result<Bound<<$type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: Bound<$type>) -> Self::Output {
                let value = (self.value / rhs.value)?;
                Ok(Bound::new(value, self.interval.intersect(&rhs.interval)))
            }
        }

        impl<'a> Div<&'a Bound<$type>> for Bound<$type> {
            type Output = Result<Bound<<$type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: &'a Bound<$type>) -> Self::Output {
                let value = (self.value / rhs.value)?;
                Ok(Bound::new(value, self.interval.intersect(&rhs.interval)))
            }
        }

        impl<'a> Div<Bound<$type>> for &'a Bound<$type> {
            type Output = Result<Bound<<$type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: Bound<$type>) -> Self::Output {
                let value = (self.value / rhs.value)?;
                Ok(Bound::new(value, self.interval.intersect(&rhs.interval)))
            }
        }

        impl<'a, 'b> Div<&'a Bound<$type>> for &'b Bound<$type> {
            type Output = Result<Bound<<$type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: &'a Bound<$type>) -> Self::Output {
                let value = (self.value / rhs.value)?;
                Ok(Bound::new(value, self.interval.intersect(&rhs.interval)))
            }
        }
    )*)
}
real_number_bound_template! { u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize f32 f64 }
