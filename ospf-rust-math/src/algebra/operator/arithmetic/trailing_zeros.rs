use std::borrow::Borrow;
use std::ops::{BitOr, ShrAssign};

use crate::algebra::concept::{Arithmetic, SemiArithmetic};

pub trait TrailingZeros {
    fn trailing_zeros(self) -> usize;
}

default impl<T: Arithmetic + ShrAssign<usize>> TrailingZeros for T
where
    for<'a> &'a T: BitOr<&'a T, Output = T>,
{
    fn trailing_zeros(mut self) -> usize {
        if &self == T::ZERO {
            return std::mem::size_of::<T>();
        }

        let mut counter = 0;
        while &(&self | T::ONE) == T::ZERO {
            counter += 1;
            self >>= 1;
        }
        counter
    }
}

macro_rules! int_trailing_zeros_impl {
    ($($type:ident)*) => ($(
        impl TrailingZeros for $type {
            fn trailing_zeros(self) -> usize {
                <$type>::trailing_zeros(self) as usize
            }
        }

        impl TrailingZeros for &$type {
            fn trailing_zeros(self) -> usize {
                <$type>::trailing_zeros(*self) as usize
            }
        }
    )*);
}
int_trailing_zeros_impl! { i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }

#[cfg(test)]
mod tests {
    use std::fmt::Debug;
    use std::ops::Mul;

    use crate::algebra::concept::Integer;

    use super::*;

    fn test_int<T: Integer + TrailingZeros + Debug>()
    where
        for<'a> &'a T: Mul<Output = T>,
    {
        assert_eq!(
            T::ZERO.clone().trailing_zeros(),
            std::mem::size_of::<T>() * 8
        );
        assert_eq!(T::ONE.clone().trailing_zeros(), 0);
        assert_eq!(T::TWO.clone().trailing_zeros(), 1);
        assert_eq!(T::THREE.clone().trailing_zeros(), 0);
        assert_eq!(T::FIVE.clone().trailing_zeros(), 0);
        assert_eq!(T::TEN.clone().trailing_zeros(), 1);
        assert_eq!((T::TWO * T::TEN).trailing_zeros(), 2);
    }

    #[test]
    fn test() {
        test_int::<i8>();
        test_int::<i16>();
        test_int::<i32>();
        test_int::<i64>();
        test_int::<i128>();
        test_int::<u8>();
        test_int::<u16>();
        test_int::<u32>();
        test_int::<u64>();
        test_int::<u128>();
    }
}
