use super::{IsScalar, MetaJudgement, Scalar};

pub trait Signed: Scalar + std::ops::Neg<Output = Self> {}

macro_rules! signed_template {
    ($($type:ty)*) => ($(
        impl Signed for $type {}
    )*)
}
signed_template! { i8 i16 i32 i64 i128 f32 f64 }

pub struct IsSigned<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> MetaJudgement for IsSigned<T> {
    default const VALUE: bool = false;
}

impl<T: Signed> MetaJudgement for IsSigned<T> {
    const VALUE: bool = true;
}
pub struct IsUnsigned<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> MetaJudgement for IsUnsigned<T> {
    default const VALUE: bool = IsScalar::<T>::VALUE && !IsSigned::<T>::VALUE;
}

#[test]
fn test_is_signed() {
    assert!(IsSigned::<i8>::VALUE);
    assert!(IsSigned::<i16>::VALUE);
    assert!(IsSigned::<i32>::VALUE);
    assert!(IsSigned::<i64>::VALUE);
    assert!(IsSigned::<i128>::VALUE);

    assert!(IsSigned::<f32>::VALUE);
    assert!(IsSigned::<f64>::VALUE);
}

#[test]
fn test_is_not_signed() {
    assert!(!IsSigned::<u8>::VALUE);
    assert!(!IsSigned::<u16>::VALUE);
    assert!(!IsSigned::<u32>::VALUE);
    assert!(!IsSigned::<u64>::VALUE);
    assert!(!IsSigned::<u128>::VALUE);

    assert!(!IsSigned::<bool>::VALUE);
}

#[test]
fn test_is_unsigned() {
    assert!(IsUnsigned::<u8>::VALUE);
    assert!(IsUnsigned::<u16>::VALUE);
    assert!(IsUnsigned::<u32>::VALUE);
    assert!(IsUnsigned::<u64>::VALUE);
    assert!(IsUnsigned::<u128>::VALUE);
}

#[test]
fn test_is_not_unsigned() {
    assert!(!IsUnsigned::<i8>::VALUE);
    assert!(!IsUnsigned::<i16>::VALUE);
    assert!(!IsUnsigned::<i32>::VALUE);
    assert!(!IsUnsigned::<i64>::VALUE);
    assert!(!IsUnsigned::<i128>::VALUE);

    assert!(!IsUnsigned::<f32>::VALUE);
    assert!(!IsUnsigned::<f64>::VALUE);

    assert!(!IsUnsigned::<bool>::VALUE);
}
