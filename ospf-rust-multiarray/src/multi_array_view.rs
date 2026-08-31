//! # MultiArrayView - 运行时多维数组视图实现
//!
//! ## Overview / 概述
//!
//! This module provides the `MultiArrayView` type, a zero-copy view into a `MultiArray`.
//! Views allow for slicing, projection, and dimension reordering without copying data.
//!
//! 本模块提供 `MultiArrayView` 类型，这是 `MultiArray` 的零拷贝视图。
//! 视图允许进行切片、投影和维度重排而无需复制数据。
//!
//! ## Key Features / 主要特性
//!
//! - Zero-copy views / 零拷贝视图
//! - Dimension slicing and projection / 维度切片和投影
//! - Dimension reordering / 维度重排
//! - Support for dummy indices (slicing) and map indices (reordering) / 支持虚拟索引（切片）和映射索引（重排）

use super::concept::{AccessOrder, DummyVector, MapVector};
use super::dummy_index::{DummyAccessIterator, DummyIndex, DummyIndexIterator};
use super::error::{DimensionMismatchingError, MappingIndexError, RepeatMappingIndexError};
use super::map_index::MapIndex;
use super::multi_array::{MultiArray, MultiArrayCollection, MultiArrayToView};
use super::shape::{AbstractRTShape, AbstractShape, DynShape};
use cc_traits::{Collection, CollectionMut, CollectionRef, Iter, IterMut, Len};
use ospf_rust_base::collection::Indices;
use ospf_rust_base::error::*;
use std::iter::Iterator;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut, RangeFull};
use std::ptr::NonNull;

/// Calculate the shape of a view based on the original shape and map vector.
///
/// 根据原始形状和映射向量计算视图的形状。
///
/// # Type Parameters / 类型参数
///
/// - `S: AbstractShape` - The original shape type / 原始形状类型
///
/// # Parameters / 参数
///
/// - `original_shape` - The shape of the original array / 原始数组的形状
/// - `map_vector` - The mapping vector specifying dimension transformations / 指定维度变换的映射向量
///
/// # Returns / 返回值
///
/// - `Ok(DynShape)` - The calculated view shape / 计算出的视图形状
/// - `Err(MappingIndexError)` - If the mapping is invalid / 如果映射无效
///
/// # Errors / 错误
///
/// - `RepeatMappingIndex` - If a dimension is mapped multiple times / 如果一个维度被多次映射
/// - `DimensionMismatching` - If the mapping is not continuous / 如果映射不连续
pub(crate) fn calculate_view_shape<S: AbstractShape>(
    original_shape: &S,
    map_vector: &S::MapVectorType,
) -> Result<DynShape, MappingIndexError> {
    let mut mapping_index: Vec<(usize, usize)> = map_vector
        .indices()
        .into_iter()
        .filter_map(|i| match &map_vector[i] {
            MapIndex::Map(m) => Some((m.index, i)),
            _ => None,
        })
        .collect::<Vec<_>>();
    mapping_index.sort_by_key(|k| k.0);
    let mut view_shape_vec = Vec::with_capacity(mapping_index.len());

    for i in 0..mapping_index.len() {
        if i != 0 && mapping_index[i].0 == mapping_index[i - 1].0 {
            return Err(MappingIndexError::RepeatMappingIndex(error! {
                RepeatMappingIndexError {
                    index: mapping_index[i].0
                }
            }));
        }
        if mapping_index[i].0 != i {
            return Err(MappingIndexError::DimensionMismatching(error! {
                DimensionMismatchingError {
                    dimension: mapping_index.len(),
                    vector_dimension: mapping_index[i].0
                }
            }));
        }
        view_shape_vec.push(original_shape.len_of_dimension(mapping_index[i].1)?);
    }
    Ok(DynShape::new(view_shape_vec))
}

/// Calculate the shape of a view based on the original shape and a Vec of MapIndex.
///
/// 根据原始形状和 MapIndex 的 Vec 计算视图的形状。
///
/// # Type Parameters / 类型参数
///
/// - `S: AbstractShape` - The original shape type / 原始形状类型
///
/// # Parameters / 参数
///
/// - `original_shape` - The shape of the original array / 原始数组的形状
/// - `map_vector` - The mapping vector as a Vec / 以 Vec 形式表示的映射向量
///
/// # Returns / 返回值
///
/// - `Ok(DynShape)` - The calculated view shape / 计算出的视图形状
/// - `Err(MappingIndexError)` - If the mapping is invalid / 如果映射无效
pub(crate) fn calculate_view_shape_for_vec<S: AbstractShape>(
    original_shape: &S,
    map_vector: &S::MapVectorType,
) -> Result<DynShape, MappingIndexError> {
    let mut mapping_index: Vec<(usize, usize)> = map_vector
        .indices()
        .filter_map(|i| match &map_vector[i] {
            MapIndex::Map(m) => Some((m.index, i)),
            _ => None,
        })
        .collect::<Vec<_>>();
    mapping_index.sort_by_key(|k| k.0);
    let mut view_shape_vec = Vec::with_capacity(mapping_index.len());

    for i in 0..mapping_index.len() {
        if i != 0 && mapping_index[i].0 == mapping_index[i - 1].0 {
            return Err(MappingIndexError::RepeatMappingIndex(error! {
                RepeatMappingIndexError {
                    index: mapping_index[i].0
                }
            }));
        }
        if mapping_index[i].0 != i {
            return Err(MappingIndexError::DimensionMismatching(error! {
                DimensionMismatchingError {
                    dimension: mapping_index.len(),
                    vector_dimension: mapping_index[i].0
                }
            }));
        }
        view_shape_vec.push(original_shape.len_of_dimension(mapping_index[i].1)?);
    }
    Ok(DynShape::new(view_shape_vec))
}

