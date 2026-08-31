//! # MultiArray - 运行时多维数组实现
//!
//! ## Overview / 概述
//!
//! This module provides the `MultiArray` type, a runtime-sized multi-dimensional array
//! implementation that supports dynamic shape configuration.
//!
//! 本模块提供 `MultiArray` 类型，这是一个运行时大小的多维数组实现，支持动态形状配置。
//!
//! ## Key Features / 主要特性
//!
//! - Dynamic shape at runtime / 运行时动态形状
//! - Support for different storage orders (Row-Major / Column-Major) / 支持不同的存储顺序（行优先/列优先）
//! - Zero-copy views via `MultiArrayView` / 通过 `MultiArrayView` 实现零拷贝视图
//! - Flexible collection types / 灵活的集合类型
//! - Reshape operations / 重塑操作

use super::concept::StorageOrder;
use super::error::MappingIndexError;
use super::multi_array_view::MultiArrayView;
use super::shape::{AbstractRTShape, AbstractShape, DynShape};
use cc_traits::{Collection, CollectionMut, CollectionRef, Iter, IterMut, Len};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};

/// # MultiArrayCollection Trait
///
/// A trait that defines the collection requirements for `MultiArray`.
/// It combines standard collection traits needed for multi-dimensional array storage.
///
/// 定义 `MultiArray` 所需集合要求的特征。
/// 它结合了多维数组存储所需的标准集合特征。
///
/// ## Required Traits / 必需特征
///
/// - `Collection<Item = T>` / 集合项为 T
/// - `Len` / 可获取长度
/// - `Index<usize, Output = T>` / 可按 usize 索引
/// - `IndexMut<usize, Output = T>` / 可按 usize 可变索引
/// - `Iter` / 可迭代
/// - `IterMut` / 可可变迭代
/// - `FromIterator<T>` / 可从迭代器构建
pub trait MultiArrayCollection<T>:
    Collection<Item = T>
    + Len
    + Index<usize, Output = T>
    + IndexMut<usize, Output = T>
    + Iter
    + IterMut
    + FromIterator<T>
{
}

impl<T, C> MultiArrayCollection<T> for C where
    C: Collection<Item = T>
        + Len
        + Index<usize, Output = T>
        + IndexMut<usize, Output = T>
        + Iter
        + IterMut
        + FromIterator<T>
{
}

/// # MultiArrayToView Trait
///
/// A trait for converting arrays or views into `MultiArrayView`.
/// This enables zero-copy slicing and projection operations.
///
/// 用于将数组或视图转换为 `MultiArrayView` 的特征。
/// 这使得零拷贝切片和投影操作成为可能。
///
/// ## Type Parameters / 类型参数
///
/// - `S: AbstractRTShape` - The runtime shape type / 运行时形状类型
///
/// ## Associated Types / 关联类型
///
/// - `ViewType<'a>` - The view type for lifetime 'a / 生命周期 'a 的视图类型
pub trait MultiArrayToView<S: AbstractRTShape> {
    /// The view type produced by this conversion / 此转换产生的视图类型
    type ViewType<'a>: MultiArrayToView<DynShape>
    where
        Self: 'a;

    /// Create a view using a dummy vector (slicing/projection)
    ///
    /// 使用虚拟向量创建视图（切片/投影）
    ///
    /// # Parameters / 参数
    ///
    /// - `dummy_vector` - A vector specifying which dimensions to keep/slice / 指定保留/切片维度的向量
    ///
    /// # Returns / 返回值
    ///
    /// - `Ok(ViewType)` - The created view / 创建的视图
    /// - `Err(MappingIndexError)` - If the mapping is invalid / 如果映射无效
    fn view(
        &self,
        dummy_vector: &S::DummyVectorType,
    ) -> Result<Self::ViewType<'_>, MappingIndexError>;

    /// Create a view using a map vector (dimension reordering/projection)
    ///
    /// 使用映射向量创建视图（维度重排/投影）
    ///
    /// # Parameters / 参数
    ///
    /// - `map_vector` - A vector specifying dimension mapping / 指定维度映射的向量
    ///
    /// # Returns / 返回值
    ///
    /// - `Ok(ViewType)` - The created view / 创建的视图
    /// - `Err(MappingIndexError)` - If the mapping is invalid / 如果映射无效
    fn map_view(
        &self,
        map_vector: &S::MapVectorType,
    ) -> Result<Self::ViewType<'_>, MappingIndexError>;
}

/// # MultiArray - Runtime Multi-Dimensional Array
///
/// A multi-dimensional array with runtime-determined shape.
/// The array stores elements in a flat collection and uses a shape
/// type to map multi-dimensional indices to linear indices.
///
/// 具有运行时确定形状的多维数组。
/// 数组将元素存储在扁平集合中，并使用形状类型将多维索引映射到线性索引。
///
/// ## Type Parameters / 类型参数
///
/// - `T` - The element type / 元素类型
/// - `S: AbstractRTShape` - The runtime shape type / 运行时形状类型
/// - `C: MultiArrayCollection<T>` - The underlying collection type (default: `Vec<T>`) / 底层集合类型（默认：`Vec<T>`）
///
/// ## Fields / 字段
///
/// - `list` - The underlying storage / 底层存储
/// - `shape` - The multi-dimensional shape / 多维形状
/// - `_marker` - Phantom marker for type safety / 类型安全的虚拟特征
///
/// ## Example / 示例
///
/// ```rust
/// use ospf_rust_multiarray::*;
///
/// // Create a 2x3 array / 创建一个 2x3 数组
/// let shape = Shape::new([2, 3]);
/// let array: MultiArray<i32, _> = MultiArrayBuilder::new_with(shape, 0);
/// ```
pub struct MultiArray<T, S, C = Vec<T>>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    /// Underlying storage / 底层存储
    list: C,
    /// Multi-dimensional shape / 多维形状
    pub shape: S,
    /// Phantom marker for type parameter T / 类型参数 T 的虚拟特征
    _marker: PhantomData<T>,
}

