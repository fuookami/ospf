use super::*;
use crate::algebra::*;

pub trait Variant: Arithmetic {
    type ValueType: Arithmetic;

    fn value(&self) -> Option<&Self::ValueType> {
        None
    }

    fn equiv(&self, rhs: &Self::ValueType) -> bool;
}

pub fn equiv<Lhs: Variant>(lhs: &Lhs, rhs: &Lhs::ValueType) -> bool {
    lhs.equiv(rhs)
}

pub trait Invariant: Arithmetic {
    type ValueType: Arithmetic;

    fn value(&self) -> &Self::ValueType;
}

macro_rules! invariant_template {
    ($($type:ty)*) => ($(
        impl Invariant for $type {
            type ValueType = Self;

            fn value(&self) -> &Self::ValueType {
                &self
            }
        }
    )*)
}
invariant_template! { i8 i16 i32 i64 i128 IntX u8 u16 u32 u64 u128 UIntX f32 f64 Decimal }
