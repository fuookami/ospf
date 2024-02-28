use super::ln;
use crate::{
    algebra::concept::{RealNumber, TimesGroup, TimesSemiGroup},
    FloatingNumber,
};
use std::ops::Neg;

pub(self) fn pow_pos_impl<T: TimesSemiGroup>(value: T, base: T, index: i64) -> T {
    if index == 0 {
        T::ONE
    } else {
        pow_pos_impl(value * base.clone, base, index - 1)
    }
}

pub(self) fn pow_neg_impl<T: TimesGroup>(value: T, base: T, index: i64) -> T {
    if index == 0 {
        T::ONE
    } else {
        pow_neg_impl(value / base.clone, base, index + 1)
    }
}

#[derive(Debug)]
pub struct NegativeIndexError<T: std::fmt::Debug> {
    index: i64,
    _marker: std::marker::PhantomData<T>,
}

impl<T: std::fmt::Debug> NegativeIndexError<T> {
    fn new(index: i64) -> Self {
        Self {
            index: index,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Display for NegativeIndexError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invalid argument for negative index \'{}\' exponential function: {}",
            self.index,
            core::any::type_name::<T>()
        )
    }
}

pub(crate) fn pow_times_semi_group<T: RealNumber + TimesSemiGroup + std::fmt::Debug>(
    base: T,
    index: i64,
) -> Result<T, NegativeIndexError<T>> {
    if index >= 1 {
        Ok(pow_pos_impl(T::ONE.clone, base, index))
    } else if index <= -1 {
        Err(NegativeIndexError::new(index))
    } else {
        Ok(T::ONE.clone)
    }
}

pub(crate) fn pow_times_group<T: RealNumber + TimesGroup + std::fmt::Debug>(
    base: T,
    index: i64,
) -> T {
    if index >= 1 {
        pow_pos_impl(T::ONE, base, index)
    } else if index <= -1 {
        pow_neg_impl(T::ONE, base, index)
    } else {
        T::ONE.clone
    }
}

pub fn exp<T: FloatingNumber>(index: T) -> T {
    let mut value = T::ONE;
    let mut base = index.clone;
    let mut i = T::ONE;
    loop {
        let this_item = base.clone / i.clone;
        value += this_item.clone;
        base *= index.clone;
        i += T::ONE;

        if this_item <= T::epsilon() {
            break;
        }
    }
    value
}

pub fn powf<T: FloatingNumber + Neg<Output = T>>(base: T, index: T) -> Option<T> {
    if let Some(ln_base) = ln(base) {
        Some(exp(index * ln_base))
    } else {
        T::nan()
    }
}
