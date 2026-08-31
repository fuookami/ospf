//! # MapIndex - 映射索引实现
//!
//! ## Overview / 概述
//!
//! This module provides the `MapIndex` type for array dimension mapping and reordering.
//! Map indices allow for:
//! - Dimension reordering (transposing) / 维度重排（转置）
//! - Dimension projection (selecting specific dimensions) / 维度投影（选择特定维度）
//! - Combined slicing and reordering / 组合切片和重排
//!
//! 本模块提供 `MapIndex` 类型用于数组维度映射和重排。
//! 映射索引允许：
//! - 维度重排（转置）
//! - 维度投影（选择特定维度）
//! - 组合切片和重排
//!
//! ## Key Types / 主要类型
//!
//! - `PlaceHolder` - Placeholder for dimension mapping / 维度映射的占位符
//! - `MapIndex` - The main map index type / 主要映射索引类型
//!
//! ## Constants / 常量
//!
//! - `_0` to `_20` - Predefined placeholders for dimensions 0-20 / 预定义的 0-20 维度占位符

use std::alloc::Allocator;
use std::ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};

use super::dummy_index::DummyIndex;
use super::index_value::TryIntoIndexValue;

/// # PlaceHolder
///
/// A placeholder representing a dimension index in mapping operations.
/// Placeholders are used to specify which dimensions to keep and their order.
///
/// 在映射操作中表示维度索引的占位符。
/// 占位符用于指定要保留的维度及其顺序。
///
/// ## Fields / 字段
///
/// - `index` - The dimension index / 维度索引
///
/// ## Example / 示例
///
/// ```rust
/// use ospf_rust_multiarray::*;
///
/// // Using predefined constants / 使用预定义常量
/// let map = [_0, _2, _1]; // Reorder dimensions: 0, 2, 1
///
/// // Creating custom placeholder / 创建自定义占位符
/// let custom = PlaceHolder { index: 3 };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct PlaceHolder {
    /// The dimension index / 维度索引
    pub index: usize,
}

/// Predefined placeholders for dimensions 0-20.
/// 预定义的 0-20 维度占位符。
pub const _0: PlaceHolder = PlaceHolder { index: 0 };
pub const _1: PlaceHolder = PlaceHolder { index: 1 };
pub const _2: PlaceHolder = PlaceHolder { index: 2 };
pub const _3: PlaceHolder = PlaceHolder { index: 3 };
pub const _4: PlaceHolder = PlaceHolder { index: 4 };
pub const _5: PlaceHolder = PlaceHolder { index: 5 };
pub const _6: PlaceHolder = PlaceHolder { index: 6 };
pub const _7: PlaceHolder = PlaceHolder { index: 7 };
pub const _8: PlaceHolder = PlaceHolder { index: 8 };
pub const _9: PlaceHolder = PlaceHolder { index: 9 };
pub const _10: PlaceHolder = PlaceHolder { index: 10 };
pub const _11: PlaceHolder = PlaceHolder { index: 11 };
pub const _12: PlaceHolder = PlaceHolder { index: 12 };
pub const _13: PlaceHolder = PlaceHolder { index: 13 };
pub const _14: PlaceHolder = PlaceHolder { index: 14 };
pub const _15: PlaceHolder = PlaceHolder { index: 15 };
pub const _16: PlaceHolder = PlaceHolder { index: 16 };
pub const _17: PlaceHolder = PlaceHolder { index: 17 };
pub const _18: PlaceHolder = PlaceHolder { index: 18 };
pub const _19: PlaceHolder = PlaceHolder { index: 19 };
pub const _20: PlaceHolder = PlaceHolder { index: 20 };

