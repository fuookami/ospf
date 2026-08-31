//! 多维数组核心模块
//! Multi-dimensional array core module
//!
//! 本模块提供多维数组的核心实现：
//! This module provides the core implementation of multi-dimensional arrays:
//!
//! - `MultiArray<T, S, C>`: 泛型多维数组，支持任意形状和存储容器
//!   Generic multi-dimensional array with support for arbitrary shapes and storage containers
//! - `MultiArrayBuilder`: 数组构建器，提供便捷的构造方法
//!   Array builder providing convenient construction methods
//! - `MultiArrayCollection`: 集合 trait，定义数组存储容器的要求
//!   Collection trait defining requirements for array storage containers
//!
//! ## 示例 / Examples
//!
//! ```rust
//! use ospf_rust_multiarray::{MultiArray, MultiArrayBuilder, Shape, Shape2};
//!
//! // 创建一个 2x3 的数组 / Create a 2x3 array
//! let shape: Shape2 = Shape   ::new([2, 3]);
//! let array = MultiArray::<i32, _>::new_with(shape, 0);
//!
//! // 通过向量索引访问 / Access via vector index
//! let value = array[&[0, 1]];
//! ```

use super::concept::{AccessOrder, StorageOrder};
use super::error::MappingIndexError;
use super::multi_array_view::MultiArrayView;
use super::shape::{
    AbstractShape, DynShape, Shape1, Shape2, Shape3, Shape4, ShapeAccessOrderExt, ShapeIndicesIter,
};
use cc_traits::{Collection, CollectionMut, CollectionRef, Iter, IterMut, Len};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};

/// 多维数组集合 trait
/// Multi-dimensional array collection trait
///
/// 定义多维数组存储容器必须实现的接口。
/// Defines the interface that multi-dimensional array storage containers must implement.
///
/// ## 类型参数 / Type Parameters
///
/// - `T`: 元素类型 / Element type
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

/// 多维数组视图转换 trait
/// Multi-dimensional array to view conversion trait
///
/// 定义将多维数组转换为视图的能力。
/// Defines the ability to convert a multi-dimensional array to a view.
///
/// ## 类型参数 / Type Parameters
///
/// - `S`: 形状类型，必须实现 `AbstractShape`
///   Shape type, must implement `AbstractShape`
pub trait MultiArrayToView<S: AbstractShape> {
    /// 视图类型
    /// View type
    type ViewType<'a>: MultiArrayToView<DynShape>
    where
        Self: 'a;

    /// 使用虚拟索引向量创建视图
    /// Create a view using dummy index vector
    ///
    /// ## 参数 / Parameters
    ///
    /// - `dummy_vector`: 虚拟索引向量
    ///   Dummy index vector
    ///
    /// ## 返回值 / Returns
    ///
    /// 返回创建的视图或映射索引错误
    /// Returns the created view or a mapping index error
    fn view(
        &self,
        dummy_vector: &S::DummyVectorType,
    ) -> Result<Self::ViewType<'_>, MappingIndexError>;

    /// 使用映射索引向量创建视图
    /// Create a view using map index vector
    ///
    /// ## 参数 / Parameters
    ///
    /// - `map_vector`: 映射索引向量
    ///   Map index vector
    ///
    /// ## 返回值 / Returns
    ///
    /// 返回创建的视图或映射索引错误
    /// Returns the created view or a mapping index error
    fn map_view(
        &self,
        map_vector: &S::MapVectorType,
    ) -> Result<Self::ViewType<'_>, MappingIndexError>;
}

/// 多维数组
/// Multi-dimensional array
///
/// 泛型多维数组，支持任意形状和存储容器。
/// Generic multi-dimensional array with support for arbitrary shapes and storage containers.
///
/// ## 类型参数 / Type Parameters
///
/// - `T`: 元素类型
///   Element type
/// - `S`: 形状类型，必须实现 `AbstractShape`
///   Shape type, must implement `AbstractShape`
/// - `C`: 存储容器类型，默认为 `Vec<T>`
///   Storage container type, defaults to `Vec<T>`
///
/// ## 示例 / Examples
///
/// ```rust
/// use ospf_rust_multiarray::{MultiArray, Shape, RowMajor};
///
/// // 使用指定值创建数组 / Create array with specified value
/// let shape: Shape<2, RowMajor> = Shape::new([2, 3]);
/// let array: MultiArray<i32, _> = MultiArray::new_with(shape, 0);
/// assert_eq!(array.len(), 6);
/// ```
pub struct MultiArray<T, S, C = Vec<T>>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    /// 元素存储容器
    /// Element storage container
    list: C,

    /// 数组形状
    /// Array shape
    pub shape: S,

    /// 元素类型标记
    /// Element type marker
    _marker: PhantomData<T>,
}

