//! # IndexValue - 索引值转换
//!
//! ## Overview / 概述
//!
//! This module provides the `TryIntoIndexValue` trait for converting various
//! integer types to `isize` for use in array indexing operations.
//!
//! 本模块提供 `TryIntoIndexValue` 特征，用于将各种整数类型转换为 `isize` 以用于数组索引操作。
//!
//! ## Key Traits / 主要特征
//!
//! - `TryIntoIndexValue` - Trait for converting to index values / 转换为索引值的特征

use std::convert::Infallible;

/// # TryIntoIndexValue Trait
///
/// A trait for types that can be converted to an index value (`isize`).
/// This is used primarily for dummy index conversions where various
/// integer types need to be normalized to a common index type.
///
/// 可转换为索引值（`isize`）的类型的特征。
/// 这主要用于虚拟索引转换，其中各种整数类型需要规范化为通用索引类型。
///
/// ## Associated Types / 关联类型
///
/// - `Error` - The error type for failed conversions / 转换失败时的错误类型
///
/// ## Implementors / 实现者
///
/// - `isize`, `usize` - Direct conversion / 直接转换
/// - `u8`, `u16`, `u32`, `u64`, `u128` - Unsigned integers / 无符号整数
/// - `i8`, `i16`, `i32`, `i64`, `i128` - Signed integers / 有符号整数
pub trait TryIntoIndexValue {
    /// The error type returned when conversion fails.
    ///
    /// 转换失败时返回的错误类型。
    type Error;

    /// Convert the value to an `isize`.
    ///
    /// 将值转换为 `isize`。
    ///
    /// # Returns / 返回值
    ///
    /// - `Ok(isize)` - The converted index value / 转换后的索引值
    /// - `Err(Self::Error)` - If the conversion fails / 如果转换失败
    fn try_into(self) -> Result<isize, Self::Error>;
}

impl TryIntoIndexValue for isize {
    type Error = Infallible;

    fn try_into(self) -> Result<isize, Self::Error> {
        Ok(self)
    }
}

impl TryIntoIndexValue for &'_ isize {
    type Error = Infallible;

    fn try_into(self) -> Result<isize, Self::Error> {
        Ok(*self)
    }
}

impl TryIntoIndexValue for usize {
    type Error = Infallible;

    fn try_into(self) -> Result<isize, Self::Error> {
        Ok(self.cast_signed())
    }
}

impl TryIntoIndexValue for &'_ usize {
    type Error = Infallible;

    fn try_into(self) -> Result<isize, Self::Error> {
        Ok(self.cast_signed())
    }
}

/// Macro for implementing TryIntoIndexValue for various integer types.
///
/// 为各种整数类型实现 TryIntoIndexValue 的宏。
macro_rules! impl_index_value_for_int {
    ($($t:ty)*) => {
        $(
            impl TryIntoIndexValue for $t {
                type Error = <$t as TryInto<isize>>::Error;

                fn try_into(self) -> Result<isize, Self::Error> {
                    Ok(<$t as TryInto<isize>>::try_into(self)?)
                }
            }

            impl TryIntoIndexValue for &'_ $t {
                type Error = <$t as TryInto<isize>>::Error;

                fn try_into(self) -> Result<isize, Self::Error> {
                    Ok(<$t as TryInto<isize>>::try_into(*self)?)
                }
            }
        )*
    };
}

// Implement for all standard integer types.
// 为标准整数类型实现。
impl_index_value_for_int! { u8 u16 u32 u64 u128 i8 i16 i32 i64 i128 }
