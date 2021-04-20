use super::{Arithmetic, MetaJudgement};

pub trait Scalar: Arithmetic + std::ops::Div<Output = Self> + std::ops::DivAssign {}

macro_rules! scale_template {
    ($($type:ty)*) => ($(
        impl Scalar for $type {}
    )*)
}
scale_template! { u8 i8 u16 i16 u32 i32 u64 i64 u128 i128 f32 f64 }

pub struct IsScalar<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> MetaJudgement for IsScalar<T> {
    default const VALUE: bool = false;
}

impl<T: Scalar> MetaJudgement for IsScalar<T> {
    const VALUE: bool = true;
}

#[test]
fn test_is_scalar() {
    assert!(IsScalar::<i8>::VALUE);
    assert!(IsScalar::<i16>::VALUE);
    assert!(IsScalar::<i32>::VALUE);
    assert!(IsScalar::<i64>::VALUE);
    assert!(IsScalar::<i128>::VALUE);

    assert!(IsScalar::<u8>::VALUE);
    assert!(IsScalar::<u16>::VALUE);
    assert!(IsScalar::<u32>::VALUE);
    assert!(IsScalar::<u64>::VALUE);
    assert!(IsScalar::<u128>::VALUE);

    assert!(IsScalar::<f32>::VALUE);
    assert!(IsScalar::<f64>::VALUE);
}

#[test]
fn test_is_not_scalar() {
    assert!(!IsScalar::<bool>::VALUE);
    assert!(!IsScalar::<String>::VALUE);
}
