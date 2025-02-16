use crate::algebra::*;

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

pub trait PowF<Index = Self> {
    type Output;

    fn powf(self, index: Index) -> Option<Self::Output>;

    fn sqr(self) -> Option<Self::Output>;
    fn cbr(self) -> Option<Self::Output>;
}

pub trait Exp {
    type Output;

    fn exp(self) -> Self::Output;
}

macro_rules! int_pow_template {
    ($($type:ty)*) => ($(
        impl Pow for $type {
            type Output = Self;

            fn pow(self, index: i64) -> Self::Output {
                ordinary::pow_times_semi_group(self, index).unwrap()
            }
        }

        impl PowF<f64> for $type {
            type Output = f64;

            fn powf(self, index: f64) -> Option<Self::Output> {
                Some((self as f64).powf(index))
            }

            fn sqr(self) -> Option<Self::Output> {
                Some((self as f64).sqrt())
            }

            fn cbr(self) -> Option<Self::Output> {
                Some((self as f64).cbrt())
            }
        }

        impl Exp for $type {
            type Output = f64;

            fn exp(self) -> Self::Output {
                (self as f64).exp()
            }
        }
    )*)
}
int_pow_template! { u8 u16 u32 u64 u128 i8 i16 i32 i64 i128 }

macro_rules! floating_pow_template {
    ($($type:ty)*) => ($(
        impl Pow for $type {
            type Output = Self;

            fn pow(self, index: i64) -> Self::Output {
                ordinary::pow_times_group(self, index)
            }
        }

        impl PowF for $type {
            type Output = Self;

            fn powf(self, index: Self) -> Option<Self::Output> {
                Some(self.powf(index))
            }

            fn sqr(self) -> Option<Self::Output> {
                Some(self.sqrt())
            }

            fn cbr(self) -> Option<Self::Output> {
                Some(self.cbrt())
            }
        }

        impl Exp for $type {
            type Output = Self;

            fn exp(self) -> Self::Output {
                self.exp()
            }
        }
    )*)
}
floating_pow_template! { f32 f64 }

impl Pow for Decimal {
    type Output = Self;

    fn pow(self, index: i64) -> Self::Output {
        ordinary::pow_times_group(self, index)
    }
}

impl PowF for Decimal {
    type Output = Self;

    fn powf(self, index: Self) -> Option<Self::Output> {
        ordinary::powf(self, index)
    }

    fn sqr(self) -> Option<Self::Output> {
        self.powf(Self::ONE / Self::TWO)
    }

    fn cbr(self) -> Option<Self::Output> {
        self.powf(Self::ONE / Self::from_i128(3).unwrap())
    }
}

impl Exp for Decimal {
    type Output = Self;

    fn exp(self) -> Self::Output {
        ordinary::exp(self)
    }
}