impl<T, S, C> MultiArray<T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    /// Create a new MultiArray with default values.
    ///
    /// 使用默认值创建新的 MultiArray。
    ///
    /// # Type Parameters / 类型参数
    ///
    /// - `T: Default` - Element type must implement Default / 元素类型必须实现 Default
    ///
    /// # Parameters / 参数
    ///
    /// - `shape` - The multi-dimensional shape / 多维形状
    ///
    /// # Returns / 返回值
    ///
    /// A new MultiArray filled with default values / 填充默认值的新 MultiArray
    pub fn new(shape: S) -> Self
    where
        T: Default,
    {
        Self {
            list: (0..shape.len()).map(|_| T::default()).collect(),
            shape,
            _marker: PhantomData,
        }
    }

    /// Create a new MultiArray with a specific value.
    ///
    /// 使用特定值创建新的 MultiArray。
    ///
    /// # Type Parameters / 类型参数
    ///
    /// - `T: Clone` - Element type must implement Clone / 元素类型必须实现 Clone
    ///
    /// # Parameters / 参数
    ///
    /// - `shape` - The multi-dimensional shape / 多维形状
    /// - `value` - The value to fill the array with / 用于填充数组的值
    ///
    /// # Returns / 返回值
    ///
    /// A new MultiArray filled with the specified value / 填充指定值的新 MultiArray
    pub fn new_with(shape: S, value: T) -> Self
    where
        T: Clone,
    {
        Self {
            list: (0..shape.len()).map(|_| value.clone()).collect(),
            shape,
            _marker: PhantomData,
        }
    }

    /// Create a new MultiArray using a generator function.
    ///
    /// 使用生成器函数创建新的 MultiArray。
    ///
    /// # Type Parameters / 类型参数
    ///
    /// - `G: Fn(usize, &VectorType) -> T` - Generator function type / 生成器函数类型
    ///
    /// # Parameters / 参数
    ///
    /// - `shape` - The multi-dimensional shape / 多维形状
    /// - `generator` - A function that takes (linear_index, vector) and returns an element / 接受（线性索引，向量）并返回元素的函数
    ///
    /// # Returns / 返回值
    ///
    /// A new MultiArray with elements generated by the function / 由函数生成元素的新 MultiArray
    pub fn new_by<G>(shape: S, generator: G) -> Self
    where
        G: Fn(usize, &<S as AbstractShape>::VectorType) -> T,
    {
        Self {
            list: (0..shape.len())
                .map(|index| generator(index, &shape.vector_of(index).unwrap()))
                .collect(),
            shape,
            _marker: PhantomData,
        }
    }

    /// Get the storage order of this array.
    ///
    /// 获取此数组的存储顺序。
    ///
    /// # Returns / 返回值
    ///
    /// The storage order (RowMajor or ColumnMajor) / 存储顺序（行优先或列优先）
    pub fn storage_order(&self) -> StorageOrder {
        self.shape.storage_order()
    }

    /// Convert the array to a different storage order.
    ///
    /// 将数组转换为不同的存储顺序。
    ///
    /// # Type Parameters / 类型参数
    ///
    /// - `T: Clone` - Element type must implement Clone / 元素类型必须实现 Clone
    /// - `S: Clone` - Shape type must implement Clone / 形状类型必须实现 Clone
    /// - `C: Clone` - Collection type must implement Clone / 集合类型必须实现 Clone
    ///
    /// # Parameters / 参数
    ///
    /// - `order` - The target storage order / 目标存储顺序
    ///
    /// # Returns / 返回值
    ///
    /// A new MultiArray with the specified storage order / 具有指定存储顺序的新 MultiArray
    ///
    /// # Note / 注意
    ///
    /// If the current storage order matches the target order, returns a clone.
    /// Otherwise, reorders elements to match the new storage layout.
    ///
    /// 如果当前存储顺序与目标顺序匹配，则返回克隆。
    /// 否则，重新排序元素以匹配新的存储布局。
    pub fn to_storage_order(&self, order: StorageOrder) -> Self
    where
        T: Clone,
        S: Clone,
        C: Clone,
    {
        if self.shape.storage_order() == order {
            return Self {
                list: self.list.clone(),
                shape: self.shape.clone(),
                _marker: PhantomData,
            };
        }

        let new_shape = self.shape.with_storage_order(order);

        let mut reordered: Vec<T> = Vec::with_capacity(self.len());
        for _ in 0..self.len() {
            reordered.push(self.list[0].clone());
        }

        for i in 0..self.len() {
            let vector = self.shape.vector_of(i).unwrap();
            let new_index = new_shape.index_of(&vector).unwrap();
            reordered[new_index] = self.list[i].clone();
        }

        Self {
            list: reordered.into_iter().collect(),
            shape: new_shape,
            _marker: PhantomData,
        }
    }

    /// Reshape the array to a new shape, filling extra elements with default values.
    ///
    /// 将数组重塑为新形状，用默认值填充额外元素。
    ///
    /// # Type Parameters / 类型参数
    ///
    /// - `T: Default + Clone` - Element type must implement Default and Clone / 元素类型必须实现 Default 和 Clone
    /// - `NS: AbstractRTShape` - New shape type / 新形状类型
    /// - `C: FromIterator<T>` - Collection must be constructible from iterator / 集合必须可从迭代器构建
    ///
    /// # Parameters / 参数
    ///
    /// - `new_shape` - The new shape / 新形状
    ///
    /// # Returns / 返回值
    ///
    /// A new MultiArray with the specified shape / 具有指定形状的新 MultiArray
    ///
    /// # Note / 注意
    ///
    /// If the new shape is larger, extra elements are filled with `T::default()`.
    /// If the new shape is smaller, excess elements are still copied (the new array
    /// will have at least as many elements as the original).
    ///
    /// 如果新形状更大，额外元素用 `T::default()` 填充。
    /// 如果新形状更小，多余元素仍会被复制（新数组将至少包含与原数组一样多的元素）。
    pub fn reshape<NS>(&self, new_shape: NS) -> MultiArray<T, NS, C>
    where
        T: Default + Clone,
        NS: AbstractRTShape,
        C: FromIterator<T>,
    {
        let mut new_list: Vec<T> = Vec::with_capacity(new_shape.len());

        for i in 0..self.len() {
            new_list.push(self.list[i].clone());
        }

        for _ in self.len()..new_shape.len() {
            new_list.push(T::default());
        }

        MultiArray {
            list: new_list.into_iter().collect(),
            shape: new_shape,
            _marker: PhantomData,
        }
    }

    /// Reshape the array to a new shape, filling extra elements with a specific value.
    ///
    /// 将数组重塑为新形状，用特定值填充额外元素。
    ///
    /// # Type Parameters / 类型参数
    ///
    /// - `T: Clone` - Element type must implement Clone / 元素类型必须实现 Clone
    /// - `NS: AbstractRTShape` - New shape type / 新形状类型
    /// - `C: FromIterator<T>` - Collection must be constructible from iterator / 集合必须可从迭代器构建
    ///
    /// # Parameters / 参数
    ///
    /// - `new_shape` - The new shape / 新形状
    /// - `fill_value` - The value to fill extra elements / 用于填充额外元素的值
    ///
    /// # Returns / 返回值
    ///
    /// A new MultiArray with the specified shape / 具有指定形状的新 MultiArray
    pub fn reshape_with<NS>(&self, new_shape: NS, fill_value: T) -> MultiArray<T, NS, C>
    where
        T: Clone,
        NS: AbstractRTShape,
        C: FromIterator<T>,
    {
        let mut new_list: Vec<T> = Vec::with_capacity(new_shape.len());

        for i in 0..self.len() {
            new_list.push(self.list[i].clone());
        }

        for _ in self.len()..new_shape.len() {
            new_list.push(fill_value.clone());
        }

        MultiArray {
            list: new_list.into_iter().collect(),
            shape: new_shape,
            _marker: PhantomData,
        }
    }

    /// Reshape the array to a new shape, using a generator for extra elements.
    ///
    /// 将数组重塑为新形状，使用生成器填充额外元素。
    ///
    /// # Type Parameters / 类型参数
    ///
    /// - `T: Clone` - Element type must implement Clone / 元素类型必须实现 Clone
    /// - `NS: AbstractRTShape` - New shape type / 新形状类型
    /// - `G: Fn(usize, &VectorType) -> T` - Generator function type / 生成器函数类型
    /// - `C: FromIterator<T>` - Collection must be constructible from iterator / 集合必须可从迭代器构建
    ///
    /// # Parameters / 参数
    ///
    /// - `new_shape` - The new shape / 新形状
    /// - `generator` - A function to generate extra elements / 用于生成额外元素的函数
    ///
    /// # Returns / 返回值
    ///
    /// A new MultiArray with the specified shape / 具有指定形状的新 MultiArray
    pub fn reshape_by<NS, G>(&self, new_shape: NS, generator: G) -> MultiArray<T, NS, C>
    where
        T: Clone,
        NS: AbstractRTShape,
        G: Fn(usize, &<NS as AbstractShape>::VectorType) -> T,
        C: FromIterator<T>,
    {
        let mut new_list: Vec<T> = Vec::with_capacity(new_shape.len());

        for i in 0..self.len() {
            new_list.push(self.list[i].clone());
        }

        for i in self.len()..new_shape.len() {
            let vector = new_shape.vector_of(i).unwrap();
            new_list.push(generator(i, &vector));
        }

        MultiArray {
            list: new_list.into_iter().collect(),
            shape: new_shape,
            _marker: PhantomData,
        }
    }

    /// Create an enumerate iterator that yields (linear_index, vector, element) tuples.
    ///
    /// 创建一个枚举迭代器，生成（线性索引，向量，元素）元组。
    ///
    /// # Returns / 返回值
    ///
    /// A MultiArrayEnumerateIter that yields tuples of (index, vector, element) / 生成（索引，向量，元素）元组的 MultiArrayEnumerateIter
    ///
    /// # Example / 示例
    ///
    /// ```rust
    /// use ospf_rust_multiarray::*;
    ///
    /// let shape = Shape::new([2, 3]);
    /// let array = MultiArrayBuilder::new_with(shape, 0);
    ///
    /// for (index, vector, value) in array.enumerate() {
    ///     println!("Index {}: vector={:?}, value={}", index, vector, value);
    /// }
    /// ```
    pub fn enumerate(&self) -> MultiArrayEnumerateIter<'_, T, S, C> {
        MultiArrayEnumerateIter::new(self)
    }
}

