use num::complex::ComplexFloat;

use super::{
    Arithmetic, Bits, Bounded, Invariant, NumberField, PlusSemiGroup, Precision, Signed,
    TimesGroup, Unsigned,
};

pub trait Scalar: Arithmetic + Bounded {}

impl<T: Arithmetic + Bounded> Scalar for T {}

pub enum RealNumberCategory {
    Nan,
    Infinite,
    NegativeInfinite,
    Zero,
    Subnormal,
    Normal,
}

pub trait RealNumber: Scalar + Precision + Invariant {
    const TWO: &'static Self;
    const THREE: &'static Self;
    const FIVE: &'static Self;
    const TEN: &'static Self;

    const NAN: &'static Option<Self> = &None;
    const INF: &'static Option<Self> = &None;
    const NEG_INF: &'static Option<Self> = &None;

    // todo: category

    fn is_nan(&self) -> bool {
        Self::NAN
            .as_ref()
            .is_some_and(|nan_value| self == nan_value)
    }

    fn is_inf(&self) -> bool {
        Self::INF
            .as_ref()
            .is_some_and(|inf_value| self == inf_value)
    }

    fn is_neg_inf(&self) -> bool {
        Self::NEG_INF
            .as_ref()
            .is_some_and(|inf_value| self == inf_value)
    }

    fn is_finite(&self) -> bool {
        return !self.is_inf() && !self.is_inf() && !self.is_neg_inf();
    }
}

pub trait Integer: RealNumber + Bits + Ord + Eq {}

impl<T: RealNumber + Bits + Ord + Eq> Integer for T {}

pub trait IntegerNumber: Integer + NumberField + Signed {}

impl<T: Integer + NumberField + Signed> IntegerNumber for T {}

pub trait UIntegerNumber: Integer + PlusSemiGroup + TimesGroup + Unsigned {}

impl<T: Integer + PlusSemiGroup + TimesGroup + Unsigned> UIntegerNumber for T {}

pub trait RationalNumber<I: Integer>: RealNumber + NumberField + Ord + Eq {
    fn num(&self) -> &I;
    fn den(&self) -> &I;
}

pub trait FloatingNumber: RealNumber + NumberField + Signed {
    const PI: &'static Self;
    const E: &'static Self;

    fn floor(&self) -> Self;
    fn ceil(&self) -> Self;
    fn round(&self) -> Self;
    fn trunc(&self) -> Self;
    fn fract(&self) -> Self;
}

pub trait NumericIntegerNumber<I: IntegerNumber>: Integer + Signed + Ord + Eq {}

pub trait NumericUIntegerNumber<I: UIntegerNumber>: Integer + Unsigned + Ord + Eq {}

macro_rules! int_real_number_template {
    ($($type:ident)*) => ($(
        impl RealNumber for $type {
            const TWO: &'static Self = &2;
            const THREE: &'static Self = &3;
            const FIVE: &'static Self = &5;
            const TEN: &'static Self = &10;
        }
    )*)
}
int_real_number_template! { i8 i16 i32 i64 i128 isize }

macro_rules! uint_real_number_template {
    ($($type:ident)*) => ($(
        impl RealNumber for $type {
            const TWO: &'static Self = &2;
            const THREE: &'static Self = &3;
            const FIVE: &'static Self = &5;
            const TEN: &'static Self = &10;
        }
    )*)
}
uint_real_number_template! { u8 u16 u32 u64 u128 usize }

macro_rules! floating_real_number_template {
    ($($type:ident)*) => ($(
        impl RealNumber for $type {
            const TWO: &'static Self = &2.;
            const THREE: &'static Self = &3.;
            const FIVE: &'static Self = &5.;
            const TEN: &'static Self = &10.;

            const NAN: &'static Option<Self> = &Some(<$type>::NAN);
            const INF: &'static Option<Self> = &Some(<$type>::INFINITY);
            const NEG_INF: &'static Option<Self> = &Some(<$type>::NEG_INFINITY);

            fn is_nan(&self) -> bool {
                <$type>::is_nan(*self)
            }

            fn is_inf(&self) -> bool {
                <$type>::is_infinite(*self) && <$type>::is_sign_positive(*self)
            }

            fn is_neg_inf(&self) -> bool {
                <$type>::is_infinite(*self) && <$type>::is_sign_negative(*self)
            }
        }

        impl FloatingNumber for $type {
                const PI: &'static Self = &std::$type::consts::PI;
                const E: &'static Self = &std::$type::consts::E;

                fn floor(&self) -> Self {
                    <$type>::floor(*self)
                }

                fn ceil(&self) -> Self {
                    <$type>::ceil(*self)
                }

                fn round(&self) -> Self {
                    <$type>::round(*self)
                }

                fn trunc(&self) -> Self {
                    <$type>::trunc(*self)
                }

                fn fract(&self) -> Self {
                    <$type>::fract(*self)
                }
            }
    )*)
}
floating_real_number_template! { f32 f64 }