impl<T, S, C> MultiArray<T, S, C>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    /// 使用默认值创建多维数组
    /// Create a multi-dimensional array with default values
    ///
    /// ## 参数 / Parameters
    ///
    /// - `shape`: 数组形状
    ///   Array shape
    ///
    /// ## 示例 / Examples
    ///
    /// ```rust
    /// use ospf_rust_multiarray::{MultiArray, Shape, RowMajor};
    ///
    /// let shape: Shape<2, RowMajor> = Shape::new([2, 3]);
    /// let array: MultiArray<i32, _> = MultiArray::new(shape);
    /// assert_eq!(array.len(), 6);
    /// ```
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

    /// 使用指定值创建多维数组
    /// Create a multi-dimensional array with a specified value
    ///
    /// ## 参数 / Parameters
    ///
    /// - `shape`: 数组形状
    ///   Array shape
    /// - `value`: 填充值
    ///   Fill value
    ///
    /// ## 示例 / Examples
    ///
    /// ```rust
    /// use ospf_rust_multiarray::{MultiArray, Shape, RowMajor};
    ///
    /// let shape: Shape<2, RowMajor> = Shape::new([2, 3]);
    /// let array: MultiArray<i32, _> = MultiArray::new_with(shape, 0);
    /// assert_eq!(array.len(), 6);
    /// ```
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

    /// 使用生成器函数创建多维数组
    /// Create a multi-dimensional array using a generator function
    ///
    /// ## 参数 / Parameters
    ///
    /// - `shape`: 数组形状
    ///   Array shape
    /// - `generator`: 元素生成器，接收线性索引和向量坐标
    ///   Element generator, receives linear index and vector coordinates
    ///
    /// ## 示例 / Examples
    ///
    /// ```rust
    /// use ospf_rust_multiarray::{MultiArray, Shape, RowMajor};
    ///
    /// let shape: Shape<2, RowMajor> = Shape::new([2, 3]);
    /// let array: MultiArray<i32, _> = MultiArray::new_by(shape, |idx, _vec| idx as i32);
    /// assert_eq!(array.len(), 6);
    /// assert_eq!(array[0], 0);
    /// assert_eq!(array[5], 5);
    /// ```
    pub fn new_by<G>(shape: S, generator: G) -> Self
    where
        G: Fn(usize, &<S as AbstractShape>::VectorType) -> T,
    {
        Self {
            list: (0..shape.len())
                .map(|index| generator(index, &shape.vector_of(index).expect("linear index should be valid in new_by")))
                .collect(),
            shape,
            _marker: PhantomData,
        }
    }

    /// 获取存储顺序
    /// Get the storage order
    ///
    /// ## 返回值 / Returns
    ///
    /// 返回当前数组的存储顺序（行主序或列主序）
    /// Returns the storage order of the current array (row-major or column-major)
    pub fn storage_order(&self) -> StorageOrder {
        self.shape.storage_order()
    }

    /// 转换存储顺序
    /// Convert storage order
    ///
    /// 创建一个具有新存储顺序的数组副本，并相应地重新排列元素。
    /// Creates a copy of the array with a new storage order, rearranging elements accordingly.
    ///
    /// ## 参数 / Parameters
    ///
    /// - `order`: 目标存储顺序
    ///   Target storage order
    ///
    /// ## 返回值 / Returns
    ///
    /// 返回具有新存储顺序的数组
    /// Returns an array with the new storage order
    pub fn to_storage_order(
        &self,
        order: StorageOrder,
    ) -> MultiArray<T, S::ShapeWithStorageOrder<StorageOrder>, C>
    where
        T: Clone,
        S: Clone,
        C: Clone,
    {
        if self.shape.storage_order() == order {
            let new_shape = self.shape.with_storage_order(order);
            return MultiArray {
                list: self.list.clone(),
                shape: new_shape,
                _marker: PhantomData,
            };
        }

        let new_shape = self.shape.with_storage_order(order);

        let mut reordered: Vec<T> = vec![self.list[0].clone(); self.len()];

        for i in 0..self.len() {
            let vector = self.shape.vector_of(i).expect("linear index should be valid in to_storage_order");
            let new_index = new_shape.index_of(&vector).expect("vector should be valid in to_storage_order");
            reordered[new_index] = self.list[i].clone();
        }

        MultiArray {
            list: reordered.into_iter().collect(),
            shape: new_shape,
            _marker: PhantomData,
        }
    }

    /// 重塑数组形状（使用默认值填充）
    /// Reshape array (fill with default values)
    ///
    /// 使用新形状创建数组，如果新形状更大则用默认值填充。
    /// Creates an array with a new shape, filling with default values if the new shape is larger.
    ///
    /// ## 类型参数 / Type Parameters
    ///
    /// - `NS`: 新形状类型
    ///   New shape type
    ///
    /// ## 参数 / Parameters
    ///
    /// - `new_shape`: 新形状
    ///   New shape
    ///
    /// ## 返回值 / Returns
    ///
    /// 返回重塑后的数组
    /// Returns the reshaped array
    pub fn reshape<NS>(&self, new_shape: NS) -> MultiArray<T, NS, C>
    where
        T: Default + Clone,
        NS: AbstractShape,
        C: FromIterator<T>,
    {
        let mut new_list: Vec<T> = Vec::with_capacity(new_shape.len());

        // 只复制新形状需要的元素数量
        // Only copy the number of elements needed by the new shape
        let copy_count = self.len().min(new_shape.len());
        for i in 0..copy_count {
            new_list.push(self.list[i].clone());
        }

        // 如果新形状更大，用默认值补充
        // If the new shape is larger, fill with default values
        for _ in copy_count..new_shape.len() {
            new_list.push(T::default());
        }

        MultiArray {
            list: new_list.into_iter().collect(),
            shape: new_shape,
            _marker: PhantomData,
        }
    }

    /// 重塑数组形状（使用指定值填充）
    /// Reshape array (fill with specified value)
    ///
    /// 使用新形状创建数组，如果新形状更大则用指定值填充。
    /// Creates an array with a new shape, filling with specified value if the new shape is larger.
    ///
    /// ## 类型参数 / Type Parameters
    ///
    /// - `NS`: 新形状类型
    ///   New shape type
    ///
    /// ## 参数 / Parameters
    ///
    /// - `new_shape`: 新形状
    ///   New shape
    /// - `fill_value`: 填充值
    ///   Fill value
    ///
    /// ## 返回值 / Returns
    ///
    /// 返回重塑后的数组
    /// Returns the reshaped array
    pub fn reshape_with<NS>(&self, new_shape: NS, fill_value: T) -> MultiArray<T, NS, C>
    where
        T: Clone,
        NS: AbstractShape,
        C: FromIterator<T>,
    {
        let mut new_list: Vec<T> = Vec::with_capacity(new_shape.len());

        // 只复制新形状需要的元素数量
        // Only copy the number of elements needed by the new shape
        let copy_count = self.len().min(new_shape.len());
        for i in 0..copy_count {
            new_list.push(self.list[i].clone());
        }

        // 如果新形状更大，用填充值补充
        // If the new shape is larger, fill with the fill value
        for _ in copy_count..new_shape.len() {
            new_list.push(fill_value.clone());
        }

        MultiArray {
            list: new_list.into_iter().collect(),
            shape: new_shape,
            _marker: PhantomData,
        }
    }

    /// 重塑数组形状（使用生成器填充）
    /// Reshape array (fill with generator)
    ///
    /// 使用新形状创建数组，如果新形状更大则用生成器填充。
    /// Creates an array with a new shape, filling with generator if the new shape is larger.
    ///
    /// ## 类型参数 / Type Parameters
    ///
    /// - `NS`: 新形状类型
    ///   New shape type
    /// - `G`: 生成器函数类型
    ///   Generator function type
    ///
    /// ## 参数 / Parameters
    ///
    /// - `new_shape`: 新形状
    ///   New shape
    /// - `generator`: 元素生成器
    ///   Element generator
    ///
    /// ## 返回值 / Returns
    ///
    /// 返回重塑后的数组
    /// Returns the reshaped array
    pub fn reshape_by<NS, G>(&self, new_shape: NS, generator: G) -> MultiArray<T, NS, C>
    where
        T: Clone,
        NS: AbstractShape,
        G: Fn(usize, &<NS as AbstractShape>::VectorType) -> T,
        C: FromIterator<T>,
    {
        let mut new_list: Vec<T> = Vec::with_capacity(new_shape.len());

        for i in 0..self.len() {
            new_list.push(self.list[i].clone());
        }

        for i in self.len()..new_shape.len() {
            let vector = new_shape.vector_of(i).expect("linear index should be valid in reshape_by");
            new_list.push(generator(i, &vector));
        }

        MultiArray {
            list: new_list.into_iter().collect(),
            shape: new_shape,
            _marker: PhantomData,
        }
    }

    /// 获取枚举迭代器
    /// Get an enumerate iterator
    ///
    /// 返回一个迭代器，产生 (线性索引, 向量坐标, 元素引用) 三元组。
    /// Returns an iterator that yields (linear index, vector coordinate, element reference) triples.
    ///
    /// ## 返回值 / Returns
    ///
    /// 返回枚举迭代器
    /// Returns the enumerate iterator
    pub fn enumerate(&self) -> MultiArrayEnumerateIter<'_, T, S, C> {
        MultiArrayEnumerateIter::new(self)
    }

    /// 使用指定访问顺序迭代元素 / Iterate elements with specified access order.
    pub fn iter_with_order(
        &self,
        access_order: AccessOrder,
    ) -> MultiArrayWithOrderIter<'_, T, S, C> {
        MultiArrayWithOrderIter::new(self, access_order)
    }

    /// 使用指定访问顺序进行枚举迭代 / Enumerate iteration with specified access order.
    pub fn enumerate_with_order(
        &self,
        access_order: AccessOrder,
    ) -> MultiArrayEnumerateWithOrderIter<'_, T, S, C> {
        MultiArrayEnumerateWithOrderIter::new(self, access_order)
    }

    /// 按访问顺序展平为列表 / Flatten to a list with specified access order.
    pub fn flatten(&self, access_order: AccessOrder) -> std::vec::Vec<T>
    where
        T: Clone,
    {
        self.iter_with_order(access_order).cloned().collect()
    }
}

