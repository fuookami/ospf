//! 虚拟索引模块
//! Dummy index module
//!
//! 本模块提供了多维数组的灵活索引机制，支持：
//! This module provides flexible indexing mechanism for multi-dimensional arrays, supporting:
//!
//! - 单个索引：`0`, `-1`（负数表示从末尾开始）
//!   Single index: `0`, `-1` (negative means counting from the end)
//! - 范围索引：`0..5`, `1..=3`, `..`, `2..`
//!   Range index: `0..5`, `1..=3`, `..`, `2..`
//! - 索引数组：`[0, 2, 4]`
//!   Index array: `[0, 2, 4]`
//!
//! ## 主要类型 / Main Types
//!
//! - `DummyIndex`: 虚拟索引枚举，表示单个索引、范围或索引数组
//!   Dummy index enum, representing single index, range, or index array
//! - `DummyIndexIterator`: 虚拟索引迭代器，用于遍历索引
//!   Dummy index iterator, used to iterate over indices
//! - `DummyAccessIterator`: 虚拟访问迭代器，用于遍历多维数组的元素
//!   Dummy access iterator, used to iterate over elements of multi-dimensional arrays
//!
//! ## 示例 / Examples
//!
//! ```rust
//! use ospf_rust_multiarray::dummy_index::DummyIndex;
//!
//! // 使用 TryFrom 创建虚拟索引
//! // Create dummy indices using TryFrom
//! let idx: DummyIndex = DummyIndex::try_from(5isize).unwrap();  // 单个索引 / Single index
//! let range: DummyIndex = DummyIndex::try_from(1..5).unwrap();  // 范围 / Range
//! let arr: DummyIndex = DummyIndex::try_from(&[1isize, 3, 5][..]).unwrap();  // 索引数组 / Index array
//! ```

use super::concept::{AccessOrder, AccessOrderTrait, ColumnMajor, RowMajor, StorageOrderTrait};
use super::index_value::TryIntoIndexValue;
use super::shape::AbstractShape;
use cc_traits::{Collection, Len};
use dyn_clone::{DynClone, clone_trait_object};
use ospf_rust_base::collection::Indices;
use ospf_rust_base::error::*;
use std::alloc::Allocator;
use std::convert::Into;
use std::fmt::{Debug, Display};
use std::ops::{
    Bound, Index, IndexMut, Range, RangeBounds, RangeFrom, RangeFull, RangeInclusive, RangeTo,
    RangeToInclusive,
};

/// 虚拟索引范围 trait
/// Dummy index range trait
///
/// 定义虚拟索引范围的行为，支持动态克隆。
/// Defines behavior for dummy index ranges, supporting dynamic cloning.
///
/// 该 trait 是对标准库 `RangeBounds` trait 的封装，
/// 允许在运行时处理不同类型的范围。
/// This trait wraps the standard library's `RangeBounds` trait,
/// allowing different range types to be handled at runtime.
pub trait DummyIndexRange: Debug + DynClone {
    /// 获取范围的起始边界
    /// Get the start bound of the range
    fn start(&self) -> Bound<&isize>;

    /// 获取范围的结束边界
    /// Get the end bound of the range
    fn end(&self) -> Bound<&isize>;

    /// 检查值是否在范围内
    /// Check if a value is contained in the range
    fn contains(&self, v: isize) -> bool;
}

clone_trait_object!(DummyIndexRange);

/// 为所有实现 `RangeBounds<isize>` 的类型提供 `DummyIndexRange` 实现
/// Provides `DummyIndexRange` implementation for all types implementing `RangeBounds<isize>`
impl<T> DummyIndexRange for T
where
    T: RangeBounds<isize> + Clone + Debug,
{
    fn start(&self) -> Bound<&isize> {
        RangeBounds::start_bound(self)
    }

    fn end(&self) -> Bound<&isize> {
        RangeBounds::end_bound(self)
    }

    fn contains(&self, value: isize) -> bool {
        RangeBounds::contains(self, &value)
    }
}

/// 虚拟索引迭代器
/// Dummy index iterator
///
/// 表示虚拟索引的迭代结果，支持三种模式：
/// Represents the iteration result of dummy indices, supporting three modes:
///
/// - `Single`: 单个索引
///   Single index
/// - `Continuous`: 连续范围索引
///   Continuous range indices
/// - `Discrete`: 离散索引集合
///   Discrete index collection
#[derive(Clone, Debug, PartialEq)]
pub enum DummyIndexIterator {
    /// 单个索引
    /// Single index
    Single(usize),

    /// 连续范围索引
    /// Continuous range indices
    Continuous(Range<usize>),

    /// 离散索引集合
    /// Discrete index collection
    Discrete(Vec<usize>),
}

impl DummyIndexIterator {
    /// 获取指定位置的索引值
    /// Get the index value at the specified position
    ///
    /// ## 参数 / Parameters
    ///
    /// - `i`: 位置索引
    ///   Position index
    ///
    /// ## 返回值 / Returns
    ///
    /// 返回该位置的索引值，如果超出范围则返回 `None`
    /// Returns the index value at that position, or `None` if out of range
    #[inline]
    pub fn get(&self, i: usize) -> Option<usize> {
        match self {
            DummyIndexIterator::Single(index) => {
                if i == 0 {
                    Some(*index)
                } else {
                    None
                }
            }
            DummyIndexIterator::Continuous(range) => {
                if i < range.end - range.start {
                    Some(range.start + i)
                } else {
                    None
                }
            }
            DummyIndexIterator::Discrete(indexes) => {
                if i < indexes.len() {
                    Some(indexes[i])
                } else {
                    None
                }
            }
        }
    }

    /// 获取迭代器的长度
    /// Get the length of the iterator
    #[inline]
    pub fn len(&self) -> usize {
        match self {
            DummyIndexIterator::Single(_) => 1,
            DummyIndexIterator::Continuous(range) => range.end - range.start,
            DummyIndexIterator::Discrete(vec) => vec.len(),
        }
    }

