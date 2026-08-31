use std::ops::Neg;

pub trait Signed {}

impl<T> Signed for T where T: Neg<Output = T> {}

pub trait Unsigned {}

macro_rules! signed_template {
    ($($type:ident)*) => ($(
        impl Signed for $type { }
    )*)
}
signed_template! { i8 i16 i32 i64 i128 isize f32 f64 }

macro_rules! unsigned_template {
    ($($type:ident)*) => ($(
        impl Unsigned for $type { }
    )*)
}
unsigned_template! { bool u8 u16 u32 u64 u128 usize }
