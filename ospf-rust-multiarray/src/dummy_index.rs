//! # DummyIndex - 虚拟索引实现
//!
//! ## Overview / 概述
//!
//! This module provides the `DummyIndex` type for array slicing operations.
//! Dummy indices allow for flexible indexing including:
//! - Single index selection / 单索引选择
//! - Range-based slicing / 基于范围的切片
//! - Index array selection / 索引数组选择
//!
//! 本模块提供 `DummyIndex` 类型用于数组切片操作。
//! 虚拟索引允许灵活的索引，包括：
//! - 单索引选择
//! - 基于范围的切片
//! - 索引数组选择
//!
//! ## Key Types / 主要类型
//!
//! - `DummyIndex` - The main dummy index type / 主要虚拟索引类型
//! - `DummyIndexIterator` - Iterator for dummy indices / 虚拟索引迭代器
//! - `DummyAccessIterator` - Iterator that uses dummy access policy / 使用虚拟访问策略的迭代器

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
use std::marker::PhantomData;
use std::mem;
use std::ops::{
    Bound, Index, IndexMut, Range, RangeBounds, RangeFrom, RangeFull, RangeInclusive, RangeTo,
    RangeToInclusive,
};

/// # DummyIndexRange Trait
///
/// A trait for types that can be used as a range in dummy indexing.
/// It extends `RangeBounds<isize>` with dynamic cloning capability.
///
/// 用于可用作虚拟索引中范围的类型的特征。
/// 它将 `RangeBounds<isize>` 扩展为支持动态克隆。
pub trait DummyIndexRange: Debug + DynClone {
    /// Get the start bound of the range.
    ///
    /// 获取范围的起始边界。
    fn start(&self) -> Bound<&isize>;

    /// Get the end bound of the range.
    ///
    /// 获取范围的结束边界。
    fn end(&self) -> Bound<&isize>;

    /// Check if a value is contained in the range.
    ///
    /// 检查值是否包含在范围内。
    fn contains(&self, v: isize) -> bool;
}

clone_trait_object!(DummyIndexRange);

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

/// # DummyIndexIterator
///
/// An iterator type for dummy indices. It can be either continuous (for ranges)
/// or discrete (for index arrays).
///
/// 虚拟索引的迭代器类型。它可以是连续的（用于范围）或离散的（用于索引数组）。
#[derive(Clone, Debug)]
pub enum DummyIndexIterator {
    /// Single index iterator / 单索引迭代器
    Single(usize),
    /// Continuous range iterator / 连续范围迭代器
    Continuous(Range<usize>),
    /// Discrete index array iterator / 离散索引数组迭代器
    Discrete(Vec<usize>),
}

impl DummyIndexIterator {
    /// Get the value at a specific position.
    ///
    /// 获取指定位置的值。
    ///
    /// # Parameters / 参数
    ///
    /// - `i` - The position to get / 要获取的位置
    ///
    /// # Returns / 返回值
    ///
    /// - `Some(usize)` - The value at the position / 该位置的值
    /// - `None` - If the position is out of bounds / 如果位置越界
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

    /// Get the length of the iterator.
    ///
    /// 获取迭代器的长度。
    ///
    /// # Returns / 返回值
    ///
    /// The number of indices / 索引数量
    #[inline]
    pub fn len(&self) -> usize {
        match self {
            DummyIndexIterator::Single(_) => 1,
            DummyIndexIterator::Continuous(range) => range.end - range.start,
            DummyIndexIterator::Discrete(vec) => vec.len(),
        }
    }

