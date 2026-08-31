use std::cmp::Ordering;
use std::fmt::{Debug, Display};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::time::{Duration, Instant};

use chrono::NaiveDateTime;

use crate::algebra::concept::*;
use crate::algebra::operator::*;

use super::bound::*;
use super::interval::*;
use super::value_wrapper::*;
use super::IllegalArgumentError;

pub(self) fn empty<T: 'static + PartialOrd>(
    lb: &ValueWrapper<T>,
    ub: &ValueWrapper<T>,
    lb_interval: Interval,
    ub_interval: Interval,
) -> bool {
    if let ValueWrapper::Inf = lb {
        true
    } else if let ValueWrapper::NegInf = ub {
        true
    } else if let (ValueWrapper::Value(new_lb), ValueWrapper::Value(new_ub)) = (lb, ub) {
        !(lb_interval.lb_op())(new_lb, new_ub) || !(ub_interval.ub_op())(new_lb, new_ub)
    } else {
        false
    }
}

pub(self) fn ls<T>(lhs: &Bound<T>, rhs: &Bound<T>) -> Ordering where ValueWrapper<T>: PartialOrd {
    match lhs.value.partial_cmp(&rhs.value).unwrap() {
        Ordering::Less => Ordering::Less,
        Ordering::Equal => {
            if lhs.interval.outer(&rhs.interval) {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }
        Ordering::Greater => Ordering::Greater,
    }
}

pub(self) fn gr<T>(lhs: &Bound<T>, rhs: &Bound<T>) -> Ordering where ValueWrapper<T>: PartialOrd {
    match lhs.value.partial_cmp(&rhs.value).unwrap() {
        Ordering::Less => Ordering::Less,
        Ordering::Equal => {
            if lhs.interval.outer(&rhs.interval) {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }
        Ordering::Greater => Ordering::Greater,
    }
}

#[derive(Clone, Copy)]
pub struct ValueRange<T> {
    pub lb: Bound<T>,
    pub ub: Bound<T>,
}

impl<T> ValueRange<T> {
    pub fn new() -> Self {
        Self {
            lb: Bound {
                value: ValueWrapper::NegInf,
                interval: Interval::Closed,
            },
            ub: Bound {
                value: ValueWrapper::Inf,
                interval: Interval::Closed,
            },
        }
    }

    pub fn new_with(
        lb: T,
        ub: T,
        lb_interval: Interval,
        ub_interval: Interval,
    ) -> Result<Self, IllegalArgumentError>
    where
        ValueWrapper<T>: Display,
        T: 'static + PartialOrd,
    {
        let lower_bound_value = ValueWrapper::Value(lb);
        let upper_bound_value = ValueWrapper::Value(ub);
        Self::new_by(
            lower_bound_value,
            upper_bound_value,
            lb_interval,
            ub_interval,
        )
    }

    pub fn new_with_lb(lb: T, lb_interval: Interval) -> Result<Self, IllegalArgumentError>
    where
        ValueWrapper<T>: Display,
        T: 'static + PartialOrd,
    {
        let lower_bound_value = ValueWrapper::Value(lb);
        Self::new_by(
            lower_bound_value,
            ValueWrapper::Inf,
            lb_interval,
            Interval::Closed,
        )
    }

    pub fn new_with_ub(ub: T, ub_interval: Interval) -> Result<Self, IllegalArgumentError>
    where
        ValueWrapper<T>: Display,
        T: 'static + PartialOrd,
    {
        let upper_bound_value = ValueWrapper::Value(ub);
        Self::new_by(
            ValueWrapper::NegInf,
            upper_bound_value,
            Interval::Closed,
            ub_interval,
        )
    }

    pub fn new_by(
        lb: ValueWrapper<T>,
        ub: ValueWrapper<T>,
        lb_interval: Interval,
        ub_interval: Interval,
    ) -> Result<Self, IllegalArgumentError>
    where
        ValueWrapper<T>: Display,
        T: 'static + PartialOrd,
    {
        if !empty(&lb, &ub, lb_interval, ub_interval) {
            Ok(Self {
                lb: Bound {
                    value: lb,
                    interval: lb_interval,
                },
                ub: Bound {
                    value: ub,
                    interval: ub_interval,
                },
            })
        } else {
            Err(IllegalArgumentError {
                msg: format!(
                    "Invalid range {}{}, {}{}",
                    lb_interval.lb_sign(),
                    lb,
                    ub,
                    ub_interval.ub_sign()
                ),
            })
        }
    }

    pub fn fixed(&self) -> bool
    where
        T: Precision + Abs<Output = T> + PartialEq,
    {
        self.lb.interval == Interval::Closed
            && self.ub.interval == Interval::Closed
            && if let (ValueWrapper::Value(lower_value), ValueWrapper::Value(upper_value)) =
                (&self.lb.value, &self.ub.value)
            {
                let eq_op = Equal::new();
                eq_op(lower_value, upper_value)
            } else {
                false
            }
    }

    pub fn intersect(&self, rhs: &ValueRange<T>) -> Option<ValueRange<T>>
    where
        ValueWrapper<T>: Display + Clone + PartialOrd,
        T: 'static + PartialOrd,
    {
        let new_lb = if self.lb.value < rhs.lb.value {
            &self.lb.value
        } else {
            &rhs.lb.value
        };
        let new_lb_interval = self.lb.interval.intersect(&rhs.lb.interval);
        let new_ub = if self.ub.value > rhs.ub.value {
            &self.ub.value
        } else {
            &rhs.ub.value
        };
        let new_ub_interval = self.ub.interval.intersect(&rhs.ub.interval);
        match ValueRange::new_by(
            new_lb.clone(),
            new_ub.clone(),
            new_lb_interval,
            new_ub_interval,
        ) {
            Ok(new_range) => Some(new_range),
            Err(_) => None,
        }
    }

    pub fn contains(&self, value: &T) -> bool
    where
        ValueWrapper<T>: PartialOrd<T>,
        T: 'static,
    {
        !(self.lb.interval.lb_op())(&self.lb.value, value)
            || !(self.ub.interval.ub_op())(&self.ub.value, value)
    }

    pub fn contains_range(&self, range: &ValueRange<T>) -> bool
    where
        ValueWrapper<T>: PartialOrd,
        T: 'static,
    {
        !(self.lb.interval.lb_op())(&self.lb.value, &range.lb.value)
            || !(self.ub.interval.ub_op())(&self.ub.value, &range.ub.value)
    }
}

impl<T: Display> Display for ValueRange<T>
where
    ValueWrapper<T>: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}, {}{}",
            self.lb.interval.lb_sign(),
            self.lb.value,
            self.ub.value,
            self.ub.interval.ub_sign()
        )
    }
}