/// # MapIndex
///
/// A type representing a single dimension's mapping specification.
/// It can be either:
/// - `Dummy(DummyIndex)` - A slicing operation (range, index, or index array) / 切片操作（范围、索引或索引数组）
/// - `Map(PlaceHolder)` - A dimension mapping (keep and possibly reorder) / 维度映射（保留并可能重排）
///
/// 表示单个维度映射规范的类型。
/// 它可以是：
/// - `Dummy(DummyIndex)` - 切片操作（范围、索引或索引数组）
/// - `Map(PlaceHolder)` - 维度映射（保留并可能重排）
///
/// ## Example / 示例
///
/// ```rust
/// use ospf_rust_multiarray::*;
///
/// // Keep dimension 0 / 保留维度 0
/// let map = MapIndex::Map(_0);
///
/// // Slice dimension with range / 使用范围切片维度
/// let slice = MapIndex::Dummy(DummyIndex::Range(Box::new(1..5)));
/// ```
#[derive(Debug, Clone)]
pub enum MapIndex {
    /// Slicing operation (range, index, or index array) / 切片操作（范围、索引或索引数组）
    Dummy(DummyIndex),
    /// Dimension mapping (keep and possibly reorder) / 维度映射（保留并可能重排）
    Map(PlaceHolder),
}

impl From<isize> for MapIndex {
    fn from(value: isize) -> Self {
        Self::Dummy(DummyIndex::from(value))
    }
}

impl From<&'_ isize> for MapIndex {
    fn from(value: &'_ isize) -> Self {
        Self::Dummy(DummyIndex::from(value))
    }
}

impl From<usize> for MapIndex {
    fn from(value: usize) -> Self {
        Self::Dummy(DummyIndex::from(value))
    }
}

impl From<&'_ usize> for MapIndex {
    fn from(value: &'_ usize) -> Self {
        Self::Dummy(DummyIndex::from(value))
    }
}

impl From<DummyIndex> for MapIndex {
    fn from(value: DummyIndex) -> Self {
        Self::Dummy(value)
    }
}

impl<T> TryFrom<Range<T>> for MapIndex
where
    T: TryIntoIndexValue,
{
    type Error = <T as TryIntoIndexValue>::Error;

    fn try_from(value: Range<T>) -> Result<Self, Self::Error> {
        Ok(Self::Dummy(DummyIndex::try_from(value)?))
    }
}

impl<'a, T> TryFrom<&'a Range<T>> for MapIndex
where
    &'a T: TryIntoIndexValue,
{
    type Error = <&'a T as TryIntoIndexValue>::Error;

    fn try_from(value: &'a Range<T>) -> Result<Self, Self::Error> {
        Ok(Self::Dummy(DummyIndex::try_from(value)?))
    }
}

impl<T> TryFrom<RangeFrom<T>> for MapIndex
where
    T: TryIntoIndexValue,
{
    type Error = <T as TryIntoIndexValue>::Error;

    fn try_from(value: RangeFrom<T>) -> Result<Self, Self::Error> {
        Ok(Self::Dummy(DummyIndex::try_from(value)?))
    }
}

impl<'a, T> TryFrom<&'a RangeFrom<T>> for MapIndex
where
    &'a T: TryIntoIndexValue,
{
    type Error = <&'a T as TryIntoIndexValue>::Error;

    fn try_from(value: &'a RangeFrom<T>) -> Result<Self, Self::Error> {
        Ok(Self::Dummy(DummyIndex::try_from(value)?))
    }
}

impl<T> TryFrom<RangeInclusive<T>> for MapIndex
where
    T: TryIntoIndexValue,
{
    type Error = <T as TryIntoIndexValue>::Error;

    fn try_from(value: RangeInclusive<T>) -> Result<Self, Self::Error> {
        Ok(Self::Dummy(DummyIndex::try_from(value)?))
    }
}

impl<'a, T> TryFrom<&'a RangeInclusive<T>> for MapIndex
where
    &'a T: TryIntoIndexValue,
{
    type Error = <&'a T as TryIntoIndexValue>::Error;

    fn try_from(value: &'a RangeInclusive<T>) -> Result<Self, Self::Error> {
        Ok(Self::Dummy(DummyIndex::try_from(value)?))
    }
}

impl<T> TryFrom<RangeTo<T>> for MapIndex
where
    T: TryIntoIndexValue,
{
    type Error = <T as TryIntoIndexValue>::Error;

    fn try_from(value: RangeTo<T>) -> Result<Self, Self::Error> {
        Ok(Self::Dummy(DummyIndex::try_from(value)?))
    }
}