    /// 检查迭代器是否为空
    /// Check if the iterator is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// 迭代器向量 trait 别名
/// Iterator vector trait alias
///
/// 定义用于存储虚拟索引迭代器的向量类型。
/// Defines the vector type used to store dummy index iterators.
pub trait IteratorVector = Collection<Item = DummyIndexIterator>
    + Len
    + Indices
    + Index<usize, Output = DummyIndexIterator>
    + IndexMut<usize, Output = DummyIndexIterator>
    + Debug;

/// 虚拟索引
/// Dummy index
///
/// 表示多维数组的索引，支持三种形式：
/// Represents indices for multi-dimensional arrays, supporting three forms:
///
/// - `Index`: 单个索引，支持负数（从末尾计数）
///   Single index, supports negative numbers (counting from the end)
/// - `Range`: 范围索引，如 `0..5`, `1..=3`, `..`, `2..`
///   Range index, e.g., `0..5`, `1..=3`, `..`, `2..`
/// - `IndexArray`: 离散索引数组，如 `[0, 2, 4]`
///   Discrete index array, e.g., `[0, 2, 4]`
///
/// ## 示例 / Examples
///
/// ```rust
/// use ospf_rust_multiarray::dummy_index::DummyIndex;
///
/// // 单个索引
/// // Single index
/// let idx = DummyIndex::Index(5);
/// let neg_idx = DummyIndex::Index(-1);  // 最后一个元素 / Last element
///
/// // 范围索引
/// // Range index
/// let range = DummyIndex::Range(Box::new(0..5));
///
/// // 索引数组
/// // Index array
/// let arr = DummyIndex::IndexArray(vec![0, 2, 4]);
/// ```
#[derive(Debug, Clone)]
pub enum DummyIndex {
    /// 单个索引
    /// Single index
    Index(isize),

    /// 范围索引
    /// Range index
    Range(Box<dyn DummyIndexRange>),

    /// 索引数组
    /// Index array
    IndexArray(Vec<isize>),
}

/// DummyIndex 的相等性比较实现
/// Equality comparison implementation for DummyIndex
///
/// 注意：范围索引之间的比较始终返回 `false`，
/// 因为动态范围无法直接比较。
/// Note: Comparison between range indices always returns `false`,
/// because dynamic ranges cannot be directly compared.
impl PartialEq for DummyIndex {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (DummyIndex::Index(a), DummyIndex::Index(b)) => a == b,
            (DummyIndex::IndexArray(a), DummyIndex::IndexArray(b)) => a == b,

            _ => false,
        }
    }
}

impl DummyIndex {
    /// 计算虚拟索引在给定维度上的长度
    /// Calculate the length of dummy index in a given dimension
    ///
    /// ## 参数 / Parameters
    ///
    /// - `shape`: 形状引用
    ///   Reference to shape
    /// - `dimension`: 维度索引
    ///   Dimension index
    ///
    /// ## 返回值 / Returns
    ///
    /// 返回该维度上的索引数量
    /// Returns the number of indices in that dimension
    pub fn len_of<S: AbstractShape>(&self, shape: &S, dimension: usize) -> usize {
        match self {
            DummyIndex::Index(_) => 1,
            DummyIndex::Range(range) => {
                let (lower_bound, upper_bound) = Self::bound_of(range, shape, dimension);
                if lower_bound.is_some() && upper_bound.is_some() {
                    upper_bound.unwrap() - lower_bound.unwrap()
                } else {
                    shape.len_of_dimension(dimension).unwrap()
                }
            }
            DummyIndex::IndexArray(indexes) => indexes.len(),
        }
    }

    /// 将虚拟索引转换为迭代器
    /// Convert dummy index to an iterator
    ///
    /// ## 参数 / Parameters
    ///
    /// - `shape`: 形状引用
    ///   Reference to shape
    /// - `dimension`: 维度索引
    ///   Dimension index
    ///
    /// ## 返回值 / Returns
    ///
    /// 返回该维度上的索引迭代器
    /// Returns the index iterator for that dimension
    pub fn iterator_of<S: AbstractShape>(&self, shape: &S, dimension: usize) -> DummyIndexIterator {
        match self {
            DummyIndex::Index(index) => match shape.actual_index(dimension, *index) {
                Some(value) => DummyIndexIterator::Single(value),
                None => DummyIndexIterator::Single(0),
            },
            DummyIndex::Range(range) => {
                let (lower_bound, upper_bound) = Self::bound_of(range, shape, dimension);
                if lower_bound.is_some() && upper_bound.is_some() {
                    DummyIndexIterator::Continuous(Range {
                        start: lower_bound.unwrap(),
                        end: upper_bound.unwrap(),
                    })
                } else {
                    DummyIndexIterator::Continuous(Range { start: 0, end: 0 })
                }
            }
            DummyIndex::IndexArray(indexes) => DummyIndexIterator::Discrete(
                indexes
                    .iter()
                    .filter_map(move |index| shape.actual_index(dimension, *index))
                    .collect(),
            ),
        }
    }

    /// 计算范围索引的边界
    /// Calculate bounds for range index
    ///
    /// ## 参数 / Parameters
    ///
    /// - `range`: 范围引用
    ///   Reference to range
    /// - `shape`: 形状引用
    ///   Reference to shape
    /// - `dimension`: 维度索引
    ///   Dimension index
    ///
    /// ## 返回值 / Returns
    ///
    /// 返回下界和上界的元组
    /// Returns a tuple of lower bound and upper bound
    fn bound_of<S: AbstractShape>(
        range: &Box<dyn DummyIndexRange>,
        shape: &S,
        dimension: usize,
    ) -> (Option<usize>, Option<usize>) {
        let len = shape.len_of_dimension(dimension).unwrap();
        let len_isize = len as isize;
        let lower_bound = match range.start() {
            Bound::Included(value) => shape.actual_index(dimension, *value),
            Bound::Excluded(value) => shape.actual_index(dimension, value - 1),
            Bound::Unbounded => Some(0),
        };
        let upper_bound = match range.end() {
            Bound::Included(value) => shape.actual_index(dimension, value + 1),
            Bound::Excluded(value) => {
                if *value >= -len_isize && *value <= len_isize {
                    let actual = if *value >= 0 {
                        *value as usize
                    } else {
                        (len_isize + value) as usize
                    };
                    Some(actual)
                } else {
                    None
                }
            }
            Bound::Unbounded => Some(len),
        };
        (lower_bound, upper_bound)
    }
}

impl From<isize> for DummyIndex {
    fn from(value: isize) -> Self {
        Self::Index(value)
    }
}

impl From<&'_ isize> for DummyIndex {
    fn from(value: &'_ isize) -> Self {
        Self::Index(*value)
    }
}

impl From<usize> for DummyIndex {
    fn from(value: usize) -> Self {
        Self::Index(value.cast_signed())
    }
}

impl From<&'_ usize> for DummyIndex {
    fn from(value: &'_ usize) -> Self {
        Self::Index(value.cast_signed())
    }
}

