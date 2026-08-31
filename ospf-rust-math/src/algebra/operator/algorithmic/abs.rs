use std::ops::Neg;

use crate::algebra::concept::{Arithmetic, SemiArithmetic, Signed, Unsigned};

pub trait Abs {
    type Output;

    fn abs(self) -> Self::Output;
}

pub fn abs<T: Abs>(value: T) -> T::Output {
    value.abs()
}

macro_rules! unsigned_abs_template {
    ($($type:ident)*) => ($(
        impl Abs for $type {
            type Output = $type;

            fn abs(self) -> $type {
                self
            }
        }

        impl Abs for &$type {
            type Output = $type;

            fn abs(self) -> $type {
                self.clone()
            }
        }
    )*)
}
unsigned_abs_template! { bool u8 u16 u32 u64 u128 usize }

macro_rules! signed_abs_template {
    ($($type:ident)*) => ($(
        impl Abs for $type {
            type Output = $type;

            fn abs(self) -> $type {
                if &self < $type::ZERO { -self } else { self }
            }
        }

        impl Abs for &$type {
            type Output = $type;

            fn abs(self) -> $type {
                if self < $type::ZERO { -self } else { *self }
            }
        }
    )*)
}
signed_abs_template! { i8 i16 i32 i64 i128 isize f32 f64 }

#[cfg(test)]
mod tests {
    use std::fmt::Debug;
    use std::ops::Add;

    use crate::algebra::concept::{Bounded, FloatingNumber, IntegerNumber, UIntegerNumber};

    use super::*;

    fn test_bounded<T: Arithmetic + Bounded + Abs<Output = T> + Debug>()
    where
        for<'a> &'a T: Abs<Output = T>,
    {
        assert_eq!(&(T::ZERO.clone().abs()), T::ZERO);
        assert_eq!(&(T::ZERO.abs()), T::ZERO);
        assert_eq!(&abs(T::ZERO.clone()), T::ZERO);
        assert_eq!(&abs(T::ZERO), T::ZERO);

        assert_eq!(&(T::POSITIVE_MINIMUM.clone()).abs(), T::POSITIVE_MINIMUM);
        assert_eq!(&(T::POSITIVE_MINIMUM.abs()), T::POSITIVE_MINIMUM);
        assert_eq!(&abs(T::POSITIVE_MINIMUM.clone()), T::POSITIVE_MINIMUM);
        assert_eq!(&abs(T::POSITIVE_MINIMUM), T::POSITIVE_MINIMUM);

        assert_eq!(&T::MAXIMUM.as_ref().map(|x| x.clone().abs()), T::MAXIMUM);
        assert_eq!(&T::MAXIMUM.as_ref().map(|x| x.abs()), T::MAXIMUM);
        assert_eq!(&T::MAXIMUM.as_ref().map(|x| abs(x.clone())), T::MAXIMUM);
        assert_eq!(&T::MAXIMUM.as_ref().map(|x| abs(x)), T::MAXIMUM);
    }

    fn test_signed<T: Arithmetic + Bounded + Signed + Abs<Output = T> + Debug>()
    where
        for<'a> &'a T: Neg<Output = T> + Abs<Output = T>,
    {
        test_bounded::<T>();

        assert_eq!(&(-T::POSITIVE_MINIMUM).abs(), T::POSITIVE_MINIMUM);
        assert_eq!(&(&-T::POSITIVE_MINIMUM).abs(), T::POSITIVE_MINIMUM);
        assert_eq!(&abs(-T::POSITIVE_MINIMUM), T::POSITIVE_MINIMUM);
        assert_eq!(&abs(&-T::POSITIVE_MINIMUM), T::POSITIVE_MINIMUM);
    }

    fn test_int<T: IntegerNumber + Abs<Output = T> + Debug>()
    where
        for<'a> &'a T: Add<Output = T> + Neg<Output = T> + Abs<Output = T>,
    {
        test_signed::<T>();

        assert_eq!(&T::MINIMUM.as_ref().map(|x| (x + T::ONE).abs()), T::MAXIMUM);
        assert_eq!(
            &T::MINIMUM.as_ref().map(|x| (&(x + T::ONE)).abs()),
            T::MAXIMUM
        );
        assert_eq!(&T::MINIMUM.as_ref().map(|x| abs(x + T::ONE)), T::MAXIMUM);
        assert_eq!(&T::MINIMUM.as_ref().map(|x| abs(&(x + T::ONE))), T::MAXIMUM);
    }

    fn test_uint<T: UIntegerNumber + Abs<Output = T> + Debug>()
    where
        for<'a> &'a T: Abs<Output = T>,
    {
        test_bounded::<T>();
    }

    fn test_flt<T: FloatingNumber + Abs<Output = T> + Debug>()
    where
        for<'a> &'a T: Neg<Output = T> + Abs<Output = T>,
    {
        test_signed::<T>();

        assert_eq!(&T::MINIMUM.as_ref().map(|x| x.clone().abs()), T::MAXIMUM);
        assert_eq!(&T::MINIMUM.as_ref().map(|x| x.abs()), T::MAXIMUM);
        assert_eq!(&T::MINIMUM.as_ref().map(|x| abs(x.clone())), T::MAXIMUM);
        assert_eq!(&T::MINIMUM.as_ref().map(|x| abs(x)), T::MAXIMUM);
    }

    #[test]
    fn test() {
        test_int::<i8>();
        test_int::<i16>();
        test_int::<i32>();
        test_int::<i64>();
        test_int::<i128>();
        test_uint::<u8>();
        test_uint::<u16>();
        test_uint::<u32>();
        test_uint::<u64>();
        test_uint::<u128>();
        test_flt::<f32>();
        test_flt::<f64>();
    }
}
