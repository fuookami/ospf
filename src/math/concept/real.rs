use super::{MetaJudgement, Scalar, Signed};

pub trait Real: Scalar + Signed {}

macro_rules! real_template {
    ($($type:ty)*) => ($(
        impl Real for $type {}
    )*)
}
real_template! { f32 f64 }

pub struct IsReal<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> MetaJudgement for IsReal<T> {
    default const VALUE: bool = false;
}

impl<T: Real> MetaJudgement for IsReal<T> {
    const VALUE: bool = true;
}

#[test]
fn test_is_real() {
    assert!(IsReal::<f32>::VALUE);
    assert!(IsReal::<f64>::VALUE);
}

#[test]
fn test_is_not_real() {
    assert!(!IsReal::<u8>::VALUE);
    assert!(!IsReal::<u16>::VALUE);
    assert!(!IsReal::<u32>::VALUE);
    assert!(!IsReal::<u64>::VALUE);
    assert!(!IsReal::<u128>::VALUE);

    assert!(!IsReal::<i8>::VALUE);
    assert!(!IsReal::<i16>::VALUE);
    assert!(!IsReal::<i32>::VALUE);
    assert!(!IsReal::<i64>::VALUE);
    assert!(!IsReal::<i128>::VALUE);

    assert!(!IsReal::<bool>::VALUE);
}
