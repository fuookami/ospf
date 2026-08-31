use std::ops::Mul;

pub trait Cross<Rhs = Self> {
    type Output;

    fn cross(self, rhs: Rhs) -> Self::Output;
}

pub fn cross<T: Cross<U>, U>(lhs: T, rhs: U) -> T::Output {
    lhs.cross(rhs)
}

macro_rules! scalar_cross_template {
    ($($type:ident)*) => ($(
        impl <U> Cross<U> for $type where $type: Mul<U> {
            type Output = <$type as Mul<U>>::Output;

            fn cross(self, rhs: U) -> Self::Output {
                self * rhs
            }
        }

        impl <U> Cross<U> for &$type where for<'a> &'a $type: Mul<U> {
            type Output = <Self as Mul<U>>::Output;

            fn cross(self, rhs: U) -> Self::Output {
                self * rhs
            }
        }
    )*)
}
scalar_cross_template! { bool i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 }

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use crate::algebra::concept::RealNumber;

    use super::*;

    fn test_real<T: RealNumber + Cross<T, Output = T> + Debug>()
    where
        for<'a> &'a T: Cross<&'a T, Output = T>,
    {
        assert_eq!(&(T::ZERO.clone().cross(T::ZERO.clone())), T::ZERO);
        assert_eq!(&(T::ZERO.cross(T::ZERO)), T::ZERO);
        assert_eq!(&cross(T::ZERO.clone(), T::ZERO.clone()), T::ZERO);
        assert_eq!(&cross(T::ZERO, T::ZERO), T::ZERO);

        assert_eq!(&(T::ONE.clone().cross(T::TWO.clone())), T::TWO);
        assert_eq!(&(T::ONE.cross(T::TWO)), T::TWO);
        assert_eq!(&cross(T::ONE.clone(), T::TWO.clone()), T::TWO);
        assert_eq!(&cross(T::ONE, T::TWO), T::TWO);

        assert_eq!(&(T::TWO.clone().cross(T::ONE.clone())), T::TWO);
        assert_eq!(&(T::TWO.cross(T::ONE)), T::TWO);
        assert_eq!(&cross(T::TWO.clone(), T::ONE.clone()), T::TWO);
        assert_eq!(&cross(T::TWO, T::ONE), T::TWO);

        assert_eq!(&(T::TWO.clone().cross(T::FIVE.clone())), T::TEN);
        assert_eq!(&(T::TWO.cross(T::FIVE)), T::TEN);
        assert_eq!(&cross(T::TWO.clone(), T::FIVE.clone()), T::TEN);
        assert_eq!(&cross(T::TWO, T::FIVE), T::TEN);

        assert_eq!(&(T::FIVE.clone().cross(T::TWO.clone())), T::TEN);
        assert_eq!(&(T::FIVE.cross(T::TWO)), T::TEN);
        assert_eq!(&cross(T::FIVE.clone(), T::TWO.clone()), T::TEN);
        assert_eq!(&cross(T::FIVE, T::TWO), T::TEN);
    }

    #[test]
    fn test() {
        test_real::<i8>();
        test_real::<i16>();
        test_real::<i32>();
        test_real::<i64>();
        test_real::<i128>();
        test_real::<u8>();
        test_real::<u16>();
        test_real::<u32>();
        test_real::<u64>();
        test_real::<u128>();
        test_real::<f32>();
        test_real::<f64>();
    }
}