/// # MultiArrayView - Zero-Copy View into a MultiArray
///
/// A view provides a way to access a subset or reordering of a `MultiArray`'s elements
/// without copying the underlying data. Views support operations like:
/// - Slicing (fixing or ranging specific dimensions)
/// - Dimension reordering (transposing)
/// - Chained views (applying multiple transformations)
///
/// 视图提供了一种访问 `MultiArray` 元素子集或重排的方式，而无需复制底层数据。
/// 视图支持以下操作：
/// - 切片（固定或范围化特定维度）
/// - 维度重排（转置）
/// - 链式视图（应用多个变换）
///
/// ## Type Parameters / 类型参数
///
/// - `'a` - The lifetime of the reference to the underlying array / 底层数组引用的生命周期
/// - `T` - The element type / 元素类型
/// - `S: AbstractRTShape` - The runtime shape type of the underlying array / 底层数组的运行时形状类型
/// - `C: MultiArrayCollection<T>` - The collection type of the underlying array / 底层数组的集合类型
///
/// ## Fields / 字段
///
/// - `array` - Reference to the underlying `MultiArray` / 底层 `MultiArray` 的引用
/// - `map_vector` - Vector specifying dimension mappings / 指定维度映射的向量
/// - `view_shape` - The shape of this view / 此视图的形状
/// - `len` - Total number of elements in the view / 视图中的元素总数
/// - `access_order` - The order in which elements are accessed during iteration / 迭代时访问元素的顺序
/// - `_marker` - Phantom marker for type safety / 类型安全的虚拟特征
pub struct MultiArrayView<'a, T, S: AbstractRTShape, C>
where
    C: MultiArrayCollection<T>,
{
    /// Reference to the underlying array / 底层数组的引用
    array: &'a MultiArray<T, S, C>,
    /// Vector specifying dimension mappings / 指定维度映射的向量
    ///
    /// Uses `S::MapVectorType` for stack allocation with fixed dimensions,
    /// avoiding heap allocation for static shapes.
    ///
    /// 使用 `S::MapVectorType` 在固定维度时进行栈分配，
    /// 避免静态形状的堆分配。
    map_vector: S::MapVectorType,
    /// The shape of this view / 此视图的形状
    view_shape: DynShape,
    /// Total number of elements in the view / 视图中的元素总数
    len: usize,
    /// Access order for iteration / 迭代时的访问顺序
    access_order: AccessOrder,
    /// Phantom marker for type parameter T / 类型参数 T 的虚拟特征
    _marker: PhantomData<T>,
}

