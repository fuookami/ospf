use std::ops::Div;

use paste::paste;

use crate::algebra::concept::{FloatingNumber, RealNumber, SemiArithmetic};
use crate::algebra::operator::Reciprocal;
use crate::algebra::ordinary::pow::*;

pub trait Pow: Sized {
    type Output;

    fn pow(self, index: i64) -> Self::Output;

    fn square(self) -> Self::Output {
        self.pow(2)
    }

    fn cubic(self) -> Self::Output {
        self.pow(3)
    }
}

pub fn pow<Lhs: Pow>(lhs: Lhs, index: i64) -> Lhs::Output {
    lhs.pow(index)
}

pub fn square<Lhs: Pow>(lhs: Lhs) -> Lhs::Output {
    lhs.square()
}

pub fn cubic<Lhs: Pow>(lhs: Lhs) -> Lhs::Output {
    lhs.cubic()
}

pub trait PowF<Index: FloatingNumber = Self>: Sized
where
    for<'a> &'a Index: Reciprocal<Output = Index>,
{
    type Output: FloatingNumber;

    fn powf(self, index: &Index) -> Option<Self::Output>;

    fn sqrt(self) -> Option<Self::Output> {
        self.powf(&Index::TWO.reciprocal().unwrap())
    }

    fn cbrt(self) -> Option<Self::Output> {
        self.powf(&Index::THREE.reciprocal().unwrap())
    }
}

pub fn powf<Lhs: PowF<Rhs>, Rhs: FloatingNumber>(lhs: Lhs, rhs: &Rhs) -> Option<Lhs::Output>
where
    for<'a> &'a Rhs: Reciprocal<Output = Rhs>,
{
    lhs.powf(rhs)
}

pub fn sqrt<Lhs: PowF<Rhs>, Rhs: FloatingNumber>(lhs: Lhs) -> Option<Lhs::Output>
where
    for<'a> &'a Rhs: Reciprocal<Output = Rhs>,
{
    lhs.sqrt()
}

pub fn cbrt<Lhs: PowF<Rhs>, Rhs: FloatingNumber>(lhs: Lhs) -> Option<Lhs::Output>
where
    for<'a> &'a Rhs: Reciprocal<Output = Rhs>,
{
    lhs.cbrt()
}

pub trait Exp {
    type Output;

    fn exp(self) -> Self::Output;
}

pub fn exp<Lhs: Exp>(lhs: Lhs) -> Lhs::Output {
    lhs.exp()
}

macro_rules! int_pow_template {
    ($($type:ident)*) => ($(
        impl Pow for $type {
            type Output = $type;

            paste! {
                fn pow(self, index: i64) -> $type {
                    if index >= 0 {
                        [<pow_times_semi_group_ $type>](&self, index as u64)
                    } else {
                        (*$type::ZERO).clone()
                    }
                }
            }
        }

        impl PowF<f64> for $type {
            type Output = f64;

            fn powf(self, index: &f64) -> Option<f64> {
                Some((self as f64).powf(*index))
            }

            fn sqrt(self) -> Option<f64> {
                Some((self as f64).sqrt())
            }

            fn cbrt(self) -> Option<f64> {
                Some((self as f64).cbrt())
            }
        }

        impl Exp for $type {
            type Output = f64;

            fn exp(self) -> f64 {
                (self as f64).exp()
            }
        }
    )*)
}
int_pow_template! { i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }

macro_rules! floating_pow_template {
    ($($type:ident)*) => ($(
        impl Pow for $type {
            type Output = $type;

            paste! {
                fn pow(self, index: i64) -> $type {
                    [<pow_times_group_ $type>](&self, index)
                }
            }
        }

        impl PowF for $type {
            type Output = $type;

            fn powf(self, index: &Self) -> Option<$type> {
                Some(<$type>::powf(self, *index))
            }

            fn sqrt(self) -> Option<$type> {
                Some(<$type>::sqrt(self))
            }

            fn cbrt(self) -> Option<$type> {
                Some(<$type>::cbrt(self))
            }
        }

        impl Exp for $type {
            type Output = $type;

            fn exp(self) -> $type {
                <$type>::exp(self)
            }
        }
    )*)
}
floating_pow_template! { f32 f64 }
