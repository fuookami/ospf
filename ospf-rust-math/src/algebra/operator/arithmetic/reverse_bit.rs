use std::ops::{BitOr, ShrAssign};
use std::intrinsics::bitreverse;

use crate::Arithmetic;

pub trait ReverseBit {
    type Output;

    fn reverse_bit(self) -> Self::Output;
}

macro_rules! int_trailing_zeros_impl {
    ($($type:ident)*) => ($(
        impl ReverseBit for $type {
            type Output = $type;

            fn reverse_bit(self) -> $type {
                self.reverse_bits()
            }
        }

        impl ReverseBit for &$type {
            type Output = $type;

            fn reverse_bit(self) -> $type {
                (*self).reverse_bit()
            }
        }
    )*);
}
int_trailing_zeros_impl! { i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }
