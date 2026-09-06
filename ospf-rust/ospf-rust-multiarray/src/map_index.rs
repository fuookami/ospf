//! 映射索引模块
//! Map index module
//!
//! 本模块提供了多维数组的映射索引机制，支持：
//! This module provides map indexing mechanism for multi-dimensional arrays, supporting:
//!
//! - 虚拟索引：与 `DummyIndex` 相同的索引功能
//!   Dummy index: same indexing functionality as `DummyIndex`
//! - 映射索引：使用占位符表示映射维度
//!   Map index: using placeholders to represent mapping dimensions
//!
//! ## 主要类型 / Main Types
//!
//! - `PlaceHolder`: 占位符结构体，表示映射维度的位置
//!   Placeholder struct, representing the position of mapping dimensions
//! - `MapIndex`: 映射索引枚举，表示虚拟索引或映射
//!   Map index enum, representing dummy index or map
//!
//! ## 占位符常量 / Placeholder Constants
//!
//! 模块提供了 `_0` 到 `_20` 共 21 个预定义占位符常量。
//! The module provides 21 predefined placeholder constants from `_0` to `_20`.
//!
//! ## 示例 / Examples
//!
//! ```rust
//! use ospf_rust_multiarray::map_index::{MapIndex, _0, _1};
//!
//! // 使用占位符创建映射索引
//! // Create map index using placeholders
//! let idx = MapIndex::Map(_0);  // 第一个映射维度
//! let idx2 = MapIndex::Map(_1);  // 第二个映射维度
//! ```

use super::dummy_index::DummyIndex;
use super::index_value::TryIntoIndexValue;
use std::alloc::Allocator;
use std::ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};

/// 占位符结构体
/// Placeholder struct
///
/// 表示映射维度的位置索引。在多维数组的映射操作中，
/// 占位符用于标记哪些维度需要被映射。
/// Represents the position index of a mapping dimension. In multi-dimensional
/// array mapping operations, placeholders mark which dimensions need to be mapped.
///
/// ## 示例 / Examples
///
/// ```rust
/// use ospf_rust_multiarray::map_index::{PlaceHolder, _0, _1};
///
/// // _0 表示第一个映射维度
/// // _0 represents the first mapping dimension
/// let placeholder = _0;
/// assert_eq!(placeholder.index, 0);
///
/// // _1 表示第二个映射维度
/// // _1 represents the second mapping dimension
/// let placeholder2 = _1;
/// assert_eq!(placeholder2.index, 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaceHolder {
    /// 映射维度的位置索引（从 0 开始）
    /// Position index of the mapping dimension (0-based)
    pub index: usize,
}

/// 占位符常量：第 0 个映射维度
/// Placeholder constant: 0th mapping dimension
pub const _0: PlaceHolder = PlaceHolder { index: 0 };

/// 占位符常量：第 1 个映射维度
/// Placeholder constant: 1st mapping dimension
pub const _1: PlaceHolder = PlaceHolder { index: 1 };

/// 占位符常量：第 2 个映射维度
/// Placeholder constant: 2nd mapping dimension
pub const _2: PlaceHolder = PlaceHolder { index: 2 };

/// 占位符常量：第 3 个映射维度
/// Placeholder constant: 3rd mapping dimension
pub const _3: PlaceHolder = PlaceHolder { index: 3 };

/// 占位符常量：第 4 个映射维度
/// Placeholder constant: 4th mapping dimension
pub const _4: PlaceHolder = PlaceHolder { index: 4 };

/// 占位符常量：第 5 个映射维度
/// Placeholder constant: 5th mapping dimension
pub const _5: PlaceHolder = PlaceHolder { index: 5 };

/// 占位符常量：第 6 个映射维度
/// Placeholder constant: 6th mapping dimension
pub const _6: PlaceHolder = PlaceHolder { index: 6 };

/// 占位符常量：第 7 个映射维度
/// Placeholder constant: 7th mapping dimension
pub const _7: PlaceHolder = PlaceHolder { index: 7 };

/// 占位符常量：第 8 个映射维度
/// Placeholder constant: 8th mapping dimension
pub const _8: PlaceHolder = PlaceHolder { index: 8 };

/// 占位符常量：第 9 个映射维度
/// Placeholder constant: 9th mapping dimension
pub const _9: PlaceHolder = PlaceHolder { index: 9 };

/// 占位符常量：第 10 个映射维度
/// Placeholder constant: 10th mapping dimension
pub const _10: PlaceHolder = PlaceHolder { index: 10 };

/// 占位符常量：第 11 个映射维度
/// Placeholder constant: 11th mapping dimension
pub const _11: PlaceHolder = PlaceHolder { index: 11 };

/// 占位符常量：第 12 个映射维度
/// Placeholder constant: 12th mapping dimension
pub const _12: PlaceHolder = PlaceHolder { index: 12 };

/// 占位符常量：第 13 个映射维度
/// Placeholder constant: 13th mapping dimension
pub const _13: PlaceHolder = PlaceHolder { index: 13 };

/// 占位符常量：第 14 个映射维度
/// Placeholder constant: 14th mapping dimension
pub const _14: PlaceHolder = PlaceHolder { index: 14 };

/// 占位符常量：第 15 个映射维度
/// Placeholder constant: 15th mapping dimension
pub const _15: PlaceHolder = PlaceHolder { index: 15 };

/// 占位符常量：第 16 个映射维度
/// Placeholder constant: 16th mapping dimension
pub const _16: PlaceHolder = PlaceHolder { index: 16 };

/// 占位符常量：第 17 个映射维度
/// Placeholder constant: 17th mapping dimension
pub const _17: PlaceHolder = PlaceHolder { index: 17 };

/// 占位符常量：第 18 个映射维度
/// Placeholder constant: 18th mapping dimension
pub const _18: PlaceHolder = PlaceHolder { index: 18 };

/// 占位符常量：第 19 个映射维度
/// Placeholder constant: 19th mapping dimension
pub const _19: PlaceHolder = PlaceHolder { index: 19 };

/// 占位符常量：第 20 个映射维度
/// Placeholder constant: 20th mapping dimension
pub const _20: PlaceHolder = PlaceHolder { index: 20 };

/// 映射索引
/// Map index
///
/// 表示多维数组的映射索引，支持两种形式：
/// Represents map index for multi-dimensional arrays, supporting two forms:
///
/// - `Dummy`: 虚拟索引，与 `DummyIndex` 相同
///   Dummy index, same as `DummyIndex`
/// - `Map`: 映射索引，使用占位符表示映射维度
///   Map index, using placeholder to represent mapping dimension
///
/// ## 示例 / Examples
///
/// ```rust
/// use ospf_rust_multiarray::map_index::{MapIndex, _0, _1};
/// use ospf_rust_multiarray::dummy_index::DummyIndex;
///
/// // 虚拟索引
/// // Dummy index
/// let idx = MapIndex::Dummy(DummyIndex::Index(5));
///
/// // 映射索引
/// // Map index
/// let map = MapIndex::Map(_0);  // 第一个映射维度
/// ```
#[derive(Debug, Clone)]
pub enum MapIndex {
    /// 虚拟索引
    /// Dummy index
    Dummy(DummyIndex),

    /// 映射索引
    /// Map index
    Map(PlaceHolder),
}

/// MapIndex 的相等性比较实现
/// Equality comparison implementation for MapIndex
///
/// 只有相同类型的索引才能比较相等。
/// Only indices of the same type can be compared for equality.
impl PartialEq for MapIndex {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MapIndex::Dummy(a), MapIndex::Dummy(b)) => a == b,
            (MapIndex::Map(a), MapIndex::Map(b)) => a == b,
            _ => false,
        }
    }
}

/// 从 `isize` 创建 `MapIndex`
/// Create `MapIndex` from `isize`
impl From<isize> for MapIndex {
    fn from(value: isize) -> Self {
        Self::Dummy(DummyIndex::from(value))
    }
}

/// 从 `&isize` 创建 `MapIndex`
/// Create `MapIndex` from `&isize`
impl From<&'_ isize> for MapIndex {
    fn from(value: &'_ isize) -> Self {
        Self::Dummy(DummyIndex::from(value))
    }
}

/// 从 `usize` 创建 `MapIndex`
/// Create `MapIndex` from `usize`
impl From<usize> for MapIndex {
    fn from(value: usize) -> Self {
        Self::Dummy(DummyIndex::from(value))
    }
}

/// 从 `&usize` 创建 `MapIndex`
/// Create `MapIndex` from `&usize`
impl From<&'_ usize> for MapIndex {
    fn from(value: &'_ usize) -> Self {
        Self::Dummy(DummyIndex::from(value))
    }
}

/// 从 `DummyIndex` 创建 `MapIndex`
/// Create `MapIndex` from `DummyIndex`
impl From<DummyIndex> for MapIndex {
    fn from(value: DummyIndex) -> Self {
        Self::Dummy(value)
    }
}