impl<T, S, C> Clone for MultiArray<T, S, C>
where
    T: Clone,
    S: AbstractRTShape + Clone,
    C: MultiArrayCollection<T> + FromIterator<T> + Clone,
{
    fn clone(&self) -> Self {
        Self {
            list: self.list.clone(),
            shape: self.shape.clone(),
            _marker: PhantomData,
        }
    }
}

/// # MultiArrayBuilder
///
/// A builder for creating `MultiArray` instances with various initialization strategies.
///
/// `MultiArray` 实例的构建器，支持各种初始化策略。
///
/// ## Usage / 用法
///
/// ```rust
/// use ospf_rust_multiarray::*;
///
/// // Create with default values / 使用默认值创建
/// let array: MultiArray<i32, _> = MultiArrayBuilder::new(Shape::new([2, 3]));
///
/// // Create with specific value / 使用特定值创建
/// let array = MultiArrayBuilder::new_with(Shape::new([2, 3]), 42);
///
/// // Create with generator / 使用生成器创建
/// let array = MultiArrayBuilder::new_by(Shape::new([2, 3]), |i, _| i);
/// ```
pub struct MultiArrayBuilder {}

impl MultiArrayBuilder {
    /// Create a new MultiArray with default values.
    ///
    /// 使用默认值创建新的 MultiArray。
    pub fn new<T, S>(shape: S) -> MultiArray<T, S>
    where
        T: Default,
        S: AbstractRTShape,
    {
        MultiArray::<T, S>::new(shape)
    }

