
//! 索引值转换模块
//! Index value conversion module
//!
//! 本模块提供了将各种整数类型转换为索引值（`isize`）的 trait。
//! This module provides traits for converting various integer types to index values (`isize`).
//!
//! ## 主要 trait / Main Trait
//!
//! - `TryIntoIndexValue`: 尝试将值转换为索引值的 trait
//!   Trait for trying to convert values to index values
//!
//! ## 支持的类型 / Supported Types
//!
//! - `isize`, `&isize`: 直接转换
//!   Direct conversion
//! - `usize`, `&usize`: 直接转换（有符号转换）
//!   Direct conversion (signed conversion)
//! - `u8`, `u16`, `u32`, `u64`, `u128`: 尝试转换（可能溢出）
//!   Try conversion (may overflow)
//! - `i8`, `i16`, `i32`, `i64`, `i128`: 尝试转换
//!   Try conversion

use std::convert::Infallible;

/// 尝试转换为索引值 trait
/// Try into index value trait
///
/// 定义将值转换为 `isize` 索引值的能力。
/// Defines the ability to convert values to `isize` index values.
///
/// ## 类型参数 / Type Parameters
///
/// - `Error`: 转换失败时的错误类型
///   Error type when conversion fails
///
/// ## 返回值 / Returns
///
/// 成功时返回 `Ok(isize)`，失败时返回 `Err(Self::Error)`。
/// Returns `Ok(isize)` on success, `Err(Self::Error)` on failure.
///
/// ## 示例 / Examples
///
/// ```rust
/// use ospf_rust_multiarray::index_value::TryIntoIndexValue;
///
/// let idx: Result<isize, _> = TryIntoIndexValue::try_into(5isize);
/// assert!(idx.is_ok());
///
/// let idx: Result<isize, _> = TryIntoIndexValue::try_into(10usize);
/// assert!(idx.is_ok());
/// ```
pub trait TryIntoIndexValue {
    /// 转换失败时的错误类型
    /// Error type when conversion fails
    type Error;

    /// 尝试将值转换为索引值
    /// Try to convert the value to an index value
    fn try_into(self) -> Result<isize, Self::Error>;
}

/// `isize` 的 `TryIntoIndexValue` 实现
/// `TryIntoIndexValue` implementation for `isize`
///
/// 直接返回值本身，不会失败。
/// Returns the value directly, never fails.
impl TryIntoIndexValue for isize {
    type Error = Infallible;

    fn try_into(self) -> Result<isize, Self::Error> {
        Ok(self)
    }
}

/// `&isize` 的 `TryIntoIndexValue` 实现
/// `TryIntoIndexValue` implementation for `&isize`
///
/// 解引用后返回值本身，不会失败。
/// Returns the dereferenced value, never fails.
impl TryIntoIndexValue for &'_ isize {
    type Error = Infallible;

    fn try_into(self) -> Result<isize, Self::Error> {
        Ok(*self)
    }
}

/// `usize` 的 `TryIntoIndexValue` 实现
/// `TryIntoIndexValue` implementation for `usize`
///
/// 转换为有符号整数，在大多数平台上不会失败。
/// Converts to signed integer, never fails on most platforms.
impl TryIntoIndexValue for usize {
    type Error = Infallible;

    fn try_into(self) -> Result<isize, Self::Error> {
        Ok(self.cast_signed())
    }
}

/// `&usize` 的 `TryIntoIndexValue` 实现
/// `TryIntoIndexValue` implementation for `&usize`
///
/// 解引用后转换为有符号整数，在大多数平台上不会失败。
/// Converts dereferenced value to signed integer, never fails on most platforms.
impl TryIntoIndexValue for &'_ usize {
    type Error = Infallible;

    fn try_into(self) -> Result<isize, Self::Error> {
        Ok(self.cast_signed())
    }
}

/// 为整数类型实现 `TryIntoIndexValue` 的宏
/// Macro for implementing `TryIntoIndexValue` for integer types
///
/// 为指定的整数类型及其引用生成 `TryIntoIndexValue` 实现。
/// Generates `TryIntoIndexValue` implementations for specified integer types and their references.
///
/// ## 示例 / Examples
///
/// ```
/// use ospf_rust_multiarray::index_value::TryIntoIndexValue;
///
/// // 测试 u8 转换 / Test u8 conversion
/// let idx: Result<isize, _> = TryIntoIndexValue::try_into(5u8);
/// assert!(idx.is_ok());
/// assert_eq!(idx.unwrap(), 5);
///
/// // 测试 i32 转换 / Test i32 conversion
/// let idx: Result<isize, _> = TryIntoIndexValue::try_into(-10i32);
/// assert!(idx.is_ok());
/// assert_eq!(idx.unwrap(), -10);
/// ```
macro_rules! impl_index_value_for_int {
    ($($t:ty)*) => {
        $(
            /// 整数类型的 `TryIntoIndexValue` 实现
            /// `TryIntoIndexValue` implementation for integer type
            impl TryIntoIndexValue for $t {
                type Error = <$t as TryInto<isize>>::Error;

                fn try_into(self) -> Result<isize, Self::Error> {
                    Ok(<$t as TryInto<isize>>::try_into(self)?)
                }
            }

            /// 整数类型引用的 `TryIntoIndexValue` 实现
            /// `TryIntoIndexValue` implementation for integer type reference
            impl TryIntoIndexValue for &'_ $t {
                type Error = <$t as TryInto<isize>>::Error;

                fn try_into(self) -> Result<isize, Self::Error> {
                    Ok(<$t as TryInto<isize>>::try_into(*self)?)
                }
            }
        )*
    };
}

// 为所有标准整数类型实现 `TryIntoIndexValue`
// Implement `TryIntoIndexValue` for all standard integer types
impl_index_value_for_int! { u8 u16 u32 u64 u128 i8 i16 i32 i64 i128 }