impl<'a, T> TryFrom<&'a RangeTo<T>> for MapIndex
where
    &'a T: TryIntoIndexValue,
{
    type Error = <&'a T as TryIntoIndexValue>::Error;

    fn try_from(value: &'a RangeTo<T>) -> Result<Self, Self::Error> {
        Ok(Self::Dummy(DummyIndex::try_from(value)?))
    }
}

impl<T> TryFrom<RangeToInclusive<T>> for MapIndex
where
    T: TryIntoIndexValue,
{
    type Error = <T as TryIntoIndexValue>::Error;

    fn try_from(value: RangeToInclusive<T>) -> Result<Self, Self::Error> {
        Ok(Self::Dummy(DummyIndex::try_from(value)?))
    }
}

impl<'a, T> TryFrom<&'a RangeToInclusive<T>> for MapIndex
where
    &'a T: TryIntoIndexValue,
{
    type Error = <&'a T as TryIntoIndexValue>::Error;

    fn try_from(value: &'a RangeToInclusive<T>) -> Result<Self, Self::Error> {
        Ok(Self::Dummy(DummyIndex::try_from(value)?))
    }
}

impl From<RangeFull> for MapIndex {
    fn from(value: RangeFull) -> Self {
        Self::Dummy(DummyIndex::from(value))
    }
}

impl<'a> From<&'a RangeFull> for MapIndex {
    fn from(value: &'a RangeFull) -> Self {
        Self::Dummy(DummyIndex::from(value))
    }
}

impl<'a, T> TryFrom<&'a [T]> for MapIndex
where
    &'a T: TryIntoIndexValue,
{
    type Error = <&'a T as TryIntoIndexValue>::Error;

    fn try_from(value: &'a [T]) -> Result<Self, Self::Error> {
        Ok(Self::Dummy(DummyIndex::try_from(value)?))
    }
}

impl<'a, T, A: Allocator> TryFrom<&'a Vec<T, A>> for MapIndex
where
    &'a T: TryIntoIndexValue,
{
    type Error = <&'a T as TryIntoIndexValue>::Error;

    fn try_from(value: &'a Vec<T, A>) -> Result<Self, Self::Error> {
        Ok(Self::Dummy(DummyIndex::try_from(value)?))
    }
}

impl From<PlaceHolder> for MapIndex {
    fn from(holder: PlaceHolder) -> Self {
        Self::Map(holder)
    }
}

impl<'a> From<&'a PlaceHolder> for MapIndex {
    fn from(holder: &'a PlaceHolder) -> Self {
        Self::Map(*holder)
    }
}

/// # map_index! Macro
///
/// A macro for creating MapIndex from various expressions.
///
/// 用于从各种表达式创建 MapIndex 的宏。
#[macro_export]
macro_rules! map_index {
    (-$x:literal) => {
        MapIndex::try_from(dummy_index_value!{ -$x })
    };
    ($x:literal) => {
        MapIndex::try_from(dummy_index_value!{ $x })
    };
    ($start:literal..-$end:literal) => {
        MapIndex::try_from(dummy_index_value!{ $start }..dummy_index_value!{ -$end })
    };
    ($start:literal..$end:literal) => {
        MapIndex::try_from(dummy_index_value!{ $start }..dummy_index_value!{ $end })
    };
    ($start:literal..) => {
        MapIndex::try_from(dummy_index_value!{ $start }..)
    };
    (..$end:literal) => {
        MapIndex::try_from(..dummy_index_value!{ $end })
    };
    (..-$end:literal) => {
        MapIndex::try_from(..dummy_index_value!{ -$end })
    };
    ($start:literal..=-$end:literal) => {
        MapIndex::try_from(dummy_index_value!{ $start }..=dummy_index_value!{ -$end })
    };
    ($start:literal..=$end:literal) => {
        MapIndex::try_from(dummy_index_value!{ $start }..=dummy_index_value!{ $end })
    };
    (..=-$end:literal) => {
        MapIndex::try_from(..=dummy_index_value!{ -$end })
    };
    (..=$end:literal) => {
        MapIndex::try_from(..=dummy_index_value!{ $end })
    };
    (..) => {
        MapIndex::try_from(..)
    };
    [$($x:literal),*] => {
        MapIndex::try_from(&[$(dummy_index_value!{ $x }),*])
    };
    ($x:expr) => {
        MapIndex::try_from(&$x)
    };
}

