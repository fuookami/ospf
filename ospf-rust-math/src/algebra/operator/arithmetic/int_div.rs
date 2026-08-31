use std::ops::Div;

use crate::algebra::concept::FloatingNumber;

pub trait IntDiv<Rhs = Self> {
    type Output;

    fn int_div(self, rhs: Rhs) -> Self::Output;
}

pub fn int_div<T: IntDiv>(lhs: T, rhs: T) -> T::Output {
    lhs.int_div(rhs)
}

macro_rules! int_int_div_template {
    ($($type:ident)*) => ($(
        impl<U> IntDiv<U> for $type where $type: Div<U> {
            type Output = <$type as Div<U>>::Output;

            default fn int_div(self, rhs: U) -> Self::Output {
                self / rhs
            }
        }

        impl<U, V> IntDiv<U> for $type where $type: Div<U, Output=V>, V: FloatingNumber {
            fn int_div(self, rhs: U) -> Self::Output {
                (self / rhs).floor()
            }
        }

        impl<U> IntDiv<U> for &$type where for<'a> &'a $type: Div<U> {
            type Output = <Self as Div<U>>::Output;

            default fn int_div(self, rhs: U) -> Self::Output {
                self / rhs
            }
        }

        impl<U, V> IntDiv<U> for &$type where for<'a> &'a $type: Div<U, Output=V>, V: FloatingNumber {
            fn int_div(self, rhs: U) -> Self::Output {
                (self / rhs).floor()
            }
        }
    )*)
}
int_int_div_template! { bool i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }

macro_rules! floating_int_div_template {
    ($($type:ident)*) => ($(
        impl<U, V> IntDiv<U> for $type where $type: Div<U, Output=V>, V: FloatingNumber {
            type Output = <$type as Div<U>>::Output;

            fn int_div(self, rhs: U) -> Self::Output {
                (self / rhs).floor()
            }
        }

        impl<U, V> IntDiv<U> for &$type where for<'a> &'a $type: Div<U, Output=V>, V: FloatingNumber {
            type Output = <Self as Div<U>>::Output;

            fn int_div(self, rhs: U) -> Self::Output {
                (self / rhs).floor()
            }
        }
    )*)
}
floating_int_div_template! { f32 f64 }

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use crate::algebra::concept::RealNumber;

    use super::*;

    fn test_real<T: RealNumber + IntDiv<Output = T> + Debug>()
    where
        for<'a> &'a T: IntDiv<&'a T, Output = T>,
    {
        assert_eq!(&(T::ONE.clone().int_div(T::TWO.clone())), T::ZERO);
        assert_eq!(&(T::ONE.int_div(T::TWO)), T::ZERO);
        assert_eq!(&int_div(T::ONE.clone(), T::TWO.clone()), T::ZERO);
        assert_eq!(&int_div(T::ONE, T::TWO), T::ZERO);

        assert_eq!(&(T::TWO.clone().int_div(T::ONE.clone())), T::TWO);
        assert_eq!(&(T::TWO.int_div(T::ONE)), T::TWO);
        assert_eq!(&int_div(T::TWO.clone(), T::ONE.clone()), T::TWO);
        assert_eq!(&int_div(T::TWO, T::ONE), T::TWO);

        assert_eq!(&(T::FIVE.clone().int_div(T::TWO.clone())), T::TWO);
        assert_eq!(&(T::FIVE.int_div(T::TWO)), T::TWO);
        assert_eq!(&int_div(T::FIVE.clone(), T::TWO.clone()), T::TWO);
        assert_eq!(&int_div(T::FIVE, T::TWO), T::TWO);
    }

    #[test]
    fn test() {
        test_real::<u8>();
        test_real::<u16>();
        test_real::<u32>();
        test_real::<u64>();
        test_real::<u128>();
        test_real::<i8>();
        test_real::<i16>();
        test_real::<i32>();
        test_real::<i64>();
        test_real::<i128>();
        test_real::<f32>();
        test_real::<f64>();
    }
}