    /// Create a new MultiArray with default values and custom collection type.
    ///
    /// 使用默认值和自定义集合类型创建新的 MultiArray。
    pub fn new_as<T, S, C>(shape: S) -> MultiArray<T, S, C>
    where
        T: Default,
        S: AbstractRTShape,
        C: MultiArrayCollection<T>,
    {
        MultiArray::<T, S, C>::new(shape)
    }

    /// Create a new MultiArray with a specific value.
    ///
    /// 使用特定值创建新的 MultiArray。
    pub fn new_with<T, S>(shape: S, value: T) -> MultiArray<T, S>
    where
        T: Clone,
        S: AbstractRTShape,
    {
        MultiArray::<T, S>::new_with(shape, value)
    }

    /// Create a new MultiArray with a specific value and custom collection type.
    ///
    /// 使用特定值和自定义集合类型创建新的 MultiArray。
    pub fn new_with_as<T, S, C>(shape: S, value: T) -> MultiArray<T, S, C>
    where
        T: Clone,
        S: AbstractRTShape,
        C: MultiArrayCollection<T>,
    {
        MultiArray::<T, S, C>::new_with(shape, value)
    }

    /// Create a new MultiArray using a generator function.
    ///
    /// 使用生成器函数创建新的 MultiArray。
    pub fn new_by<T, S, G>(shape: S, generator: G) -> MultiArray<T, S>
    where
        S: AbstractRTShape,
        G: Fn(usize, &<S as AbstractShape>::VectorType) -> T,
    {
        MultiArray::<T, S>::new_by(shape, generator)
    }

    /// Create a new MultiArray using a generator function and custom collection type.
    ///
    /// 使用生成器函数和自定义集合类型创建新的 MultiArray。
    pub fn new_by_as<T, S, C, G>(shape: S, generator: G) -> MultiArray<T, S, C>
    where
        S: AbstractRTShape,
        C: MultiArrayCollection<T>,
        G: Fn(usize, &<S as AbstractShape>::VectorType) -> T,
    {
        MultiArray::<T, S, C>::new_by(shape, generator)
    }

    /// Create a new MultiArray with a specific storage order.
    ///
    /// 使用特定存储顺序创建新的 MultiArray。
    ///
    /// # Parameters / 参数
    ///
    /// - `shape` - The shape vector / 形状向量
    /// - `order` - The storage order / 存储顺序
    pub fn new_with_order<T, S>(shape: &S::ShapeVectorType, order: StorageOrder) -> MultiArray<T, S>
    where
        T: Default,
        S: AbstractRTShape,
    {
        let shape_with_order = S::from_shape_vector_with_order(shape, order);
        MultiArray::<T, S>::new(shape_with_order)
    }

    /// Create a new MultiArray with a specific storage order and custom collection type.
    ///
    /// 使用特定存储顺序和自定义集合类型创建新的 MultiArray。
    pub fn new_with_order_as<T, S, C>(
        shape: &S::ShapeVectorType,
        order: StorageOrder,
        _collection: &C,
    ) -> MultiArray<T, S, C>
    where
        T: Default,
        S: AbstractRTShape,
        C: MultiArrayCollection<T>,
    {
        let shape_with_order = S::from_shape_vector_with_order(shape, order);
        MultiArray::<T, S, C>::new(shape_with_order)
    }

    /// Create a new MultiArray with a specific storage order and value.
    ///
    /// 使用特定存储顺序和值创建新的 MultiArray。
    pub fn new_with_order_and_value<T, S>(
        shape: &S::ShapeVectorType,
        value: T,
        order: StorageOrder,
    ) -> MultiArray<T, S>
    where
        T: Clone,
        S: AbstractRTShape,
    {
        let shape_with_order = S::from_shape_vector_with_order(shape, order);
        MultiArray::<T, S>::new_with(shape_with_order, value)
    }

    /// Create a new MultiArray with a specific storage order, value, and custom collection type.
    ///
    /// 使用特定存储顺序、值和自定义集合类型创建新的 MultiArray。
    pub fn new_with_order_and_value_as<T, S, C>(
        shape: &S::ShapeVectorType,
        value: T,
        order: StorageOrder,
        _collection: &C,
    ) -> MultiArray<T, S, C>
    where
        T: Clone,
        S: AbstractRTShape,
        C: MultiArrayCollection<T>,
    {
        let shape_with_order = S::from_shape_vector_with_order(shape, order);
        MultiArray::<T, S, C>::new_with(shape_with_order, value)
    }

    /// Create a new MultiArray with a specific storage order using a generator.
    ///
    /// 使用特定存储顺序和生成器创建新的 MultiArray。
    pub fn new_by_with_order<T, S, G>(
        shape: &S::ShapeVectorType,
        generator: G,
        order: StorageOrder,
    ) -> MultiArray<T, S>
    where
        S: AbstractRTShape,
        G: Fn(usize, &<S as AbstractShape>::VectorType) -> T,
    {
        let shape_with_order = S::from_shape_vector_with_order(shape, order);
        MultiArray::<T, S>::new_by(shape_with_order, generator)
    }

    /// Create a new MultiArray with a specific storage order, generator, and custom collection type.
    ///
    /// 使用特定存储顺序、生成器和自定义集合类型创建新的 MultiArray。
    pub fn new_by_with_order_as<T, S, C, G>(
        shape: &S::ShapeVectorType,
        generator: G,
        order: StorageOrder,
        _collection: &C,
    ) -> MultiArray<T, S, C>
    where
        S: AbstractRTShape,
        C: MultiArrayCollection<T>,
        G: Fn(usize, &<S as AbstractShape>::VectorType) -> T,
    {
        let shape_with_order = S::from_shape_vector_with_order(shape, order);
        MultiArray::<T, S, C>::new_by(shape_with_order, generator)
    }
}

impl<T, S, C> Deref for MultiArray<T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    type Target = C;

    fn deref(&self) -> &Self::Target {
        &self.list
    }
}