impl<T, S, C> Clone for MultiArray<T, S, C>
where
    T: Clone,
    S: AbstractShape + Clone,
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

/// 带访问顺序的数组迭代器 / Multi-array iterator with explicit access order.
pub struct MultiArrayWithOrderIter<'a, T, S, C>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    array: &'a MultiArray<T, S, C>,
    indices: ShapeIndicesIter<'a, S>,
}

impl<'a, T, S, C> MultiArrayWithOrderIter<'a, T, S, C>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    pub fn new(array: &'a MultiArray<T, S, C>, access_order: AccessOrder) -> Self {
        Self {
            array,
            indices: array.shape.iterate(access_order),
        }
    }
}

impl<'a, T, S, C> Iterator for MultiArrayWithOrderIter<'a, T, S, C>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let vector = self.indices.next()?;
        let index = self.array.shape.index_of(&vector).ok()?;
        Some(&self.array[index])
    }
}

/// 带访问顺序的数组枚举迭代器 / Multi-array enumerate iterator with explicit access order.
pub struct MultiArrayEnumerateWithOrderIter<'a, T, S, C>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    array: &'a MultiArray<T, S, C>,
    indices: ShapeIndicesIter<'a, S>,
    current_index: usize,
}

impl<'a, T, S, C> MultiArrayEnumerateWithOrderIter<'a, T, S, C>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    pub fn new(array: &'a MultiArray<T, S, C>, access_order: AccessOrder) -> Self {
        Self {
            array,
            indices: array.shape.iterate(access_order),
            current_index: 0,
        }
    }
}

impl<'a, T, S, C> Iterator for MultiArrayEnumerateWithOrderIter<'a, T, S, C>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    type Item = (usize, S::VectorType, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        let vector = self.indices.next()?;
        let linear_index = self.array.shape.index_of(&vector).ok()?;
        let index = self.current_index;
        self.current_index += 1;
        Some((index, vector, &self.array[linear_index]))
    }
}

/// 多维数组构建器
/// Multi-dimensional array builder
///
/// 提供便捷的静态方法来创建多维数组。
/// Provides convenient static methods to create multi-dimensional arrays.
///
/// ## 示例 / Examples
///
/// ```rust
/// use ospf_rust_multiarray::{MultiArrayBuilder, Shape, RowMajor};
///
/// let shape: Shape<2, RowMajor> = Shape::new([2, 3]);
///
/// // 使用默认值创建 / Create with default values
/// let array = MultiArrayBuilder::new::<i32, _>(shape.clone());
/// assert_eq!(array.len(), 6);
///
/// // 使用指定值创建 / Create with specified value
/// let array = MultiArrayBuilder::new_with(shape.clone(), 0);
/// assert_eq!(array.len(), 6);
///
/// // 使用生成器创建 / Create with generator
/// let array = MultiArrayBuilder::new_by(shape, |idx, _vec| idx as i32);
/// assert_eq!(array.len(), 6);
/// ```
pub struct MultiArrayBuilder {}

impl MultiArrayBuilder {
    /// 使用默认值创建多维数组
    /// Create a multi-dimensional array with default values
    pub fn new<T, S>(shape: S) -> MultiArray<T, S>
    where
        T: Default,
        S: AbstractShape,
    {
        MultiArray::<T, S>::new(shape)
    }

    /// 使用默认值创建多维数组（指定容器类型）
    /// Create a multi-dimensional array with default values (specify container type)
    pub fn new_as<T, S, C>(shape: S) -> MultiArray<T, S, C>
    where
        T: Default,
        S: AbstractShape,
        C: MultiArrayCollection<T>,
    {
        MultiArray::<T, S, C>::new(shape)
    }

