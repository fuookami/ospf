use std::ops::{Div, Mul};

use paste::paste;

pub use crate::algebra::ordinary::gcd::*;

pub trait GcdLcm: Sized {
    type Output;

    fn gcd(self, rhs: Self) -> Self::Output;
    fn gcd_list(values: &[Self]) -> Self::Output;

    fn lcm(self, rhs: Self) -> Self::Output;
    fn lcm_list(values: &[Self]) -> Self::Output;

    fn gcd_lcm(self, rhs: Self) -> (Self::Output, Self::Output);
    fn gcd_lcm_list(values: &[Self]) -> (Self::Output, Self::Output);
}

pub fn gcd<T: GcdLcm>(lhs: T, rhs: T) -> T::Output {
    lhs.gcd(rhs)
}

pub fn gcd_list<T: GcdLcm>(values: &[T]) -> T::Output {
    T::gcd_list(values)
}

pub fn lcm<T: GcdLcm>(lhs: T, rhs: T) -> T::Output {
    lhs.lcm(rhs)
}

pub fn lcm_list<T: GcdLcm>(values: &[T]) -> T::Output {
    T::lcm_list(values)
}

pub fn gcd_lcm<T: GcdLcm>(lhs: T, rhs: T) -> (T::Output, T::Output) {
    lhs.gcd_lcm(rhs)
}

pub fn gcd_lcm_list<T: GcdLcm>(values: &[T]) -> (T::Output, T::Output) {
    T::gcd_lcm_list(values)
}

macro_rules! gcd_template {
    ($type:ident, $gcd:ident) => {
        impl GcdLcm for $type {
            type Output = $type;

            paste! {
                fn gcd(self, rhs: $type) -> $type {
                    [<$gcd "_" $type>](self.clone(), rhs.clone())
                }
            }

            fn gcd_list(values: &[$type]) -> $type {
                todo!()
            }

            fn lcm(self, rhs: $type) -> $type {
                let this_gcd = (&self).gcd(&rhs);
                self * (rhs / this_gcd)
            }

            fn lcm_list(values: &[$type]) -> $type {
                let this_gcd = Self::gcd_list(values);
                let this_lcm = values
                    .iter()
                    .fold(this_gcd.clone(), |acc, x| acc * (x / &this_gcd));
                this_lcm
            }

            fn gcd_lcm(self, rhs: $type) -> ($type, $type) {
                let this_gcd = (&self).gcd(&rhs);
                let this_lcm = self * (rhs / &this_gcd);
                (this_gcd, this_lcm)
            }

            fn gcd_lcm_list(values: &[$type]) -> ($type, $type) {
                let this_gcd = Self::gcd_list(values);
                let this_lcm = values
                    .iter()
                    .fold(this_gcd.clone(), |acc, x| acc * (x / &this_gcd));
                (this_gcd, this_lcm)
            }
        }

        impl GcdLcm for &$type {
            type Output = $type;

            paste! {
                fn gcd(self, rhs: &$type) -> $type {
                    [<$gcd _ $type>]((*self).clone(), (*rhs).clone())
                }
            }

            fn gcd_list(values: &[Self]) -> $type {
                todo!()
            }

            fn lcm(self, rhs: &$type) -> $type {
                let this_gcd = self.gcd(rhs);
                self * (rhs / &this_gcd)
            }

            fn lcm_list(values: &[Self]) -> $type {
                let this_gcd = Self::gcd_list(values);
                let this_lcm = values
                    .iter()
                    .fold(this_gcd.clone(), |acc, x| acc * (*x / &this_gcd));
                this_lcm
            }

            fn gcd_lcm(self, rhs: &$type) -> ($type, $type) {
                let this_gcd = self.gcd(rhs);
                let this_lcm = self * (rhs / &this_gcd);
                (this_gcd, this_lcm)
            }

            fn gcd_lcm_list(values: &[Self]) -> ($type, $type) {
                let this_gcd = Self::gcd_list(values);
                let this_lcm = values
                    .iter()
                    .fold(this_gcd.clone(), |acc, x| acc * (*x / &this_gcd));
                (this_gcd, this_lcm)
            }
        }
    };
}

macro_rules! small_gcd_template {
    ($($type:ident)*) => ($(
        gcd_template!($type, gcd_euclid);
    )*)
}
small_gcd_template! { i8 i16 i32 u8 u16 u32 }

macro_rules! floating_gcd_template {
    ($($type:ident)*) => ($(
        gcd_template!($type, gcd_euclid);
    )*)
}
floating_gcd_template! { f32 f64 }

macro_rules! big_gcd_template {
    ($($type:ident)*) => ($(
        gcd_template!($type, gcd_stein);
    )*)
}
big_gcd_template! { i64 i128 isize u64 u128 usize }

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use crate::algebra::concept::RealNumber;

    use super::*;

    fn test_real<T: RealNumber + GcdLcm<Output=T> + Debug>()
    where
            for<'a> &'a T: Mul<&'a T, Output = T> + GcdLcm<Output=T>,
    {
        assert_eq!(&(T::TWO.gcd(T::FIVE)), T::ONE);
        assert_eq!(&(T::TEN.gcd(&(T::TWO * T::TWO))), T::TWO);
        assert_eq!(&(T::TEN.gcd(&(T::FIVE * T::FIVE))), T::FIVE);

        assert_eq!(&(T::TWO.lcm(T::FIVE)), T::TEN);
        assert_eq!(&(T::TEN.lcm(&T::TWO)), T::TEN);
        assert_eq!(&(T::TEN.lcm(&(T::TWO * T::TWO))), &(T::TEN * T::TWO));
        assert_eq!(&(T::TEN.lcm(&T::FIVE)), T::TEN);
        assert_eq!(&(T::TWO.lcm(&(T::FIVE * T::FIVE))), &(T::TEN * T::FIVE));
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
