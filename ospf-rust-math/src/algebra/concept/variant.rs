use crate::SemiArithmetic;

pub trait Variant: SemiArithmetic {
    type ValueType: SemiArithmetic;

    fn value(&self) -> Option<&Self::ValueType> {
        None
    }
    fn equiv(&self, rhs: &Self::ValueType) -> bool;
}

pub fn equiv<Lhs: Variant>(lhs: &Lhs, rhs: &Lhs::ValueType) -> bool {
    lhs.equiv(rhs)
}

pub trait Invariant: SemiArithmetic {
    type ValueType: SemiArithmetic;

    fn from(value: &Self::ValueType) -> Self;
    fn value(&self) -> &Self::ValueType;
}

macro_rules! invariant_template {
    ($($type:ident)*) => ($(
        impl Invariant for $type {
            type ValueType = Self;

            fn from(value: &Self::ValueType) -> Self {
                value.clone()
            }

            fn value(&self) -> &Self::ValueType {
                &self
            }
        }
    )*)
}
invariant_template! { bool i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 }