    /// 使用指定值创建多维数组
    /// Create a multi-dimensional array with a specified value
    pub fn new_with<T, S>(shape: S, value: T) -> MultiArray<T, S>
    where
        T: Clone,
        S: AbstractShape,
    {
        MultiArray::<T, S>::new_with(shape, value)
    }

    /// 使用指定值创建多维数组（指定容器类型）
    /// Create a multi-dimensional array with a specified value (specify container type)
    pub fn new_with_as<T, S, C>(shape: S, value: T) -> MultiArray<T, S, C>
    where
        T: Clone,
        S: AbstractShape,
        C: MultiArrayCollection<T>,
    {
        MultiArray::<T, S, C>::new_with(shape, value)
    }

    /// 使用生成器函数创建多维数组
    /// Create a multi-dimensional array using a generator function
    pub fn new_by<T, S, G>(shape: S, generator: G) -> MultiArray<T, S>
    where
        S: AbstractShape,
        G: Fn(usize, &<S as AbstractShape>::VectorType) -> T,
    {
        MultiArray::<T, S>::new_by(shape, generator)
    }

    /// 使用生成器函数创建多维数组（指定容器类型）
    /// Create a multi-dimensional array using a generator function (specify container type)
    pub fn new_by_as<T, S, C, G>(shape: S, generator: G) -> MultiArray<T, S, C>
    where
        S: AbstractShape,
        C: MultiArrayCollection<T>,
        G: Fn(usize, &<S as AbstractShape>::VectorType) -> T,
    {
        MultiArray::<T, S, C>::new_by(shape, generator)
    }

    /// 按默认访问顺序（RowMajor）从列表创建数组 / Create array from list in default access order (RowMajor).
    pub fn from_list<T, S>(shape: S, list: std::vec::Vec<T>) -> MultiArray<T, S>
    where
        S: AbstractShape,
    {
        Self::from_list_with_order(shape, list, AccessOrder::default())
    }

    /// 按指定访问顺序从列表创建数组 / Create array from list in specified access order.
    pub fn from_list_with_order<T, S>(
        shape: S,
        list: std::vec::Vec<T>,
        access_order: AccessOrder,
    ) -> MultiArray<T, S>
    where
        S: AbstractShape,
    {
        assert_eq!(
            list.len(),
            shape.len(),
            "List size ({}) must match shape size ({}) / 列表长度 ({}) 必须等于形状长度 ({})",
            list.len(),
            shape.len(),
            list.len(),
            shape.len()
        );

        let mut reordered: std::vec::Vec<Option<T>> = (0..shape.len()).map(|_| None).collect();
        let mut list_iter = list.into_iter();

        for vector in shape.iterate(access_order) {
            let linear_index = shape.index_of(&vector).expect(
                "Shape iteration must always yield valid indices / 形状迭代必须产生有效索引",
            );
            let value = list_iter
                .next()
                .expect("List length must match shape size / 列表长度必须与形状长度匹配");
            reordered[linear_index] = Some(value);
        }

        let values: std::vec::Vec<T> = reordered
            .into_iter()
            .map(|value| {
                value.expect(
                    "All linear indices must be assigned exactly once / 所有线性索引必须且只能被赋值一次",
                )
            })
            .collect();

        MultiArray {
            list: values.into_iter().collect(),
            shape,
            _marker: PhantomData,
        }
    }

    /// 使用指定存储顺序创建多维数组
    /// Create a multi-dimensional array with specified storage order
    pub fn new_with_order<T, S>(
        shape: &S::ShapeVectorType,
        order: StorageOrder,
    ) -> MultiArray<T, S::ShapeWithStorageOrder<StorageOrder>>
    where
        T: Default,
        S: AbstractShape,
    {
        let shape_with_order = S::from_shape_vector_with_order(shape, order);
        MultiArray::<T, S::ShapeWithStorageOrder<StorageOrder>>::new(shape_with_order)
    }

    /// 使用指定存储顺序创建多维数组（指定容器类型）
    /// Create a multi-dimensional array with specified storage order (specify container type)
    pub fn new_with_order_as<T, S, C>(
        shape: &S::ShapeVectorType,
        order: StorageOrder,
        _collection: &C,
    ) -> MultiArray<T, S::ShapeWithStorageOrder<StorageOrder>, C>
    where
        T: Default,
        S: AbstractShape,
        C: MultiArrayCollection<T>,
    {
        let shape_with_order = S::from_shape_vector_with_order(shape, order);
        MultiArray::<T, S::ShapeWithStorageOrder<StorageOrder>, C>::new(shape_with_order)
    }

    /// 使用指定存储顺序和填充值创建多维数组
    /// Create a multi-dimensional array with specified storage order and fill value
    pub fn new_with_order_and_value<T, S>(
        shape: &S::ShapeVectorType,
        value: T,
        order: StorageOrder,
    ) -> MultiArray<T, S::ShapeWithStorageOrder<StorageOrder>>
    where
        T: Clone,
        S: AbstractShape,
    {
        let shape_with_order = S::from_shape_vector_with_order(shape, order);
        MultiArray::<T, S::ShapeWithStorageOrder<StorageOrder>>::new_with(shape_with_order, value)
    }

    /// 使用指定存储顺序和填充值创建多维数组（指定容器类型）
    /// Create a multi-dimensional array with specified storage order and fill value (specify container type)
    pub fn new_with_order_and_value_as<T, S, C>(
        shape: &S::ShapeVectorType,
        value: T,
        order: StorageOrder,
        _collection: &C,
    ) -> MultiArray<T, S::ShapeWithStorageOrder<StorageOrder>, C>
    where
        T: Clone,
        S: AbstractShape,
        C: MultiArrayCollection<T>,
    {
        let shape_with_order = S::from_shape_vector_with_order(shape, order);
        MultiArray::<T, S::ShapeWithStorageOrder<StorageOrder>, C>::new_with(
            shape_with_order,
            value,
        )
    }

    /// 使用指定存储顺序和生成器创建多维数组
    /// Create a multi-dimensional array with specified storage order and generator
    pub fn new_by_with_order<T, S, G>(
        shape: &S::ShapeVectorType,
        generator: G,
        order: StorageOrder,
    ) -> MultiArray<T, S::ShapeWithStorageOrder<StorageOrder>>
    where
        S: AbstractShape,
        G: Fn(usize, &<S as AbstractShape>::VectorType) -> T,
    {
        let shape_with_order = S::from_shape_vector_with_order(shape, order);
        MultiArray::<T, S::ShapeWithStorageOrder<StorageOrder>>::new_by(shape_with_order, generator)
    }