    /// Check if the iterator is empty.
    ///
    /// 检查迭代器是否为空。
    ///
    /// # Returns / 返回值
    ///
    /// `true` if the iterator is empty / 如果迭代器为空则返回 `true`
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// # IteratorVector Trait
///
/// A trait for vectors of dummy index iterators.
/// Used for abstracting over fixed-size arrays and dynamic vectors.
///
/// 虚拟索引迭代器向量的特征。
/// 用于抽象固定大小数组和动态向量。
///
/// ## Implementors / 实现者
///
/// - `[DummyIndexIterator; D]` - Fixed-size arrays / 固定大小数组
/// - `Vec<DummyIndexIterator>` - Dynamic vectors / 动态向量
pub trait IteratorVector = Collection<Item = DummyIndexIterator>
    + Len
    + Indices
    + Index<usize, Output = DummyIndexIterator>
    + IndexMut<usize, Output = DummyIndexIterator>
    + Debug;

/// # DummyIndex
///
/// A type representing a single dimension's index specification in array slicing.
/// It supports three forms:
/// - `Index(isize)` - A single index (supports negative indexing) / 单个索引（支持负索引）
/// - `Range(Box<dyn DummyIndexRange>)` - A range of indices / 索引范围
/// - `IndexArray(Vec<isize>)` - An array of specific indices / 特定索引数组
///
/// 表示数组切片中单个维度索引规范的类型。
/// 它支持三种形式：
/// - `Index(isize)` - 单个索引（支持负索引）
/// - `Range(Box<dyn DummyIndexRange>)` - 索引范围
/// - `IndexArray(Vec<isize>)` - 特定索引数组
///
/// ## Example / 示例
///
/// ```rust
/// use ospf_rust_multiarray::*;
///
/// // Single index / 单个索引
/// let idx = DummyIndex::Index(5);
///
/// // Range / 范围
/// let range = DummyIndex::Range(Box::new(2..5));
///
/// // Index array / 索引数组
/// let array = DummyIndex::IndexArray(vec![0, 2, 4]);
/// ```
#[derive(Debug, Clone)]
pub enum DummyIndex {
    /// Single index (supports negative indexing) / 单个索引（支持负索引）
    Index(isize),
    /// Range of indices / 索引范围
    Range(Box<dyn DummyIndexRange>),
    /// Array of specific indices / 特定索引数组
    IndexArray(Vec<isize>),
}

impl DummyIndex {
    /// Get the number of indices this DummyIndex represents for a given dimension.
    ///
    /// 获取此 DummyIndex 为给定维度表示的索引数量。
    ///
    /// # Type Parameters / 类型参数
    ///
    /// - `S: AbstractShape` - The shape type / 形状类型
    ///
    /// # Parameters / 参数
    ///
    /// - `shape` - The shape of the array / 数组的形状
    /// - `dimension` - The dimension index / 维度索引
    ///
    /// # Returns / 返回值
    ///
    /// The number of indices / 索引数量
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

    /// Get an iterator over the actual indices for a given dimension.
    ///
    /// 获取给定维度的实际索引上的迭代器。
    ///
    /// # Type Parameters / 类型参数
    ///
    /// - `S: AbstractShape` - The shape type / 形状类型
    ///
    /// # Parameters / 参数
    ///
    /// - `shape` - The shape of the array / 数组的形状
    /// - `dimension` - The dimension index / 维度索引
    ///
    /// # Returns / 返回值
    ///
    /// A DummyIndexIterator yielding the actual indices / 生成实际索引的 DummyIndexIterator
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

    /// Calculate the actual bounds of a range for a given dimension.
    ///
    /// 计算给定维度范围的实际边界。
    ///
    /// # Type Parameters / 类型参数
    ///
    /// - `S: AbstractShape` - The shape type / 形状类型
    ///
    /// # Parameters / 参数
    ///
    /// - `range` - The range to calculate bounds for / 要计算边界的范围
    /// - `shape` - The shape of the array / 数组的形状
    /// - `dimension` - The dimension index / 维度索引
    ///
    /// # Returns / 返回值
    ///
    /// A tuple of (lower_bound, upper_bound), where None means unbounded / 边界元组 (lower_bound, upper_bound)，None 表示无边界
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

/// # AdvancePolicy Trait
///
/// A trait for policies that control how to advance through dummy indices.
/// This is used for implementing different access orders (RowMajor/ColumnMajor).
///
/// 控制如何通过虚拟索引推进的策略特征。
/// 这用于实现不同的访问顺序（行优先/列优先）。
///
/// ## Type Parameters / 类型参数
///
/// - `S: AbstractShape` - The shape type / 形状类型
pub trait AdvancePolicy<S: AbstractShape>: AccessOrderTrait {
    /// Initialize the iterator to the first element.
    ///
    /// 将迭代器初始化到第一个元素。
    ///
    /// # Parameters / 参数
    ///
    /// - `iterators` - The iterator vector / 迭代器向量
    /// - `current_vector` - The current vector to initialize / 要初始化的当前向量
    ///
    /// # Returns / 返回值
    ///
    /// `true` if initialization succeeded / 如果初始化成功则返回 `true`
    fn init_iterator<IV: IteratorVector>(
        iterators: &IV,
        current_vector: &mut S::VectorType,
    ) -> bool;