impl<T> TryFrom<Range<T>> for DummyIndex
where
    T: TryIntoIndexValue,
{
    type Error = <T as TryIntoIndexValue>::Error;

    fn try_from(value: Range<T>) -> Result<Self, Self::Error> {
        Ok(Self::Range(Box::new(Range {
            start: value.start.try_into()?,
            end: value.end.try_into()?,
        })))
    }
}

impl<'a, T> TryFrom<&'a Range<T>> for DummyIndex
where
    &'a T: TryIntoIndexValue,
{
    type Error = <&'a T as TryIntoIndexValue>::Error;

    fn try_from(value: &'a Range<T>) -> Result<Self, Self::Error> {
        Ok(Self::Range(Box::new(Range {
            start: <&'a T as TryIntoIndexValue>::try_into(&value.start)?,
            end: <&'a T as TryIntoIndexValue>::try_into(&value.end)?,
        })))
    }
}

impl<T> TryFrom<RangeFrom<T>> for DummyIndex
where
    T: TryIntoIndexValue,
{
    type Error = <T as TryIntoIndexValue>::Error;

    fn try_from(value: RangeFrom<T>) -> Result<Self, Self::Error> {
        Ok(Self::Range(Box::new(RangeFrom {
            start: value.start.try_into()?,
        })))
    }
}

impl<'a, T> TryFrom<&'a RangeFrom<T>> for DummyIndex
where
    &'a T: TryIntoIndexValue,
{
    type Error = <&'a T as TryIntoIndexValue>::Error;

    fn try_from(value: &'a RangeFrom<T>) -> Result<Self, Self::Error> {
        Ok(Self::Range(Box::new(RangeFrom {
            start: <&'a T as TryIntoIndexValue>::try_into(&value.start)?,
        })))
    }
}

impl<T> TryFrom<RangeInclusive<T>> for DummyIndex
where
    T: TryIntoIndexValue,
{
    type Error = <T as TryIntoIndexValue>::Error;

    fn try_from(value: RangeInclusive<T>) -> Result<Self, Self::Error> {
        let (start, end) = value.into_inner();
        Ok(Self::Range(Box::new(RangeInclusive::<isize>::new(
            start.try_into()?,
            end.try_into()?,
        ))))
    }
}

impl<'a, T> TryFrom<&'a RangeInclusive<T>> for DummyIndex
where
    &'a T: TryIntoIndexValue,
{
    type Error = <&'a T as TryIntoIndexValue>::Error;

    fn try_from(value: &'a RangeInclusive<T>) -> Result<Self, Self::Error> {
        Ok(Self::Range(Box::new(RangeInclusive::<isize>::new(
            <&'a T as TryIntoIndexValue>::try_into(value.start())?,
            <&'a T as TryIntoIndexValue>::try_into(value.end())?,
        ))))
    }
}

impl<T> TryFrom<RangeTo<T>> for DummyIndex
where
    T: TryIntoIndexValue,
{
    type Error = <T as TryIntoIndexValue>::Error;

    fn try_from(value: RangeTo<T>) -> Result<Self, Self::Error> {
        Ok(Self::Range(Box::new(RangeTo {
            end: value.end.try_into()?,
        })))
    }
}

impl<'a, T> TryFrom<&'a RangeTo<T>> for DummyIndex
where
    &'a T: TryIntoIndexValue,
{
    type Error = <&'a T as TryIntoIndexValue>::Error;

    fn try_from(value: &'a RangeTo<T>) -> Result<Self, Self::Error> {
        Ok(Self::Range(Box::new(RangeTo {
            end: <&'a T as TryIntoIndexValue>::try_into(&value.end)?,
        })))
    }
}

impl<T> TryFrom<RangeToInclusive<T>> for DummyIndex
where
    T: TryIntoIndexValue,
{
    type Error = <T as TryIntoIndexValue>::Error;

    fn try_from(value: RangeToInclusive<T>) -> Result<Self, Self::Error> {
        Ok(Self::Range(Box::new(RangeToInclusive {
            end: value.end.try_into()?,
        })))
    }
}

impl<'a, T> TryFrom<&'a RangeToInclusive<T>> for DummyIndex
where
    &'a T: TryIntoIndexValue,
{
    type Error = <&'a T as TryIntoIndexValue>::Error;

    fn try_from(value: &'a RangeToInclusive<T>) -> Result<Self, Self::Error> {
        Ok(Self::Range(Box::new(RangeToInclusive {
            end: <&'a T as TryIntoIndexValue>::try_into(&value.end)?,
        })))
    }
}

impl From<RangeFull> for DummyIndex {
    fn from(value: RangeFull) -> Self {
        Self::Range(Box::new(RangeFull))
    }
}

impl<'a> From<&'a RangeFull> for DummyIndex {
    fn from(value: &'a RangeFull) -> Self {
        Self::Range(Box::new(RangeFull))
    }
}