    /// 使用指定存储顺序和生成器创建多维数组（指定容器类型）
    /// Create a multi-dimensional array with specified storage order and generator (specify container type)
    pub fn new_by_with_order_as<T, S, C, G>(
        shape: &S::ShapeVectorType,
        generator: G,
        order: StorageOrder,
        _collection: &C,
    ) -> MultiArray<T, S::ShapeWithStorageOrder<StorageOrder>, C>
    where
        S: AbstractShape,
        C: MultiArrayCollection<T>,
        G: Fn(usize, &<S as AbstractShape>::VectorType) -> T,
    {
        let shape_with_order = S::from_shape_vector_with_order(shape, order);
        MultiArray::<T, S::ShapeWithStorageOrder<StorageOrder>, C>::new_by(
            shape_with_order,
            generator,
        )
    }
}

/// 一维多维数组类型别名。
/// 1D multi-array type alias.
pub type MultiArray1<T, SO = StorageOrder, C = Vec<T>> = MultiArray<T, Shape1<SO>, C>;

/// 二维多维数组类型别名。
/// 2D multi-array type alias.
pub type MultiArray2<T, SO = StorageOrder, C = Vec<T>> = MultiArray<T, Shape2<SO>, C>;

/// 三维多维数组类型别名。
/// 3D multi-array type alias.
pub type MultiArray3<T, SO = StorageOrder, C = Vec<T>> = MultiArray<T, Shape3<SO>, C>;

/// 四维多维数组类型别名。
/// 4D multi-array type alias.
pub type MultiArray4<T, SO = StorageOrder, C = Vec<T>> = MultiArray<T, Shape4<SO>, C>;

/// 动态维度多维数组类型别名。
/// Dynamic-dimensional multi-array type alias.
pub type DynMultiArray<T, C = Vec<T>> = MultiArray<T, DynShape, C>;

/// 使用给定形状和值创建多维数组。
/// Create a multi-array with a given shape and fill value.
pub fn multi_array_of_shape<T, S>(shape: S, value: T) -> MultiArray<T, S>
where
    T: Clone,
    S: AbstractShape,
{
    MultiArray::new_with(shape, value)
}

/// 创建一维多维数组。
/// Create a 1D multi-array.
pub fn multi_array_1<T>(d1: usize, value: T) -> MultiArray1<T>
where
    T: Clone,
{
    MultiArray::new_with(Shape1::new([d1]), value)
}

/// 创建二维多维数组。
/// Create a 2D multi-array.
pub fn multi_array_2<T>(d1: usize, d2: usize, value: T) -> MultiArray2<T>
where
    T: Clone,
{
    MultiArray::new_with(Shape2::new([d1, d2]), value)
}

/// 创建三维多维数组。
/// Create a 3D multi-array.
pub fn multi_array_3<T>(d1: usize, d2: usize, d3: usize, value: T) -> MultiArray3<T>
where
    T: Clone,
{
    MultiArray::new_with(Shape3::new([d1, d2, d3]), value)
}

impl<T, S, C> Deref for MultiArray<T, S, C>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    type Target = C;

    fn deref(&self) -> &Self::Target {
        &self.list
    }
}

impl<T, S, C> DerefMut for MultiArray<T, S, C>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.list
    }
}

impl<T, S, C> Collection for MultiArray<T, S, C>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    type Item = T;
}

impl<T, S, C> CollectionRef for MultiArray<T, S, C>
where
    S: AbstractShape,
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
    S: AbstractShape,
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
    S: AbstractShape,
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
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.list[index]
    }
}

impl<T, S, C> IndexMut<usize> for MultiArray<T, S, C>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.list[index]
    }
}

impl<T, S, C> Index<&S::VectorType> for MultiArray<T, S, C>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    type Output = T;

    fn index(&self, vector: &S::VectorType) -> &Self::Output {
        let index = self
            .shape
            .index_of(vector)
            .expect("Vector index out of bounds / 向量索引越界");
        &self.list[index]
    }
}

impl<T, S, C> IndexMut<&S::VectorType> for MultiArray<T, S, C>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    fn index_mut(&mut self, vector: &S::VectorType) -> &mut Self::Output {
        let index = self
            .shape
            .index_of(vector)
            .expect("Vector index out of bounds / 向量索引越界");
        &mut self.list[index]
    }
}

impl<T, S: AbstractShape<StorageOrder = crate::concept::StorageOrder>, C: MultiArrayCollection<T>>
    MultiArrayToView<S> for MultiArray<T, S, C>
{
    type ViewType<'a>
        = MultiArrayView<'a, T, S, AccessOrder, C>
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

/// 多维数组不可变迭代器
/// Multi-dimensional array immutable iterator
///
/// 遍历多维数组中所有元素的不可变引用。
/// Iterates over immutable references to all elements in a multi-dimensional array.
///
/// ## 类型参数 / Type Parameters
///
/// - `T`: 元素类型
///   Element type
/// - `C`: 存储容器类型
///   Storage container type
pub struct MultiArrayIter<'a, T, C: MultiArrayCollection<T> + 'a> {
    /// 内部迭代器
    /// Inner iterator
    inner: <C as Iter>::Iter<'a>,
    /// 类型标记
    /// Type marker
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

/// 多维数组可变迭代器
/// Multi-dimensional array mutable iterator
///
/// 遍历多维数组中所有元素的可变引用。
/// Iterates over mutable references to all elements in a multi-dimensional array.
///
/// ## 类型参数 / Type Parameters
///
/// - `T`: 元素类型
///   Element type
/// - `C`: 存储容器类型
///   Storage container type
pub struct MultiArrayIterMut<'a, T, C: MultiArrayCollection<T> + 'a> {
    /// 内部迭代器
    /// Inner iterator
    inner: <C as IterMut>::IterMut<'a>,
    /// 类型标记
    /// Type marker
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

/// 多维数组枚举迭代器
/// Multi-dimensional array enumerate iterator
///
/// 遍历多维数组，产生 (线性索引, 向量坐标, 元素引用) 三元组。
/// Iterates over a multi-dimensional array, yielding (linear index, vector coordinate, element reference) triples.
///
/// ## 类型参数 / Type Parameters
///
/// - `T`: 元素类型
///   Element type
/// - `S`: 形状类型
///   Shape type
/// - `C`: 存储容器类型
///   Storage container type
pub struct MultiArrayEnumerateIter<'a, T, S, C>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    /// 数组引用
    /// Array reference
    array: &'a MultiArray<T, S, C>,
    /// 当前索引
    /// Current index
    current_index: usize,
}

impl<'a, T, S, C> MultiArrayEnumerateIter<'a, T, S, C>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    /// 创建新的枚举迭代器
    /// Create a new enumerate iterator
    ///
    /// ## 参数 / Parameters
    ///
    /// - `array`: 数组引用
    ///   Array reference
    pub fn new(array: &'a MultiArray<T, S, C>) -> Self {
        Self {
            array,
            current_index: 0,
        }
    }
}

impl<'a, T, S, C> Iterator for MultiArrayEnumerateIter<'a, T, S, C>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    type Item = (usize, S::VectorType, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.array.len() {
            return None;
        }

        let index = self.current_index;
        let vector = self.array.shape.vector_of(index).expect("linear index should be valid in MultiArrayEnumerateIter::next");
        let element = &self.array[index];
        self.current_index += 1;

        Some((index, vector, element))
    }
}

