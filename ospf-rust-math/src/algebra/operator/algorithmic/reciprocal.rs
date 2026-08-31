use std::ops::Div;

use crate::algebra::concept::{Arithmetic, RealNumber};

pub trait Reciprocal {
    type Output;

    fn reciprocal(self) -> Option<Self::Output>;
}

pub fn reciprocal<T: Reciprocal>(value: T) -> Option<T::Output> {
    value.reciprocal()
}

macro_rules! int_reciprocal_template {
    ($($type:ident)*) => ($(
        impl Reciprocal for $type {
            type Output = $type;

            fn reciprocal(self) -> Option<$type> {
                if self == 0 {
                    <$type as RealNumber>::NAN.clone()
                } else if self == 1 {
                    Some(1)
                } else if self == -1 {
                    Some(-1)
                } else {
                    Some(0)
                }
            }
        }

        impl Reciprocal for &$type {
            type Output = $type;

            fn reciprocal(self) -> Option<$type> {
                if self == &0 {
                    <$type as RealNumber>::NAN.clone()
                } else if self == &1 {
                    Some(1)
                } else if self == &-1 {
                    Some(-1)
                } else {
                    Some(0)
                }
            }
        }
    )*)
}
int_reciprocal_template! { i8 i16 i32 i64 i128 isize }

macro_rules! uint_reciprocal_template {
    ($($type:ident)*) => ($(
        impl Reciprocal for $type {
            type Output = $type;

            fn reciprocal(self) -> Option<$type> {
                if self == 0 {
                    <$type as RealNumber>::NAN.clone()
                } else if self == 1 {
                    Some(1)
                } else {
                    Some(0)
                }
            }
        }

        impl Reciprocal for &$type {
            type Output = $type;

            fn reciprocal(self) -> Option<$type> {
                if self == &0 {
                    <$type as RealNumber>::NAN.clone()
                } else if self == &1 {
                    Some(1)
                } else {
                    Some(0)
                }
            }
        }
    )*)
}
uint_reciprocal_template! { u8 u16 u32 u64 u128 usize }

macro_rules! flt_reciprocal_template {
    ($($type:ident)*) => ($(
        impl Reciprocal for $type {
            type Output = $type;

            fn reciprocal(self) -> Option<$type> {
                if self == 0. {
                    return <$type as RealNumber>::NAN.clone();
                } else {
                    return Some(1. / self);
                }
            }
        }

        impl Reciprocal for &$type {
            type Output = $type;

            fn reciprocal(self) -> Option<$type> {
                if self == &0. {
                    return <$type as RealNumber>::NAN.clone();
                } else {
                    return Some(1. / self);
                }
            }
        }
    )*)
}
flt_reciprocal_template! { f32 f64 }

#[cfg(test)]
mod tests {
    use std::fmt::Debug;
    use std::ops::Neg;

    use crate::algebra::concept::{FloatingNumber, Integer, IntegerNumber, UIntegerNumber};

    use super::*;

    fn test_integer<T: Integer + Reciprocal<Output = T> + Debug>()
    where
        for<'a> &'a T: Reciprocal<Output = T>,
    {
        assert_eq!(&(T::ZERO.clone().reciprocal()), T::NAN);
        assert_eq!(&(T::ZERO.reciprocal()), T::NAN);
        assert_eq!(&reciprocal(T::ZERO.clone()), T::NAN);
        assert_eq!(&reciprocal(T::ZERO), T::NAN);

        assert_eq!(T::ONE.clone().reciprocal(), Some(T::ONE.clone()));
        assert_eq!(T::ONE.reciprocal(), Some(T::ONE.clone()));
        assert_eq!(reciprocal(T::ONE.clone()), Some(T::ONE.clone()));
        assert_eq!(reciprocal(T::ONE), Some(T::ONE.clone()));

        assert_eq!(T::TWO.clone().reciprocal(), Some(T::ZERO.clone()));
        assert_eq!(T::TWO.reciprocal(), Some(T::ZERO.clone()));
        assert_eq!(reciprocal(T::TWO.clone()), Some(T::ZERO.clone()));
        assert_eq!(reciprocal(T::TWO), Some(T::ZERO.clone()));
    }

    fn test_int<T: IntegerNumber + Reciprocal<Output = T> + Debug>()
    where
        for<'a> &'a T: Neg<Output = T> + Reciprocal<Output = T>,
    {
        test_integer::<T>();

        assert_eq!((-T::ONE).reciprocal(), Some(-T::ONE.clone()));
        assert_eq!((&-T::ONE).reciprocal(), Some(-T::ONE.clone()));
        assert_eq!(reciprocal(-T::ONE), Some(-T::ONE.clone()));
        assert_eq!(reciprocal(&-T::ONE), Some(-T::ONE.clone()));
    }

    fn test_uint<T: UIntegerNumber + Reciprocal<Output = T> + Debug>()
    where
        for<'a> &'a T: Reciprocal<Output = T>,
    {
        test_integer::<T>();
    }

    fn test_flt<T: FloatingNumber + Div<Output = T> + Reciprocal<Output = T> + Debug>()
    where
        for<'a> &'a T: Div<Output = T> + Neg<Output = T> + Reciprocal<Output = T>,
    {
        assert!(T::ZERO.reciprocal().unwrap().is_nan());
        assert!((&T::ZERO).reciprocal().unwrap().is_nan());
        assert!(reciprocal(T::ZERO.clone()).unwrap().is_nan());
        assert!(reciprocal(T::ZERO).unwrap().is_nan());

        assert_eq!(T::ONE.clone().reciprocal(), Some(T::ONE.clone()));
        assert_eq!(T::ONE.reciprocal(), Some(T::ONE.clone()));
        assert_eq!(reciprocal(T::ONE.clone()), Some(T::ONE.clone()));
        assert_eq!(reciprocal(T::ONE), Some(T::ONE.clone()));

        assert_eq!((-T::ONE).reciprocal(), Some(-T::ONE.clone()));
        assert_eq!((&-T::ONE).reciprocal(), Some(-T::ONE.clone()));
        assert_eq!(reciprocal(-T::ONE), Some(-T::ONE.clone()));
        assert_eq!(reciprocal(&-T::ONE), Some(-T::ONE.clone()));

        assert_eq!(T::TWO.clone().reciprocal(), Some(T::ONE / T::TWO));
        assert_eq!(T::TWO.reciprocal(), Some(T::ONE / T::TWO));
        assert_eq!(reciprocal(T::TWO.clone()), Some(T::ONE / T::TWO));
        assert_eq!(reciprocal(T::TWO), Some(T::ONE / T::TWO));
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