/// 尝试从 `Range<T>` 创建 `MapIndex`
/// Try to create `MapIndex` from `Range<T>`
impl<T> TryFrom<Range<T>> for MapIndex
where
    T: TryIntoIndexValue,
{
    type Error = <T as TryIntoIndexValue>::Error;

    fn try_from(value: Range<T>) -> Result<Self, Self::Error> {
        Ok(Self::Dummy(DummyIndex::try_from(value)?))
    }
}

/// 尝试从 `&Range<T>` 创建 `MapIndex`
/// Try to create `MapIndex` from `&Range<T>`
impl<'a, T> TryFrom<&'a Range<T>> for MapIndex
where
    &'a T: TryIntoIndexValue,
{
    type Error = <&'a T as TryIntoIndexValue>::Error;

    fn try_from(value: &'a Range<T>) -> Result<Self, Self::Error> {
        Ok(Self::Dummy(DummyIndex::try_from(value)?))
    }
}

/// 尝试从 `RangeFrom<T>` 创建 `MapIndex`
/// Try to create `MapIndex` from `RangeFrom<T>`
impl<T> TryFrom<RangeFrom<T>> for MapIndex
where
    T: TryIntoIndexValue,
{
    type Error = <T as TryIntoIndexValue>::Error;

    fn try_from(value: RangeFrom<T>) -> Result<Self, Self::Error> {
        Ok(Self::Dummy(DummyIndex::try_from(value)?))
    }
}

/// 尝试从 `&RangeFrom<T>` 创建 `MapIndex`
/// Try to create `MapIndex` from `&RangeFrom<T>`
impl<'a, T> TryFrom<&'a RangeFrom<T>> for MapIndex
where
    &'a T: TryIntoIndexValue,
{
    type Error = <&'a T as TryIntoIndexValue>::Error;

    fn try_from(value: &'a RangeFrom<T>) -> Result<Self, Self::Error> {
        Ok(Self::Dummy(DummyIndex::try_from(value)?))
    }
}

/// 尝试从 `RangeInclusive<T>` 创建 `MapIndex`
/// Try to create `MapIndex` from `RangeInclusive<T>`
impl<T> TryFrom<RangeInclusive<T>> for MapIndex
where
    T: TryIntoIndexValue,
{
    type Error = <T as TryIntoIndexValue>::Error;

    fn try_from(value: RangeInclusive<T>) -> Result<Self, Self::Error> {
        Ok(Self::Dummy(DummyIndex::try_from(value)?))
    }
}

/// 尝试从 `&RangeInclusive<T>` 创建 `MapIndex`
/// Try to create `MapIndex` from `&RangeInclusive<T>`
impl<'a, T> TryFrom<&'a RangeInclusive<T>> for MapIndex
where
    &'a T: TryIntoIndexValue,
{
    type Error = <&'a T as TryIntoIndexValue>::Error;

    fn try_from(value: &'a RangeInclusive<T>) -> Result<Self, Self::Error> {
        Ok(Self::Dummy(DummyIndex::try_from(value)?))
    }
}

/// 尝试从 `RangeTo<T>` 创建 `MapIndex`
/// Try to create `MapIndex` from `RangeTo<T>`
impl<T> TryFrom<RangeTo<T>> for MapIndex
where
    T: TryIntoIndexValue,
{
    type Error = <T as TryIntoIndexValue>::Error;

    fn try_from(value: RangeTo<T>) -> Result<Self, Self::Error> {
        Ok(Self::Dummy(DummyIndex::try_from(value)?))
    }
}

/// 尝试从 `&RangeTo<T>` 创建 `MapIndex`
/// Try to create `MapIndex` from `&RangeTo<T>`
impl<'a, T> TryFrom<&'a RangeTo<T>> for MapIndex
where
    &'a T: TryIntoIndexValue,
{
    type Error = <&'a T as TryIntoIndexValue>::Error;

    fn try_from(value: &'a RangeTo<T>) -> Result<Self, Self::Error> {
        Ok(Self::Dummy(DummyIndex::try_from(value)?))
    }
}

/// 尝试从 `RangeToInclusive<T>` 创建 `MapIndex`
/// Try to create `MapIndex` from `RangeToInclusive<T>`
impl<T> TryFrom<RangeToInclusive<T>> for MapIndex
where
    T: TryIntoIndexValue,
{
    type Error = <T as TryIntoIndexValue>::Error;

    fn try_from(value: RangeToInclusive<T>) -> Result<Self, Self::Error> {
        Ok(Self::Dummy(DummyIndex::try_from(value)?))
    }
}