impl<T, S, C> DerefMut for MultiArray<T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.list
    }
}

impl<T, S, C> Collection for MultiArray<T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    type Item = T;
}

impl<T, S, C> CollectionRef for MultiArray<T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    type ItemRef<'a>
        = &'a T
    where
        T: 'a,
        S: 'a,
        C: 'a;

    cc_traits::covariant_item_ref!();
}

impl<T, S, C> CollectionMut for MultiArray<T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    type ItemMut<'a>
        = &'a mut T
    where
        T: 'a,
        S: 'a,
        C: 'a;

    cc_traits::covariant_item_mut!();
}

impl<T, S, C> Len for MultiArray<T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    fn len(&self) -> usize {
        self.list.len()
    }

    fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

impl<T, S, C> Index<usize> for MultiArray<T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    type Output = T;

    /// Index by linear index / 按线性索引访问
    fn index(&self, index: usize) -> &Self::Output {
        &self.list[index]
    }
}

impl<T, S, C> IndexMut<usize> for MultiArray<T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    /// Mutable index by linear index / 按线性索引可变访问
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.list[index]
    }
}

impl<T, S, C> Index<&S::VectorType> for MultiArray<T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    type Output = T;

    /// Index by multi-dimensional vector / 按多维向量索引访问
    fn index(&self, vector: &S::VectorType) -> &Self::Output {
        let index = self
            .shape
            .index_of(vector)
            .expect("Vector index out of bounds");
        &self.list[index]
    }
}

impl<T, S, C> IndexMut<&S::VectorType> for MultiArray<T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    /// Mutable index by multi-dimensional vector / 按多维向量索引可变访问
    fn index_mut(&mut self, vector: &S::VectorType) -> &mut Self::Output {
        let index = self
            .shape
            .index_of(vector)
            .expect("Vector index out of bounds");
        &mut self.list[index]
    }
}

impl<T, S: AbstractRTShape, C: MultiArrayCollection<T>> MultiArrayToView<S>
    for MultiArray<T, S, C>
{
    type ViewType<'a>
        = MultiArrayView<'a, T, S, C>
    where
        C: 'a,
        S: 'a,
        T: 'a;

    fn view(
        &self,
        dummy_vector: &S::DummyVectorType,
    ) -> Result<Self::ViewType<'_>, MappingIndexError> {
        Ok(MultiArrayView::new_by_dummy(self, dummy_vector))
    }

    fn map_view(
        &self,
        map_vector: &S::MapVectorType,
    ) -> Result<Self::ViewType<'_>, MappingIndexError> {
        MultiArrayView::new_by_map(self, map_vector.clone())
    }
}

/// # MultiArrayIter - Immutable Iterator for MultiArray
///
/// An iterator that yields immutable references to elements in a MultiArray.
///
/// MultiArray 的不可变迭代器，生成元素的不可变引用。
pub struct MultiArrayIter<'a, T, C: MultiArrayCollection<T> + 'a> {
    inner: <C as Iter>::Iter<'a>,
    _marker: PhantomData<&'a T>,
}

impl<'a, T: 'a, C: MultiArrayCollection<T> + 'a> Iterator for MultiArrayIter<'a, T, C>
where
    for<'b> <C as CollectionRef>::ItemRef<'b>: Into<&'b T>,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|item_ref| item_ref.into())
    }
}

/// # MultiArrayIterMut - Mutable Iterator for MultiArray
///
/// An iterator that yields mutable references to elements in a MultiArray.
///
/// MultiArray 的可变迭代器，生成元素的可变引用。
pub struct MultiArrayIterMut<'a, T, C: MultiArrayCollection<T> + 'a> {
    inner: <C as IterMut>::IterMut<'a>,
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T: 'a, C: MultiArrayCollection<T> + 'a> Iterator for MultiArrayIterMut<'a, T, C>
where
    for<'b> <C as CollectionMut>::ItemMut<'b>: Into<&'b mut T>,
{
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|item_mut| item_mut.into())
    }
}

/// # MultiArrayEnumerateIter - Enumerate Iterator for MultiArray
///
/// An iterator that yields tuples of (iteration_index, linear_index, vector, element_reference).
/// This allows accessing the iteration count, linear index, multi-dimensional vector, and element simultaneously.
///
/// MultiArray 的枚举迭代器，生成（迭代序号，线性索引，向量，元素引用）的四元组。
/// 这使得可以同时访问迭代序号、线性索引、多维向量和元素。
pub struct MultiArrayEnumerateIter<'a, T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    array: &'a MultiArray<T, S, C>,
    current_index: usize,
}

impl<'a, T, S, C> MultiArrayEnumerateIter<'a, T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    /// Create a new enumerate iterator from a MultiArray.
    ///
    /// 从 MultiArray 创建新的枚举迭代器。
    pub fn new(array: &'a MultiArray<T, S, C>) -> Self {
        Self {
            array,
            current_index: 0,
        }
    }
}

impl<'a, T, S, C> Iterator for MultiArrayEnumerateIter<'a, T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    type Item = (usize, S::VectorType, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.array.len() {
            return None;
        }

        let index = self.current_index;
        let vector = self.array.shape.vector_of(index).unwrap();
        let element = &self.array[index];
        self.current_index += 1;
        
        Some((index, vector, element))
    }
}

impl<T, S: AbstractRTShape, C: MultiArrayCollection<T>> Iter for MultiArray<T, S, C>
where
    for<'a> <C as CollectionRef>::ItemRef<'a>: Into<&'a T>,
{
    type Iter<'a>
        = MultiArrayIter<'a, T, C>
    where
        C: 'a,
        S: 'a,
        T: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        MultiArrayIter {
            inner: self.list.iter(),
            _marker: PhantomData,
        }
    }
}