    /// Advance to the next element.
    ///
    /// 推进到下一个元素。
    ///
    /// # Parameters / 参数
    ///
    /// - `iterators` - The iterator vector / 迭代器向量
    /// - `current_positions` - Current positions in each iterator / 每个迭代器的当前位置
    /// - `current_vector` - The current vector to update / 要更新的当前向量
    ///
    /// # Returns / 返回值
    ///
    /// `true` if successfully advanced to next element / 如果成功推进到下一个元素
    fn advance<IV: IteratorVector>(
        iterators: &IV,
        current_positions: &mut S::VectorType,
        current_vector: &mut S::VectorType,
    ) -> bool;
}

impl<S: AbstractShape> AdvancePolicy<S> for RowMajor {
    fn init_iterator<IV: IteratorVector>(
        iterators: &IV,
        current_vector: &mut S::VectorType,
    ) -> bool {
        for i in (0..iterators.len()).rev() {
            match iterators[i].get(0) {
                Some(val) => current_vector[i] = val,
                None => return false,
            }
        }
        true
    }

    fn advance<IV: IteratorVector>(
        iterators: &IV,
        current_positions: &mut S::VectorType,
        current_vector: &mut S::VectorType,
    ) -> bool {
        for i in (0..iterators.len()).rev() {
            current_positions[i] += 1;
            if let Some(val) = iterators[i].get(current_positions[i]) {
                current_vector[i] = val;
                return true;
            }
            // Reset this dimension and carry to the next
            // 重置此维度并进位到下一个
            current_positions[i] = 0;
            if let Some(val) = iterators[i].get(0) {
                current_vector[i] = val;
            }
        }
        false
    }
}

impl<S: AbstractShape> AdvancePolicy<S> for ColumnMajor {
    fn init_iterator<IV: IteratorVector>(
        iterators: &IV,
        current_vector: &mut S::VectorType,
    ) -> bool {
        for i in 0..iterators.len() {
            match iterators[i].get(0) {
                Some(val) => current_vector[i] = val,
                None => return false,
            }
        }
        true
    }

