use std::ops::{Div, Mul, Sub};

use paste::paste;

use crate::algebra::concept::{Arithmetic, FloatingNumber, Precision, RealNumber};
use crate::algebra::operator::Reciprocal;

use super::ln;

pub fn exp<T: FloatingNumber>(index: &T) -> T
where
    for<'a> &'a T: Div<Output = T>,
{
    let mut value = T::ONE.clone();
    let mut base = index.clone();
    let mut i = T::ONE.clone();
    loop {
        let this_item = &base / &i;
        value += &this_item;
        base *= index;
        i += T::ONE;

        if &this_item <= <T as Precision>::EPSILON {
            break;
        }
    }
    value
}

pub fn powf<T: FloatingNumber>(base: &T, index: &T) -> Option<T>
where
    for<'a> &'a T: Sub<Output = T> + Mul<Output = T> + Div<Output = T> + Reciprocal<Output = T>,
{
    if let Some(ln_base) = ln(base) {
        Some(exp(&(index * &ln_base)))
    } else {
        <T as RealNumber>::NAN.clone()
    }
}

macro_rules! exp_template {
    ($($type:ident)*) => ($(
        paste! {
            pub fn [<exp_ $type>](index: &$type) -> $type {
                let mut value = (*$type::ONE).clone();
                let mut base = index.clone();
                let mut i = (*$type::ONE).clone();
                loop {
                    let this_item = &base / &i;
                    value += &this_item;
                    base *= index;
                    i += $type::ONE;

                    if &this_item <= <$type as Precision>::EPSILON {
                        break;
                    }
                }
                value
            }
        }
    )*)
}
exp_template! { f32 f64 }

macro_rules! powf_template {
    ($($type:ident)*) => ($(
        paste! {
            pub fn [<powf_ $type>](base: &$type, index: &$type) -> Option<$type> {
                if let Some(ln_base) = [<ln_ $type>](base) {
                    Some([<exp_ $type>](&(index * &ln_base)))
                } else {
                    (*<$type as RealNumber>::NAN).clone()
                }
            }
        }
    )*)
}