impl<T: Display> Debug for ValueRange<T>
where
    ValueWrapper<T>: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}, {}{}",
            self.lb.interval.lb_sign(),
            self.lb.value,
            self.ub.value,
            self.ub.interval.ub_sign()
        )
    }
}

impl<T: SemiArithmetic, U: SemiArithmetic> From<&ValueRange<U>> for ValueRange<T>
where
    Bound<T>: for<'a> From<&'a Bound<U>>,
{
    fn from(value: &ValueRange<U>) -> Self {
        Self {
            lb: Bound::from(&value.lb),
            ub: Bound::from(&value.ub),
        }
    }
}

impl<T, U> PartialEq<ValueRange<U>> for ValueRange<T> where ValueWrapper<T>: PartialEq<ValueWrapper<U>> {
    fn eq(&self, other: &ValueRange<U>) -> bool {
        self.lb == other.lb && self.ub == other.ub
    }
}

macro_rules! value_range_template {
    ($type:ident, $rhs:ident) => {
        impl Add<$rhs> for ValueRange<$type> {
            type Output = Result<ValueRange<<$type as Add<$rhs>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: $rhs) -> Self::Output {
                let new_lb = (self.lb + rhs)?;
                let new_ub = (self.ub + rhs)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl<'a> Add<&'a $rhs> for ValueRange<$type> {
            type Output = Result<ValueRange<<$type as Add<$rhs>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'a $rhs) -> Self::Output {
                let new_lb = (self.lb + rhs)?;
                let new_ub = (self.ub + rhs)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl<'a> Add<$rhs> for &'a ValueRange<$type> {
            type Output = Result<ValueRange<<$type as Add<$rhs>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: $rhs) -> Self::Output {
                let new_lb = (self.lb + rhs)?;
                let new_ub = (self.ub + rhs)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl<'a, 'b> Add<&'b $rhs> for &'a ValueRange<$type> {
            type Output = Result<ValueRange<<$type as Add<$rhs>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'b $rhs) -> Self::Output {
                let new_lb = (self.lb + rhs)?;
                let new_ub = (self.ub + rhs)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl AddAssign<$rhs> for ValueRange<$type> {
            fn add_assign(&mut self, rhs: $rhs) {
                let new_lb = (self.lb + rhs).expect("illegal argument");
                let new_ub = (self.ub + rhs).expect("illegal argument");
                self.lb = new_lb;
                self.ub = new_ub;
            }
        }

        impl<'a> AddAssign<&'a $rhs> for ValueRange<$type> {
            fn add_assign(&mut self, rhs: &'a $rhs) {
                let new_lb = (self.lb + rhs).expect("illegal argument");
                let new_ub = (self.ub + rhs).expect("illegal argument");
                self.lb = new_lb;
                self.ub = new_ub;
            }
        }

        impl Sub<$rhs> for ValueRange<$type> {
            type Output = Result<ValueRange<<$type as Sub<$rhs>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: $rhs) -> Self::Output {
                let new_lb = (self.lb - rhs)?;
                let new_ub = (self.ub - rhs)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl<'a> Sub<&'a $rhs> for ValueRange<$type> {
            type Output = Result<ValueRange<<$type as Sub<$rhs>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'a $rhs) -> Self::Output {
                let new_lb = (self.lb - rhs)?;
                let new_ub = (self.ub - rhs)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl<'a> Sub<$rhs> for &'a ValueRange<$type> {
            type Output = Result<ValueRange<<$type as Sub<$rhs>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: $rhs) -> Self::Output {
                let new_lb = (self.lb - rhs)?;
                let new_ub = (self.ub - rhs)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl<'a, 'b> Sub<&'b $rhs> for &'a ValueRange<$type> {
            type Output = Result<ValueRange<<$type as Sub<$rhs>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'b $rhs) -> Self::Output {
                let new_lb = (self.lb - rhs)?;
                let new_ub = (self.ub - rhs)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl SubAssign<$rhs> for ValueRange<$type> {
            fn sub_assign(&mut self, rhs: $rhs) {
                let new_lb = (self.lb - rhs).expect("illegal argument");
                let new_ub = (self.ub - rhs).expect("illegal argument");
                self.lb = new_lb;
                self.ub = new_ub;
            }
        }

        impl<'a> SubAssign<&'a $rhs> for ValueRange<$type> {
            fn sub_assign(&mut self, rhs: &'a $rhs) {
                let new_lb = (self.lb - rhs).expect("illegal argument");
                let new_ub = (self.ub - rhs).expect("illegal argument");
                self.lb = new_lb;
                self.ub = new_ub;
            }
        }
    };
}
value_range_template!(Instant, Duration);
value_range_template!(NaiveDateTime, Duration);

macro_rules! signed_value_range_template {
    ($($type:ident)*) => ($(
        impl Neg for ValueRange<$type> {
            type Output = Result<ValueRange<<$type as Neg>::Output>, IllegalArgumentError>;

            fn neg(self) -> Self::Output {
                let new_lb = (-self.ub)?;
                let new_ub = (-self.lb)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl Neg for &ValueRange<$type> {
            type Output = Result<ValueRange<<$type as Neg>::Output>, IllegalArgumentError>;

            fn neg(self) -> Self::Output {
                let new_lb = (-self.ub)?;
                let new_ub = (-self.lb)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }
    )*)
}
signed_value_range_template! { i8 i16 i32 i64 i128 isize f32 f64 }

macro_rules! real_number_value_range_template {
    ($($type:ident)*) => ($(
        impl Add<$type> for ValueRange<$type> {
            type Output = Result<ValueRange<<$type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: $type) -> Self::Output {
                let new_lb = (self.lb + rhs)?;
                let new_ub = (self.ub + rhs)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl<'a> Add<&'a $type> for ValueRange<$type> {
            type Output = Result<ValueRange<<$type as Add<&'a $type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'a $type) -> Self::Output {
                let new_lb = (self.lb + rhs)?;
                let new_ub = (self.ub + rhs)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl<'a> Add<$type> for &'a ValueRange<$type> {
            type Output = Result<ValueRange<<&'a $type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: $type) -> Self::Output {
                let new_lb = (self.lb + rhs)?;
                let new_ub = (self.ub + rhs)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl<'a, 'b> Add<&'b $type> for &'a ValueRange<$type> {
            type Output = Result<ValueRange<<&'a $type as Add<&'b $type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'b $type) -> Self::Output {
                let new_lb = (self.lb + rhs)?;
                let new_ub = (self.ub + rhs)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl AddAssign<$type> for ValueRange<$type> {
            fn add_assign(&mut self, rhs: $type) {
                let new_lb = (self.lb + rhs).expect("illegal argument");
                let new_ub = (self.ub + rhs).expect("illegal argument");
                self.lb = new_lb;
                self.ub = new_ub;
            }
        }

        impl<'a> AddAssign<&'a $type> for ValueRange<$type> {
            fn add_assign(&mut self, rhs: &'a $type) {
                let new_lb = (self.lb + rhs).expect("illegal argument");
                let new_ub = (self.ub + rhs).expect("illegal argument");
                self.lb = new_lb;
                self.ub = new_ub;
            }
        }

        impl Add<ValueRange<$type>> for ValueRange<$type> {
            type Output = Result<ValueRange<<$type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: ValueRange<$type>) -> Self::Output {
                let new_lb = (self.lb + rhs.lb)?;
                let new_ub = (self.ub + rhs.ub)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl<'a> Add<&'a ValueRange<$type>> for ValueRange<$type> {
            type Output = Result<ValueRange<<$type as Add<&'a $type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'a ValueRange<$type>) -> Self::Output {
                let new_lb = (self.lb + rhs.lb)?;
                let new_ub = (self.ub + rhs.ub)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl<'a> Add<ValueRange<$type>> for &'a ValueRange<$type> {
            type Output = Result<ValueRange<<&'a $type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: ValueRange<$type>) -> Self::Output {
                let new_lb = (self.lb + rhs.lb)?;
                let new_ub = (self.ub + rhs.ub)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl<'a, 'b> Add<&'b ValueRange<$type>> for &'a ValueRange<$type> {
            type Output = Result<ValueRange<<&'a $type as Add<&'b $type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'b ValueRange<$type>) -> Self::Output {
                let new_lb = (self.lb + rhs.lb)?;
                let new_ub = (self.ub + rhs.ub)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl AddAssign<ValueRange<$type>> for ValueRange<$type> {
            fn add_assign(&mut self, rhs: ValueRange<$type>) {
                let new_lb = (self.lb + rhs.lb).expect("illegal argument");
                let new_ub = (self.ub + rhs.ub).expect("illegal argument");
                self.lb = new_lb;
                self.ub = new_ub;
            }
        }

        impl<'a> AddAssign<&'a ValueRange<$type>> for ValueRange<$type> {
            fn add_assign(&mut self, rhs: &'a ValueRange<$type>) {
                let new_lb = (self.lb + rhs.lb).expect("illegal argument");
                let new_ub = (self.ub + rhs.ub).expect("illegal argument");
                self.lb = new_lb;
                self.ub = new_ub;
            }
        }

        impl Sub<$type> for ValueRange<$type> {
            type Output = Result<ValueRange<<$type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: $type) -> Self::Output {
                let new_lb = (self.lb - rhs)?;
                let new_ub = (self.ub - rhs)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl<'a> Sub<&'a $type> for ValueRange<$type> {
            type Output = Result<ValueRange<<$type as Sub<&'a $type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'a $type) -> Self::Output {
                let new_lb = (self.lb - rhs)?;
                let new_ub = (self.ub - rhs)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl<'a> Sub<$type> for &'a ValueRange<$type> {
            type Output = Result<ValueRange<<&'a $type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: $type) -> Self::Output {
                let new_lb = (self.lb - rhs)?;
                let new_ub = (self.ub - rhs)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl<'a, 'b> Sub<&'b $type> for &'a ValueRange<$type> {
            type Output = Result<ValueRange<<&'a $type as Sub<&'b $type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'b $type) -> Self::Output {
                let new_lb = (self.lb - rhs)?;
                let new_ub = (self.ub - rhs)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl SubAssign<$type> for ValueRange<$type> {
            fn sub_assign(&mut self, rhs: $type) {
                let new_lb = (self.lb - rhs).expect("illegal argument");
                let new_ub = (self.ub - rhs).expect("illegal argument");
                self.lb = new_lb;
                self.ub = new_ub;
            }
        }

        impl<'a> SubAssign<&'a $type> for ValueRange<$type> {
            fn sub_assign(&mut self, rhs: &'a $type) {
                let new_lb = (self.lb - rhs).expect("illegal argument");
                let new_ub = (self.ub - rhs).expect("illegal argument");
                self.lb = new_lb;
                self.ub = new_ub;
            }
        }

        impl Sub<ValueRange<$type>> for ValueRange<$type> {
            type Output = Result<ValueRange<<$type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: ValueRange<$type>) -> Self::Output {
                let new_lb = (self.lb - rhs.ub)?;
                let new_ub = (self.ub - rhs.lb)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl<'a> Sub<&'a ValueRange<$type>> for ValueRange<$type> {
            type Output = Result<ValueRange<<$type as Sub<&'a $type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'a ValueRange<$type>) -> Self::Output {
                let new_lb = (self.lb - rhs.ub)?;
                let new_ub = (self.ub - rhs.lb)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl<'a> Sub<ValueRange<$type>> for &'a ValueRange<$type> {
            type Output = Result<ValueRange<<&'a $type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: ValueRange<$type>) -> Self::Output {
                let new_lb = (self.lb - rhs.ub)?;
                let new_ub = (self.ub - rhs.lb)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl<'a, 'b> Sub<&'b ValueRange<$type>> for &'a ValueRange<$type> {
            type Output = Result<ValueRange<<&'a $type as Sub<&'b $type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'b ValueRange<$type>) -> Self::Output {
                let new_lb = (self.lb - rhs.ub)?;
                let new_ub = (self.ub - rhs.lb)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl SubAssign<ValueRange<$type>> for ValueRange<$type> {
            fn sub_assign(&mut self, rhs: ValueRange<$type>) {
                let new_lb = (self.lb - rhs.ub).expect("illegal argument");
                let new_ub = (self.ub - rhs.lb).expect("illegal argument");
                self.lb = new_lb;
                self.ub = new_ub;
            }
        }

        impl<'a> SubAssign<&'a ValueRange<$type>> for ValueRange<$type> {
            fn sub_assign(&mut self, rhs: &'a ValueRange<$type>) {
                let new_lb = (self.lb - rhs.ub).expect("illegal argument");
                let new_ub = (self.ub - rhs.lb).expect("illegal argument");
                self.lb = new_lb;
                self.ub = new_ub;
            }
        }

        impl Mul<$type> for ValueRange<$type> {
            type Output = Result<ValueRange<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: $type) -> Self::Output {
                if &rhs >= $type::ZERO {
                    let new_lb = (self.lb * rhs)?;
                    let new_ub = (self.ub * rhs)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                } else {
                    let new_lb = (self.ub * rhs)?;
                    let new_ub = (self.lb * rhs)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                }
            }
        }

        impl<'a> Mul<&'a $type> for ValueRange<$type> {
            type Output = Result<ValueRange<<$type as Mul<&'a $type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: &'a $type) -> Self::Output {
                if rhs >= $type::ZERO {
                    let new_lb = (self.lb * rhs)?;
                    let new_ub = (self.ub * rhs)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                } else {
                    let new_lb = (self.ub * rhs)?;
                    let new_ub = (self.lb * rhs)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                }
            }
        }

        impl<'a> Mul<$type> for &'a ValueRange<$type> {
            type Output = Result<ValueRange<<&'a $type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: $type) -> Self::Output {
                if &rhs >= $type::ZERO {
                    let new_lb = (self.lb * rhs)?;
                    let new_ub = (self.ub * rhs)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                } else {
                    let new_lb = (self.ub * rhs)?;
                    let new_ub = (self.lb * rhs)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                }
            }
        }

        impl<'a, 'b> Mul<&'b $type> for &'a ValueRange<$type> {
            type Output = Result<ValueRange<<&'a $type as Mul<&'b $type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: &'b $type) -> Self::Output {
                if rhs >= $type::ZERO {
                    let new_lb = (self.lb * rhs)?;
                    let new_ub = (self.ub * rhs)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                } else {
                    let new_lb = (self.ub * rhs)?;
                    let new_ub = (self.lb * rhs)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                }
            }
        }

        impl MulAssign<$type> for ValueRange<$type> {
            fn mul_assign(&mut self, rhs: $type) {
                if &rhs >= $type::ZERO {
                    let new_lb = (self.lb * rhs).expect("illegal argument");
                    let new_ub = (self.ub * rhs).expect("illegal argument");
                    self.lb = new_lb;
                    self.ub = new_ub;
                } else {
                    let new_lb = (self.ub * rhs).expect("illegal argument");
                    let new_ub = (self.lb * rhs).expect("illegal argument");
                    self.lb = new_lb;
                    self.ub = new_ub;
                }
            }
        }

        impl<'a> MulAssign<&'a $type> for ValueRange<$type> {
            fn mul_assign(&mut self, rhs: &'a $type) {
                if rhs >= $type::ZERO {
                    let new_lb = (self.lb * rhs).expect("illegal argument");
                    let new_ub = (self.ub * rhs).expect("illegal argument");
                    self.lb = new_lb;
                    self.ub = new_ub;
                } else {
                    let new_lb = (self.ub * rhs).expect("illegal argument");
                    let new_ub = (self.lb * rhs).expect("illegal argument");
                    self.lb = new_lb;
                    self.ub = new_ub;
                }
            }
        }

        impl Mul<ValueRange<$type>> for ValueRange<$type> {
            type Output = Result<ValueRange<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: ValueRange<$type>) -> Self::Output {
                let bounds = vec![(self.lb * rhs.lb)?, (self.lb * rhs.ub)?, (self.ub * rhs.lb)?, (self.ub * rhs.ub)?];
                let new_lb = bounds.iter().min_by(|lhs, rhs| ls(lhs, rhs)).unwrap();
                let new_ub = bounds.iter().max_by(|lhs, rhs| gr(lhs, rhs)).unwrap();
                Ok(ValueRange {
                    lb: *new_lb,
                    ub: *new_ub,
                })
            }
        }

        impl<'a> Mul<&'a ValueRange<$type>> for ValueRange<$type> {
            type Output = Result<ValueRange<<$type as Mul<&'a $type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: &'a ValueRange<$type>) -> Self::Output {
                let bounds = vec![(self.lb * rhs.lb)?, (self.lb * rhs.ub)?, (self.ub * rhs.lb)?, (self.ub * rhs.ub)?];
                let new_lb = bounds.iter().min_by(|lhs, rhs| ls(lhs, rhs)).unwrap();
                let new_ub = bounds.iter().max_by(|lhs, rhs| gr(lhs, rhs)).unwrap();
                Ok(ValueRange {
                    lb: *new_lb,
                    ub: *new_ub,
                })
            }
        }

        impl<'a> Mul<ValueRange<$type>> for &'a ValueRange<$type> {
            type Output = Result<ValueRange<<&'a $type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: ValueRange<$type>) -> Self::Output {
                let bounds = vec![(self.lb * rhs.lb)?, (self.lb * rhs.ub)?, (self.ub * rhs.lb)?, (self.ub * rhs.ub)?];
                let new_lb = bounds.iter().min_by(|lhs, rhs| ls(lhs, rhs)).unwrap();
                let new_ub = bounds.iter().max_by(|lhs, rhs| gr(lhs, rhs)).unwrap();
                Ok(ValueRange {
                    lb: *new_lb,
                    ub: *new_ub,
                })
            }
        }

        impl<'a, 'b> Mul<&'b ValueRange<$type>> for &'a ValueRange<$type> {
            type Output = Result<ValueRange<<&'a $type as Mul<&'b $type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: &'b ValueRange<$type>) -> Self::Output {
                let bounds = vec![(self.lb * rhs.lb)?, (self.lb * rhs.ub)?, (self.ub * rhs.lb)?, (self.ub * rhs.ub)?];
                let new_lb = bounds.iter().min_by(|lhs, rhs| ls(lhs, rhs)).unwrap();
                let new_ub = bounds.iter().max_by(|lhs, rhs| gr(lhs, rhs)).unwrap();
                Ok(ValueRange {
                    lb: *new_lb,
                    ub: *new_ub,
                })
            }
        }

        impl MulAssign<ValueRange<$type>> for ValueRange<$type> {
            fn mul_assign(&mut self, rhs: ValueRange<$type>) {
                let bounds = vec![
                    (self.lb * rhs.lb).expect("illegal argument"),
                    (self.lb * rhs.ub).expect("illegal argument"),
                    (self.ub * rhs.lb).expect("illegal argument"),
                    (self.ub * rhs.ub).expect("illegal argument")
                ];
                let new_lb = bounds.iter().min_by(|lhs, rhs| ls(lhs, rhs)).unwrap();
                let new_ub = bounds.iter().max_by(|lhs, rhs| gr(lhs, rhs)).unwrap();
                self.lb = *new_lb;
                self.ub = *new_ub;
            }
        }

        impl<'a> MulAssign<&'a ValueRange<$type>> for ValueRange<$type> {
            fn mul_assign(&mut self, rhs: &'a ValueRange<$type>) {
                let bounds = vec![
                    (self.lb * rhs.lb).expect("illegal argument"),
                    (self.lb * rhs.ub).expect("illegal argument"),
                    (self.ub * rhs.lb).expect("illegal argument"),
                    (self.ub * rhs.ub).expect("illegal argument")
                ];
                let new_lb = bounds.iter().min_by(|lhs, rhs| ls(lhs, rhs)).unwrap();
                let new_ub = bounds.iter().max_by(|lhs, rhs| gr(lhs, rhs)).unwrap();
                self.lb = *new_lb;
                self.ub = *new_ub;
            }
        }

        impl Div<$type> for ValueRange<$type> {
            type Output = Result<ValueRange<<$type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: $type) -> Self::Output {
                if &rhs > $type::ZERO {
                    let new_lb = (self.lb / rhs)?;
                    let new_ub = (self.ub / rhs)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                } else if &rhs < $type::ZERO {
                    let new_lb = (self.ub / rhs)?;
                    let new_ub = (self.lb / rhs)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                } else {
                    Err(IllegalArgumentError {
                        msg: "division by zero".to_string()
                    })
                }
            }
        }

        impl<'a> Div<&'a $type> for ValueRange<$type> {
            type Output = Result<ValueRange<<$type as Div<&'a $type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: &'a $type) -> Self::Output {
                if rhs > $type::ZERO {
                    let new_lb = (self.lb / rhs)?;
                    let new_ub = (self.ub / rhs)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                } else if rhs < $type::ZERO {
                    let new_lb = (self.ub / rhs)?;
                    let new_ub = (self.lb / rhs)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                } else {
                    Err(IllegalArgumentError {
                        msg: "division by zero".to_string()
                    })
                }
            }
        }

        impl<'a> Div<$type> for &'a ValueRange<$type> {
            type Output = Result<ValueRange<<&'a $type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: $type) -> Self::Output {
                if &rhs > $type::ZERO {
                    let new_lb = (self.lb / rhs)?;
                    let new_ub = (self.ub / rhs)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                } else if &rhs < $type::ZERO {
                    let new_lb = (self.ub / rhs)?;
                    let new_ub = (self.lb / rhs)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                } else {
                    Err(IllegalArgumentError {
                        msg: "division by zero".to_string()
                    })
                }
            }
        }

        impl<'a, 'b> Div<&'b $type> for &'a ValueRange<$type> {
            type Output = Result<ValueRange<<&'a $type as Div<&'b $type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: &'b $type) -> Self::Output {
                if rhs > $type::ZERO {
                    let new_lb = (self.lb / rhs)?;
                    let new_ub = (self.ub / rhs)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                } else if rhs < $type::ZERO {
                    let new_lb = (self.ub / rhs)?;
                    let new_ub = (self.lb / rhs)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                } else {
                    Err(IllegalArgumentError {
                        msg: "division by zero".to_string()
                    })
                }
            }
        }

        impl DivAssign<$type> for ValueRange<$type> {
            fn div_assign(&mut self, rhs: $type) {
                if &rhs > $type::ZERO {
                    let new_lb = (self.lb / rhs).expect("illegal argument");
                    let new_ub = (self.ub / rhs).expect("illegal argument");
                    self.lb = new_lb;
                    self.ub = new_ub;
                } else if &rhs < $type::ZERO {
                    let new_lb = (self.ub / rhs).expect("illegal argument");
                    let new_ub = (self.lb / rhs).expect("illegal argument");
                    self.lb = new_lb;
                    self.ub = new_ub;
                } else {
                    panic!("division by zero")
                }
            }
        }

        impl<'a> DivAssign<&'a $type> for ValueRange<$type> {
            fn div_assign(&mut self, rhs: &'a $type) {
                if rhs > $type::ZERO {
                    let new_lb = (self.lb / rhs).expect("illegal argument");
                    let new_ub = (self.ub / rhs).expect("illegal argument");
                    self.lb = new_lb;
                    self.ub = new_ub;
                } else if rhs < $type::ZERO {
                    let new_lb = (self.ub / rhs).expect("illegal argument");
                    let new_ub = (self.lb / rhs).expect("illegal argument");
                    self.lb = new_lb;
                    self.ub = new_ub;
                } else {
                    panic!("division by zero")
                }
            }
        }
    )*)
}
real_number_value_range_template! { u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize f32 f64 }
