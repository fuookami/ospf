use super::{Arithmetic, Precision};

pub trait Bounded: 'static + Sized {
    const MINIMUM: &'static Option<Self>;
    const MAXIMUM: &'static Option<Self>;
    const POSITIVE_MINIMUM: &'static Self;
}

impl Bounded for bool {
    const MINIMUM: &'static Option<bool> = &Some(false);
    const MAXIMUM: &'static Option<bool> = &Some(true);
    const POSITIVE_MINIMUM: &'static Self = &true;
}

macro_rules! int_bound_template {
    ($($type:ident)*) => ($(
        impl Bounded for $type {
            const MINIMUM: &'static Option<Self> = &Some(<$type>::MIN);
            const MAXIMUM: &'static Option<Self> = &Some(<$type>::MAX);
            const POSITIVE_MINIMUM: &'static Self = Self::ONE;
        }
    )*)
}
int_bound_template! { i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }

macro_rules! floating_bound_template {
    ($($type:ident)*) => ($(
        impl Bounded for $type {
            const MINIMUM: &'static Option<Self> = &Some(<$type>::MIN);
            const MAXIMUM: &'static Option<Self> = &Some(<$type>::MAX);
            const POSITIVE_MINIMUM: &'static Self = &Self::EPSILON;
        }
    )*)
}
floating_bound_template! { f32 f64 }