impl<'a, T> TryFrom<&'a [T]> for DummyIndex
where
    &'a T: TryIntoIndexValue,
{
    type Error = <&'a T as TryIntoIndexValue>::Error;

    fn try_from(value: &'a [T]) -> Result<Self, Self::Error> {
        let mut ret = Vec::new();
        ret.reserve(value.len());
        for i in value.indices() {
            ret.push(<&'a T as TryIntoIndexValue>::try_into(&value[i])?);
        }
        Ok(Self::IndexArray(ret))
    }
}

impl<'a, T, A: Allocator> TryFrom<&'a Vec<T, A>> for DummyIndex
where
    &'a T: TryIntoIndexValue,
{
    type Error = <&'a T as TryIntoIndexValue>::Error;

    fn try_from(value: &'a Vec<T, A>) -> Result<Self, Self::Error> {
        Self::try_from(value.as_slice())
    }
}

/// 迭代推进策略 trait
/// Advance policy trait
///
/// 定义多维数组迭代时的推进策略，根据访问顺序决定维度遍历顺序。
/// Defines the advance policy for multi-dimensional array iteration,
/// determining the dimension traversal order based on access order.
///
/// ## 类型参数 / Type Parameters
///
/// - `S`: 形状类型，必须实现 `AbstractShape`
///   Shape type, must implement `AbstractShape`
pub trait AdvancePolicy<S: AbstractShape>: AccessOrderTrait {
    /// 初始化迭代器
    /// Initialize the iterator
    ///
    /// 设置当前向量中每个维度的初始值。
    /// Sets the initial value for each dimension in the current vector.
    ///
    /// ## 参数 / Parameters
    ///
    /// - `iterators`: 各维度的索引迭代器
    ///   Index iterators for each dimension
    /// - `current_vector`: 当前索引向量（输出参数）
    ///   Current index vector (output parameter)
    ///
    /// ## 返回值 / Returns
    ///
    /// 如果初始化成功返回 `true`，否则返回 `false`
    /// Returns `true` if initialization succeeded, `false` otherwise
    fn init_iterator<IV: IteratorVector>(
        &self,
        iterators: &IV,
        current_vector: &mut S::VectorType,
    ) -> bool;

    /// 推进迭代器
    /// Advance the iterator
    ///
    /// 将迭代器推进到下一个位置。
    /// Advances the iterator to the next position.
    ///
    /// ## 参数 / Parameters
    ///
    /// - `iterators`: 各维度的索引迭代器
    ///   Index iterators for each dimension
    /// - `current_positions`: 当前位置向量
    ///   Current position vector
    /// - `current_vector`: 当前索引向量（输出参数）
    ///   Current index vector (output parameter)
    ///
    /// ## 返回值 / Returns
    ///
    /// 如果推进成功返回 `true`，如果已遍历完毕返回 `false`
    /// Returns `true` if advance succeeded, `false` if iteration is complete
    fn advance<IV: IteratorVector>(
        &self,
        iterators: &IV,
        current_positions: &mut S::VectorType,
        current_vector: &mut S::VectorType,
    ) -> bool;
}

/// 行优先迭代推进策略
/// Row-major iteration advance policy
///
/// 从最后一个维度开始遍历，依次向前推进。
/// Starts traversing from the last dimension, advancing forward.
///
/// 例如对于 2x3 数组，遍历顺序为：(0,0), (0,1), (0,2), (1,0), (1,1), (1,2)
/// E.g., for a 2x3 array, traversal order is: (0,0), (0,1), (0,2), (1,0), (1,1), (1,2)
impl<S: AbstractShape> AdvancePolicy<S> for RowMajor {
    fn init_iterator<IV: IteratorVector>(
        &self,
        iterators: &IV,
        current_vector: &mut S::VectorType,
    ) -> bool {
        // 行优先：从后向前初始化
        // Row-major: initialize from back to front
        for i in (0..iterators.len()).rev() {
            match iterators[i].get(0) {
                Some(val) => current_vector[i] = val,
                None => return false,
            }
        }
        true
    }

    fn advance<IV: IteratorVector>(
        &self,
        iterators: &IV,
        current_positions: &mut S::VectorType,
        current_vector: &mut S::VectorType,
    ) -> bool {
        // 行优先：从后向前推进
        // Row-major: advance from back to front
        for i in (0..iterators.len()).rev() {
            current_positions[i] += 1;
            if let Some(val) = iterators[i].get(current_positions[i]) {
                current_vector[i] = val;
                return true;
            }

            // 当前维度已遍历完毕，重置并继续推进前一维度
            // Current dimension exhausted, reset and continue with previous dimension
            current_positions[i] = 0;
            if let Some(val) = iterators[i].get(0) {
                current_vector[i] = val;
            }
        }
        false
    }
}

/// 列优先迭代推进策略
/// Column-major iteration advance policy
///
/// 从第一个维度开始遍历，依次向后推进。
/// Starts traversing from the first dimension, advancing backward.
///
/// 例如对于 2x3 数组，遍历顺序为：(0,0), (1,0), (0,1), (1,1), (0,2), (1,2)
/// E.g., for a 2x3 array, traversal order is: (0,0), (1,0), (0,1), (1,1), (0,2), (1,2)
impl<S: AbstractShape> AdvancePolicy<S> for ColumnMajor {
    fn init_iterator<IV: IteratorVector>(
        &self,
        iterators: &IV,
        current_vector: &mut S::VectorType,
    ) -> bool {
        // 列优先：从前向后初始化
        // Column-major: initialize from front to back
        for i in 0..iterators.len() {
            match iterators[i].get(0) {
                Some(val) => current_vector[i] = val,
                None => return false,
            }
        }
        true
    }

    fn advance<IV: IteratorVector>(
        &self,
        iterators: &IV,
        current_positions: &mut S::VectorType,
        current_vector: &mut S::VectorType,
    ) -> bool {
        // 列优先：从前向后推进
        // Column-major: advance from front to back
        for i in 0..iterators.len() {
            current_positions[i] += 1;
            if let Some(val) = iterators[i].get(current_positions[i]) {
                current_vector[i] = val;
                return true;
            }

            // 当前维度已遍历完毕，重置并继续推进下一维度
            // Current dimension exhausted, reset and continue with next dimension
            current_positions[i] = 0;
            if let Some(val) = iterators[i].get(0) {
                current_vector[i] = val;
            }
        }
        false
    }
}

/// 访问顺序枚举的迭代推进策略实现
/// Advance policy implementation for access order enum
///
/// 根据运行时访问顺序委托给具体实现。
/// Delegates to specific implementation based on runtime access order.
impl<S: AbstractShape> AdvancePolicy<S> for AccessOrder {
    fn init_iterator<IV: IteratorVector>(
        &self,
        iterators: &IV,
        current_vector: &mut S::VectorType,
    ) -> bool {
        match self {
            AccessOrder::RowMajor => {
                <RowMajor as AdvancePolicy<S>>::init_iterator(&RowMajor, iterators, current_vector)
            }
            AccessOrder::ColumnMajor => <ColumnMajor as AdvancePolicy<S>>::init_iterator(
                &ColumnMajor,
                iterators,
                current_vector,
            ),
        }
    }

    fn advance<IV: IteratorVector>(
        &self,
        iterators: &IV,
        current_positions: &mut S::VectorType,
        current_vector: &mut S::VectorType,
    ) -> bool {
        match self {
            AccessOrder::RowMajor => <RowMajor as AdvancePolicy<S>>::advance(
                &RowMajor,
                iterators,
                current_positions,
                current_vector,
            ),
            AccessOrder::ColumnMajor => <ColumnMajor as AdvancePolicy<S>>::advance(
                &ColumnMajor,
                iterators,
                current_positions,
                current_vector,
            ),
        }
    }
}

/// 虚拟访问迭代器
/// Dummy access iterator
///
/// 用于遍历多维数组中由虚拟索引指定的元素子集。
/// Used to iterate over a subset of elements in a multi-dimensional array
/// specified by dummy indices.
///
/// ## 类型参数 / Type Parameters
///
/// - `'a`: 形状的生命周期
///   Lifetime of the shape
/// - `S`: 形状类型，必须实现 `AbstractShape`
///   Shape type, must implement `AbstractShape`
/// - `AO`: 访问顺序类型，默认为 `AccessOrder`
///   Access order type, defaults to `AccessOrder`
///
/// **注意**: 此结构体是内部实现细节，不对外公开。
/// **Note**: This struct is an internal implementation detail and is not public.
pub(crate) struct DummyAccessIterator<'a, S: AbstractShape, AO: AccessOrderTrait = AccessOrder>
where
    S: 'a,
{
    /// 形状引用
    /// Reference to shape
    shape: &'a S,

    /// 各维度的索引迭代器
    /// Index iterators for each dimension
    iterators: S::IteratorVectorType,

    /// 当前位置向量（记录每个维度已迭代的次数）
    /// Current position vector (records iteration count for each dimension)
    current_positions: S::VectorType,

    /// 当前索引向量（当前迭代位置的实际索引值）
    /// Current index vector (actual index values at current iteration position)
    current_vector: S::VectorType,

    /// 是否已完成迭代
    /// Whether iteration is complete
    finished: bool,

    /// 是否是第一次调用 next()
    /// Whether this is the first call to next()
    first_call: bool,

    /// 访问顺序
    /// Access order
    access_order: AO,
}

impl<'a, S: AbstractShape, AO: AccessOrderTrait + AdvancePolicy<S>> DummyAccessIterator<'a, S, AO> {
    /// 创建新的虚拟访问迭代器
    /// Create a new dummy access iterator
    ///
    /// ## 参数 / Parameters
    ///
    /// - `shape`: 形状引用
    ///   Reference to shape
    /// - `iterators`: 各维度的索引迭代器
    ///   Index iterators for each dimension
    /// - `access_order`: 访问顺序
    ///   Access order
    pub(crate) fn new(shape: &'a S, iterators: S::IteratorVectorType, access_order: AO) -> Self {
        let current_positions = shape.zero();
        let current_vector = shape.zero();
        Self {
            shape,
            iterators,
            current_positions,
            current_vector,
            finished: false,
            first_call: true,
            access_order,
        }
    }

    /// 初始化迭代器
    /// Initialize the iterator
    fn init_iterator(&mut self) -> bool {
        self.current_vector = self.shape.zero();
        <AO as AdvancePolicy<S>>::init_iterator(
            &self.access_order,
            &self.iterators,
            &mut self.current_vector,
        )
    }

    /// 获取下一个索引向量
    /// Get the next index vector
    ///
    /// ## 返回值 / Returns
    ///
    /// 返回下一个索引向量的引用，如果迭代完成则返回 `None`
    /// Returns a reference to the next index vector, or `None` if iteration is complete
    pub(crate) fn next(&mut self) -> Option<&S::VectorType> {
        if self.finished {
            return None;
        }

        // 第一次调用时初始化迭代器
        // Initialize iterator on first call
        if self.first_call {
            self.first_call = false;
            if !self.init_iterator() {
                self.finished = true;
                return None;
            }
            return Some(&self.current_vector);
        }

        // 推进到下一个位置
        // Advance to next position
        let advanced = <AO as AdvancePolicy<S>>::advance(
            &self.access_order,
            &self.iterators,
            &mut self.current_positions,
            &mut self.current_vector,
        );

        if advanced {
            Some(&self.current_vector)
        } else {
            self.finished = true;
            None
        }
    }
}

/// 虚拟索引值宏
/// Dummy index value macro
///
/// 用于将字面量转换为 `isize` 类型，支持负数。
/// Used to convert literals to `isize` type, supporting negative numbers.
///
/// ## 示例 / Examples
///
/// ```ignore
/// dummy_index_value!(5)    // 5isize
/// dummy_index_value!(-5)   // -5isize
/// dummy_index_value!(x)    // x (表达式不变)
/// ```
#[macro_export]
macro_rules! dummy_index_value {
    // 负数字面量：转换为负 isize
    // Negative literal: convert to negative isize
    (-$x:literal) => {
        -paste! { [<$x isize>] }
    };
    // 正数字面量：转换为 isize
    // Positive literal: convert to isize
    ($x:literal) => {
        paste! { [<$x isize>] }
    };
    // 表达式：保持不变
    // Expression: keep as is
    ($x:expr) => {
        $x
    };
}

/// 虚拟索引创建宏
/// Dummy index creation macro
///
/// 用于便捷地创建 `DummyIndex` 实例。
/// Used to conveniently create `DummyIndex` instances.
///
/// ## 支持的语法 / Supported Syntax
///
/// - 单个索引：`dummy_index!(5)`, `dummy_index!(-1)`
///   Single index: `dummy_index!(5)`, `dummy_index!(-1)`
/// - 范围索引：`dummy_index!(0..5)`, `dummy_index!(1..=3)`, `dummy_index!(..)`
///   Range index: `dummy_index!(0..5)`, `dummy_index!(1..=3)`, `dummy_index!(..)`
/// - 索引数组：`dummy_index!([0, 2, 4])`
///   Index array: `dummy_index!([0, 2, 4])`
///
/// ## 返回值 / Returns
///
/// 返回 `Result<DummyIndex, _>`，需要处理可能的错误。
/// Returns `Result<DummyIndex, _>`, requires handling potential errors.
#[macro_export]
macro_rules! dummy_index {
    // 负数单个索引
    // Negative single index
    (-$x:literal) => {
        DummyIndex::try_from(dummy_index_value!{ -$x })
    };
    // 正数单个索引
    // Positive single index
    ($x:literal) => {
        DummyIndex::try_from(dummy_index_value!{ $x })
    };
    // 范围：start..-end
    ($start:literal..-$end:literal) => {
        DummyIndex::try_from(dummy_index_value!{ $start }..dummy_index_value!{ -$end })
    };
    // 范围：start..end
    ($start:literal..$end:literal) => {
        DummyIndex::try_from(dummy_index_value!{ $start }..dummy_index_value!{ $end })
    };
    // 范围：start..
    ($start:literal..) => {
        DummyIndex::try_from(dummy_index_value!{ $start }..)
    };
    // 范围：..end
    (..$end:literal) => {
        DummyIndex::try_from(..dummy_index_value!{ $end })
    };
    // 范围：..-end
    (..-$end:literal) => {
        DummyIndex::try_from(..dummy_index_value!{ -$end })
    };
    // 范围：start..=-end
    ($start:literal..=-$end:literal) => {
        DummyIndex::try_from(dummy_index_value!{ $start }..=dummy_index_value!{ -$end })
    };
    // 范围：start..=end
    ($start:literal..=$end:literal) => {
        DummyIndex::try_from(dummy_index_value!{ $start }..=dummy_index_value!{ $end })
    };
    // 范围：..=-end
    (..=-$end:literal) => {
        DummyIndex::try_from(..=dummy_index_value!{ -$end })
    };
    // 范围：..=end
    (..=$end:literal) => {
        DummyIndex::try_from(..=dummy_index_value!{ $end })
    };
    // 全范围
    // Full range
    (..) => {
        DummyIndex::try_from(..)
    };
    // 索引数组
    // Index array
    [$($x:literal),*] => {
        DummyIndex::try_from(&[$(dummy_index_value!{ $x }),*])
    };
    // 表达式
    // Expression
    ($x:expr) => {{
        DummyIndex::try_from(&$x)
    }};
}

/// 虚拟索引数组创建宏
/// Dummy index array creation macro
///
/// 创建虚拟索引数组，错误转换为 `InvalidDummyIndexError`。
/// Creates dummy index array, errors converted to `InvalidDummyIndexError`.
///
/// ## 示例 / Examples
///
/// ```ignore
/// let indices = dummy![0, 1..5, [2, 4]];  // 创建索引数组
/// let view = dummy!(array[0..2, 3..5]);  // 创建数组视图
/// ```
#[macro_export]
macro_rules! dummy {
    // 创建索引数组
    // Create index array
    [$($x:expr),*] => {
        [$(dummy_index!{ $x }.map_err(|e| error! { InvalidDummyIndexError {} })?),*]
    };
    // 创建数组视图
    // Create array view
    ($a:ident[$($x:expr),*]) => {
        $a.view(dummy![$($x),*])
    };
}

/// 带详细错误的虚拟索引数组创建宏
/// Dummy index array creation macro with detailed error
///
/// 与 `dummy!` 类似，但保留原始错误信息。
/// Similar to `dummy!`, but preserves original error information.
#[macro_export]
macro_rules! dummy_with_err {
    [$($x:expr),*] => {
        [$(dummy_index!{ $x }.map_err(|e| error! { ExInvalidDummyIndexError { origin: e } })?),*]
    };
    ($a:ident[$($x:expr),*]) => {
        $a.view(dummy_with_err![$($x),*])
    };
}

/// 期望成功的虚拟索引数组创建宏
/// Expect-success dummy index array creation macro
///
/// 创建虚拟索引数组，如果失败则 panic。
/// Creates dummy index array, panics on failure.
///
/// ## 示例 / Examples
///
/// ```ignore
/// let indices = dummy_expect![0, 1..5];           // 失败时 unwrap
/// let indices = dummy_expect!([0, 1..5], "msg");  // 失败时 expect
/// let view = dummy_expect!(array[0..2, 3..5]);   // 创建视图
/// ```
#[macro_export]
macro_rules! dummy_expect {
    ([$($x:expr),*], $msg:expr) => {
        [$(dummy_index!{ $x }.expect($msg)),*]
    };
    [$($x:expr),*] => {
        [$(dummy_index!{ $x }.unwrap()),*]
    };
    ($a:ident[$($x:expr),*], $msg:expr) => {
        $a.view(dummy_expect!{ [$($x),*], $msg })
    };
    ($a:ident[$($x:expr),*]) => {
        $a.view(dummy_expect![$($x),*])
    };
}

/// 动态虚拟索引数组创建宏
/// Dynamic dummy index array creation macro
///
/// 创建 `Vec<DummyIndex>` 而非数组。
/// Creates `Vec<DummyIndex>` instead of an array.
#[macro_export]
macro_rules! dyn_dummy {
    [$($x:expr),*] => {
        vec![$(dummy_index!{ $x }.map_err(|e| error! { InvalidDummyIndexError {} })?),*]
    };
    ($a:ident[$($x:expr),*]) => {
        $a.view(dyn_dummy![$($x),*])
    };
}

/// 带详细错误的动态虚拟索引数组创建宏
/// Dynamic dummy index array creation macro with detailed error
#[macro_export]
macro_rules! dyn_dummy_with_err {
    [$($x:expr),*] => {
        [$(dummy_index!{ $x }.map_err(|e| error! { ExInvalidDummyIndexError { origin: e } })?),*]
    };
    ($a:ident[$($x:expr),*]) => {
        $a.view(dyn_dummy_with_err![$($x),*])
    };
}

/// 期望成功的动态虚拟索引数组创建宏
/// Expect-success dynamic dummy index array creation macro
#[macro_export]
macro_rules! dyn_dummy_expect {
    ([$($x:expr),*], $msg:expr) => {
        vec![$(dummy_index!{ $x }.expect($msg)),*]
    };
    [$($x:expr),*] => {
        vec![$(dummy_index!($x).unwrap(),)*]
    };
    ($a:ident[$($x:expr),*], $msg:expr) => {
        $a.view(dyn_dummy_expect!{ [$($x),*], $msg })
    };
    ($a:ident[$($x:expr),*]) => {
        $a.view(dyn_dummy_expect![$($x),*])
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shape::Shape;
    use paste::paste;

    #[test]
    fn test_dummy_index_from_primitives() {
        let idx: DummyIndex = dummy_index!(5).expect("invalid dummy index");
        assert!(matches!(idx, DummyIndex::Index(5)));

        let idx: DummyIndex = dummy_index!(10).expect("invalid dummy index");
        assert!(matches!(idx, DummyIndex::Index(10)));
    }

    #[test]
    fn test_dummy_index_from_ranges() {
        let idx: DummyIndex = dummy_index!(2..5).expect("invalid dummy index");
        match idx {
            DummyIndex::Range(boxed) => {
                assert_eq!(boxed.start(), Bound::Included(&2));
                assert_eq!(boxed.end(), Bound::Excluded(&5));
                assert!(boxed.contains(3));
                assert!(!boxed.contains(1));
                assert!(!boxed.contains(5));
            }
            _ => panic!("Expected DummyIndex::Range"),
        }

        let idx: DummyIndex = dummy_index!(3..).expect("invalid dummy index");
        match idx {
            DummyIndex::Range(boxed) => {
                assert_eq!(boxed.start(), Bound::Included(&3));
                assert_eq!(boxed.end(), Bound::Unbounded);
                assert!(boxed.contains(100));
                assert!(!boxed.contains(2));
            }
            _ => panic!("Expected DummyIndex::Range"),
        }

        let idx: DummyIndex = dummy_index!(1..=4).expect("invalid dummy index");
        match idx {
            DummyIndex::Range(boxed) => {
                assert_eq!(boxed.start(), Bound::Included(&1));
                assert_eq!(boxed.end(), Bound::Included(&4));
                assert!(boxed.contains(2));
                assert!(boxed.contains(4));
                assert!(!boxed.contains(5));
            }
            _ => panic!("Expected DummyIndex::Range"),
        }

        let idx: DummyIndex = dummy_index!(..5).expect("invalid dummy index");
        match idx {
            DummyIndex::Range(boxed) => {
                assert_eq!(boxed.start(), Bound::Unbounded);
                assert_eq!(boxed.end(), Bound::Excluded(&5));
                assert!(boxed.contains(0));
                assert!(!boxed.contains(5));
            }
            _ => panic!("Expected DummyIndex::Range"),
        }

        let idx: DummyIndex = dummy_index!(..=5).expect("invalid dummy index");
        match idx {
            DummyIndex::Range(boxed) => {
                assert_eq!(boxed.start(), Bound::Unbounded);
                assert_eq!(boxed.end(), Bound::Included(&5));
                assert!(boxed.contains(5));
                assert!(!boxed.contains(6));
            }
            _ => panic!("Expected DummyIndex::Range"),
        }

        let idx: DummyIndex = dummy_index!(..).expect("invalid dummy index");
        match idx {
            DummyIndex::Range(boxed) => {
                assert_eq!(boxed.start(), Bound::Unbounded);
                assert_eq!(boxed.end(), Bound::Unbounded);
                assert!(boxed.contains(-100));
                assert!(boxed.contains(100));
            }
            _ => panic!("Expected DummyIndex::Range"),
        }
    }

    #[test]
    fn test_dummy_index_from_slice_and_vec() {
        let arr: &[isize] = &[1, 2, 3];
        let idx: DummyIndex = arr.try_into().expect("invalid dummy index");
        match idx {
            DummyIndex::IndexArray(vec) => assert_eq!(vec, vec![1, 2, 3]),
            _ => panic!("Expected DummyIndex::IndexArray"),
        }

        let vec: Vec<isize> = vec![4, 5, 6];
        let idx: DummyIndex = (&vec).try_into().expect("invalid dummy index");
        match idx {
            DummyIndex::IndexArray(vec) => assert_eq!(vec, vec![4, 5, 6]),
            _ => panic!("Expected DummyIndex::IndexArray"),
        }
    }

    #[test]
    fn test_dummy_index_iterator_get() {
        let iter = DummyIndexIterator::Single(5);
        assert_eq!(iter.get(0), Some(5));
        assert_eq!(iter.get(1), None);

        let iter = DummyIndexIterator::Continuous(2..5);
        assert_eq!(iter.get(0), Some(2));
        assert_eq!(iter.get(1), Some(3));
        assert_eq!(iter.get(2), Some(4));
        assert_eq!(iter.get(3), None);

        let iter = DummyIndexIterator::Discrete(vec![0, 2, 4]);
        assert_eq!(iter.get(0), Some(0));
        assert_eq!(iter.get(1), Some(2));
        assert_eq!(iter.get(2), Some(4));
        assert_eq!(iter.get(3), None);
    }

    #[test]
    fn test_dummy_index_iterator_len() {
        let iter = DummyIndexIterator::Single(5);
        assert_eq!(iter.len(), 1);

        let iter = DummyIndexIterator::Continuous(2..5);
        assert_eq!(iter.len(), 3);

        let iter = DummyIndexIterator::Discrete(vec![0, 2, 4]);
        assert_eq!(iter.len(), 3);
    }

    #[test]
    fn test_iterator_of_for_index() {
        let shape: Shape<2> = Shape::new([5, 3]);

        let idx = DummyIndex::Index(2);
        let iter = idx.iterator_of(&shape, 0);
        assert_eq!(iter.get(0), Some(2));

        let idx = DummyIndex::Index(-1);
        let iter = idx.iterator_of(&shape, 0);
        assert_eq!(iter.get(0), Some(4));
    }

    #[test]
    fn test_dummy_access_iterator_column_major() {
        let shape: Shape<2> = Shape::new([2, 3]);
        let dummy_vec: [DummyIndex; 2] = dummy_expect![0..2, 1..3];
        let iterators = shape.dummy_to_iterator_vector(&dummy_vec);

        let mut iter = DummyAccessIterator::new(&shape, iterators, AccessOrder::ColumnMajor);

        let mut vectors = Vec::new();
        while let Some(vec) = iter.next() {
            vectors.push(vec.clone());
        }

        assert_eq!(vectors.len(), 4);

        assert_eq!(vectors[0], [0, 1]);
        assert_eq!(vectors[1], [1, 1]);
        assert_eq!(vectors[2], [0, 2]);
        assert_eq!(vectors[3], [1, 2]);
    }

    #[test]
    fn test_dummy_index_from_isize() {
        let idx: DummyIndex = DummyIndex::try_from(5isize).expect("invalid dummy index");
        assert!(matches!(idx, DummyIndex::Index(5)));
    }

    #[test]
    fn test_dummy_index_from_usize() {
        let idx: DummyIndex = DummyIndex::try_from(10usize).expect("invalid dummy index");
        assert!(matches!(idx, DummyIndex::Index(10)));
    }

    #[test]
    fn test_dummy_index_range_full() {
        let idx: DummyIndex = DummyIndex::try_from(..).expect("invalid dummy index");
        assert!(matches!(idx, DummyIndex::Range(_)));
    }

    #[test]
    fn test_dummy_index_range_to() {
        let idx: DummyIndex = DummyIndex::try_from(..5).expect("invalid dummy index");
        assert!(matches!(idx, DummyIndex::Range(_)));
    }

    #[test]
    fn test_dummy_index_range_from() {
        let idx: DummyIndex = DummyIndex::try_from(3..).expect("invalid dummy index");
        assert!(matches!(idx, DummyIndex::Range(_)));
    }

    #[test]
    fn test_dummy_index_range_inclusive() {
        let idx: DummyIndex = DummyIndex::try_from(1..=4).expect("invalid dummy index");
        assert!(matches!(idx, DummyIndex::Range(_)));
    }

    #[test]
    fn test_dummy_index_range_to_inclusive() {
        let idx: DummyIndex = DummyIndex::try_from(..=5).expect("invalid dummy index");
        assert!(matches!(idx, DummyIndex::Range(_)));
    }

    #[test]
    fn test_dummy_index_array() {
        let arr: &[isize] = &[1, 2, 3];
        let idx: DummyIndex = arr.try_into().expect("invalid dummy index");
        assert!(matches!(idx, DummyIndex::IndexArray(_)));
    }

    #[test]
    fn test_dummy_index_iterator_single() {
        let iter = DummyIndexIterator::Single(5);
        assert_eq!(iter.get(0), Some(5));
        assert_eq!(iter.get(1), None);
        assert_eq!(iter.len(), 1);
    }

    #[test]
    fn test_dummy_index_iterator_continuous() {
        let iter = DummyIndexIterator::Continuous(2..5);
        assert_eq!(iter.get(0), Some(2));
        assert_eq!(iter.get(1), Some(3));
        assert_eq!(iter.get(2), Some(4));
        assert_eq!(iter.get(3), None);
        assert_eq!(iter.len(), 3);
    }

    #[test]
    fn test_dummy_index_iterator_discrete() {
        let iter = DummyIndexIterator::Discrete(vec![0, 2, 4]);
        assert_eq!(iter.get(0), Some(0));
        assert_eq!(iter.get(1), Some(2));
        assert_eq!(iter.get(2), Some(4));
        assert_eq!(iter.get(3), None);
        assert_eq!(iter.len(), 3);
    }

    #[test]
    fn test_dummy_index_iterator_empty() {
        let iter = DummyIndexIterator::Continuous(2..2);
        assert!(iter.is_empty());
        assert_eq!(iter.len(), 0);
    }

    #[test]
    fn test_advance_policy_row_major() {
        let shape: Shape<2> = Shape::new([2, 3]);
        let policy = RowMajor;

        let mut current_positions = [0, 0];
        let mut current_vector = [0, 0];
        let dummy_vec: [DummyIndex; 2] = dummy_expect![0..2, 0..3];
        let iterators = shape.dummy_to_iterator_vector(&dummy_vec);

        assert!(<RowMajor as AdvancePolicy<Shape<2>>>::init_iterator::<
            [DummyIndexIterator; 2],
        >(&policy, &iterators, &mut current_vector));

        let mut count = 0;
        while <RowMajor as AdvancePolicy<Shape<2>>>::advance::<[DummyIndexIterator; 2]>(
            &policy,
            &iterators,
            &mut current_positions,
            &mut current_vector,
        ) {
            count += 1;
        }
        assert_eq!(count, 5);
    }

    #[test]
    fn test_advance_policy_column_major() {
        let shape: Shape<2> = Shape::new([2, 3]);
        let policy = ColumnMajor;

        let mut current_positions = [0, 0];
        let mut current_vector = [0, 0];
        let dummy_vec: [DummyIndex; 2] = dummy_expect![0..2, 0..3];
        let iterators = shape.dummy_to_iterator_vector(&dummy_vec);

        assert!(<ColumnMajor as AdvancePolicy<Shape<2>>>::init_iterator::<
            [DummyIndexIterator; 2],
        >(&policy, &iterators, &mut current_vector));

        let mut count = 0;
        while <ColumnMajor as AdvancePolicy<Shape<2>>>::advance::<[DummyIndexIterator; 2]>(
            &policy,
            &iterators,
            &mut current_positions,
            &mut current_vector,
        ) {
            count += 1;
        }
        assert_eq!(count, 5);
    }

    #[test]
    fn test_dummy_index_negative() {
        let shape: Shape<2> = Shape::new([3, 4]);
        let idx = DummyIndex::Index(-1);
        let result = idx.iterator_of(&shape, 0);
        assert_eq!(result.get(0), Some(2));
    }

    #[test]
    fn test_dummy_index_len_of() {
        let shape: Shape<2> = Shape::new([3, 4]);

        let idx_full = DummyIndex::Range(Box::new(..));
        assert_eq!(idx_full.len_of(&shape, 0), 3);
        assert_eq!(idx_full.len_of(&shape, 1), 4);

        let idx_single = DummyIndex::Index(1);
        assert_eq!(idx_single.len_of(&shape, 0), 1);
    }

    #[test]
    fn test_iterator_of_for_index_array() {
        let shape: Shape<2> = Shape::new([5, 3]);

        let idx: DummyIndex = dummy_index! { vec![0, 2, 4] }.expect("invalid dummy index");
        let iter = idx.iterator_of(&shape, 0);
        match iter {
            DummyIndexIterator::Discrete(vec) => {
                assert_eq!(vec, vec![0, 2, 4]);
            }
            _ => panic!("Expected Discrete iterator"),
        }
    }

    #[test]
    fn test_dummy_access_iterator() {
        let shape: Shape<2> = Shape::new([2, 3]);
        let dummy_vec: [DummyIndex; 2] = dummy_expect![0..2, 1..3];
        let iterators = shape.dummy_to_iterator_vector(&dummy_vec);

        let mut iter = DummyAccessIterator::new(&shape, iterators, AccessOrder::RowMajor);

        let mut vectors = Vec::new();
        while let Some(vec) = iter.next() {
            vectors.push(vec.clone());
        }

        assert_eq!(vectors.len(), 4);
        assert!(vectors.contains(&[0, 1]));
        assert!(vectors.contains(&[0, 2]));
        assert!(vectors.contains(&[1, 1]));
        assert!(vectors.contains(&[1, 2]));
    }

    #[test]
    fn test_dummy_macros() {
        let arr = dummy_expect![0, 1..3, vec![2, 4]];
        assert_eq!(arr.len(), 3);

        match &arr[0] {
            DummyIndex::Index(0) => {}
            _ => panic!("Expected Index(0)"),
        }

        match &arr[1] {
            DummyIndex::Range(_) => {}
            _ => panic!("Expected Range"),
        }

        match &arr[2] {
            DummyIndex::IndexArray(vec) => assert_eq!(vec, &[2, 4]),
            _ => panic!("Expected IndexArray"),
        }
    }
}

pub use dummy;
pub use dummy_expect;
pub use dummy_with_err;
pub use dyn_dummy;
pub use dyn_dummy_expect;
pub use dyn_dummy_with_err;
pub use paste::paste;