/// # map! Macro
///
/// A macro for creating fixed-size map index arrays.
///
/// 用于创建固定大小映射索引数组的宏。
#[macro_export]
macro_rules! map {
    [$($x:expr),*] => {
        [$(map_index!{ $x }.map_err(|e| error! { InvalidDummyIndexError {} })?),*]
    };
    ($a:ident[$($x:expr),*]) => {
        $a.map_view(map![$($x),*])
    };
}

/// # map_with_err! Macro
///
/// A macro for creating fixed-size map index arrays with custom error handling.
///
/// 用于创建具有自定义错误处理的固定大小映射索引数组的宏。
#[macro_export]
macro_rules! map_with_err {
    [$($x:expr),*] => {
        [$(map_index!{ $x }.map_err(|e| error! { ExInvalidDummyIndexError { origin: e } })?),*]
    };
    ($a:ident[$($x:expr),*]) => {
        $a.map_view(map_with_err![$($x),*])
    }
}

/// # map_expect! Macro
///
/// A macro for creating fixed-size map index arrays, panicking on error.
///
/// 用于创建固定大小映射索引数组的宏，出错时 panic。
#[macro_export]
macro_rules! map_expect {
    ([$($x:expr),*], $msg:expr) => {
        [$(map_index!{ $x }.expect($msg)),*]
    };
    [$($x:expr),*] => {
        [$(map_index!{ $x }.unwrap()),*]
    };
    ($a:ident[$($x:expr),*], $msg:expr) => {
        $a.map_view(map_expect!{ [$($x),*], $msg })
    };
    ($a:ident[$($x:expr),*]) => {
        $a.map_view(map_expect![$($x),*])
    }
}

/// # dyn_map! Macro
///
/// A macro for creating dynamic (Vec-based) map index arrays.
///
/// 用于创建动态（基于 Vec）映射索引数组的宏。
#[macro_export]
macro_rules! dyn_map {
    [$($x:expr),*] => {
        vec![$(map_index!{ $x }.map_err(|e| error! { InvalidDummyIndexError {} })?),*]
    };
    ($a:ident[$($x:expr),*]) => {
        $a.map_view(dyn_map![$($x),*])
    };
}

/// # dyn_map_with_err! Macro
///
/// A macro for creating dynamic map index arrays with custom error handling.
///
/// 用于创建具有自定义错误处理的动态映射索引数组的宏。
#[macro_export]
macro_rules! dyn_map_with_err {
    [$($x:expr),*] => {
        vec![$(map_index!{ $x }.map_err(|e| error! { ExInvalidDummyIndexError { origin: e } })?),*]
    };
    ($a:ident[$($x:expr),*]) => {
        $a.map_view(dyn_map_with_err![$($x),*])
    };
}

/// # dyn_map_expect! Macro
///
/// A macro for creating dynamic map index arrays, panicking on error.
///
/// 用于创建动态映射索引数组的宏，出错时 panic。
#[macro_export]
macro_rules! dyn_map_expect {
    ([$($x:expr),*], $msg:expr) => {
        vec![$(map_index!{ $x }.expect($msg)),*]
    };
    [$($x:expr),*] => {
        vec![$(map_index!($x).unwrap(),)*]
    };
    ($a:ident[$($x:expr),*], $msg:expr) => {
        $a.map_view(dyn_map_expect!{ [$($x),*], $msg })
    };
    ($a:ident[$($x:expr),*]) => {
        $a.map_view(dyn_map_expect![$($x),*])
    };
}

pub use dyn_map;
pub use dyn_map_expect;
pub use dyn_map_with_err;
pub use map;
pub use map_expect;
pub use map_with_err;
