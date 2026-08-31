use super::{Arithmetic, SemiArithmetic};

pub trait Precision: 'static {
    const EPSILON: &'static Self;
    const DECIMAL_DIGITS: Option<usize>;
    const DECIMAL_PRECISION: &'static Self;
}

default impl<T: Arithmetic> Precision for T {
    const EPSILON: &'static Self = Self::ZERO;
    const DECIMAL_DIGITS: Option<usize> = None;
    const DECIMAL_PRECISION: &'static Self = Self::EPSILON;
}

macro_rules! int_precision_template {
    ($($type:ident)*) => ($(
        impl Precision for $type {
            const EPSILON: &'static Self = Self::ZERO;
            const DECIMAL_DIGITS: Option<usize> = None;
            const DECIMAL_PRECISION: &'static Self = Self::EPSILON;
         }
    )*)
}
int_precision_template! { bool i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }

impl Precision for f32 {
    const EPSILON: &'static Self = &<f32>::MIN_POSITIVE;
    const DECIMAL_DIGITS: Option<usize> = Some(<f32>::DIGITS as usize);
    const DECIMAL_PRECISION: &'static Self = &Self::EPSILON;
}

impl Precision for f64 {
    const EPSILON: &'static Self = &<f64>::MIN_POSITIVE;
    const DECIMAL_DIGITS: Option<usize> = Some(<f64>::DIGITS as usize);
    const DECIMAL_PRECISION: &'static Self = &Self::EPSILON;
}