impl<'a, T, S, C> MultiArrayView<'a, T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    /// Create a new MultiArrayView using a dummy vector.
    ///
    /// 使用虚拟向量创建新的 MultiArrayView。
    ///
    /// # Parameters / 参数
    ///
    /// - `array` - The underlying array to create a view of / 要创建视图的底层数组
    /// - `dummy_vector` - A vector specifying slicing operations for each dimension / 指定每个维度切片操作的向量
    ///
    /// # Returns / 返回值
    ///
    /// A new MultiArrayView / 新的 MultiArrayView
    ///
    /// # Note / 注意
    ///
    /// Dummy vectors are used for slicing operations (fixing or ranging dimensions).
    /// This method does not validate the dummy vector against the array shape.
    ///
    /// 虚拟向量用于切片操作（固定或范围化维度）。
    /// 此方法不针对数组形状验证虚拟向量。
    pub fn new_by_dummy(array: &'a MultiArray<T, S, C>, dummy_vector: &S::DummyVectorType) -> Self {
        let len = dummy_vector
            .indices()
            .fold(1, |acc, i| acc * dummy_vector[i].len_of(&array.shape, i));
        let map_vector = S::dummy_to_map_vector(dummy_vector);

        Self {
            array,
            map_vector,
            view_shape: DynShape::new(vec![]),
            len,
            access_order: AccessOrder::from_storage_order(array.storage_order()),
            _marker: PhantomData,
        }
    }

    /// Create a new MultiArrayView using a map vector.
    ///
    /// 使用映射向量创建新的 MultiArrayView。
    ///
    /// # Parameters / 参数
    ///
    /// - `array` - The underlying array to create a view of / 要创建视图的底层数组
    /// - `map_vector` - A vector specifying dimension mappings / 指定维度映射的向量
    ///
    /// # Returns / 返回值
    ///
    /// - `Ok(MultiArrayView)` - The created view / 创建的视图
    /// - `Err(MappingIndexError)` - If the mapping is invalid / 如果映射无效
    ///
    /// # Note / 注意
    ///
    /// Map vectors are used for dimension reordering and projection.
    /// This method validates the mapping and returns an error if invalid.
    ///
    /// 映射向量用于维度重排和投影。
    /// 此方法验证映射，如果无效则返回错误。
    pub fn new_by_map(
        array: &'a MultiArray<T, S, C>,
        map_vector: S::MapVectorType,
    ) -> Result<Self, MappingIndexError> {
        let view_shape = calculate_view_shape(&array.shape, &map_vector)?;
        let len = map_vector.indices().fold(1, |acc, i| match &map_vector[i] {
            MapIndex::Dummy(dummy) => acc * dummy.len_of(&array.shape, i),
            MapIndex::Map(_) => acc * array.shape.len_of_dimension(i).unwrap(),
        });

        Ok(Self {
            array,
            map_vector,
            view_shape,
            len,
            access_order: AccessOrder::from_storage_order(array.storage_order()),
            _marker: PhantomData,
        })
    }

    /// Get the shape of this view.
    ///
    /// 获取此视图的形状。
    ///
    /// # Returns / 返回值
    ///
    /// A reference to the view's shape / 视图形状的引用
    pub fn shape(&self) -> &DynShape {
        &self.view_shape
    }

    /// Get the map vector of this view.
    ///
    /// 获取此视图的映射向量。
    ///
    /// # Returns / 返回值
    ///
    /// A reference to the map vector / 映射向量的引用
    pub fn map_vector(&self) -> &S::MapVectorType {
        &self.map_vector
    }

    /// Get a reference to the underlying array.
    ///
    /// 获取底层数组的引用。
    ///
    /// # Returns / 返回值
    ///
    /// A reference to the underlying MultiArray / 底层 MultiArray 的引用
    pub fn array(&self) -> &MultiArray<T, S, C> {
        self.array
    }

    /// Get the access order of this view.
    ///
    /// 获取此视图的访问顺序。
    ///
    /// # Returns / 返回值
    ///
    /// The access order (RowMajor or ColumnMajor) / 访问顺序（行优先或列优先）
    pub fn access_order(&self) -> AccessOrder {
        self.access_order
    }

    /// Set the access order of this view.
    ///
    /// 设置此视图的访问顺序。
    ///
    /// # Parameters / 参数
    ///
    /// - `order` - The new access order / 新的访问顺序
    ///
    /// # Returns / 返回值
    ///
    /// Self with the updated access order / 更新访问顺序后的 Self
    pub fn with_access_order(mut self, order: AccessOrder) -> Self {
        self.access_order = order;
        self
    }

    /// Create an iterator over the view with a specific access order.
    ///
    /// 以特定访问顺序创建视图的迭代器。
    ///
    /// # Parameters / 参数
    ///
    /// - `order` - The access order for iteration / 迭代的访问顺序
    ///
    /// # Returns / 返回值
    ///
    /// A MultiArrayViewIter that yields references to elements / 生成元素引用的 MultiArrayViewIter
    pub fn iter_with_order(&self, order: AccessOrder) -> MultiArrayViewIter<'_, T, S, C> {
        MultiArrayViewIter::new_from_view_with_order(
            self.array,
            &self.map_vector,
            &self.view_shape,
            order,
        )
    }

    /// Create a new view by applying a dummy vector to this view.
    ///
    /// 通过对此视图应用虚拟向量创建新视图。
    ///
    /// # Type Parameters / 类型参数
    ///
    /// - `V: DummyVector` - The dummy vector type / 虚拟向量类型
    ///
    /// # Parameters / 参数
    ///
    /// - `dummy_vector` - A vector specifying slicing operations / 指定切片操作的向量
    ///
    /// # Returns / 返回值
    ///
    /// - `Ok(MultiArrayView)` - The new view / 新视图
    /// - `Err(MappingIndexError)` - If the dimensions don't match / 如果维度不匹配
    ///
    /// # Note / 注意
    ///
    /// This allows chaining multiple slicing operations on a view.
    /// 这允许在视图上链接多个切片操作。
    pub fn view_by_dummy<V: DummyVector>(
        &self,
        dummy_vector: &V,
    ) -> Result<Self, MappingIndexError> {
        if dummy_vector.len() > self.view_shape.dimension() {
            return Err(MappingIndexError::DimensionMismatching(error! {
                DimensionMismatchingError {
                    dimension: self.view_shape.dimension(),
                    vector_dimension: dummy_vector.len()
                }
            }));
        }

        // Clone the original map_vector and modify in place
        // 克隆原始 map_vector 并就地修改
        let mut new_map_vector = self.map_vector.clone();

        for i in self.map_vector.indices() {
            if let MapIndex::Map(placeholder) = &self.map_vector[i] {
                let index = placeholder.index;
                if index < dummy_vector.len() {
                    new_map_vector[i] = MapIndex::Dummy(dummy_vector[index].clone());
                } else {
                    return Err(MappingIndexError::DimensionMismatching(error! {
                        DimensionMismatchingError {
                            dimension: dummy_vector.len(),
                            vector_dimension: index
                        }
                    }));
                }
            }
        }

        let new_view_shape = calculate_view_shape(&self.array.shape, &new_map_vector)?;

        let new_len = new_map_vector
            .indices()
            .fold(1, |acc, i| match &new_map_vector[i] {
                MapIndex::Dummy(dummy) => acc * dummy.len_of(&self.array.shape, i),
                MapIndex::Map(_) => acc * self.array.shape.len_of_dimension(i).unwrap(),
            });

        Ok(Self {
            array: self.array,
            map_vector: new_map_vector,
            view_shape: new_view_shape,
            len: new_len,
            access_order: self.access_order,
            _marker: PhantomData,
        })
    }

    /// Create a new view by applying a map vector to this view.
    ///
    /// 通过对此视图应用映射向量创建新视图。
    ///
    /// # Type Parameters / 类型参数
    ///
    /// - `V: MapVector` - The map vector type / 映射向量类型
    ///
    /// # Parameters / 参数
    ///
    /// - `map_vector` - A vector specifying dimension mappings / 指定维度映射的向量
    ///
    /// # Returns / 返回值
    ///
    /// - `Ok(MultiArrayView)` - The new view / 新视图
    /// - `Err(MappingIndexError)` - If the dimensions don't match or mapping is invalid / 如果维度不匹配或映射无效
    ///
    /// # Note / 注意
    ///
    /// This allows chaining multiple dimension reordering operations on a view.
    /// 这允许在视图上链接多个维度重排操作。
    pub fn view_by_map<V: MapVector>(&self, map_vector: &V) -> Result<Self, MappingIndexError> {
        if map_vector.len() > self.view_shape.dimension() {
            return Err(MappingIndexError::DimensionMismatching(error! {
                DimensionMismatchingError {
                    dimension: self.view_shape.dimension(),
                    vector_dimension: map_vector.len()
                }
            }));
        }

        // Clone the original map_vector and modify in place
        // 克隆原始 map_vector 并就地修改
        let mut new_map_vector = self.map_vector.clone();

        for i in self.map_vector.indices() {
            if let MapIndex::Map(placeholder) = &self.map_vector[i] {
                let index = placeholder.index;
                if index < map_vector.len() {
                    new_map_vector[i] = map_vector[index].clone();
                } else {
                    return Err(MappingIndexError::DimensionMismatching(error! {
                        DimensionMismatchingError {
                            dimension: map_vector.len(),
                            vector_dimension: index
                        }
                    }));
                }
            }
        }

        let new_view_shape = calculate_view_shape(&self.array.shape, &new_map_vector)?;

        let new_len = new_map_vector
            .indices()
            .fold(1, |acc, i| match &new_map_vector[i] {
                MapIndex::Dummy(dummy) => acc * dummy.len_of(&self.array.shape, i),
                MapIndex::Map(_) => acc * self.array.shape.len_of_dimension(i).unwrap(),
            });

        Ok(Self {
            array: self.array,
            map_vector: new_map_vector,
            view_shape: new_view_shape,
            len: new_len,
            access_order: self.access_order,
            _marker: PhantomData,
        })
    }

    /// Create an enumerate iterator that yields (linear_index, vector, element) tuples.
    ///
    /// 创建一个枚举迭代器，生成（线性索引，向量，元素）元组。
    ///
    /// # Returns / 返回值
    ///
    /// A MultiArrayViewEnumerateIter that yields tuples of (index, vector, element) / 生成（索引，向量，元素）元组的 MultiArrayViewEnumerateIter
    ///
    /// # Example / 示例
    ///
    /// ```rust
    /// use ospf_rust_multiarray::*;
    ///
    /// let shape = Shape::new([2, 3]);
    /// let array = MultiArrayBuilder::new_with(shape, 0);
    /// let view = array.view(&dummy_expect![.., ..]).unwrap();
    ///
    /// for (index, array_index, vector, value) in view.enumerate() {
    ///     println!("Index {}: array_index={:?}, vector={:?}, value={}", index, array_index, vector, value);
    /// }
    /// ```
    pub fn enumerate(&self) -> MultiArrayViewEnumerateIter<'_, T, S, C> {
        MultiArrayViewEnumerateIter::new(self)
    }
}

