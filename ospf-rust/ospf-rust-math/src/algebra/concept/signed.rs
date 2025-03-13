use crate::algebra::*;

pub trait Signed: Neg<Output = Self> {}

pub trait Unsigned {}

macro_rules! signed_template {
    ($($type:ty)*) => ($(
        impl Signed for $type { }
    )*)
}
signed_template! { i8 i16 i32 i64 i128 IntX f32 f64 Decimal }

macro_rules! unsigned_template {
    ($($type:ty)*) => ($(
        impl Unsigned for $type { }
    )*)
}
unsigned_template! { u8 u16 u32 u64 u128 UIntX }