    fn advance<IV: IteratorVector>(
        iterators: &IV,
        current_positions: &mut S::VectorType,
        current_vector: &mut S::VectorType,
    ) -> bool {
        for i in 0..iterators.len() {
            current_positions[i] += 1;
            if let Some(val) = iterators[i].get(current_positions[i]) {
                current_vector[i] = val;
                return true;
            }
            // Reset this dimension and carry to the next
            // 重置此维度并进位到下一个
            current_positions[i] = 0;
            if let Some(val) = iterators[i].get(0) {
                current_vector[i] = val;
            }
        }
        false
    }
}

/// # DummyAccessIterator
///
/// An iterator that yields vectors by iterating through dummy indices.
/// Uses S::IteratorVectorType for iterators and S::VectorType for positions.
///
/// 通过迭代虚拟索引生成向量的迭代器。
/// 使用 S::IteratorVectorType 存储迭代器，使用 S::VectorType 存储位置。
pub(crate) struct DummyAccessIterator<'a, S: AbstractShape>
where
    S: 'a,
{
    /// Reference to the shape / 形状的引用
    shape: &'a S,
    /// Iterators for each dimension / 每个维度的迭代器
    iterators: S::IteratorVectorType,
    /// Current positions in each iterator / 每个迭代器的当前位置
    current_positions: S::VectorType,
    /// Current vector / 当前向量
    current_vector: S::VectorType,
    /// Whether iteration is finished / 迭代是否完成
    finished: bool,
    /// Whether this is the first call / 是否是第一次调用
    first_call: bool,
    /// Access order / 访问顺序
    access_order: AccessOrder,
}

impl<'a, S: AbstractShape> DummyAccessIterator<'a, S> {
    /// Create a new DummyAccessIterator.
    ///
    /// 创建新的 DummyAccessIterator。
    ///
    /// # Parameters / 参数
    ///
    /// - `shape` - Reference to the shape / 形状的引用
    /// - `iterators` - Iterator vector / 迭代器向量
    /// - `access_order` - Access order / 访问顺序
    pub(crate) fn new(
        shape: &'a S,
        iterators: S::IteratorVectorType,
        access_order: AccessOrder,
    ) -> Self {
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

    /// Initialize the iterator to the first element.
    ///
    /// 将迭代器初始化到第一个元素。
    fn init_iterator(&mut self) -> bool {
        self.current_vector = self.shape.zero();

        match self.access_order {
            AccessOrder::RowMajor => {
                <RowMajor as AdvancePolicy<S>>::init_iterator(
                    &self.iterators,
                    &mut self.current_vector,
                )
            }
            AccessOrder::ColumnMajor => {
                <ColumnMajor as AdvancePolicy<S>>::init_iterator(
                    &self.iterators,
                    &mut self.current_vector,
                )
            }
        }
    }

    /// Advance to the next vector.
    ///
    /// 推进到下一个向量。
    ///
    /// # Returns / 返回值
    ///
    /// - `Some(&VectorType)` - The next vector / 下一个向量
    /// - `None` - If all vectors have been iterated / 如果所有向量已迭代完成
    pub(crate) fn next(&mut self) -> Option<&S::VectorType> {
        if self.finished {
            return None;
        }

        if self.first_call {
            // First call, initialize
            // 第一次调用，初始化
            self.first_call = false;
            if !self.init_iterator() {
                self.finished = true;
                return None;
            }
            return Some(&self.current_vector);
        }

        // Advance to next element based on access order
        // 根据访问顺序推进到下一个元素
        let advanced = match self.access_order {
            AccessOrder::RowMajor => <RowMajor as AdvancePolicy<S>>::advance(
                &self.iterators,
                &mut self.current_positions,
                &mut self.current_vector,
            ),
            AccessOrder::ColumnMajor => <ColumnMajor as AdvancePolicy<S>>::advance(
                &self.iterators,
                &mut self.current_positions,
                &mut self.current_vector,
            ),
        };

        if advanced {
            Some(&self.current_vector)
        } else {
            self.finished = true;
            None
        }
    }
}

/// # CTDummyAccessIterator
///
/// A compile-time version of DummyAccessIterator with a fixed access order.
///
/// 具有固定访问顺序的 DummyAccessIterator 的编译时版本。
pub(crate) struct CTDummyAccessIterator<'a, S: AbstractShape, AO: AccessOrderTrait + AdvancePolicy<S>> {
    /// Reference to the shape / 形状的引用
    shape: &'a S,
    /// Iterators for each dimension (owned) / 每个维度的迭代器（拥有）
    iterators: S::IteratorVectorType,
    /// Current positions in each iterator / 每个迭代器的当前位置
    current_positions: S::VectorType,
    /// Current vector / 当前向量
    current_vector: S::VectorType,
    /// Whether iteration is finished / 迭代是否完成
    finished: bool,
    /// Whether this is the first call / 是否是第一次调用
    first_call: bool,
    /// Phantom marker for access order type / 访问顺序类型的虚拟特征
    _marker: PhantomData<AO>,
}

impl<'a, S: AbstractShape, AO: AccessOrderTrait + AdvancePolicy<S>> CTDummyAccessIterator<'a, S, AO> {
    /// Create a new CTDummyAccessIterator from iterators.
    ///
    /// 从迭代器创建新的 CTDummyAccessIterator。
    pub(crate) fn new(shape: &'a S, iterators: S::IteratorVectorType) -> Self {
        let current_positions = shape.zero();
        let current_vector = shape.zero();
        Self {
            shape,
            iterators,
            current_positions,
            current_vector,
            finished: false,
            first_call: true,
            _marker: PhantomData,
        }
    }

    /// Initialize the iterator to the first element.
    ///
    /// 将迭代器初始化到第一个元素。
    fn init_iterator(&mut self) -> bool {
        self.current_vector = self.shape.zero();

        // Initialize based on access order
        // 根据访问顺序初始化
        <AO as AdvancePolicy<S>>::init_iterator(
            &self.iterators,
            &mut self.current_vector,
        )
    }