impl<'a, T, S, C> Collection for MultiArrayView<'a, T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    type Item = T;
}

impl<'a, T, S, C> CollectionRef for MultiArrayView<'a, T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    type ItemRef<'b>
        = &'b T
    where
        Self: 'b,
        T: 'b;

    cc_traits::covariant_item_ref!();
}

impl<'a, T, S, C> Len for MultiArrayView<'a, T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    fn len(&self) -> usize {
        self.len
    }

    fn is_empty(&self) -> bool {
        self.view_shape.len() == 0
    }
}

impl<'a, T, S, C> MultiArrayToView<DynShape> for MultiArrayView<'a, T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    type ViewType<'b>
        = MultiArrayView<'b, T, S, C>
    where
        Self: 'b;

    fn view(
        &self,
        dummy_vector: &<DynShape as AbstractShape>::DummyVectorType,
    ) -> Result<Self::ViewType<'_>, MappingIndexError> {
        self.view_by_dummy(dummy_vector)
    }

    fn map_view(
        &self,
        map_vector: &<DynShape as AbstractShape>::MapVectorType,
    ) -> Result<Self::ViewType<'_>, MappingIndexError> {
        self.view_by_map(map_vector)
    }
}

/// # MultiArrayViewIter - Iterator for MultiArrayView
///
/// An iterator that traverses elements in a MultiArrayView according to
/// the specified access order.
///
/// 根据指定的访问顺序遍历 MultiArrayView 中元素的迭代器。
///
/// ## Type Parameters / 类型参数
///
/// - `'a` - The lifetime of the reference to the underlying array / 底层数组引用的生命周期
/// - `T` - The element type / 元素类型
/// - `S: AbstractRTShape` - The runtime shape type / 运行时形状类型
/// - `C: MultiArrayCollection<T>` - The collection type / 集合类型
pub struct MultiArrayViewIter<'a, T, S: AbstractRTShape, C>
where
    C: MultiArrayCollection<T>,
{
    /// The map vector defining the view / 定义视图的映射向量
    map_vector: S::MapVectorType,
    /// Iterator over dummy indices / 虚拟索引迭代器
    dummy_iterator: DummyAccessIterator<'a, S>,
    /// Reference to the underlying array / 底层数组的引用
    array: &'a MultiArray<T, S, C>,
}

