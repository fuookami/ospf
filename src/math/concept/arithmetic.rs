use super::MetaJudgement;

pub trait Arithmetic:
    std::marker::Sized
    + std::clone::Clone
    + std::ops::Add<Output = Self>
    + std::ops::AddAssign
    + std::ops::Sub<Output = Self>
    + std::ops::SubAssign
    + std::ops::Mul<Output = Self>
    + std::ops::MulAssign
    + std::cmp::PartialEq
    + std::cmp::PartialOrd
    + std::fmt::Display
{
    fn zero() -> Self;
    fn minimum() -> Self;
    fn maximum() -> Self;
    fn positive_minimum() -> Self {
        Self::zero()
    }

    fn decimal_digits() -> Option<u32> {
        Option::None
    }

    fn decimal_precision() -> Self {
        Self::zero()
    }

    fn nan() -> Option<Self> {
        Option::None
    }
    fn is_nan(&self) -> bool {
        false
    }

    fn infinity() -> Option<Self> {
        Option::None
    }

    fn neg_infinity() -> Option<Self> {
        Option::None
    }

    fn is_infinity(&self) -> bool {
        false
    }

    fn is_normal(&self) -> bool {
        true
    }

    fn abs(v: Self) -> Self {
        v.clone()
    }
}

macro_rules! uint_arithmetic_template {
    ($($type:ty)*) => ($(
        impl Arithmetic for $type {
            fn zero() -> $type { 0 }
            fn minimum() -> $type { <$type>::MIN }
            fn maximum() -> $type { <$type>::MAX }
        }
    )*)
}
uint_arithmetic_template! { u8 u16 u32 u64 u128 }

macro_rules! int_arithmetic_template {
    ($($type:ty)*) => ($(
        impl Arithmetic for $type {
            fn zero() -> $type { 0 }
            fn minimum() -> $type { <$type>::MIN }
            fn maximum() -> $type { <$type>::MAX }

            fn abs(v: Self) -> $type { return if v < 0 { -v } else { v.clone() } }
        }
    )*)
}
int_arithmetic_template! { i8 i16 i32 i64 i128 }

macro_rules! floating_point_arithmetic_template {
    ($($type:ty)*) => ($(
        impl Arithmetic for $type {
            fn zero() -> $type { 0. }
            fn minimum() -> $type { <$type>::MIN }
            fn maximum() -> $type { <$type>::MAX }
            fn positive_minimum() -> $type { <$type>::MIN_POSITIVE }
            fn decimal_digits() -> Option<u32> { Option::Some(<$type>::DIGITS) }
            fn decimal_precision() -> Self { (10. as Self).powi(<$type>::DIGITS as i32) }
            fn nan() -> Option<$type> { Option::Some(<$type>::NAN) }
            fn is_nan(&self) -> bool { (*self).is_nan() }
            fn infinity() -> Option<$type> { Option::Some(<$type>::INFINITY) }
            fn neg_infinity() -> Option<$type> { Option::Some(<$type>::NEG_INFINITY) }
            fn is_infinity(&self) -> bool { (*self).is_infinite() }
            fn is_normal(&self) -> bool { (*self).is_normal() }
            fn abs(v: Self) -> $type { return if v < 0. { -v } else { v.clone() } }
        }
    )*)
}
floating_point_arithmetic_template! { f32 f64 }

pub struct IsArithmetic<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> MetaJudgement for IsArithmetic<T> {
    default const VALUE: bool = false;
}

impl<T: Arithmetic> MetaJudgement for IsArithmetic<T> {
    const VALUE: bool = true;
}

#[test]
fn test_is_arithmetic() {
    assert!(IsArithmetic::<i8>::VALUE);
    assert!(IsArithmetic::<i16>::VALUE);
    assert!(IsArithmetic::<i32>::VALUE);
    assert!(IsArithmetic::<i64>::VALUE);
    assert!(IsArithmetic::<i128>::VALUE);

    assert!(IsArithmetic::<u8>::VALUE);
    assert!(IsArithmetic::<u16>::VALUE);
    assert!(IsArithmetic::<u32>::VALUE);
    assert!(IsArithmetic::<u64>::VALUE);
    assert!(IsArithmetic::<u128>::VALUE);

    assert!(IsArithmetic::<f32>::VALUE);
    assert!(IsArithmetic::<f64>::VALUE);
}

#[test]
fn test_is_not_arithmetic() {
    assert!(!IsArithmetic::<bool>::VALUE);
    assert!(!IsArithmetic::<String>::VALUE);
}