impl<T, S: AbstractShape, C: MultiArrayCollection<T>> Iter for MultiArray<T, S, C>
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

impl<T, S: AbstractShape, C: MultiArrayCollection<T>> IterMut for MultiArray<T, S, C>
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
    use crate::DummyIndex;
    use crate::concept::StorageOrder as RuntimeStorageOrder;
    use crate::shape::Shape;

    type RTShape<const D: usize> = Shape<D, RuntimeStorageOrder>;

    #[test]
    fn test_multi_array_creation() {
        let shape: RTShape<2> = Shape::new([2, 3]);
        let array: MultiArray<i32, _> = MultiArrayBuilder::new(shape);

        assert_eq!(array.len(), 6);
        assert!(!array.is_empty());

        let array_with: MultiArray<i32, RTShape<2>> =
            MultiArrayBuilder::new_with(Shape::new([2, 3]), 42);
        assert_eq!(array_with.len(), 6);

        let array_by: MultiArray<i32, RTShape<2>> =
            MultiArrayBuilder::new_by(Shape::new([2, 3]), |index, _vec| (index * 2) as i32);
        assert_eq!(array_by.len(), 6);
    }

    #[test]
    fn test_collection_traits() {
        let shape: RTShape<2> = Shape::new([2, 3]);
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
    fn test_index_with_vector_type() {
        let shape: RTShape<2> = Shape::new([2, 3]);
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
    }

    #[test]
    fn test_index_with_usize() {
        let shape: RTShape<2> = Shape::new([2, 3]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        array[0] = 100;
        assert_eq!(array[0], 100);

        array[5] = 200;
        assert_eq!(array[5], 200);

        let vector = [0, 1];
        array[&vector] = 300;
        assert_eq!(array[1], 300);
    }

    #[test]
    fn test_storage_order() {
        use crate::concept::{ColumnMajor, RowMajor, StorageOrder};

        let shape: RTShape<2> = Shape::new([2, 3]);
        let array: MultiArray<i32, _> = MultiArrayBuilder::new_with(shape, 0);

        assert_eq!(array.storage_order(), StorageOrder::RowMajor);

        let shape_rm: Shape<2, RowMajor> = Shape::new([2, 3]);
        let array_rm: MultiArray<i32, Shape<2, RowMajor>> =
            MultiArrayBuilder::new_with(shape_rm, 0);
        assert_eq!(array_rm.storage_order(), StorageOrder::RowMajor);

        let shape_cm: Shape<2, ColumnMajor> = Shape::new([2, 3]);
        let array_cm: MultiArray<i32, Shape<2, ColumnMajor>> =
            MultiArrayBuilder::new_with(shape_cm, 0);
        assert_eq!(array_cm.storage_order(), StorageOrder::ColumnMajor);
    }

    #[test]
    fn test_enumerate_iterator() {
        let shape: RTShape<2> = Shape::new([2, 3]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        let mut count = 0;
        for (index, vector, value) in array.enumerate() {
            assert_eq!(index, count);
            assert_eq!(*value, (count + 1) as i32);
            count += 1;
        }
        assert_eq!(count, 6);
    }

    #[test]
    fn test_reshape() {
        let shape: RTShape<2> = Shape::new([2, 2]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        let new_shape: RTShape<2> = Shape::new([3, 3]);
        let reshaped = array.reshape_with(new_shape, -1);

        assert_eq!(reshaped.len(), 9);
        assert_eq!(reshaped[0], 1);
        assert_eq!(reshaped[1], 2);
        assert_eq!(reshaped[2], 3);
        assert_eq!(reshaped[3], 4);
        assert_eq!(reshaped[4], -1);
    }

    #[test]
    fn test_to_storage_order() {
        use crate::concept::{ColumnMajor, RowMajor};

        let shape_rm: Shape<2, RowMajor> = Shape::new([2, 3]);
        let array_rm: MultiArray<i32, Shape<2, RowMajor>> =
            MultiArrayBuilder::new_with(shape_rm, 1);

        let array_cm = array_rm.to_storage_order(StorageOrder::ColumnMajor);
        assert_eq!(array_cm.storage_order(), StorageOrder::ColumnMajor);
        assert_eq!(array_cm.len(), 6);

        let shape_cm: Shape<2, ColumnMajor> = Shape::new([2, 3]);
        let array_cm2: MultiArray<i32, Shape<2, ColumnMajor>> =
            MultiArrayBuilder::new_with(shape_cm, 1);

        let array_rm2 = array_cm2.to_storage_order(StorageOrder::RowMajor);
        assert_eq!(array_rm2.storage_order(), StorageOrder::RowMajor);
        assert_eq!(array_rm2.len(), 6);
    }

    #[test]
    fn test_clone() {
        let shape: RTShape<2> = Shape::new([2, 3]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        let cloned = array.clone();
        assert_eq!(cloned.len(), array.len());
        for i in 0..array.len() {
            assert_eq!(cloned[i], array[i]);
        }
    }

    #[test]
    fn test_reshape_shrink() {
        let shape: RTShape<2> = Shape::new([3, 3]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        let new_shape: RTShape<2> = Shape::new([2, 2]);
        let reshaped = array.reshape_with(new_shape, -1);

        assert_eq!(reshaped.len(), 4);
        // reshape 保持线性存储顺序，前 4 个元素是 [1, 2, 3, 4]
        // reshape preserves linear storage order, first 4 elements are [1, 2, 3, 4]
        // 使用向量索引访问：Use vector indexing:
        assert_eq!(reshaped[&[0, 0]], 1);
        assert_eq!(reshaped[&[0, 1]], 2);
        assert_eq!(reshaped[&[1, 0]], 3);
        assert_eq!(reshaped[&[1, 1]], 4);
    }

    #[test]
    fn test_reshape_empty_fill() {
        let shape: RTShape<2> = Shape::new([2, 2]);
        let array = MultiArrayBuilder::new_with(shape, 100);

        let new_shape: RTShape<2> = Shape::new([3, 3]);
        let reshaped = array.reshape_with(new_shape, -1);

        assert_eq!(reshaped.len(), 9);
        assert_eq!(reshaped[0], 100);
        assert_eq!(reshaped[1], 100);
        assert_eq!(reshaped[2], 100);
        assert_eq!(reshaped[3], 100);
        assert_eq!(reshaped[4], -1);
        assert_eq!(reshaped[8], -1);
    }

    #[test]
    fn test_3d_array() {
        let shape: Shape<3> = Shape::new([2, 3, 4]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = i as i32;
        }

        assert_eq!(array.len(), 24);
        assert_eq!(array[0], 0);
        assert_eq!(array[12], 12);
        assert_eq!(array[23], 23);

        let vector = [1, 2, 3];
        let expected_index = 1 * 12 + 2 * 4 + 3;
        assert_eq!(array[&vector], expected_index as i32);
    }

    #[test]
    fn test_multi_array_view_conversion() {
        use crate::{MultiArrayToView, dummy_expect};

        let shape: RTShape<2> = Shape::new([3, 4]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        let view = array.view(&dummy_expect![0..2, 1..3]).unwrap();
        assert_eq!(view.len(), 4);

        let values: std::vec::Vec<i32> = view.iter().copied().collect();
        assert_eq!(values, vec![2, 3, 6, 7]);
    }

    #[test]
    fn test_multi_array_reshape_empty() {
        let shape: RTShape<2> = Shape::new([0, 0]);
        let array: MultiArray<i32, _> = MultiArrayBuilder::new(shape);

        assert_eq!(array.len(), 0);
        assert!(array.is_empty());

        let new_shape: RTShape<2> = Shape::new([2, 2]);
        let reshaped = array.reshape_with(new_shape, -1);

        assert_eq!(reshaped.len(), 4);
        for i in 0..reshaped.len() {
            assert_eq!(reshaped[i], -1);
        }
    }

    #[test]
    fn test_multi_array_to_storage_order_same_order() {
        use crate::concept::RowMajor;

        let shape: Shape<2, RowMajor> = Shape::new([2, 3]);
        let array: MultiArray<i32, _> = MultiArrayBuilder::new_with(shape, 5);

        let converted = array.to_storage_order(StorageOrder::RowMajor);
        assert_eq!(converted.storage_order(), StorageOrder::RowMajor);
        assert_eq!(converted.len(), 6);
        for i in 0..converted.len() {
            assert_eq!(converted[i], 5);
        }
    }

    #[test]
    fn test_multi_array_index_out_of_bounds() {
        let shape: RTShape<2> = Shape::new([2, 3]);
        let array: MultiArray<i32, _> = MultiArrayBuilder::new_with(shape, 0);

        let vector = [5, 0];
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = array[&vector];
        }));
        assert!(result.is_err());
    }

    // NOTE: 此测试被注释掉，因为静态维度不匹配是编译期错误，无法在运行时测试
    // #[test]
    // fn test_multi_array_dimension_mismatch() {
    //     let shape: RTShape<2> = Shape::new([2, 3]);
    //     let array: MultiArray<i32, _> = MultiArrayBuilder::new_with(shape, 0);
    //
    //     let vector_3d = [1, 2, 3];
    //     let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    //         let _ = array[&vector_3d];
    //     }));
    //     assert!(result.is_err());
    // }

    #[test]
    fn test_multi_array_new_by_generator() {
        let shape: RTShape<2> = Shape::new([3, 3]);
        let array: MultiArray<i32, _> =
            MultiArrayBuilder::new_by(shape, |idx, vec| (idx + vec[0] + vec[1]) as i32);

        assert_eq!(array.len(), 9);
        assert_eq!(array[0], 0);
        // RowMajor: index_of([1, 1]) = 1*3 + 1 = 4, value = 4 + 1 + 1 = 6
        assert_eq!(array[&[1, 1]], 6);
        // RowMajor: index_of([2, 2]) = 2*3 + 2 = 8, value = 8 + 2 + 2 = 12
        assert_eq!(array[&[2, 2]], 12);
    }

    #[test]
    fn test_multi_array_reshape_same_size() {
        let shape: RTShape<2> = Shape::new([2, 3]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        let new_shape: RTShape<2> = Shape::new([2, 3]);
        let reshaped = array.reshape_with(new_shape, -1);

        assert_eq!(reshaped.len(), 6);
        for i in 0..reshaped.len() {
            assert_eq!(reshaped[i], (i + 1) as i32);
        }
    }

    #[test]
    fn test_multi_array_to_storage_order_data_integrity() {
        use crate::concept::RowMajor;

        let shape_rm: Shape<2, RowMajor> = Shape::new([2, 3]);
        let mut array_rm: MultiArray<i32, _> = MultiArrayBuilder::new_with(shape_rm, 0);

        for i in 0..array_rm.len() {
            array_rm[i] = (i + 1) as i32;
        }

        let array_cm = array_rm.to_storage_order(StorageOrder::ColumnMajor);
        assert_eq!(array_cm.storage_order(), StorageOrder::ColumnMajor);

        // 验证向量坐标对应的数据保持不变
        // Verify that data at vector coordinates remains unchanged
        // RowMajor [2,3]: index 0=[0,0], 1=[0,1], 2=[0,2], 3=[1,0], 4=[1,1], 5=[1,2]
        // ColumnMajor [2,3]: index 0=[0,0], 1=[1,0], 2=[0,1], 3=[1,1], 4=[0,2], 5=[1,2]
        assert_eq!(array_cm[&[0, 0]], 1); // [0,0] -> 1
        assert_eq!(array_cm[&[0, 1]], 2); // [0,1] -> 2
        assert_eq!(array_cm[&[0, 2]], 3); // [0,2] -> 3
        assert_eq!(array_cm[&[1, 0]], 4); // [1,0] -> 4
        assert_eq!(array_cm[&[1, 1]], 5); // [1,1] -> 5
        assert_eq!(array_cm[&[1, 2]], 6); // [1,2] -> 6
    }

    #[test]
    fn test_multi_array_empty_reshape_to_larger() {
        let shape: RTShape<2> = Shape::new([0, 0]);
        let array: MultiArray<i32, _> = MultiArrayBuilder::new(shape);

        let new_shape: RTShape<2> = Shape::new([2, 2]);
        let reshaped = array.reshape(new_shape);

        assert_eq!(reshaped.len(), 4);
        for i in 0..reshaped.len() {
            assert_eq!(reshaped[i], 0);
        }
    }

    #[test]
    fn test_multi_array_iter_mut() {
        let shape: RTShape<2> = Shape::new([2, 3]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for (i, val) in array.iter_mut().enumerate() {
            *val = (i + 1) as i32;
        }

        for i in 0..array.len() {
            assert_eq!(array[i], (i + 1) as i32);
        }
    }

    #[test]
    fn test_multi_array_deref_deref_mut() {
        let shape: RTShape<2> = Shape::new([2, 3]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        array.list[0] = 100;
        assert_eq!(array[0], 100);

        let list_ref: &Vec<i32> = array.deref();
        assert_eq!(list_ref.len(), 6);
    }

    #[test]
    fn test_multi_array_iter_with_order() {
        let shape: RTShape<2> = Shape::new([2, 3]);
        let array: MultiArray<i32, _> =
            MultiArrayBuilder::new_by(shape, |_idx, vector| (vector[0] * 10 + vector[1]) as i32);

        let row_major: std::vec::Vec<i32> = array
            .iter_with_order(AccessOrder::RowMajor)
            .copied()
            .collect();
        assert_eq!(row_major, vec![0, 1, 2, 10, 11, 12]);

        let column_major: std::vec::Vec<i32> = array
            .iter_with_order(AccessOrder::ColumnMajor)
            .copied()
            .collect();
        assert_eq!(column_major, vec![0, 10, 1, 11, 2, 12]);
    }

    #[test]
    fn test_multi_array_enumerate_with_order() {
        let shape: RTShape<2> = Shape::new([2, 2]);
        let array: MultiArray<i32, _> =
            MultiArrayBuilder::new_by(shape, |_idx, vector| (vector[0] * 10 + vector[1]) as i32);

        let entries: std::vec::Vec<(usize, [usize; 2], i32)> = array
            .enumerate_with_order(AccessOrder::ColumnMajor)
            .map(|(index, vector, value)| (index, vector, *value))
            .collect();

        assert_eq!(
            entries,
            vec![
                (0, [0, 0], 0),
                (1, [1, 0], 10),
                (2, [0, 1], 1),
                (3, [1, 1], 11),
            ]
        );
    }

    #[test]
    fn test_multi_array_flatten_with_order() {
        let shape: RTShape<2> = Shape::new([2, 3]);
        let array: MultiArray<i32, _> =
            MultiArrayBuilder::new_by(shape, |_idx, vector| (vector[0] * 10 + vector[1]) as i32);

        let flattened = array.flatten(AccessOrder::ColumnMajor);
        assert_eq!(flattened, vec![0, 10, 1, 11, 2, 12]);
    }

    #[test]
    fn test_multi_array_from_list_with_order() {
        let shape: RTShape<2> = Shape::new([2, 3]);

        let row_array = MultiArrayBuilder::from_list(shape.clone(), vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(row_array[&[0, 0]], 1);
        assert_eq!(row_array[&[0, 1]], 2);
        assert_eq!(row_array[&[1, 2]], 6);

        let column_array = MultiArrayBuilder::from_list_with_order(
            shape,
            vec![1, 2, 3, 4, 5, 6],
            AccessOrder::ColumnMajor,
        );
        assert_eq!(column_array[&[0, 0]], 1);
        assert_eq!(column_array[&[1, 0]], 2);
        assert_eq!(column_array[&[0, 1]], 3);
        assert_eq!(column_array[&[1, 2]], 6);
    }

    #[test]
    fn test_kotlin_parity_constructors() {
        let array1 = multi_array_1(3, 7);
        assert_eq!(array1.len(), 3);
        assert_eq!(array1[&[2]], 7);

        let array2 = multi_array_2(2, 3, 5);
        assert_eq!(array2.len(), 6);
        assert_eq!(array2[&[1, 2]], 5);

        let array3 = multi_array_3(2, 2, 2, 9);
        assert_eq!(array3.len(), 8);
        assert_eq!(array3[&[1, 1, 1]], 9);
    }

    #[test]
    fn test_kotlin_parity_type_aliases() {
        let array1: MultiArray1<i32> = MultiArray::new_with(Shape1::new([2]), 1);
        let array2: MultiArray2<i32> = MultiArray::new_with(Shape2::new([2, 2]), 2);
        let array3: MultiArray3<i32> = MultiArray::new_with(Shape3::new([2, 2, 2]), 3);
        let array4: MultiArray4<i32> = MultiArray::new_with(Shape4::new([1, 2, 2, 2]), 4);
        let dyn_array: DynMultiArray<i32> = MultiArray::new_with(DynShape::new(vec![2, 2]), 5);

        assert_eq!(array1[&[1]], 1);
        assert_eq!(array2[&[1, 1]], 2);
        assert_eq!(array3[&[1, 1, 1]], 3);
        assert_eq!(array4[&[0, 1, 1, 1]], 4);
        assert_eq!(dyn_array[&vec![1, 1]], 5);
    }

    #[test]
    fn test_multi_array_of_shape() {
        let array: MultiArray2<i32> = multi_array_of_shape(Shape2::new([2, 3]), 11);
        assert_eq!(array.len(), 6);
        assert_eq!(array[&[1, 2]], 11);
    }
}
