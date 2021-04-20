use super::{Arithmetic, MetaJudgement};

pub trait Constant: Arithmetic {}

macro_rules! constant_template {
    ($($type:ty)*) => ($(
        impl Constant for $type {}
    )*)
}
constant_template! { u8 i8 u16 i16 u32 i32 u64 i64 u128 i128 f32 f64 }

pub struct IsConstant<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> MetaJudgement for IsConstant<T> {
    default const VALUE: bool = false;
}

impl<T: Constant> MetaJudgement for IsConstant<T> {
    const VALUE: bool = true;
}