    /// Advance to the next vector.
    ///
    /// 推进到下一个向量。
    ///
    /// # Returns / 返回值
    ///
    /// - `Some(&VectorType)` - The next vector / 下一个向量
    /// - `None` - If all vectors have been iterated / 如果所有向量已迭代完成
    pub(crate) fn next(&mut self) -> Option<&S::VectorType> {
        if self.finished {
            return None;
        }

        if self.first_call {
            // First call, initialize
            // 第一次调用，初始化
            self.first_call = false;
            if !self.init_iterator() {
                self.finished = true;
                return None;
            }
            return Some(&self.current_vector);
        }

        // Advance to next element based on access order
        // 根据访问顺序推进到下一个元素
        let advanced = <AO as AdvancePolicy<S>>::advance(
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

    fn advance_row_major(&mut self) -> bool {
        for i in (0..self.iterators.len()).rev() {
            self.current_positions[i] += 1;
            if let Some(val) = self.iterators[i].get(self.current_positions[i]) {
                self.current_vector[i] = val;
                return true;
            }
            // Reset this dimension and carry to the next
            // 重置此维度并进位到下一个
            self.current_positions[i] = 0;
            if let Some(val) = self.iterators[i].get(0) {
                self.current_vector[i] = val;
            }
        }
        false
    }

    fn advance_column_major(&mut self) -> bool {
        for i in 0..self.iterators.len() {
            self.current_positions[i] += 1;
            if let Some(val) = self.iterators[i].get(self.current_positions[i]) {
                self.current_vector[i] = val;
                return true;
            }
            // Reset this dimension and carry to the next
            // 重置此维度并进位到下一个
            self.current_positions[i] = 0;
            if let Some(val) = self.iterators[i].get(0) {
                self.current_vector[i] = val;
            }
        }
        false
    }
}

/// # dummy_index_value! Macro
///
/// A helper macro for converting literals to isize values.
///
/// 用于将字面量转换为 isize 值的辅助宏。
#[macro_export]
macro_rules! dummy_index_value {
    (-$x:literal) => {
        -paste! { [<$x isize>] }
    };
    ($x:literal) => {
        paste! { [<$x isize>] }
    };
    ($x:expr) => {
        $x
    };
}

/// # dummy_index! Macro
///
/// A macro for creating DummyIndex from various range expressions.
///
/// 用于从各种范围表达式创建 DummyIndex 的宏。
///
/// ## Usage / 用法
///
/// ```rust
/// use paste::paste;
/// use ospf_rust_multiarray::*;
///
/// // Single index / 单个索引
/// let idx = dummy_index!(5).unwrap();
///
/// // Range / 范围
/// let range = dummy_index!(2..5).unwrap();
///
/// // Inclusive range / 包含结束的范围
/// let inclusive = dummy_index!(1..=4).unwrap();
///
/// // Open-ended range / 开放范围
/// let open = dummy_index!(3..).unwrap();
/// ```
#[macro_export]
macro_rules! dummy_index {
    (-$x:literal) => {
        DummyIndex::try_from(dummy_index_value!{ -$x })
    };
    ($x:literal) => {
        DummyIndex::try_from(dummy_index_value!{ $x })
    };
    ($start:literal..-$end:literal) => {
        DummyIndex::try_from(dummy_index_value!{ $start }..dummy_index_value!{ -$end })
    };
    ($start:literal..$end:literal) => {
        DummyIndex::try_from(dummy_index_value!{ $start }..dummy_index_value!{ $end })
    };
    ($start:literal..) => {
        DummyIndex::try_from(dummy_index_value!{ $start }..)
    };
    (..$end:literal) => {
        DummyIndex::try_from(..dummy_index_value!{ $end })
    };
    (..-$end:literal) => {
        DummyIndex::try_from(..dummy_index_value!{ -$end })
    };
    ($start:literal..=-$end:literal) => {
        DummyIndex::try_from(dummy_index_value!{ $start }..=dummy_index_value!{ -$end })
    };
    ($start:literal..=$end:literal) => {
        DummyIndex::try_from(dummy_index_value!{ $start }..=dummy_index_value!{ $end })
    };
    (..=-$end:literal) => {
        DummyIndex::try_from(..=dummy_index_value!{ -$end })
    };
    (..=$end:literal) => {
        DummyIndex::try_from(..=dummy_index_value!{ $end })
    };
    (..) => {
        DummyIndex::try_from(..)
    };
    [$($x:literal),*] => {
        DummyIndex::try_from(&[$(dummy_index_value!{ $x }),*])
    };
    ($x:expr) => {{
        DummyIndex::try_from(&$x)
    }};
}

/// # dummy! Macro
///
/// A macro for creating fixed-size dummy index arrays.
///
/// 用于创建固定大小虚拟索引数组的宏。
#[macro_export]
macro_rules! dummy {
    [$($x:expr),*] => {
        [$(dummy_index!{ $x }.map_err(|e| error! { InvalidDummyIndexError {} })?),*]
    };
    ($a:ident[$($x:expr),*]) => {
        $a.view(dummy![$($x),*])
    };
}

/// # dummy_with_err! Macro
///
/// A macro for creating fixed-size dummy index arrays with custom error handling.
///
/// 用于创建具有自定义错误处理的固定大小虚拟索引数组的宏。
#[macro_export]
macro_rules! dummy_with_err {
    [$($x:expr),*] => {
        [$(dummy_index!{ $x }.map_err(|e| error! { ExInvalidDummyIndexError { origin: e } })?),*]
    };
    ($a:ident[$($x:expr),*]) => {
        $a.view(dummy_with_err![$($x),*])
    };
}

/// # dummy_expect! Macro
///
/// A macro for creating fixed-size dummy index arrays, panicking on error.
///
/// 用于创建固定大小虚拟索引数组的宏，出错时 panic。
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

/// # dyn_dummy! Macro
///
/// A macro for creating dynamic (Vec-based) dummy index arrays.
///
/// 用于创建动态（基于 Vec）虚拟索引数组的宏。
#[macro_export]
macro_rules! dyn_dummy {
    [$($x:expr),*] => {
        vec![$(dummy_index!{ $x }.map_err(|e| error! { InvalidDummyIndexError {} })?),*]
    };
    ($a:ident[$($x:expr),*]) => {
        $a.view(dyn_dummy![$($x),*])
    };
}

/// # dyn_dummy_with_err! Macro
///
/// A macro for creating dynamic dummy index arrays with custom error handling.
///
/// 用于创建具有自定义错误处理的动态虚拟索引数组的宏。
#[macro_export]
macro_rules! dyn_dummy_with_err {
    [$($x:expr),*] => {
        vec![$(dummy_index!{ $x }.map_err(|e| error! { ExInvalidDummyIndexError { origin: e } })?),*]
    };
    ($a:ident[$($x:expr),*]) => {
        $a.view(dyn_dummy_with_err![$($x),*])
    };
}

/// # dyn_dummy_expect! Macro
///
/// A macro for creating dynamic dummy index arrays, panicking on error.
///
/// 用于创建动态虚拟索引数组的宏，出错时 panic。
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
        let shape = Shape::new([5, 3]);

        let idx = DummyIndex::Index(2);
        let iter = idx.iterator_of(&shape, 0);
        assert_eq!(iter.get(0), Some(2));

        let idx = DummyIndex::Index(-1);
        let iter = idx.iterator_of(&shape, 0);
        assert_eq!(iter.get(0), Some(4));
    }

