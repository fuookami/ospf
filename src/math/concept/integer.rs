use super::{MetaJudgement, Scalar};

pub trait Integer: Scalar + std::ops::Rem<Output = Self> + std::ops::RemAssign {}

macro_rules! integer_template {
    ($($type:ty)*) => ($(
        impl Integer for $type {}
    )*)
}
integer_template! { u8 i8 u16 i16 u32 i32 u64 i64 u128 i128 }

pub trait Natural:
    Integer
    + std::ops::BitAnd<Output = Self>
    + std::ops::BitAndAssign
    + std::ops::BitOr<Output = Self>
    + std::ops::BitOrAssign
    + std::ops::BitXor<Output = Self>
    + std::ops::BitXorAssign
{
}

macro_rules! natural_template {
    ($($type:ty)*) => ($(
        impl Natural for $type {}
    )*)
}
natural_template! { u8 u16 u32 u64 u128 }

pub struct IsInteger<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> MetaJudgement for IsInteger<T> {
    default const VALUE: bool = false;
}

impl<T: Integer> MetaJudgement for IsInteger<T> {
    const VALUE: bool = true;
}

pub struct IsNatural<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> MetaJudgement for IsNatural<T> {
    default const VALUE: bool = false;
}

impl<T: Natural> MetaJudgement for IsNatural<T> {
    const VALUE: bool = true;
}

#[test]
fn test_is_integer() {
    assert!(IsInteger::<i8>::VALUE);
    assert!(IsInteger::<i16>::VALUE);
    assert!(IsInteger::<i32>::VALUE);
    assert!(IsInteger::<i64>::VALUE);
    assert!(IsInteger::<i128>::VALUE);

    assert!(IsInteger::<u8>::VALUE);
    assert!(IsInteger::<u16>::VALUE);
    assert!(IsInteger::<u32>::VALUE);
    assert!(IsInteger::<u64>::VALUE);
    assert!(IsInteger::<u128>::VALUE);
}

#[test]
fn test_is_not_integer() {
    assert!(!IsInteger::<f32>::VALUE);
    assert!(!IsInteger::<f64>::VALUE);

    assert!(!IsInteger::<bool>::VALUE);
}

#[test]
fn test_is_natural() {
    assert!(IsNatural::<u8>::VALUE);
    assert!(IsNatural::<u16>::VALUE);
    assert!(IsNatural::<u32>::VALUE);
    assert!(IsNatural::<u64>::VALUE);
    assert!(IsNatural::<u128>::VALUE);
}

#[test]
fn test_is_not_natural() {
    assert!(!IsNatural::<i8>::VALUE);
    assert!(!IsNatural::<i16>::VALUE);
    assert!(!IsNatural::<i32>::VALUE);
    assert!(!IsNatural::<i64>::VALUE);
    assert!(!IsNatural::<i128>::VALUE);

    assert!(!IsNatural::<f32>::VALUE);
    assert!(!IsNatural::<f64>::VALUE);

    assert!(!IsNatural::<bool>::VALUE);
}