/// 尝试从 `&RangeToInclusive<T>` 创建 `MapIndex`
/// Try to create `MapIndex` from `&RangeToInclusive<T>`
impl<'a, T> TryFrom<&'a RangeToInclusive<T>> for MapIndex
where
    &'a T: TryIntoIndexValue,
{
    type Error = <&'a T as TryIntoIndexValue>::Error;

    fn try_from(value: &'a RangeToInclusive<T>) -> Result<Self, Self::Error> {
        Ok(Self::Dummy(DummyIndex::try_from(value)?))
    }
}

/// 从 `RangeFull` 创建 `MapIndex`
/// Create `MapIndex` from `RangeFull`
impl From<RangeFull> for MapIndex {
    fn from(value: RangeFull) -> Self {
        Self::Dummy(DummyIndex::from(value))
    }
}

/// 从 `&RangeFull` 创建 `MapIndex`
/// Create `MapIndex` from `&RangeFull`
impl<'a> From<&'a RangeFull> for MapIndex {
    fn from(value: &'a RangeFull) -> Self {
        Self::Dummy(DummyIndex::from(value))
    }
}

/// 尝试从切片创建 `MapIndex`
/// Try to create `MapIndex` from a slice
impl<'a, T> TryFrom<&'a [T]> for MapIndex
where
    &'a T: TryIntoIndexValue,
{
    type Error = <&'a T as TryIntoIndexValue>::Error;

    fn try_from(value: &'a [T]) -> Result<Self, Self::Error> {
        Ok(Self::Dummy(DummyIndex::try_from(value)?))
    }
}

/// 尝试从 `Vec<T>` 创建 `MapIndex`
/// Try to create `MapIndex` from `Vec<T>`
impl<'a, T, A: Allocator> TryFrom<&'a Vec<T, A>> for MapIndex
where
    &'a T: TryIntoIndexValue,
{
    type Error = <&'a T as TryIntoIndexValue>::Error;

    fn try_from(value: &'a Vec<T, A>) -> Result<Self, Self::Error> {
        Ok(Self::Dummy(DummyIndex::try_from(value)?))
    }
}

/// 从 `PlaceHolder` 创建 `MapIndex`
/// Create `MapIndex` from `PlaceHolder`
impl From<PlaceHolder> for MapIndex {
    fn from(holder: PlaceHolder) -> Self {
        Self::Map(holder)
    }
}

/// 从 `&PlaceHolder` 创建 `MapIndex`
/// Create `MapIndex` from `&PlaceHolder`
impl<'a> From<&'a PlaceHolder> for MapIndex {
    fn from(holder: &'a PlaceHolder) -> Self {
        Self::Map(*holder)
    }
}

/// 映射索引创建宏
/// Map index creation macro
///
/// 用于便捷地创建 `MapIndex` 实例。
/// Used to conveniently create `MapIndex` instances.
///
/// ## 支持的语法 / Supported Syntax
///
/// - 单个索引：`map_index!(5)`, `map_index!(-1)`
///   Single index: `map_index!(5)`, `map_index!(-1)`
/// - 范围索引：`map_index!(0..5)`, `map_index!(1..=3)`, `map_index!(..)`
///   Range index: `map_index!(0..5)`, `map_index!(1..=3)`, `map_index!(..)`
/// - 索引数组：`map_index!([0, 2, 4])`
///   Index array: `map_index!([0, 2, 4])`
/// - 表达式：`map_index!(expr)`
///   Expression: `map_index!(expr)`
///
/// ## 返回值 / Returns
///
/// 返回 `Result<MapIndex, _>`，需要处理可能的错误。
/// Returns `Result<MapIndex, _>`, requires handling potential errors.
#[macro_export]
macro_rules! map_index {
    // 负数单个索引
    // Negative single index
    (-$x:literal) => {
        MapIndex::try_from(dummy_index_value!{ -$x })
    };
    // 正数单个索引
    // Positive single index
    ($x:literal) => {
        MapIndex::try_from(dummy_index_value!{ $x })
    };
    // 范围：start..-end
    ($start:literal..-$end:literal) => {
        MapIndex::try_from(dummy_index_value!{ $start }..dummy_index_value!{ -$end })
    };
    // 范围：start..end
    ($start:literal..$end:literal) => {
        MapIndex::try_from(dummy_index_value!{ $start }..dummy_index_value!{ $end })
    };
    // 范围：start..
    ($start:literal..) => {
        MapIndex::try_from(dummy_index_value!{ $start }..)
    };
    // 范围：..end
    (..$end:literal) => {
        MapIndex::try_from(..dummy_index_value!{ $end })
    };
    // 范围：..-end
    (..-$end:literal) => {
        MapIndex::try_from(..dummy_index_value!{ -$end })
    };
    // 范围：start..=-end
    ($start:literal..=-$end:literal) => {
        MapIndex::try_from(dummy_index_value!{ $start }..=dummy_index_value!{ -$end })
    };
    // 范围：start..=end
    ($start:literal..=$end:literal) => {
        MapIndex::try_from(dummy_index_value!{ $start }..=dummy_index_value!{ $end })
    };
    // 范围：..=-end
    (..=-$end:literal) => {
        MapIndex::try_from(..=dummy_index_value!{ -$end })
    };
    // 范围：..=end
    (..=$end:literal) => {
        MapIndex::try_from(..=dummy_index_value!{ $end })
    };
    // 全范围
    // Full range
    (..) => {
        MapIndex::try_from(..)
    };
    // 索引数组
    // Index array
    [$($x:literal),*] => {
        MapIndex::try_from(&[$(dummy_index_value!{ $x }),*])
    };
    // 表达式
    // Expression
    ($x:expr) => {
        MapIndex::try_from(&$x)
    };
}