    #[test]
    fn test_iterator_of_for_range() {
        let shape = Shape::new([5, 3]);

        let idx: DummyIndex = dummy_index!(1..=3).expect("invalid dummy index");
        let iter = idx.iterator_of(&shape, 0);
        match iter {
            DummyIndexIterator::Continuous(range) => {
                assert_eq!(range.start, 1);
                assert_eq!(range.end, 4);
            }
            _ => panic!("Expected Continuous iterator for range"),
        }

        let idx: DummyIndex = dummy_index!(..).expect("invalid dummy index");
        let iter = idx.iterator_of(&shape, 0);
        match iter {
            DummyIndexIterator::Continuous(range) => {
                assert_eq!(range.start, 0);
                assert_eq!(range.end, 5);
            }
            _ => panic!("Expected Continuous iterator"),
        }
    }

    #[test]
    fn test_iterator_of_for_index_array() {
        let shape = Shape::new([5, 3]);

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
        let shape = Shape::new([2, 3]);
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
    fn test_dummy_access_iterator_column_major() {
        let shape = Shape::new([2, 3]);
        let dummy_vec: [DummyIndex; 2] = dummy_expect![0..2, 1..3];
        let iterators = shape.dummy_to_iterator_vector(&dummy_vec);

        let mut iter = DummyAccessIterator::new(&shape, iterators, AccessOrder::ColumnMajor);

        let mut vectors = Vec::new();
        while let Some(vec) = iter.next() {
            vectors.push(vec.clone());
        }

        assert_eq!(vectors.len(), 4);
        // Column major: first dimension changes fastest
        // 列优先：第一维度变化最快
        assert_eq!(vectors[0], [0, 1]);
        assert_eq!(vectors[1], [1, 1]);
        assert_eq!(vectors[2], [0, 2]);
        assert_eq!(vectors[3], [1, 2]);
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
