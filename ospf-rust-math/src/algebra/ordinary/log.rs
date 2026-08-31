use std::ops::{Mul, Sub};

use paste::paste;

use crate::algebra::concept::{Arithmetic, FloatingNumber, Precision, RealNumber, SemiArithmetic};
use crate::algebra::operator::Reciprocal;

pub fn ln<T: FloatingNumber>(x: &T) -> Option<T>
where
    for<'a> &'a T: Sub<Output = T> + Mul<Output = T> + Reciprocal<Output = T>,
{
    if x <= T::ZERO {
        (*<T as RealNumber>::NAN).clone()
    } else {
        let frac_e = T::E.reciprocal().unwrap();

        let mut val = (T::ZERO).clone();
        let mut xp = x.clone();
        if &xp < T::ONE {
            while xp <= frac_e {
                xp *= T::E;
                val -= T::ONE;
            }
        } else if &xp > T::ONE {
            while &xp >= T::E {
                xp /= T::E;
                val += T::ONE;
            }
        }
        let mut base = &xp - T::ONE;
        let mut signed = (*T::ONE).clone();
        let mut i = (*T::ONE).clone();
        loop {
            let this_item = &signed * &base / &i;
            val += this_item.clone();
            base *= &xp - T::ONE;
            signed = -signed;
            i += T::ONE;

            if &this_item <= &<T as Precision>::EPSILON {
                break;
            }
        }
        Some(val)
    }
}

pub fn log<T: FloatingNumber>(nature: &T, x: &T) -> Option<T>
where
    for<'a> &'a T: Sub<Output = T> + Mul<Output = T> + Reciprocal<Output = T>,
{
    if let (Some(ln_nature), Some(ln_x)) = (ln(nature), ln(x)) {
        Some(ln_x / ln_nature)
    } else {
        None
    }
}

pub fn lg10<T: FloatingNumber>(x: &T) -> Option<T>
where
    for<'a> &'a T: Sub<Output = T> + Mul<Output = T> + Reciprocal<Output = T>,
{
    log(T::TEN, x)
}

pub fn lg2<T: FloatingNumber>(x: &T) -> Option<T>
where
    for<'a> &'a T: Sub<Output = T> + Mul<Output = T> + Reciprocal<Output = T>,
{
    log(T::TWO, x)
}

macro_rules! ln_template {
    ($($type:ident)*) => ($(
        paste! {
            pub fn [<ln_ $type>](x: &$type) -> Option<$type> {
                if x <= $type::ZERO {
                    (*<$type as RealNumber>::NAN).clone()
                } else {
                    let frac_e = $type::E.reciprocal().unwrap();

                    let mut val = (*$type::ZERO).clone();
                    let mut xp = x.clone();
                    if &xp < $type::ONE {
                        while xp <= frac_e {
                            xp *= $type::E;
                            val -= $type::ONE;
                        }
                    } else if &xp > $type::ONE {
                        while &xp >= $type::E {
                            xp /= $type::E;
                            val += $type::ONE;
                        }
                    }
                    let mut base = &xp - $type::ONE;
                    let mut signed = (*$type::ONE).clone();
                    let mut i = (*$type::ONE).clone();
                    loop {
                        let this_item = &signed * &base / &i;
                        val += this_item.clone();
                        base *= &xp - $type::ONE;
                        signed = -signed;
                        i += $type::ONE;

                        if &this_item <= &<$type as Precision>::EPSILON {
                            break;
                        }
                    }
                    Some(val)
                }
            }
        }
    )*)
}
ln_template! { f32 f64 }

macro_rules! log_template {
    ($($type:ident)*) => ($(
        paste! {
            pub fn [<log_ $type>](nature: &$type, x: &$type) -> Option<$type> {
                if let (Some(ln_nature), Some(ln_x)) = ([<ln_ $type>](nature), [<ln_ $type>](x)) {
                    Some(ln_x / ln_nature)
                } else {
                    None
                }
            }

            pub fn [<lg10_ $type>](x: &$type) -> Option<$type> {
                [<log_ $type>]($type::TEN, x)
            }

            pub fn [<lg2_ $type>](x: &$type) -> Option<$type> {
                [<log_ $type>]($type::TWO, x)
            }
        }
    )*)
}
log_template! { f32 f64 }
