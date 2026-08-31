use std::ops::{Div, Mul};

use paste::paste;

use crate::algebra::concept::Arithmetic;

pub(self) fn pow_pos_impl<T: Arithmetic>(value: &T, base: &T, index: u64) -> T
where
    for<'a> &'a T: Mul<Output = T>,
{
    if index == 0 {
        T::ONE.clone()
    } else {
        pow_pos_impl(&(value * base), base, index - 1)
    }
}

pub(self) fn pow_neg_impl<T: Arithmetic>(value: &T, base: &T, index: i64) -> T
where
    for<'a> &'a T: Div<Output = T>,
{
    if index == 0 {
        T::ONE.clone()
    } else {
        pow_neg_impl(&(value / base), base, index + 1)
    }
}

pub(crate) fn pow_times_semi_group<T: Arithmetic>(base: &T, index: u64) -> T
where
    for<'a> &'a T: Mul<Output = T>,
{
    if index >= 1 {
        pow_pos_impl(T::ONE, base, index)
    } else {
        T::ONE.clone()
    }
}

pub(crate) fn pow_times_group<T: Arithmetic>(base: &T, index: i64) -> T
where
    for<'a> &'a T: Mul<Output = T> + Div<Output = T>,
{
    if index >= 1 {
        pow_pos_impl(T::ONE, base, index as u64)
    } else if index <= -1 {
        pow_neg_impl(T::ONE, base, index)
    } else {
        T::ONE.clone()
    }
}

macro_rules! pow_pos_template {
    ($($type:ident)*) => ($(
        paste! {
            pub(self) fn [<pow_pos_ $type>](value: &$type, base: &$type, index: u64) -> $type {
                if index == 0 {
                    (*$type::ONE).clone()
                } else {
                    [<pow_pos_ $type>](&(value * base), base, index - 1)
                }
            }
        }
    )*)
}
pow_pos_template! { i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 }

macro_rules! pow_neg_template {
    ($($type:ident)*) => ($(
        paste! {
            pub(self) fn [<pow_neg_ $type>](value: &$type, base: &$type, index: i64) -> $type {
                if index == 0 {
                    (*$type::ONE).clone()
                } else {
                    [<pow_neg_ $type>](&(value / base), base, index + 1)
                }
            }
        }
    )*)
}
pow_neg_template! { i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 }

macro_rules! pow_times_semi_group_template {
    ($($type:ident)*) => ($(
        paste! {
            pub(crate) fn [<pow_times_semi_group_ $type>](base: &$type, index: u64) -> $type {
                if index >= 1 {
                    [<pow_pos_ $type>]($type::ONE, base, index)
                } else {
                    (*$type::ONE).clone()
                }
            }
        }
    )*)
}
pow_times_semi_group_template! { i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 }

macro_rules! pow_times_group_template {
    ($($type:ident)*) => ($(
        paste! {
            pub(crate) fn [<pow_times_group_ $type>](base: &$type, index: i64) -> $type {
                if index >= 1 {
                    [<pow_pos_ $type>]($type::ONE, base, index as u64)
                } else if index <= -1 {
                    [<pow_neg_ $type>]($type::ONE, base, index)
                } else {
                    (*$type::ONE).clone()
                }
            }
        }
    )*)
}
pow_times_group_template! { i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 }
