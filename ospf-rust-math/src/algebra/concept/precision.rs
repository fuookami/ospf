use super::*;
use crate::algebra::*;

pub trait Precision: Arithmetic {
    const EPSILON: Self;
    const DECIMAL_DIGITS: Option<usize>;
    const DECIMAL_PRECISION: Self;
}

default impl<T: Arithmetic> Precision for T {
    const EPSILON: Self = Self::ZERO;
    const DECIMAL_DIGITS: Option<usize> = None;
    const DECIMAL_PRECISION: Self = Self::EPSILON;
}

macro_rules! int_precision_template {
    ($($type:ty)*) => ($(
        impl Precision for $type { }
    )*)
}
int_precision_template! { i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 IntX UIntX }

impl Precision for f32 {
    const EPSILON: Self = <f32>::EPSILON;
    const DECIMAL_DIGITS: Option<usize> = Some(<f32>::DIGITS as usize);
}

impl Precision for f64 {
    const EPSILON: Self = <f64>::EPSILON;
    const DECIMAL_DIGITS: Option<usize> = Some(<f64>::DIGITS as usize);
}

impl Precision for Decimal {
    const EPSILON: Self = Decimal::from_f64(1e-28).unwrap();
    const DECIMAL_DIGITS: Option<usize> = Some(28);
}