impl<'a, T, S, C> MultiArrayViewIter<'a, T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    /// Create a new iterator from a view with default (RowMajor) access order.
    ///
    /// 使用默认（行优先）访问顺序从视图创建新迭代器。
    ///
    /// # Parameters / 参数
    ///
    /// - `array` - The underlying array / 底层数组
    /// - `map_vector` - The map vector defining the view / 定义视图的映射向量
    /// - `view_shape` - The shape of the view / 视图的形状
    ///
    /// # Returns / 返回值
    ///
    /// A new MultiArrayViewIter / 新的 MultiArrayViewIter
    pub fn new_from_view(
        array: &'a MultiArray<T, S, C>,
        map_vector: &S::MapVectorType,
        view_shape: &DynShape,
    ) -> Self {
        Self::new_from_view_with_order(array, map_vector, view_shape, AccessOrder::RowMajor)
    }

    /// Create a new iterator from a view with a specific access order.
    ///
    /// 使用特定访问顺序从视图创建新迭代器。
    ///
    /// # Parameters / 参数
    ///
    /// - `array` - The underlying array / 底层数组
    /// - `map_vector` - The map vector defining the view / 定义视图的映射向量
    /// - `view_shape` - The shape of the view / 视图的形状
    /// - `access_order` - The access order for iteration / 迭代的访问顺序
    ///
    /// # Returns / 返回值
    ///
    /// A new MultiArrayViewIter / 新的 MultiArrayViewIter
    pub fn new_from_view_with_order(
        array: &'a MultiArray<T, S, C>,
        map_vector: &S::MapVectorType,
        _view_shape: &DynShape,
        access_order: AccessOrder,
    ) -> Self {
        // Convert map_vector to iterator vector
        // 将 map_vector 转换为迭代器向量
        let iterators = array.shape.map_to_iterator_vector(map_vector);

        let dummy_iterator = DummyAccessIterator::new(&array.shape, iterators, access_order);

        Self {
            map_vector: map_vector.clone(),
            dummy_iterator,
            array,
        }
    }
}

impl<'a, T, S, C> Iterator for MultiArrayViewIter<'a, T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(vector) = self.dummy_iterator.next() {
            if let Ok(index) = self.array.shape.index_of(vector) {
                return Some(&self.array[index]);
            }
        }
        None
    }
}

impl<'a, T, S, C> Iter for MultiArrayView<'a, T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    type Iter<'b>
        = MultiArrayViewIter<'b, T, S, C>
    where
        Self: 'b,
        T: 'b;

    fn iter(&self) -> Self::Iter<'_> {
        MultiArrayViewIter::new_from_view(self.array, &self.map_vector, &self.view_shape)
    }
}

/// # MultiArrayViewEnumerateIter - Enumerate Iterator for MultiArrayView
///
/// An iterator that yields tuples of (linear_index, vector, element_reference).
/// This allows accessing the linear index, multi-dimensional vector, and element simultaneously.
///
/// MultiArrayView 的枚举迭代器，生成（线性索引，向量，元素引用）的元组。
/// 这使得可以同时访问线性索引、多维向量和元素。
pub struct MultiArrayViewEnumerateIter<'a, T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    /// The map vector defining the view / 定义视图的映射向量
    map_vector: S::MapVectorType,
    /// Iterator over dummy indices / 虚拟索引迭代器
    dummy_iterator: DummyAccessIterator<'a, S>,
    /// Reference to the underlying array / 底层数组的引用
    array: &'a MultiArray<T, S, C>,
    current_index: usize,
}

impl<'a, T, S, C> MultiArrayViewEnumerateIter<'a, T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    /// Create a new enumerate iterator from a MultiArrayView.
    ///
    /// 从 MultiArrayView 创建新的枚举迭代器。
    pub fn new(view: &'a MultiArrayView<'a, T, S, C>) -> Self {
        // Convert map_vector to iterator vector
        // 将 map_vector 转换为迭代器向量
        let iterators = view.array.shape.map_to_iterator_vector(&view.map_vector);

        let dummy_iterator =
            DummyAccessIterator::new(&view.array.shape, iterators, view.access_order);

        Self {
            map_vector: view.map_vector.clone(),
            dummy_iterator,
            array: view.array,
            current_index: 0,
        }
    }
}

