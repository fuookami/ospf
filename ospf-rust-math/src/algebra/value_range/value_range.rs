use std::cmp::{max, Ordering};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Range, RangeBounds, RangeFrom, RangeFull,
    RangeInclusive, RangeTo, RangeToInclusive, Sub, SubAssign
};
use std::time::{Duration, Instant};

use chrono::NaiveDateTime;

use crate::algebra::concept::*;
use crate::algebra::operator::*;

use super::bound::*;
use super::interval::*;
use super::value_wrapper::*;
use super::error::IllegalArgumentError;

pub(self) fn empty<T: 'static + PartialOrd>(
    lb: &ValueWrapper<T>,
    ub: &ValueWrapper<T>,
    lb_interval: Interval,
    ub_interval: Interval,
) -> bool {
    if let ValueWrapper::NegInf = lb {
        false
    } else if let ValueWrapper::Inf = ub {
        false
    } else if let (ValueWrapper::Value(new_lb), ValueWrapper::Value(new_ub)) = (lb, ub) {
        !(lb_interval.lb_op())(new_lb, new_ub) || !(ub_interval.ub_op())(new_ub, new_lb)
    } else {
        true
    }
}

pub(self) fn ls<T>(lhs: &Bound<T>, rhs: &Bound<T>) -> Ordering
where
    ValueWrapper<T>: PartialOrd,
{
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

pub(self) fn gr<T>(lhs: &Bound<T>, rhs: &Bound<T>) -> Ordering
where
    ValueWrapper<T>: PartialOrd,
{
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
            lb: Bound::new(ValueWrapper::NegInf, Interval::Open),
            ub: Bound::new(ValueWrapper::Inf, Interval::Open),
        }
    }

    pub fn new_with_constant(value: T) -> Self where T: Clone {
        Self {
            lb: Bound::new(ValueWrapper::Value(value.clone()), Interval::Closed),
            ub: Bound::new(ValueWrapper::Value(value), Interval::Closed),
        }
    }

    pub fn new_with(
        lb: T,
        ub: T,
        lb_interval: Interval,
        ub_interval: Interval,
    ) -> Result<Self, IllegalArgumentError>
    where
        T: 'static + PartialOrd + Display,
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
        T: 'static + PartialOrd + Display,
    {
        let lower_bound_value = ValueWrapper::Value(lb);
        Self::new_by(
            lower_bound_value,
            ValueWrapper::Inf,
            lb_interval,
            Interval::Open,
        )
    }

    pub fn new_with_ub(ub: T, ub_interval: Interval) -> Result<Self, IllegalArgumentError>
    where
        T: 'static + PartialOrd + Display,
    {
        let upper_bound_value = ValueWrapper::Value(ub);
        Self::new_by(
            ValueWrapper::NegInf,
            upper_bound_value,
            Interval::Open,
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
        T: 'static + PartialOrd + Display,
    {
        if !empty(&lb, &ub, lb_interval, ub_interval) {
            Ok(Self {
                lb: Bound::new(lb, lb_interval),
                ub: Bound::new(ub, ub_interval),
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
        T: PartialEq
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

    pub fn fixed_value(&self) -> Option<&ValueWrapper<T>>
    where
        T: PartialEq
    {
        if self.fixed() {
            Some(&self.lb.value)
        } else {
            None
        }
    }

    pub fn mean(&self) -> Result<ValueWrapper<T>, IllegalArgumentError>
    where
        T: RealNumber,
        for<'a> &'a ValueWrapper<T>: Add<&'a ValueWrapper<T>, Output = Result<ValueWrapper<T>, IllegalArgumentError>>,
        ValueWrapper<T>: for<'a> Div<&'a T, Output = Result<ValueWrapper<T>, IllegalArgumentError>>,
    {
        (&self.lb.value + &self.ub.value)? / T::TWO
    }

    pub fn diff(&self) -> Result<ValueWrapper<T>, IllegalArgumentError>
    where
        for<'a> &'a ValueWrapper<T>: Sub<&'a ValueWrapper<T>, Output = Result<ValueWrapper<T>, IllegalArgumentError>>,
    {
        &self.ub.value - &self.lb.value
    }

    pub fn gap(&self) -> Result<ValueWrapper<T>, IllegalArgumentError>
    where
        T: RealNumber + Ord,
        for<'a> &'a T: Abs<Output = T>,
        for<'a> &'a ValueWrapper<T>: Add<&'a ValueWrapper<T>, Output = Result<ValueWrapper<T>, IllegalArgumentError>>,
        for<'a> &'a ValueWrapper<T>: Sub<&'a ValueWrapper<T>, Output = Result<ValueWrapper<T>, IllegalArgumentError>>,
        ValueWrapper<T>: for<'a> Div<&'a T, Output = Result<ValueWrapper<T>, IllegalArgumentError>>,
    {
        self.diff()? / max(T::DECIMAL_PRECISION, &self.mean()?.unwrap().abs())
    }

    pub fn union(&self, rhs: &ValueRange<T>) -> Option<ValueRange<T>>
    where
        T: 'static + PartialOrd + Display + Clone,
    {
        if self.ub.value < rhs.lb.value || rhs.ub.value < self.lb.value {
            return None;
        }

        let new_lb = if self.lb.value < rhs.lb.value {
            &self.lb.value
        } else {
            &rhs.lb.value
        };
        let new_lb_interval = match self.lb.value.partial_cmp(&rhs.lb.value) {
            Some(Ordering::Less) => self.lb.interval,
            Some(Ordering::Greater) => rhs.lb.interval,
            _ => self.lb.interval.union(&rhs.lb.interval),
        };
        let new_ub = if self.ub.value < rhs.ub.value {
            &rhs.ub.value
        } else {
            &self.ub.value
        };
        let new_ub_interval = match self.ub.value.partial_cmp(&rhs.ub.value) {
            Some(Ordering::Less) => rhs.ub.interval,
            Some(Ordering::Greater) => self.ub.interval,
            _ => self.ub.interval.union(&rhs.ub.interval),
        };
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

    pub fn intersect(&self, rhs: &ValueRange<T>) -> Option<ValueRange<T>>
    where
        T: 'static + PartialOrd + Display + Clone,
    {
        let new_lb = if self.lb.value < rhs.lb.value {
            &rhs.lb.value
        } else {
            &self.lb.value
        };
        let new_lb_interval = if self.lb.value.is_inf_or_neg_inf() {
            rhs.lb.interval
        } else if rhs.lb.value.is_inf_or_neg_inf() {
            self.lb.interval
        } else {
            match self.lb.value.partial_cmp(&rhs.lb.value) {
                Some(Ordering::Less) => rhs.lb.interval,
                Some(Ordering::Greater) => self.lb.interval,
                _ => self.lb.interval.intersect(&rhs.lb.interval),
            }
        };
        let new_ub = if self.ub.value < rhs.ub.value {
            &self.ub.value
        } else {
            &rhs.ub.value
        };
        let new_ub_interval = if self.ub.value.is_inf_or_neg_inf() {
            rhs.ub.interval
        } else if rhs.ub.value.is_inf_or_neg_inf() {
            self.ub.interval
        } else {
            match self.ub.value.partial_cmp(&rhs.ub.value) {
                Some(Ordering::Less) => self.ub.interval,
                Some(Ordering::Greater) => rhs.ub.interval,
                _ => self.ub.interval.intersect(&rhs.ub.interval),
            }
        };
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
        (self.lb.interval.lb_op())(&self.lb.value, value)
            && (self.ub.interval.ub_op())(&self.ub.value, value)
    }

    pub fn contains_range(&self, range: &ValueRange<T>) -> bool
    where
        ValueWrapper<T>: PartialOrd,
        T: 'static,
    {
        let lb_interval = self.lb.interval.intersect(&range.lb.interval);
        let ub_interval = self.ub.interval.intersect(&range.ub.interval);
        (lb_interval.lb_op())(&self.lb.value, &range.lb.value)
            && (ub_interval.ub_op())(&self.ub.value, &range.ub.value)
    }
}

impl<T: Display> Display for ValueRange<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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

impl<T: Display> Debug for ValueRange<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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

impl<T> From<Range<T>> for ValueRange<T> {
    fn from(range: Range<T>) -> Self {
        Self {
            lb: Bound::new(ValueWrapper::Value(range.start), Interval::Closed),
            ub: Bound::new(ValueWrapper::Value(range.end), Interval::Open),
        }
    }
}

impl<T: Clone> From<&Range<T>> for ValueRange<T> {
    fn from(range: &Range<T>) -> Self {
        Self {
            lb: Bound::new(ValueWrapper::Value(range.start.clone()), Interval::Closed),
            ub: Bound::new(ValueWrapper::Value(range.end.clone()), Interval::Open),
        }
    }
}

impl<T> From<RangeFrom<T>> for ValueRange<T> {
    fn from(range: RangeFrom<T>) -> Self {
        Self {
            lb: Bound::new(ValueWrapper::Value(range.start), Interval::Closed),
            ub: Bound::new(ValueWrapper::Inf, Interval::Open),
        }
    }
}

impl<T: Clone> From<&RangeFrom<T>> for ValueRange<T> {
    fn from(range: &RangeFrom<T>) -> Self {
        Self {
            lb: Bound::new(ValueWrapper::Value(range.start.clone()), Interval::Closed),
            ub: Bound::new(ValueWrapper::Inf, Interval::Open),
        }
    }
}

impl<T: Clone> From<RangeInclusive<T>> for ValueRange<T> {
    fn from(range: RangeInclusive<T>) -> Self {
        Self {
            lb: Bound::new(ValueWrapper::Value(range.start().clone()), Interval::Closed),
            ub: Bound::new(ValueWrapper::Value(range.end().clone()), Interval::Closed),
        }
    }
}

impl<T: Clone> From<&RangeInclusive<T>> for ValueRange<T> {
    fn from(range: &RangeInclusive<T>) -> Self {
        Self {
            lb: Bound::new(ValueWrapper::Value(range.start().clone()), Interval::Closed),
            ub: Bound::new(ValueWrapper::Value(range.end().clone()), Interval::Closed),
        }
    }
}

impl<T> From<RangeTo<T>> for ValueRange<T> {
    fn from(range: RangeTo<T>) -> Self {
        Self {
            lb: Bound::new(ValueWrapper::NegInf, Interval::Open),
            ub: Bound::new(ValueWrapper::Value(range.end), Interval::Open),
        }
    }
}

impl<T: Clone> From<&RangeTo<T>> for ValueRange<T> {
    fn from(range: &RangeTo<T>) -> Self {
        Self {
            lb: Bound::new(ValueWrapper::NegInf, Interval::Open),
            ub: Bound::new(ValueWrapper::Value(range.end.clone()), Interval::Open),
        }
    }
}

impl<T> From<RangeToInclusive<T>> for ValueRange<T> {
    fn from(range: RangeToInclusive<T>) -> Self {
        Self {
            lb: Bound::new(ValueWrapper::NegInf, Interval::Open),
            ub: Bound::new(ValueWrapper::Value(range.end), Interval::Closed),
        }
    }
}

impl<T: Clone> From<&RangeToInclusive<T>> for ValueRange<T> {
    fn from(range: &RangeToInclusive<T>) -> Self {
        Self {
            lb: Bound::new(ValueWrapper::NegInf, Interval::Open),
            ub: Bound::new(ValueWrapper::Value(range.end.clone()), Interval::Closed),
        }
    }
}

impl From<RangeFull> for ValueRange<i64> {
    fn from(_range: RangeFull) -> Self {
        Self {
            lb: Bound::new(ValueWrapper::NegInf, Interval::Open),
            ub: Bound::new(ValueWrapper::Inf, Interval::Open),
        }
    }
}

impl<T> From<&RangeFull> for ValueRange<T> {
    fn from(_range: &RangeFull) -> Self {
        Self {
            lb: Bound::new(ValueWrapper::NegInf, Interval::Open),
            ub: Bound::new(ValueWrapper::Inf, Interval::Open),
        }
    }
}

impl<T, U> From<&ValueRange<U>> for ValueRange<T>
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

impl<T, U> PartialEq<ValueRange<U>> for ValueRange<T>
where
    ValueWrapper<T>: PartialEq<ValueWrapper<U>>,
{
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

        impl Add<ValueRange<$type>> for $type {
            type Output = Result<ValueRange<<$type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: ValueRange<$type>) -> Self::Output {
                let new_lb = (self + rhs.lb)?;
                let new_ub = (self + rhs.ub)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl<'a> Add<ValueRange<$type>> for &'a $type {
            type Output = Result<ValueRange<<&'a $type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: ValueRange<$type>) -> Self::Output {
                let new_lb = (self + rhs.lb)?;
                let new_ub = (self + rhs.ub)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl<'a> Add<&'a ValueRange<$type>> for $type {
            type Output = Result<ValueRange<<$type as Add<&'a $type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'a ValueRange<$type>) -> Self::Output {
                let new_lb = (self + rhs.lb)?;
                let new_ub = (self + rhs.ub)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl<'a, 'b> Add<&'b ValueRange<$type>> for &'a $type {
            type Output = Result<ValueRange<<&'a $type as Add<&'b $type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'b ValueRange<$type>) -> Self::Output {
                let new_lb = (self + rhs.lb)?;
                let new_ub = (self + rhs.ub)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
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

        impl Sub<ValueRange<$type>> for $type {
            type Output = Result<ValueRange<<$type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: ValueRange<$type>) -> Self::Output {
                let new_lb = (self - rhs.ub)?;
                let new_ub = (self - rhs.lb)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl<'a> Sub<ValueRange<$type>> for &'a $type {
            type Output = Result<ValueRange<<&'a $type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: ValueRange<$type>) -> Self::Output {
                let new_lb = (self - rhs.ub)?;
                let new_ub = (self - rhs.lb)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl<'a> Sub<&'a ValueRange<$type>> for $type {
            type Output = Result<ValueRange<<$type as Sub<&'a $type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'a ValueRange<$type>) -> Self::Output {
                let new_lb = (self - rhs.ub)?;
                let new_ub = (self - rhs.lb)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
            }
        }

        impl<'a, 'b> Sub<&'b ValueRange<$type>> for &'a $type {
            type Output = Result<ValueRange<<&'a $type as Sub<&'b $type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'b ValueRange<$type>) -> Self::Output {
                let new_lb = (self - rhs.ub)?;
                let new_ub = (self - rhs.lb)?;
                Ok(ValueRange {
                    lb: new_lb,
                    ub: new_ub,
                })
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

        impl Mul<ValueRange<$type>> for $type {
            type Output = Result<ValueRange<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: ValueRange<$type>) -> Self::Output {
                if &self >= $type::ZERO {
                    let new_lb = (self * rhs.lb)?;
                    let new_ub = (self * rhs.ub)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                } else {
                    let new_lb = (self * rhs.ub)?;
                    let new_ub = (self * rhs.lb)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                }
            }
        }

        impl<'a> Mul<ValueRange<$type>> for &'a $type {
            type Output = Result<ValueRange<<&'a $type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: ValueRange<$type>) -> Self::Output {
                if self >= $type::ZERO {
                    let new_lb = (self * rhs.lb)?;
                    let new_ub = (self * rhs.ub)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                } else {
                    let new_lb = (self * rhs.ub)?;
                    let new_ub = (self * rhs.lb)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                }
            }
        }

        impl<'a> Mul<&'a ValueRange<$type>> for $type {
            type Output = Result<ValueRange<<$type as Mul<&'a $type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: &'a ValueRange<$type>) -> Self::Output {
                if &self >= $type::ZERO {
                    let new_lb = (self * rhs.lb)?;
                    let new_ub = (self * rhs.ub)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                } else {
                    let new_lb = (self * rhs.ub)?;
                    let new_ub = (self * rhs.lb)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                }
            }
        }

        impl<'a, 'b> Mul<&'b ValueRange<$type>> for &'a $type {
            type Output = Result<ValueRange<<&'a $type as Mul<&'b $type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: &'b ValueRange<$type>) -> Self::Output {
                if self >= $type::ZERO {
                    let new_lb = (self * rhs.lb)?;
                    let new_ub = (self * rhs.ub)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                } else {
                    let new_lb = (self * rhs.ub)?;
                    let new_ub = (self * rhs.lb)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
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

        impl Div<ValueRange<$type>> for $type {
            type Output = Result<ValueRange<<$type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: ValueRange<$type>) -> Self::Output {
                if &self > $type::ZERO {
                    let new_lb = (self / rhs.lb)?;
                    let new_ub = (self / rhs.ub)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                } else if &self < $type::ZERO {
                    let new_lb = (self / rhs.ub)?;
                    let new_ub = (self / rhs.lb)?;
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

        impl<'a> Div<ValueRange<$type>> for &'a $type {
            type Output = Result<ValueRange<<&'a $type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: ValueRange<$type>) -> Self::Output {
                if self > $type::ZERO {
                    let new_lb = (self / rhs.lb)?;
                    let new_ub = (self / rhs.ub)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                } else if self < $type::ZERO {
                    let new_lb = (self / rhs.ub)?;
                    let new_ub = (self / rhs.lb)?;
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

        impl<'a> Div<&'a ValueRange<$type>> for $type {
            type Output = Result<ValueRange<<$type as Div<&'a $type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: &'a ValueRange<$type>) -> Self::Output {
                if &self > $type::ZERO {
                    let new_lb = (self / rhs.lb)?;
                    let new_ub = (self / rhs.ub)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                } else if &self < $type::ZERO {
                    let new_lb = (self / rhs.ub)?;
                    let new_ub = (self / rhs.lb)?;
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

        impl<'a, 'b> Div<&'b ValueRange<$type>> for &'a $type {
            type Output = Result<ValueRange<<&'a $type as Div<&'b $type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: &'b ValueRange<$type>) -> Self::Output {
                if self > $type::ZERO {
                    let new_lb = (self / rhs.lb)?;
                    let new_ub = (self / rhs.ub)?;
                    Ok(ValueRange {
                        lb: new_lb,
                        ub: new_ub,
                    })
                } else if self < $type::ZERO {
                    let new_lb = (self / rhs.ub)?;
                    let new_ub = (self / rhs.lb)?;
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
    )*)
}
real_number_value_range_template! { u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize f32 f64 }

#[macro_export]
macro_rules! value_range {
    ((..,$ub:expr)) => {
        ValueRange::new_with_ub($ub, Interval::Open)
    };
    ([..,$ub:expr]) => {
        ValueRange::new_with_ub($ub, Interval::Closed)
    };
    (($lb:expr,..)) => {
        ValueRange::new_with_lb($lb, Interval::Open)
    };
    ([$lb:expr,..]) => {
        ValueRange::new_with_lb($lb, Interval::Closed)
    };
    (($lb:expr, $ub:expr)) => {
        ValueRange::new_with($lb, $ub, Interval::Open, Interval::Open)
    };
    ([$lb:expr, $ub:expr]) => {
        ValueRange::new_with($lb, $ub, Interval::Closed, Interval::Closed)
    };
    ((($lb:expr)..$ub:expr)) => {
        ValueRange::new_with($lb, $ub, Interval::Closed, Interval::Open)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constructor() {
        let open_range = value_range!((1.0f64, 2.0f64));
        assert!(open_range.is_ok());
        assert_eq!(open_range.as_ref().unwrap().lb.value.unwrap(), &1.0);
        assert_eq!(open_range.as_ref().unwrap().ub.interval, Interval::Open);
        assert_eq!(open_range.as_ref().unwrap().ub.value.unwrap(), &2.0);
        assert_eq!(open_range.as_ref().unwrap().ub.interval, Interval::Open);
        let closed_range = value_range!([1.0f64, 2.0f64]);
        assert!(closed_range.is_ok());
        assert_eq!(closed_range.as_ref().unwrap().lb.value.unwrap(), &1.0);
        assert_eq!(closed_range.as_ref().unwrap().ub.interval, Interval::Closed);
        assert_eq!(closed_range.as_ref().unwrap().ub.value.unwrap(), &2.0);
        assert_eq!(closed_range.as_ref().unwrap().ub.interval, Interval::Closed);
        let range_range = value_range!((1.0f64, 2.0f64)).unwrap();
        assert_eq!(range_range.lb.value.unwrap(), &1.0);
        assert_eq!(range_range.ub.interval, Interval::Open);
        assert_eq!(range_range.ub.value.unwrap(), &2.0);
        assert_eq!(range_range.ub.interval, Interval::Open);
        let range_inclusive_range = value_range!([1.0f64, 2.0f64]).unwrap();
        assert_eq!(range_inclusive_range.lb.value.unwrap(), &1.0);
        assert_eq!(range_inclusive_range.ub.interval, Interval::Closed);
        assert_eq!(range_inclusive_range.ub.value.unwrap(), &2.0);
        assert_eq!(range_inclusive_range.ub.interval, Interval::Closed);
        let invalid_range = value_range!([2.0f64, 1.0f64]);
        assert!(invalid_range.is_err());
    }

    #[test]
    fn test_plus() {
        let range = value_range!([1.0f64, 2.0f64]).unwrap();
        let added_range = (range + 1.0f64).unwrap();
        assert_eq!(added_range.lb.value.unwrap(), &2.0);
        assert_eq!(added_range.ub.value.unwrap(), &3.0);
        let twice_range = (range + range).unwrap();
        assert_eq!(twice_range.lb.value.unwrap(), &2.0);
        assert_eq!(twice_range.ub.value.unwrap(), &4.0);
        let inf_range = (range + f64::INFINITY).unwrap();
        assert_eq!(inf_range.lb.value.unwrap(), f64::INF.as_ref().unwrap());
        assert_eq!(inf_range.lb.interval, Interval::Open);
        assert_eq!(inf_range.ub.value.unwrap(), f64::INF.as_ref().unwrap());
        assert_eq!(inf_range.ub.interval, Interval::Open);
        let neg_inf_range = (range - f64::INFINITY).unwrap();
        assert_eq!(
            neg_inf_range.lb.value.unwrap(),
            f64::NEG_INF.as_ref().unwrap()
        );
        assert_eq!(neg_inf_range.lb.interval, Interval::Open);
        assert_eq!(
            neg_inf_range.ub.value.unwrap(),
            f64::NEG_INF.as_ref().unwrap()
        );
        assert_eq!(neg_inf_range.ub.interval, Interval::Open);
        let inf_range2 = (value_range!([1.0f64, ..]).unwrap() + 1.0f64).unwrap();
        assert_eq!(inf_range2.lb.value.unwrap(), &2.0);
        assert_eq!(inf_range2.lb.interval, Interval::Closed);
        assert_eq!(inf_range2.ub.value.unwrap(), f64::INF.as_ref().unwrap());
        assert_eq!(inf_range2.ub.interval, Interval::Open);
        let neg_inf_range2 = (value_range!([.., 1.0f64]).unwrap() + 1.0f64).unwrap();
        assert_eq!(
            neg_inf_range2.lb.value.unwrap(),
            f64::NEG_INF.as_ref().unwrap()
        );
        assert_eq!(neg_inf_range2.lb.interval, Interval::Open);
        assert_eq!(neg_inf_range2.ub.value.unwrap(), &2.0);
        assert_eq!(neg_inf_range2.ub.interval, Interval::Closed);
    }

    #[test]
    fn test_subtract() {
        let range = value_range!([1.0f64, 2.0f64]).unwrap();
        let added_range = (range - 1.0f64).unwrap();
        assert_eq!(added_range.lb.value.unwrap(), &0.0);
        assert_eq!(added_range.ub.value.unwrap(), &1.0);
        let twice_range = (range - range).unwrap();
        assert_eq!(twice_range.lb.value.unwrap(), &-1.0);
        assert_eq!(twice_range.ub.value.unwrap(), &1.0);
    }

    #[test]
    fn test_multiply() {
        let range = value_range!([1.0f64, 2.0f64]).unwrap();
        let zero_range = (range * 0.0f64).unwrap();
        assert!(zero_range.fixed());
        assert_eq!(zero_range.fixed_value().unwrap().unwrap(), &0.0);
        let twice_range = (range * 2.0f64).unwrap();
        assert_eq!(twice_range.lb.value.unwrap(), &2.0);
        assert_eq!(twice_range.ub.value.unwrap(), &4.0);
        let neg_twice_range = (range * -2.0f64).unwrap();
        assert_eq!(neg_twice_range.lb.value.unwrap(), &-4.0);
        assert_eq!(neg_twice_range.ub.value.unwrap(), &-2.0);
        let square_range = (range * range).unwrap();
        assert_eq!(square_range.lb.value.unwrap(), &1.0);
        assert_eq!(square_range.ub.value.unwrap(), &4.0);
    }

    #[test]
    fn test_divide() {
        let range = value_range!([1.0f64, 2.0f64]).unwrap();
        let half_range = (range / 2.0f64).unwrap();
        assert_eq!(half_range.lb.value.unwrap(), &0.5);
        assert_eq!(half_range.ub.value.unwrap(), &1.0);
        let neg_half_range = (range / -2.0f64).unwrap();
        assert_eq!(neg_half_range.lb.value.unwrap(), &-1.0);
        assert_eq!(neg_half_range.ub.value.unwrap(), &-0.5);
    }

    #[test]
    fn test_intersection() {
        let range = value_range!([1.0f64, 3.0f64]).unwrap();
        let left_half_range1 = value_range!([0.0f64, 2.0f64]).unwrap().intersect(&range);
        assert!(left_half_range1.is_some());
        assert_eq!(left_half_range1.unwrap().lb.value.unwrap(), &1.0);
        assert_eq!(left_half_range1.unwrap().ub.value.unwrap(), &2.0);
        let right_half_range1 = range.intersect(&value_range!([0.0f64, 2.0f64]).unwrap());
        assert!(right_half_range1.is_some());
        assert_eq!(right_half_range1.unwrap().lb.value.unwrap(), &1.0);
        assert_eq!(right_half_range1.unwrap().ub.value.unwrap(), &2.0);
        let left_half_range2 = value_range!([2.0f64, 4.0f64]).unwrap().intersect(&range);
        assert!(left_half_range2.is_some());
        assert_eq!(left_half_range2.unwrap().lb.value.unwrap(), &2.0);
        assert_eq!(left_half_range2.unwrap().ub.value.unwrap(), &3.0);
        let right_half_range2 = range.intersect(&value_range!([2.0f64, 4.0f64]).unwrap());
        assert!(right_half_range2.is_some());
        assert_eq!(right_half_range2.unwrap().lb.value.unwrap(), &2.0);
        assert_eq!(right_half_range2.unwrap().ub.value.unwrap(), &3.0);
        let none_range = range.intersect(&value_range!([4.0f64, 10.0f64]).unwrap());
        assert!(none_range.is_none());
        let inf_range = range.intersect(&value_range!([1.0f64, ..]).unwrap());
        assert!(inf_range.is_some());
        assert_eq!(inf_range.as_ref().unwrap().lb.value.unwrap(), &1.0);
        assert_eq!(inf_range.as_ref().unwrap().lb.interval, Interval::Closed);
        assert_eq!(inf_range.as_ref().unwrap().ub.value.unwrap(), &3.0);
        assert_eq!(inf_range.as_ref().unwrap().ub.interval, Interval::Closed);
        let neg_inf_range = range.intersect(&value_range!([.., 2.0f64]).unwrap());
        assert!(neg_inf_range.is_some());
        assert_eq!(neg_inf_range.as_ref().unwrap().lb.value.unwrap(), &1.0);
        assert_eq!(
            neg_inf_range.as_ref().unwrap().lb.interval,
            Interval::Closed
        );
        assert_eq!(neg_inf_range.as_ref().unwrap().ub.value.unwrap(), &2.0);
        assert_eq!(
            neg_inf_range.as_ref().unwrap().ub.interval,
            Interval::Closed
        );
    }

    #[test]
    fn test_union() {
        let range = value_range!([1.0f64, 3.0f64]).unwrap();
        let union_range1 = range.union(&value_range!([0.0f64, 2.0f64]).unwrap());
        assert!(union_range1.is_some());
        assert_eq!(union_range1.unwrap().lb.value.unwrap(), &0.0);
        assert_eq!(union_range1.unwrap().ub.value.unwrap(), &3.0);
        let union_range2 = range.union(&value_range!([2.0f64, 10.0f64]).unwrap());
        assert!(union_range2.is_some());
        assert_eq!(union_range2.unwrap().lb.value.unwrap(), &1.0);
        assert_eq!(union_range2.unwrap().ub.value.unwrap(), &10.0);
        let union_range3 = range.union(&value_range!([0.0f64, 10.0f64]).unwrap());
        assert!(union_range3.is_some());
        assert_eq!(union_range3.unwrap().lb.value.unwrap(), &0.0);
        assert_eq!(union_range3.unwrap().ub.value.unwrap(), &10.0);
        let none_range = range.union(&value_range!([4.0f64, 10.0f64]).unwrap());
        assert!(none_range.is_none());
        let inf_range = range.union(&value_range!([1.0f64, ..]).unwrap());
        assert!(inf_range.is_some());
        assert_eq!(inf_range.as_ref().unwrap().lb.value.unwrap(), &1.0);
        assert_eq!(inf_range.as_ref().unwrap().lb.interval, Interval::Closed);
        assert_eq!(
            inf_range.as_ref().unwrap().ub.value.unwrap(),
            f64::INF.as_ref().unwrap()
        );
        assert_eq!(inf_range.as_ref().unwrap().ub.interval, Interval::Open);
        let neg_inf_range = range.union(&value_range!([.., 1.0f64]).unwrap());
        assert!(neg_inf_range.is_some());
        assert_eq!(
            neg_inf_range.as_ref().unwrap().lb.value.unwrap(),
            f64::NEG_INF.as_ref().unwrap()
        );
        assert_eq!(neg_inf_range.as_ref().unwrap().lb.interval, Interval::Open);
        assert_eq!(neg_inf_range.as_ref().unwrap().ub.value.unwrap(), &3.0);
        assert_eq!(
            neg_inf_range.as_ref().unwrap().ub.interval,
            Interval::Closed
        );
    }

    #[test]
    fn test_contains() {
        let range = value_range!([1.0f64, 3.0f64]).unwrap();
        assert!(range.contains(&1.0f64));
        assert!(range.contains(&2.0f64));
        assert!(range.contains(&3.0f64));
        assert!(!range.contains(&0.0f64));
        assert!(!range.contains(&4.0f64));

        assert!(range.contains_range(&value_range!([1.0f64, 2.0f64]).unwrap()));
        assert!(range.contains_range(&value_range!([2.0f64, 3.0f64]).unwrap()));
        assert!(range.contains_range(&value_range!([1.0f64, 3.0f64]).unwrap()));
        assert!(!range.contains_range(&value_range!([0.0f64, 1.0f64]).unwrap()));
        assert!(!range.contains_range(&value_range!([0.0f64, 2.0f64]).unwrap()));
        assert!(!range.contains_range(&value_range!([2.0f64, 10.0f64]).unwrap()));
    }
}