/// 映射索引数组创建宏
/// Map index array creation macro
///
/// 创建映射索引数组，错误转换为 `InvalidDummyIndexError`。
/// Creates map index array, errors converted to `InvalidDummyIndexError`.
///
/// ## 示例 / Examples
///
/// ```ignore
/// use ospf_rust_multiarray::map_index::{map, _0, _1};
/// use ospf_rust_multiarray::map_index::MapIndex;
///
/// // 创建索引数组 / Create index array
/// let indices: [MapIndex; 3] = map![_0, 1..5, _1];
/// assert_eq!(indices.len(), 3);
/// ```
#[macro_export]
macro_rules! map {
    // 创建索引数组
    // Create index array
    [$($x:expr),*] => {
        [$(map_index!{ $x }.map_err(|e| error! { InvalidDummyIndexError {} })?),*]
    };
    // 创建映射视图
    // Create map view
    ($a:ident[$($x:expr),*]) => {
        $a.map_view(map![$($x),*])
    };
}

/// 带详细错误的映射索引数组创建宏
/// Map index array creation macro with detailed error
///
/// 与 `map!` 类似，但保留原始错误信息。
/// Similar to `map!`, but preserves original error information.
#[macro_export]
macro_rules! map_with_err {
    [$($x:expr),*] => {
        [$(map_index!{ $x }.map_err(|e| error! { ExInvalidDummyIndexError { origin: e } })?),*]
    };
    ($a:ident[$($x:expr),*]) => {
        $a.map_view(map_with_err![$($x),*])
    }
}

/// 期望成功的映射索引数组创建宏
/// Expect-success map index array creation macro
///
/// 创建映射索引数组，如果失败则 panic。
/// Creates map index array, panics on failure.
///
/// ## 示例 / Examples
///
/// ```ignore
/// use ospf_rust_multiarray::map_index::{map_expect, _0, _1};
/// use ospf_rust_multiarray::map_index::MapIndex;
///
/// // 创建索引数组 / Create index array
/// let indices: [MapIndex; 3] = map_expect![_0, 1..5, _1];
/// assert_eq!(indices.len(), 3);
/// ```
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

/// 动态映射索引数组创建宏
/// Dynamic map index array creation macro
///
/// 创建 `Vec<MapIndex>` 而非数组。
/// Creates `Vec<MapIndex>` instead of an array.
#[macro_export]
macro_rules! dyn_map {
    [$($x:expr),*] => {
        vec![$(map_index!{ $x }.map_err(|e| error! { InvalidDummyIndexError {} })?),*]
    };
    ($a:ident[$($x:expr),*]) => {
        $a.map_view(dyn_map![$($x),*])
    };
}

/// 带详细错误的动态映射索引数组创建宏
/// Dynamic map index array creation macro with detailed error
#[macro_export]
macro_rules! dyn_map_with_err {
    [$($x:expr),*] => {
        vec![$(map_index!{ $x }.map_err(|e| error! { ExInvalidDummyIndexError { origin: e } })?),*]
    };
    ($a:ident[$($x:expr),*]) => {
        $a.map_view(dyn_map_with_err![$($x),*])
    };
}

/// 期望成功的动态映射索引数组创建宏
/// Expect-success dynamic map index array creation macro
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
