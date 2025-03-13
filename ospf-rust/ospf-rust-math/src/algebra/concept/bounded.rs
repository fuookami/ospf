use crate::algebra::*;

pub trait Bounded: Sized {
    const MINIMUM: Option<Self>;
    const MAXIMUM: Option<Self>;
    const POSITIVE_MINIMUM: Self;
}

macro_rules! int_bound_template {
    ($($type:ty)*) => ($(
        impl Bounded for $type {
            const MINIMUM: Option<Self> = Some(<$type>::MIN);
            const MAXIMUM: Option<Self> = Some(<$type>::MAX);
            const POSITIVE_MINIMUM: Self = Self::ONE;
        }
    )*)
}
int_bound_template! { i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 }

macro_rules! floating_bound_template {
    ($($type:ty)*) => ($(
        impl Bounded for $type {
            const MINIMUM: Option<Self> = Some(<$type>::MIN);
            const MAXIMUM: Option<Self> = Some(<$type>::MAX);
            const POSITIVE_MINIMUM: Self = Self::EPSILON;
        }
    )*)
}
floating_bound_template! { f32 f64 Decimal }

impl Bounded for IntX {
    const MINIMUM: Option<Self> = None;
    const MAXIMUM: Option<Self> = None;
    const POSITIVE_MINIMUM: Self = Self::ONE;
}

impl Bounded for UIntX {
    const MINIMUM: Option<Self> = Some(Self::ZERO);
    const MAXIMUM: Option<Self> = None;
    const POSITIVE_MINIMUM: Self = Self::ONE;
}