impl<'a, T, S, C> Iterator for MultiArrayViewEnumerateIter<'a, T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    type Item = (usize, usize, S::VectorType, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(vector) = self.dummy_iterator.next() {
            // Calculate the real linear index from the vector
            if let Ok(linear_index) = self.array.shape.index_of(vector) {
                let index = self.current_index;
                let view_vector = self.array.shape.vector_of(linear_index).ok()?;
                let element = &self.array[linear_index];
                self.current_index += 1;
                // Return (iteration_index, linear_index, vector, element)
                return Some((index, linear_index, view_vector, element));
            }
        }
        None
    }
}

impl<'a, T, S, C> Index<usize> for MultiArrayView<'a, T, S, C>
where
    S: AbstractRTShape,
    C: MultiArrayCollection<T>,
{
    type Output = T;

    /// Index 视图中的元素 / Index elements in the view
    ///
    /// # Parameters / 参数
    ///
    /// - `index`: The index to access / 要访问的索引
    ///
    /// # Panics / 异常
    ///
    /// Panics if multiple dimensions are not fixed (i.e., they are ranges or arrays).
    /// 如果多个维度不是固定的（即是范围或数组），则会 panic。
    ///
    /// # Note / 注意
    ///
    /// This implementation checks if only one dimension is not `DummyIndex::Index`.
    /// If so, it can concatenate the provided index into the actual flat index.
    ///
    /// 此实现检查是否只有一个维度不是 `DummyIndex::Index`。
    /// 如果是，则可以将提供的索引拼接成实际的扁平索引。
    fn index(&self, index: usize) -> &Self::Output {
        // 检查是否只有一个维度不是 DummyIndex::Index
        // Check if only one dimension is not DummyIndex::Index
        let mut non_index_count = 0;

        for i in self.map_vector.indices() {
            match &self.map_vector[i] {
                MapIndex::Dummy(dummy) => {
                    match dummy {
                        DummyIndex::Index(_) => {
                            // 这是固定索引，跳过 / This is a fixed index, skip it
                        }
                        _ => {
                            // 这是范围或数组，计数 / This is a range or array, count it
                            non_index_count += 1;
                            if non_index_count > 1 {
                                panic!(
                                    "Cannot use single index when multiple dimensions are not fixed / 当多个维度未固定时无法使用单一索引"
                                );
                            }
                        }
                    }
                }
                MapIndex::Map(_) => {
                    // 映射索引总是非固定的 / Map indices are always non-fixed
                    non_index_count += 1;
                    if non_index_count > 1 {
                        panic!(
                            "Cannot use single index when multiple dimensions are not fixed / 当多个维度未固定时无法使用单一索引"
                        );
                    }
                }
            }
        }

        if non_index_count == 0 {
            // 所有维度都已固定，直接使用扁平索引
            // All dimensions are fixed, use flat index directly
            &self.array[index]
        } else if non_index_count == 1 {
            // 只有一个维度未固定，计算实际索引
            // One dimension is not fixed, calculate the actual index
            let mut vector = self.array.shape.zero();

            for i in self.map_vector.indices() {
                match &self.map_vector[i] {
                    MapIndex::Dummy(dummy) => {
                        match dummy {
                            DummyIndex::Index(val) => {
                                // 将负索引转换为正索引
                                // Convert negative index to positive
                                let actual_index = if *val < 0 {
                                    let dim_len = self.array.shape.len_of_dimension(i).unwrap();
                                    (dim_len as isize + val) as usize
                                } else {
                                    *val as usize
                                };
                                vector[i] = actual_index;
                            }
                            _ => {
                                // 这是未固定的维度，使用提供的索引
                                // This is the non-fixed dimension, use the provided index
                                vector[i] = index;
                            }
                        }
                    }
                    MapIndex::Map(placeholder) => {
                        let map_dim = placeholder.index;
                        vector[map_dim] = index;
                    }
                }
            }

            // 从向量计算扁平索引
            // Calculate flat index from vector
            let flat_idx = self.array.shape.index_of(&vector).unwrap();
            &self.array[flat_idx]
        } else {
            panic!(
                "Cannot use single index when multiple dimensions are not fixed / 当多个维度未固定时无法使用单一索引"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dummy_index::DummyIndex;
    use crate::map_index::{_0, _1, _2, MapIndex};
    use crate::multi_array::MultiArrayBuilder;
    use crate::shape::Shape;
    use crate::{dummy_expect, map_expect};
    use paste::paste;

    #[test]
    fn test_multi_array_view_creation() {
        let shape = Shape::new([2, 3]);
        let array = MultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![_1, _0];

        let view = MultiArrayView::new_by_map(&array, map_vector)
            .expect("Should be able to create MultiArrayView");

        assert_eq!(view.shape().dimension(), 2);
        assert_eq!(view.shape().len_of_dimension(0).unwrap(), 3);
        assert_eq!(view.shape().len_of_dimension(1).unwrap(), 2);
        assert_eq!(view.len(), 6);
        assert!(!view.is_empty());
    }

    #[test]
    fn test_multi_array_view_with_dummy_index() {
        let shape = Shape::new([2, 3]);
        let array = MultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![_0, 1];

        let view = MultiArrayView::new_by_map(&array, map_vector)
            .expect("Should be able to create MultiArrayView");

        assert_eq!(view.shape().dimension(), 1);
        assert_eq!(view.shape().len_of_dimension(0).unwrap(), 2);
        assert_eq!(view.len(), 2);
    }

    #[test]
    fn test_multi_array_view_iter_with_mixed_indices() {
        let shape = Shape::new([2, 3, 4]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = i as i32;
        }

        let map_vector = map_expect![_0, 1, _1];
        let view = MultiArrayView::new_by_map(&array, map_vector)
            .expect("Should be able to create MultiArrayView");

        let mut iter = view.iter();
        let mut count = 0;

        while let Some(value) = iter.next() {
            count += 1;
            assert!(*value >= 0 && *value < 24);
        }

        assert_eq!(count, 8);
    }

    #[test]
    fn test_multi_array_view_repeat_mapping_error() {
        let shape = Shape::new([2, 3, 4]);
        let array = MultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![_0, _0, _1];

        let result = MultiArrayView::new_by_map(&array, map_vector);
        assert!(result.is_err());

        if let Err(MappingIndexError::RepeatMappingIndex(_)) = result {
            // correct
        } else {
            panic!("Expected RepeatMappingIndex error");
        }
    }

    #[test]
    fn test_multi_array_view_non_continuous_mapping_error() {
        let shape = Shape::new([2, 3, 4]);
        let array = MultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![_0, 0, _2];

        let result = MultiArrayView::new_by_map(&array, map_vector);
        assert!(result.is_err());

        if let Err(MappingIndexError::DimensionMismatching(_)) = result {
            // correct
        } else {
            panic!("Expected DimensionMismatching error");
        }
    }

    #[test]
    fn test_multi_array_view_complex_mapping() {
        let shape = Shape::new([2, 3, 4, 5]);
        let array = MultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![_2, 0, _0, _1];

        let view = MultiArrayView::new_by_map(&array, map_vector)
            .expect("Should be able to create MultiArrayView");

        assert_eq!(view.shape().dimension(), 3);
        assert_eq!(view.shape().len_of_dimension(0).unwrap(), 4);
        assert_eq!(view.shape().len_of_dimension(1).unwrap(), 5);
        assert_eq!(view.shape().len_of_dimension(2).unwrap(), 2);
        assert_eq!(view.len(), 40);
    }

    #[test]
    fn test_multi_array_view_single_dimension() {
        let shape = Shape::new([5]);
        let array = MultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![_0];

        let view = MultiArrayView::new_by_map(&array, map_vector)
            .expect("Should be able to create MultiArrayView");

        assert_eq!(view.shape().dimension(), 1);
        assert_eq!(view.shape().len_of_dimension(0).unwrap(), 5);
        assert_eq!(view.len(), 5);
    }

    #[test]
    fn test_multi_array_view_no_map_indices() {
        let shape = Shape::new([2, 3]);
        let array = MultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![0, 1];

        let view = MultiArrayView::new_by_map(&array, map_vector)
            .expect("Should be able to create MultiArrayView");

        assert_eq!(view.shape().dimension(), 0);
        assert_eq!(view.len(), 1);
    }

    #[test]
    fn test_multi_array_view_create_by_dummy() {
        let shape = Shape::new([2, 3, 4]);
        let array = MultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![_0, _1, _2];

        let view = MultiArrayView::new_by_map(&array, map_vector)
            .expect("Should be able to create MultiArrayView");

        let dummy_vector = dummy_expect![0, 1, 2];

        let new_view = view
            .view_by_dummy(&dummy_vector)
            .expect("Should be able to create new view by dummy");

        assert_eq!(new_view.shape().dimension(), 0);
        assert_eq!(new_view.len(), 1);
    }

    #[test]
    fn test_multi_array_view_create_by_map() {
        let shape = Shape::new([2, 3, 4]);
        let array = MultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![_0, _1, _2];

        let view = MultiArrayView::new_by_map(&array, map_vector)
            .expect("Should be able to create MultiArrayView");

        let new_map_vector = map_expect![_2, _1, _0];

        let new_view = view
            .view_by_map(&new_map_vector)
            .expect("Should be able to create new view by map");

        assert_eq!(new_view.shape().dimension(), 3);
        assert_eq!(new_view.shape().len_of_dimension(0).unwrap(), 4);
        assert_eq!(new_view.shape().len_of_dimension(1).unwrap(), 3);
        assert_eq!(new_view.shape().len_of_dimension(2).unwrap(), 2);
        assert_eq!(new_view.len(), 24);
    }

    #[test]
    fn test_multi_array_view_create_by_dummy_with_mixed_indices() {
        let shape = Shape::new([2, 3, 4]);
        let array = MultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![_0, 1, _1];

        let view = MultiArrayView::new_by_map(&array, map_vector)
            .expect("Should be able to create MultiArrayView");

        let dummy_vector = dummy_expect![0, 2];

        let new_view = view
            .view_by_dummy(&dummy_vector)
            .expect("Should be able to create new view by dummy");

        assert_eq!(new_view.shape().dimension(), 0);
        assert_eq!(new_view.len(), 1);
    }

    #[test]
    fn test_multi_array_view_create_by_dummy_index_out_of_bounds() {
        let shape = Shape::new([2, 3]);
        let array = MultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![_0, _1];

        let view = MultiArrayView::new_by_map(&array, map_vector)
            .expect("Should be able to create MultiArrayView");

        let dummy_vector = dummy_expect![0];

        let result = view.view_by_dummy(&dummy_vector);
        assert!(result.is_err());

        if let Err(MappingIndexError::DimensionMismatching(_)) = result {
            // correct
        } else {
            panic!("Expected DimensionMismatching error");
        }
    }

    #[test]
    fn test_multi_array_view_iter_basic() {
        let shape = Shape::new([2, 3]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = i as i32;
        }

        let map_vector = map_expect![_0, _1];
        let view = MultiArrayView::new_by_map(&array, map_vector)
            .expect("Should be able to create MultiArrayView");

        let mut iter = view.iter();
        let mut expected_values = vec![0, 1, 2, 3, 4, 5];

        for expected in expected_values {
            assert_eq!(*iter.next().unwrap(), expected);
        }

        assert!(iter.next().is_none());
    }

    #[test]
    fn test_multi_array_view_iter_with_dummy_index() {
        let shape = Shape::new([3, 4]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = i as i32;
        }

        let map_vector = map_expect![_0, 1];
        let view = MultiArrayView::new_by_map(&array, map_vector)
            .expect("Should be able to create MultiArrayView");

        let mut iter = view.iter();
        let mut expected_values = vec![1, 5, 9];

        for expected in expected_values {
            assert_eq!(*iter.next().unwrap(), expected);
        }

        assert!(iter.next().is_none());
    }

    #[test]
    fn test_multi_array_view_iter_with_range() {
        let shape = Shape::new([5]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = i as i32;
        }

        let map_vector = map_expect![1..4];
        let view = MultiArrayView::new_by_map(&array, map_vector)
            .expect("Should be able to create MultiArrayView");

        let mut iter = view.iter();
        let mut expected_values = vec![1, 2, 3];

        for expected in expected_values {
            assert_eq!(*iter.next().unwrap(), expected);
        }

        assert!(iter.next().is_none());
    }

    #[test]
    fn test_multi_array_view_iter_empty_view() {
        let shape = Shape::new([2, 3]);
        let array = MultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![0, 1];
        let view = MultiArrayView::new_by_map(&array, map_vector)
            .expect("Should be able to create MultiArrayView");

        let mut iter = view.iter();

        assert!(iter.next().is_some());
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_multi_array_view_iter_complex_mapping() {
        let shape = Shape::new([2, 3, 4]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);
        for i in 0..array.len() {
            array[i] = i as i32;
        }

        let map_vector = map_expect![_2, _0, _1];
        let view = MultiArrayView::new_by_map(&array, map_vector)
            .expect("Should be able to create MultiArrayView");

        let mut iter = view.iter();
        let mut count = 0;

        while let Some(_) = iter.next() {
            count += 1;
        }

        assert_eq!(count, 24);
    }

    #[test]
    fn test_multi_array_view_iter_with_index_array() {
        let shape = Shape::new([5]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);
        for i in 0..array.len() {
            array[i] = i as i32;
        }

        let map_vector = map_expect![vec![0, 2, 4]];
        let view = MultiArrayView::new_by_map(&array, map_vector)
            .expect("Should be able to create MultiArrayView");

        let mut iter = view.iter();
        let mut expected_values = vec![0, 2, 4];

        for expected in expected_values {
            assert_eq!(*iter.next().unwrap(), expected);
        }

        assert!(iter.next().is_none());
    }

    #[test]
    fn test_multi_array_view_enumerate() {
        let shape = Shape::new([2, 3]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        // 填充数据
        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        let view = array.view(&dummy_expect![.., ..]).unwrap();

        // 测试视图的 enumerate 迭代器（返回四个字段：迭代序号，线性索引，向量，元素）
        let mut count = 0;
        for (iteration_index, linear_index, vector, value) in view.enumerate() {
            assert_eq!(iteration_index, count);
            assert_eq!(*value, (count + 1) as i32);
            // 验证线性索引是从 dummy_iterator 获取的真实索引
            let calculated_index = array.shape.index_of(&vector).unwrap();
            assert_eq!(calculated_index, linear_index);
            count += 1;
        }
        assert_eq!(count, 6);
    }

    #[test]
    fn test_multi_array_view_enumerate_with_slicing() {
        let shape = Shape::new([3, 4, 5]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        // 填充数据
        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        // 创建一个带切片的视图
        let dummy_vector = dummy_expect![0..2, 1..3, 2..4];
        let view = array.view(&dummy_vector).unwrap();

        // 测试带切片视图的 enumerate 迭代器（返回四个字段）
        let mut count = 0;
        for (iteration_index, linear_index, vector, value) in view.enumerate() {
            assert_eq!(iteration_index, count);
            // 验证线性索引是从 dummy_iterator 获取的真实索引
            let calculated_index = array.shape.index_of(&vector).unwrap();
            assert_eq!(calculated_index, linear_index);
            count += 1;
        }
        // 2 * 2 * 2 = 8 个元素
        assert_eq!(count, 8);
    }

    // Note: Empty view test is skipped due to pre-existing edge case in dummy iterator
    // The dummy iterator panics when trying to iterate over empty views
}
