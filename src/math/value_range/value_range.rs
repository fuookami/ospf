use super::extra_integer::ExInteger;
use super::interval_type::{Closed, IntervalType};
use crate::math::comparison_operator::equal::{EqualInteger, EqualOp, EqualReal};
use crate::math::{Arithmetic, Integer, IsInteger, Real};
use crate::meta_programming::{ConditionalType, MetaJudgement};
use std::fmt;
use std::fmt::Display;
use std::marker::PhantomData;

pub trait ValueRange: Display + Clone {
    type ValueType: Arithmetic;
    type EqualOp: EqualOp<ValueType = Self::ValueType>;
    type LeftIntervalType: IntervalType = Closed;
    type RightIntervalType: IntervalType = Closed;

    fn new_empty() -> Self;
    fn new_range(lhs: &Self::ValueType, rhs: &Self::ValueType) -> Self;

    fn range(&self) -> &Option<(Self::ValueType, Self::ValueType)>;

    fn empty(&self) -> bool {
        self.range().is_none()
    }

    fn fixed(&self) -> bool {
        let eq_op = Self::EqualOp::new();

        match self.range() {
            Option::Some((lower_bound, upper_bound)) => {
                eq_op.exec(lower_bound.clone(), upper_bound.clone())
            }
            Option::None => false,
        }
    }

    fn lower_bound(&self) -> Option<&Self::ValueType> {
        match self.range() {
            Option::Some((lower_bound, _)) => Option::Some(lower_bound),
            Option::None => Option::None,
        }
    }

    fn upper_bound(&self) -> Option<&Self::ValueType> {
        match self.range() {
            Option::Some((_, upper_bound)) => Option::Some(upper_bound),
            Option::None => Option::None,
        }
    }

    fn fixed_value(&self) -> Option<&Self::ValueType> {
        match self.fixed() {
            true => self.lower_bound(),
            false => Option::None,
        }
    }

    fn _fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.empty() {
            write!(f, "empty")
        } else {
            write!(
                f,
                "{}{}, {}{}",
                Self::LeftIntervalType::left_bracket(),
                self.lower_bound().unwrap(),
                self.upper_bound().unwrap(),
                Self::RightIntervalType::right_bracket()
            )
        }
    }
}

#[derive(Clone)]
pub struct ValueRangeInteger<
    T: Integer,
    LeftIntervalType: IntervalType = Closed,
    RightIntervalType: IntervalType = Closed,
> {
    _range: Option<(ExInteger<T>, ExInteger<T>)>,
    _marker: PhantomData<(LeftIntervalType, RightIntervalType)>,
}

#[derive(Clone)]
pub struct ValueRangeReal<
    T: Real,
    LeftIntervalType: IntervalType = Closed,
    RightIntervalType: IntervalType = Closed,
> {
    _range: Option<(T, T)>,
    _marker: PhantomData<(LeftIntervalType, RightIntervalType)>,
}

impl<T: Integer, LeftIntervalType: IntervalType, RightIntervalType: IntervalType> Display
    for ValueRangeInteger<T, LeftIntervalType, RightIntervalType>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self._fmt(f)
    }
}

impl<T: Integer, LeftIntervalType: IntervalType, RightIntervalType: IntervalType> ValueRange
    for ValueRangeInteger<T, LeftIntervalType, RightIntervalType>
{
    type ValueType = ExInteger<T>;
    type EqualOp = EqualInteger<ExInteger<T>>;
    type LeftIntervalType = LeftIntervalType;
    type RightIntervalType = RightIntervalType;

    fn new_empty() -> Self {
        Self {
            _range: Option::None,
            _marker: PhantomData::<(LeftIntervalType, RightIntervalType)> {},
        }
    }

    fn new_range(lhs: &Self::ValueType, rhs: &Self::ValueType) -> Self {
        match LeftIntervalType::le(lhs, rhs) && RightIntervalType::ge(rhs, lhs) {
            true => Self {
                _range: Option::Some((lhs.clone(), rhs.clone())),
                _marker: PhantomData::<(LeftIntervalType, RightIntervalType)> {},
            },
            false => Self::new_empty(),
        }
    }

    fn range(&self) -> &Option<(Self::ValueType, Self::ValueType)> {
        &self._range
    }
}

impl<T: Real, LeftIntervalType: IntervalType, RightIntervalType: IntervalType> Display
    for ValueRangeReal<T, LeftIntervalType, RightIntervalType>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self._fmt(f)
    }
}

impl<T: Real, LeftIntervalType: IntervalType, RightIntervalType: IntervalType> ValueRange
    for ValueRangeReal<T, LeftIntervalType, RightIntervalType>
{
    type ValueType = T;
    type EqualOp = EqualReal<T>;
    type LeftIntervalType = LeftIntervalType;
    type RightIntervalType = RightIntervalType;

    fn new_empty() -> Self {
        Self {
            _range: Option::None,
            _marker: PhantomData::<(LeftIntervalType, RightIntervalType)> {},
        }
    }

    fn new_range(lhs: &Self::ValueType, rhs: &Self::ValueType) -> Self {
        match LeftIntervalType::le(lhs, rhs) && RightIntervalType::ge(rhs, lhs) {
            true => Self {
                _range: Option::Some((lhs.clone(), rhs.clone())),
                _marker: PhantomData::<(LeftIntervalType, RightIntervalType)> {},
            },
            false => Self::new_empty(),
        }
    }

    fn range(&self) -> &Option<(Self::ValueType, Self::ValueType)> {
        &self._range
    }
}

pub type ValueRangeType<T, LeftIntervalType = Closed, RightIntervalType = Closed> = ConditionalType<
    ValueRangeInteger<T, LeftIntervalType, RightIntervalType>,
    ValueRangeReal<T, LeftIntervalType, RightIntervalType>,
    { IsInteger::<T>::VALUE },
>;

#[test]
fn test_value_range1() {
    type ValueRangeI32 = ValueRangeType<i32>;
    let range = ValueRangeI32::new_empty();
    assert!(range._range.is_none());
}

#[test]
fn test_value_range2() {
    type ValueRangeF32 = ValueRangeType<f32>;
    let range = ValueRangeF32::new_empty();
    assert!(range._range.is_none());
}