impl<T, S: AbstractRTShape, C: MultiArrayCollection<T>> IterMut for MultiArray<T, S, C>
where
    for<'a> <C as CollectionMut>::ItemMut<'a>: Into<&'a mut T>,
{
    type IterMut<'a>
        = MultiArrayIterMut<'a, T, C>
    where
        C: 'a,
        S: 'a,
        T: 'a;

    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        MultiArrayIterMut {
            inner: self.list.iter_mut(),
            _marker: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AbstractShape;
    use crate::dummy_index::DummyIndex;
    use crate::map_index::{_0, _1, _2, _3, MapIndex};
    use crate::shape::Shape;
    use cc_traits::{Iter, IterMut, Len};
    use paste::paste;

    #[test]
    fn test_multi_array_creation() {
        let shape = Shape::new([2, 3]);
        let array: MultiArray<i32, _, _> = MultiArrayBuilder::new(shape);

        assert_eq!(array.len(), 6);
        assert!(!array.is_empty());

        let array_with = MultiArrayBuilder::new_with(Shape::new([2, 3]), 42);
        assert_eq!(array_with.len(), 6);

        let array_by = MultiArrayBuilder::new_by(Shape::new([2, 3]), |index, _vec| index * 2);
        assert_eq!(array_by.len(), 6);
    }

    #[test]
    fn test_collection_traits() {
        let shape = Shape::new([2, 3]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        assert_eq!(array.len(), 6);
        assert!(!array.is_empty());

        let items: Vec<i32> = array.iter().copied().collect();
        assert_eq!(items.len(), 6);
        for item in &items {
            assert_eq!(*item, 0);
        }

        for item in array.iter_mut() {
            *item = 42;
        }

        let modified_items: Vec<i32> = array.iter().copied().collect();
        for item in &modified_items {
            assert_eq!(*item, 42);
        }
    }

    #[test]
    fn test_iter_traits() {
        let shape = Shape::new([2, 3]);
        let array: MultiArray<_, _, _> =
            MultiArrayBuilder::new_by(shape.clone(), |index, _vec| index);

        let mut sum = 0;
        for &item in array.iter() {
            sum += item;
        }
        assert_eq!(sum, 0 + 1 + 2 + 3 + 4 + 5);

        assert_eq!(array.iter().count(), 6);

        let mut array_mut = MultiArrayBuilder::new_by(shape, |index, _vec| index);

        for item in array_mut.iter_mut() {
            *item *= 2;
        }

        let expected: Vec<usize> = vec![0, 2, 4, 6, 8, 10];
        for (i, &item) in array_mut.iter().enumerate() {
            assert_eq!(item, expected[i]);
        }
    }

    #[test]
    fn test_multi_array_with_different_shapes() {
        let shape1d = Shape::new([5]);
        let array1d = MultiArrayBuilder::new_with(shape1d, 1);
        assert_eq!(array1d.len(), 5);

        let shape3d = Shape::new([2, 2, 2]);
        let array3d = MultiArrayBuilder::new_with(shape3d, 2);
        assert_eq!(array3d.len(), 8);

        let mut expected = 0;
        for &item in array3d.iter() {
            assert_eq!(item, 2);
            expected += 1;
        }
        assert_eq!(expected, 8);
    }

    #[test]
    fn test_empty_array() {
        let shape = Shape::new([0, 5]);
        let array = MultiArrayBuilder::new::<i32, _>(shape);

        assert_eq!(array.len(), 0);
        assert!(array.is_empty());
        assert_eq!(array.iter().count(), 0);

        let shape2 = Shape::new([5, 0]);
        let array2 = MultiArrayBuilder::new::<i32, _>(shape2);

        assert_eq!(array2.len(), 0);
        assert!(array2.is_empty());
    }

    #[test]
    fn test_new_by_with_vector() {
        let shape = Shape::new([2, 3]);
        let shape_clone = shape.clone();
        let array = MultiArrayBuilder::new_by(shape, |index, vec| {
            let calculated_index = shape_clone.index_of(vec).unwrap();
            assert_eq!(index, calculated_index);
            index * 10
        });

        for (i, &item) in array.iter().enumerate() {
            assert_eq!(item, i * 10);
        }
    }

    #[test]
    fn test_index_with_vector_type() {
        let shape = Shape::new([2, 3]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        let vector = [0, 0];
        array[&vector] = 10;
        assert_eq!(array[&vector], 10);

        let vector = [0, 1];
        array[&vector] = 20;
        assert_eq!(array[&vector], 20);

        let vector = [1, 2];
        array[&vector] = 30;
        assert_eq!(array[&vector], 30);

        assert_eq!(array[&[0, 2]], 0);
        assert_eq!(array[&[1, 0]], 0);
        assert_eq!(array[&[1, 1]], 0);
    }

    #[test]
    fn test_index_with_usize() {
        let shape = Shape::new([2, 3]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        array[0] = 100;
        assert_eq!(array[0], 100);

        array[5] = 200;
        assert_eq!(array[5], 200);

        let vector = [0, 1];
        array[&vector] = 300;
        assert_eq!(array[1], 300);

        let vector = [1, 2];
        array[&vector] = 400;
        assert_eq!(array[5], 400);
    }

    #[test]
    fn test_to_view_trait_view() {
        use crate::dummy_expect;

        let shape = Shape::new([2, 3, 4]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = i as i32;
        }

        let dummy_vector = dummy_expect![0, 1..3, vec![0, 2, 3]];
        let view = array.view(&dummy_vector).expect("Should create view");

        assert_eq!(view.shape().dimension(), 0);
        assert_eq!(view.len(), 6);

        let dummy_vector2 = dummy_expect![0..2, 1, 2..4];
        let view2 = array.view(&dummy_vector2).expect("Should create view");

        assert_eq!(view2.shape().dimension(), 0);
        assert_eq!(view2.len(), 4);
    }

    #[test]
    fn test_to_view_trait_map_view() {
        use crate::map_expect;

        let shape = Shape::new([2, 3, 4]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = i as i32;
        }

        let map_vector = map_expect![_0, _1, _2];

        let view = array.map_view(&map_vector).expect("Should create map view");

        assert_eq!(view.shape().dimension(), 3);
        assert_eq!(view.shape().len_of_dimension(0).unwrap(), 2);
        assert_eq!(view.shape().len_of_dimension(1).unwrap(), 3);
        assert_eq!(view.shape().len_of_dimension(2).unwrap(), 4);
        assert_eq!(view.len(), 24);

        let map_vector2 = map_expect![_2, _1, _0];
        let view2 = array
            .map_view(&map_vector2)
            .expect("Should create map view");

        assert_eq!(view2.shape().dimension(), 3);
        assert_eq!(view2.shape().len_of_dimension(0).unwrap(), 4);
        assert_eq!(view2.shape().len_of_dimension(1).unwrap(), 3);
        assert_eq!(view2.shape().len_of_dimension(2).unwrap(), 2);
        assert_eq!(view2.len(), 24);

        let map_vector3 = map_expect![_0, 1, _1];
        let view3 = array
            .map_view(&map_vector3)
            .expect("Should create map view");

        assert_eq!(view3.shape().dimension(), 2);
        assert_eq!(view3.shape().len_of_dimension(0).unwrap(), 2);
        assert_eq!(view3.shape().len_of_dimension(1).unwrap(), 4);
        assert_eq!(view3.len(), 8);
    }

    #[test]
    fn test_to_view_trait_error_cases() {
        use crate::map_expect;

        let shape = Shape::new([2, 3, 4]);
        let array = MultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![_0, _0, _1];
        let result = array.map_view(&map_vector);
        assert!(result.is_err());

        // let map_vector2 = map_expect![_0, _2];
        // let result2 = array.map_view(&map_vector2);
        // assert!(result2.is_err());

        let map_vector3 = map_expect![_0, _1, _3];
        let result3 = array.map_view(&map_vector3);
        assert!(result3.is_err());
    }

    #[test]
    fn test_to_view_trait_chained_views() {
        use crate::map_expect;

        let shape = Shape::new([2, 3, 4]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = i as i32;
        }

        let map_vector = map_expect![_0, _1, _2];
        let view1 = array
            .map_view(&map_vector)
            .expect("Should create first view");

        let map_vector = dyn_map_expect![0, 1..3, _0];
        let view2 = view1
            .map_view(&map_vector)
            .expect("Should create second view from first view");

        assert_eq!(view2.shape().dimension(), 1);
        assert_eq!(view2.len(), 8);

        // Test dimension mismatch error with wrong size map_vector
        // 测试维度不匹配错误
        let map_vector2 = dyn_map_expect![_2, _1, _0];
        let view3 = view2.map_view(&map_vector2);
        assert!(view3.is_err());
    }

    #[test]
    fn test_storage_order_row_major_creation() {
        use crate::concept::StorageOrder;

        let shape_vec = vec![2, 3];
        let array: MultiArray<i32, DynShape> =
            MultiArrayBuilder::new_with_order(&shape_vec, StorageOrder::RowMajor);

        assert_eq!(array.storage_order(), StorageOrder::RowMajor);
        assert_eq!(array.len(), 6);

        assert_eq!(array.shape.offset_of_dimension(0).unwrap(), 3);
        assert_eq!(array.shape.offset_of_dimension(1).unwrap(), 1);
    }

    #[test]
    fn test_storage_order_column_major_creation() {
        use crate::concept::StorageOrder;

        let shape_vec = vec![2, 3];
        let array: MultiArray<i32, DynShape> =
            MultiArrayBuilder::new_with_order(&shape_vec, StorageOrder::ColumnMajor);

        assert_eq!(array.storage_order(), StorageOrder::ColumnMajor);
        assert_eq!(array.len(), 6);

        assert_eq!(array.shape.offset_of_dimension(0).unwrap(), 1);
        assert_eq!(array.shape.offset_of_dimension(1).unwrap(), 2);
    }

    #[test]
    fn test_storage_order_conversion() {
        use crate::concept::StorageOrder;

        let shape_vec = vec![2, 3];
        let array_row: MultiArray<i32, DynShape> =
            MultiArrayBuilder::new_with_order(&shape_vec, StorageOrder::RowMajor);

        let mut array = array_row;
        for i in 0..2 {
            for j in 0..3 {
                array[&vec![i, j]] = (i * 3 + j) as i32;
            }
        }

        let array_col = array.to_storage_order(StorageOrder::ColumnMajor);

        assert_eq!(array_col.storage_order(), StorageOrder::ColumnMajor);
        assert_eq!(array_col.len(), 6);

        let original_values: Vec<i32> = (0..array.len()).map(|i| array[i]).collect();
        let converted_values: Vec<i32> = (0..array_col.len()).map(|i| array_col[i]).collect();

        let mut original_sorted = original_values.clone();
        let mut converted_sorted = converted_values.clone();
        original_sorted.sort();
        converted_sorted.sort();
        assert_eq!(original_sorted, converted_sorted);
    }

    #[test]
    fn test_storage_order_3d_column_major() {
        use crate::concept::StorageOrder;

        let shape_vec = vec![2, 3, 4];
        let array: MultiArray<i32, DynShape> =
            MultiArrayBuilder::new_with_order(&shape_vec, StorageOrder::ColumnMajor);

        assert_eq!(array.storage_order(), StorageOrder::ColumnMajor);
        assert_eq!(array.len(), 24);

        assert_eq!(array.shape.offset_of_dimension(0).unwrap(), 1);
        assert_eq!(array.shape.offset_of_dimension(1).unwrap(), 2);
        assert_eq!(array.shape.offset_of_dimension(2).unwrap(), 6);
    }

    #[test]
    fn test_access_order_in_view() {
        use crate::concept::AccessOrder;

        let shape = Shape::new([2, 3, 4]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = i as i32;
        }

        let dummy_vector = dummy_expect![0..2, 0..3, 0..2];
        let view = array.view(&dummy_vector).unwrap();

        assert_eq!(view.access_order(), AccessOrder::RowMajor);

        let row_major_values: Vec<i32> = view
            .iter_with_order(AccessOrder::RowMajor)
            .copied()
            .collect();
        let col_major_values: Vec<i32> = view
            .iter_with_order(AccessOrder::ColumnMajor)
            .copied()
            .collect();

        assert_eq!(row_major_values.len(), col_major_values.len());
        assert_eq!(row_major_values.len(), 12);
    }

    #[test]
    fn test_access_order_column_major_iteration() {
        use crate::concept::AccessOrder;
        use crate::dummy_expect;

        let shape = Shape::new([2, 2]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        array[&[0, 0]] = 0;
        array[&[0, 1]] = 1;
        array[&[1, 0]] = 2;
        array[&[1, 1]] = 3;

        let dummy_vector = dummy_expect![0..2, 0..2];
        let view = array.view(&dummy_vector).unwrap();

        let row_major: Vec<i32> = view
            .iter_with_order(AccessOrder::RowMajor)
            .copied()
            .collect();
        assert_eq!(row_major, vec![0, 1, 2, 3]);

        let col_major: Vec<i32> = view
            .iter_with_order(AccessOrder::ColumnMajor)
            .copied()
            .collect();
        assert_eq!(col_major, vec![0, 2, 1, 3]);
    }

    #[test]
    fn test_combined_storage_and_access_order() {
        use crate::concept::{AccessOrder, StorageOrder};
        use crate::dyn_dummy_expect;

        let shape_vec = vec![2, 3];
        let mut array: MultiArray<i32, DynShape> =
            MultiArrayBuilder::new_with_order(&shape_vec, StorageOrder::ColumnMajor);

        for i in 0..array.len() {
            array[i] = i as i32;
        }

        assert_eq!(array.storage_order(), StorageOrder::ColumnMajor);

        let dummy_vector = dyn_dummy_expect![0..2, 0..3];
        let view = array.view(&dummy_vector).unwrap();

        let row_order: Vec<i32> = view
            .iter_with_order(AccessOrder::RowMajor)
            .copied()
            .collect();
        assert_eq!(row_order.len(), 6);

        let col_order: Vec<i32> = view
            .iter_with_order(AccessOrder::ColumnMajor)
            .copied()
            .collect();
        assert_eq!(col_order.len(), 6);

        let mut row_sorted = row_order.clone();
        let mut col_sorted = col_order.clone();
        row_sorted.sort();
        col_sorted.sort();
        assert_eq!(row_sorted, col_sorted);
    }

    #[test]
    fn test_enumerate_iterator() {
        let shape = Shape::new([2, 3]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        // 填充数据
        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        // 测试 enumerate 迭代器（返回四个字段：迭代序号，线性索引，向量，元素）
        let mut count = 0;
        for (index, vector, value) in array.enumerate() {
            assert_eq!(index, count);
            assert_eq!(*value, (count + 1) as i32);
            count += 1;
        }
        assert_eq!(count, 6);
    }

    #[test]
    fn test_enumerate_iterator_3d() {
        let shape = Shape::new([2, 3, 4]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        // 填充数据
        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        // 测试 3D 数组的 enumerate 迭代器（返回四个字段）
        let mut count = 0;
        for (index, vector, value) in array.enumerate() {
            assert_eq!(index, count);
            assert_eq!(*value, (count + 1) as i32);
            // 验证向量索引可以转换回线性索引
            let calculated_index = array.shape.index_of(&vector).unwrap();
            assert_eq!(calculated_index, index);
            count += 1;
        }
        assert_eq!(count, 24);
    }

    #[test]
    fn test_enumerate_iterator_empty() {
        let shape = Shape::new([0, 3]);
        let array: MultiArray<i32, _> = MultiArrayBuilder::new(shape);

        // 测试空数组的 enumerate 迭代器
        let mut count = 0;
        for (index, _vector, _value) in array.enumerate() {
            count += 1;
        }
        assert_eq!(count, 0);
    }

    #[test]
    fn test_reshape_with_generator() {
        let shape = Shape::new([2, 2]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        // 填充数据
        array[0] = 10;
        array[1] = 20;
        array[2] = 30;
        array[3] = 40;

        // 重塑为更大的形状，使用生成器填充
        let new_shape = Shape::new([3, 3]);
        let reshaped = array.reshape_by(new_shape, |index, _vec| -(index as i32));

        assert_eq!(reshaped.len(), 9);

        // 验证原数据保持不变
        assert_eq!(reshaped[0], 10);
        assert_eq!(reshaped[1], 20);
        assert_eq!(reshaped[2], 30);
        assert_eq!(reshaped[3], 40);

        // 验证新增元素由生成器生成
        assert_eq!(reshaped[4], -4);
        assert_eq!(reshaped[5], -5);
        assert_eq!(reshaped[6], -6);
        assert_eq!(reshaped[7], -7);
        assert_eq!(reshaped[8], -8);
    }

    #[test]
    fn test_reshape_smaller() {
        let shape = Shape::new([2, 2]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        // 填充数据
        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        // 重塑为更小的形状
        // 注意：reshape 会按顺序拷贝所有原数据，如果原数据更多，则新数组会包含所有原数据
        // 新 shape 只决定数组的维度结构，不截断数据
        let new_shape = Shape::new([1, 2]);
        let reshaped = array.reshape_with(new_shape, -1);

        // reshape 会保留原数据，所以长度是原数组长度和新形状长度的较大值
        assert_eq!(reshaped.len(), 4);

        // 验证所有原数据都保留了
        assert_eq!(reshaped[0], 1);
        assert_eq!(reshaped[1], 2);
        assert_eq!(reshaped[2], 3);
        assert_eq!(reshaped[3], 4);
    }

    #[test]
    fn test_reshape_different_dimensions() {
        let shape = Shape::new([2, 6]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        // 填充数据
        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        // 重塑为不同维度
        let new_shape = Shape::new([3, 4]);
        let reshaped = array.reshape_with(new_shape, -1);

        assert_eq!(reshaped.len(), 12);

        // 验证原数据保持不变
        for i in 0..12 {
            assert_eq!(reshaped[i], (i + 1) as i32);
        }
    }
}
