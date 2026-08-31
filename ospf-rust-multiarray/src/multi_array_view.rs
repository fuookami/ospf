//! 多维数组视图模块
//! Multi-dimensional array view module
//!
//! 本模块提供多维数组视图的实现：
//! This module provides the implementation of multi-dimensional array views:
//!
//! - `MultiArrayView`: 多维数组视图，支持切片、索引和转置等操作
//!   Multi-dimensional array view supporting slicing, indexing, and transposition
//! - `MultiArrayViewIter`: 视图迭代器
//!   View iterator
//! - `MultiArrayViewEnumerateIter`: 视图枚举迭代器
//!   View enumerate iterator
//! - `MultiArrayViewBuilder`: 视图构建器
//!   View builder
//!
//! ## 示例 / Examples
//!
//! ```rust
//! use ospf_rust_multiarray::{MultiArray, Shape, MultiArrayBuilder, RowMajor};
//!
//! // 创建数组视图 / Create array view
//! let shape: Shape<2, RowMajor> = Shape::new([2, 3]);
//! let array: MultiArray<i32, _> = MultiArrayBuilder::new_with(shape, 0);
//! ```

use super::concept::{
    AccessOrder, AccessOrderTrait, ColumnMajor, DummyVector, MapVector, RowMajor,
};
use super::dummy_index::{AdvancePolicy, DummyAccessIterator, DummyIndex, DummyIndexIterator};
use super::error::{DimensionMismatchingError, MappingIndexError, RepeatMappingIndexError};
use super::map_index::MapIndex;
use super::multi_array::{MultiArray, MultiArrayCollection, MultiArrayToView};
use super::shape::{AbstractShape, DynShape};
use cc_traits::{Collection, CollectionMut, CollectionRef, Iter, IterMut, Len};
use ospf_rust_base::collection::Indices;
use ospf_rust_base::container::Vec;
use ospf_rust_base::error::*;
use std::iter::Iterator;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut, RangeFull};
use std::ptr::NonNull;

/// 计算视图形状（内部函数）
/// Calculate view shape (internal function)
///
/// 根据原始形状和映射向量计算新视图的形状。
/// Calculates the shape of a new view based on the original shape and map vector.
///
/// ## 类型参数 / Type Parameters
///
/// - `S`: 形状类型
///   Shape type
///
/// ## 参数 / Parameters
///
/// - `original_shape`: 原始数组形状
///   Original array shape
/// - `map_vector`: 映射向量
///   Map vector
///
/// ## 返回值 / Returns
///
/// 成功返回动态形状，失败返回映射索引错误
/// Returns dynamic shape on success, or mapping index error on failure
pub(crate) fn calculate_view_shape<
    S: AbstractShape<StorageOrder = crate::concept::StorageOrder>,
>(
    original_shape: &S,
    map_vector: &S::MapVectorType,
) -> Result<DynShape<Vec, S::StorageOrder>, MappingIndexError> {
    let mut mapping_index: std::vec::Vec<(usize, usize)> = map_vector
        .indices()
        .into_iter()
        .filter_map(|i| match &map_vector[i] {
            MapIndex::Map(m) => Some((m.index, i)),
            _ => None,
        })
        .collect();
    mapping_index.sort_by_key(|k| k.0);
    let mut view_shape_vec = std::vec::Vec::new();

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
    Ok(DynShape::<Vec, S::StorageOrder>::new_with_order(
        view_shape_vec,
        original_shape.storage_order(),
    ))
}

/// 计算向量视图形状（内部函数）
/// Calculate vector view shape (internal function)
///
/// 根据原始形状和映射向量计算向量视图的形状，处理虚拟索引的特殊情况。
/// Calculates the shape of a vector view based on the original shape and map vector, handling dummy index special cases.
///
/// ## 类型参数 / Type Parameters
///
/// - `S`: 形状类型
///   Shape type
///
/// ## 参数 / Parameters
///
/// - `original_shape`: 原始数组形状
///   Original array shape
/// - `map_vector`: 映射向量
///   Map vector
///
/// ## 返回值 / Returns
///
/// 成功返回动态形状，失败返回映射索引错误
/// Returns dynamic shape on success, or mapping index error on failure
pub(crate) fn calculate_view_shape_for_vec<
    S: AbstractShape<StorageOrder = crate::concept::StorageOrder>,
>(
    original_shape: &S,
    map_vector: &S::MapVectorType,
) -> Result<DynShape<Vec, S::StorageOrder>, MappingIndexError> {
    let mut view_shape_vec = std::vec::Vec::new();

    for i in map_vector.indices() {
        match &map_vector[i] {
            MapIndex::Map(m) => {}
            MapIndex::Dummy(dummy) => {
                let len = dummy.len_of(original_shape, i);
                if len > 1 {
                    view_shape_vec.push(len);
                }
            }
        }
    }

    let has_map = map_vector
        .indices()
        .any(|i| matches!(&map_vector[i], MapIndex::Map(_)));

    if has_map {
        let mut mapping_index: std::vec::Vec<(usize, usize)> = map_vector
            .indices()
            .filter_map(|i| match &map_vector[i] {
                MapIndex::Map(m) => Some((m.index, i)),
                _ => None,
            })
            .collect();
        mapping_index.sort_by_key(|k| k.0);
        view_shape_vec = std::vec::Vec::with_capacity(mapping_index.len());

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
    }

    Ok(DynShape::<Vec, S::StorageOrder>::new_with_order(
        view_shape_vec,
        original_shape.storage_order(),
    ))
}

/// 多维数组视图
/// Multi-dimensional array view
///
/// 提供对多维数组的非拥有视图，支持切片、索引和转置等操作。
/// Provides a non-owning view of a multi-dimensional array, supporting slicing, indexing, and transposition.
///
/// ## 类型参数 / Type Parameters
///
/// - `'a`: 生命周期参数
///   Lifetime parameter
/// - `T`: 元素类型
///   Element type
/// - `S`: 形状类型
///   Shape type
/// - `AO`: 访问顺序类型，默认为 `AccessOrder`
///   Access order type, defaults to `AccessOrder`
/// - `C`: 存储容器类型
///   Storage container type
///
/// ## 示例 / Examples
///
/// ```ignore
/// use ospf_rust_multiarray::{MultiArray, Shape, MultiArrayBuilder, MultiArrayToView, dummy_expect};
///
/// let shape = Shape::new([2, 3]);
/// let array = MultiArrayBuilder::new_with(shape, 0i32);
/// let view = array.view(&dummy_expect![.., 1..3]).unwrap();
/// assert_eq!(view.len(), 4);
/// ```
pub struct MultiArrayView<
    'a,
    T,
    S: AbstractShape,
    AO: AccessOrderTrait = AccessOrder,
    C: MultiArrayCollection<T> = std::vec::Vec<T>,
> {
    /// 原始数组引用
    /// Reference to the original array
    array: &'a MultiArray<T, S, C>,

    /// 映射向量
    /// Map vector
    map_vector: S::MapVectorType,

    /// 视图形状
    /// View shape
    view_shape: DynShape<Vec, S::StorageOrder>,

    /// 元素数量
    /// Element count
    len: usize,

    /// 类型标记
    /// Type marker
    _marker: PhantomData<(T, AO)>,
}

impl<'a, T, S, AO, C> MultiArrayView<'a, T, S, AO, C>
where
    S: AbstractShape<StorageOrder = crate::concept::StorageOrder>,
    AO: AccessOrderTrait,
    C: MultiArrayCollection<T>,
{
    /// 使用虚拟索引向量创建视图
    /// Create a view using dummy index vector
    ///
    /// ## 参数 / Parameters
    ///
    /// - `array`: 原始数组引用
    ///   Reference to the original array
    /// - `dummy_vector`: 虚拟索引向量
    ///   Dummy index vector
    pub fn new_by_dummy(array: &'a MultiArray<T, S, C>, dummy_vector: &S::DummyVectorType) -> Self {
        let len = dummy_vector
            .indices()
            .fold(1, |acc, i| acc * dummy_vector[i].len_of(&array.shape, i));
        let map_vector = S::dummy_to_map_vector(dummy_vector);

        let view_shape =
            calculate_view_shape_for_vec(&array.shape, &map_vector).unwrap_or_else(|_| {
                DynShape::<Vec, S::StorageOrder>::new_with_order(
                    std::vec::Vec::new(),
                    array.shape.storage_order(),
                )
            });

        Self {
            array,
            map_vector,
            view_shape,
            len,
            _marker: PhantomData,
        }
    }

    /// 使用映射向量创建视图
    /// Create a view using map vector
    ///
    /// ## 参数 / Parameters
    ///
    /// - `array`: 原始数组引用
    ///   Reference to the original array
    /// - `map_vector`: 映射向量
    ///   Map vector
    ///
    /// ## 返回值 / Returns
    ///
    /// 成功返回视图，失败返回映射索引错误
    /// Returns the view on success, or mapping index error on failure
    pub fn new_by_map(
        array: &'a MultiArray<T, S, C>,
        map_vector: S::MapVectorType,
    ) -> Result<Self, MappingIndexError> {
        let view_shape = calculate_view_shape(&array.shape, &map_vector)?;
        let len = map_vector.indices().fold(1, |acc, i| match &map_vector[i] {
            MapIndex::Dummy(dummy) => acc * dummy.len_of(&array.shape, i),
            MapIndex::Map(_) => {
                acc * array
                    .shape
                    .len_of_dimension(i)
                    .expect("dimension should be valid in new_by_map")
            }
        });

        Ok(Self {
            array,
            map_vector,
            view_shape,
            len,
            _marker: PhantomData,
        })
    }

    /// 获取视图形状
    /// Get the view shape
    pub fn shape(&self) -> &DynShape<Vec, S::StorageOrder> {
        &self.view_shape
    }

    /// 获取映射向量
    /// Get the map vector
    pub fn map_vector(&self) -> &S::MapVectorType {
        &self.map_vector
    }

    /// 获取原始数组引用
    /// Get reference to the original array
    pub fn array(&self) -> &MultiArray<T, S, C> {
        self.array
    }

    /// 获取访问顺序
    /// Get the access order
    pub fn access_order(&self) -> AccessOrder {
        AO::default_value()
    }

    /// 使用虚拟索引向量创建子视图
    /// Create a sub-view using dummy index vector
    ///
    /// ## 参数 / Parameters
    ///
    /// - `dummy_vector`: 虚拟索引向量
    ///   Dummy index vector
    ///
    /// ## 返回值 / Returns
    ///
    /// 成功返回子视图，失败返回映射索引错误
    /// Returns the sub-view on success, or mapping index error on failure
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
                MapIndex::Map(_) => {
                    acc * self
                        .array
                        .shape
                        .len_of_dimension(i)
                        .expect("dimension should be valid in view_by_dummy")
                }
            });

        Ok(Self {
            array: self.array,
            map_vector: new_map_vector,
            view_shape: new_view_shape,
            len: new_len,
            _marker: PhantomData,
        })
    }

    /// 使用映射向量创建子视图
    /// Create a sub-view using map vector
    ///
    /// ## 参数 / Parameters
    ///
    /// - `map_vector`: 映射向量
    ///   Map vector
    ///
    /// ## 返回值 / Returns
    ///
    /// 成功返回子视图，失败返回映射索引错误
    /// Returns the sub-view on success, or mapping index error on failure
    pub fn view_by_map<V: MapVector>(&self, map_vector: &V) -> Result<Self, MappingIndexError> {
        if map_vector.len() > self.view_shape.dimension() {
            return Err(MappingIndexError::DimensionMismatching(error! {
                DimensionMismatchingError {
                    dimension: self.view_shape.dimension(),
                    vector_dimension: map_vector.len()
                }
            }));
        }

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
                MapIndex::Map(_) => {
                    acc * self
                        .array
                        .shape
                        .len_of_dimension(i)
                        .expect("dimension should be valid in view_by_map")
                }
            });

        Ok(Self {
            array: self.array,
            map_vector: new_map_vector,
            view_shape: new_view_shape,
            len: new_len,
            _marker: PhantomData,
        })
    }

    /// 获取枚举迭代器
    /// Get an enumerate iterator
    ///
    /// 返回一个迭代器，产生 (视图索引, 线性索引, 向量坐标, 元素引用) 四元组。
    /// Returns an iterator that yields (view index, linear index, vector coordinate, element reference) quadruples.
    ///
    /// ## 返回值 / Returns
    ///
    /// 返回枚举迭代器
    /// Returns the enumerate iterator
    pub fn enumerate(&self) -> MultiArrayViewEnumerateIter<'_, T, S, AO, C>
    where
        AO: AdvancePolicy<S>,
    {
        MultiArrayViewEnumerateIter::new(self)
    }
}

impl<'a, T, S, C> MultiArrayView<'a, T, S, AccessOrder, C>
where
    S: AbstractShape<StorageOrder = crate::concept::StorageOrder>,
    C: MultiArrayCollection<T>,
{
    /// 设置访问顺序
    /// Set the access order
    ///
    /// ## 参数 / Parameters
    ///
    /// - `order`: 访问顺序
    ///   Access order
    ///
    /// ## 返回值 / Returns
    ///
    /// 返回设置了新访问顺序的视图
    /// Returns the view with the new access order set
    pub fn with_access_order(self, order: AccessOrder) -> Self {
        Self {
            array: self.array,
            map_vector: self.map_vector,
            view_shape: self.view_shape,
            len: self.len,
            _marker: PhantomData,
        }
    }

    /// 使用指定访问顺序创建迭代器
    /// Create an iterator with specified access order
    ///
    /// ## 参数 / Parameters
    ///
    /// - `order`: 访问顺序
    ///   Access order
    ///
    /// ## 返回值 / Returns
    ///
    /// 返回视图迭代器
    /// Returns the view iterator
    pub fn iter_with_order(
        &self,
        order: AccessOrder,
    ) -> MultiArrayViewIter<'_, T, S, AccessOrder, C> {
        MultiArrayViewIter::new_from_view_with_order(
            self.array,
            &self.map_vector,
            &self.view_shape,
            order,
        )
    }
}

impl<'a, T, S, AO, C> Clone for MultiArrayView<'a, T, S, AO, C>
where
    S: AbstractShape<StorageOrder = crate::concept::StorageOrder>,
    AO: AccessOrderTrait,
    C: MultiArrayCollection<T>,
{
    fn clone(&self) -> Self {
        Self {
            array: self.array,
            map_vector: self.map_vector.clone(),
            view_shape: self.view_shape.clone(),
            len: self.len,
            _marker: PhantomData,
        }
    }
}

impl<'a, T, S, AO, C> Collection for MultiArrayView<'a, T, S, AO, C>
where
    S: AbstractShape<StorageOrder = crate::concept::StorageOrder>,
    AO: AccessOrderTrait,
    C: MultiArrayCollection<T>,
{
    type Item = T;
}

impl<'a, T, S, AO, C> CollectionRef for MultiArrayView<'a, T, S, AO, C>
where
    S: AbstractShape<StorageOrder = crate::concept::StorageOrder>,
    AO: AccessOrderTrait,
    C: MultiArrayCollection<T>,
{
    type ItemRef<'b>
        = &'b T
    where
        Self: 'b,
        T: 'b;

    cc_traits::covariant_item_ref!();
}

impl<'a, T, S, AO, C> Len for MultiArrayView<'a, T, S, AO, C>
where
    S: AbstractShape<StorageOrder = crate::concept::StorageOrder>,
    AO: AccessOrderTrait,
    C: MultiArrayCollection<T>,
{
    fn len(&self) -> usize {
        self.len
    }

    fn is_empty(&self) -> bool {
        self.view_shape.len() == 0
    }
}

impl<'a, T, S, AO, C> MultiArrayToView<DynShape> for MultiArrayView<'a, T, S, AO, C>
where
    S: AbstractShape<StorageOrder = crate::concept::StorageOrder>,
    AO: AccessOrderTrait,
    C: MultiArrayCollection<T>,
{
    type ViewType<'b>
        = MultiArrayView<'b, T, S, AO, C>
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

/// 多维数组视图迭代器
/// Multi-dimensional array view iterator
///
/// 遍历多维数组视图中所有元素的不可变引用。
/// Iterates over immutable references to all elements in a multi-dimensional array view.
///
/// ## 类型参数 / Type Parameters
///
/// - `'a`: 生命周期参数
///   Lifetime parameter
/// - `T`: 元素类型
///   Element type
/// - `S`: 形状类型
///   Shape type
/// - `AO`: 访问顺序类型
///   Access order type
/// - `C`: 存储容器类型
///   Storage container type
pub struct MultiArrayViewIter<
    'a,
    T,
    S: AbstractShape<StorageOrder = crate::concept::StorageOrder>,
    AO: AccessOrderTrait,
    C: MultiArrayCollection<T> = std::vec::Vec<T>,
> {
    /// 映射向量
    /// Map vector
    map_vector: S::MapVectorType,

    /// 虚拟迭代器
    /// Dummy iterator
    dummy_iterator: DummyAccessIterator<'a, S, AO>,

    /// 原始数组引用
    /// Reference to the original array
    array: &'a MultiArray<T, S, C>,
}

impl<'a, T, S, AO, C> MultiArrayViewIter<'a, T, S, AO, C>
where
    S: AbstractShape<StorageOrder = crate::concept::StorageOrder>,
    AO: AccessOrderTrait + AdvancePolicy<S>,
    C: MultiArrayCollection<T>,
{
    /// 从视图创建迭代器
    /// Create an iterator from a view
    ///
    /// ## 参数 / Parameters
    ///
    /// - `array`: 原始数组引用
    ///   Reference to the original array
    /// - `map_vector`: 映射向量
    ///   Map vector
    /// - `_view_shape`: 视图形状（未使用）
    ///   View shape (unused)
    pub fn new_from_view(
        array: &'a MultiArray<T, S, C>,
        map_vector: &S::MapVectorType,
        _view_shape: &DynShape<Vec, S::StorageOrder>,
    ) -> Self {
        let iterators = array.shape.map_to_iterator_vector(map_vector);

        let dummy_iterator = DummyAccessIterator::new(&array.shape, iterators, AO::default());

        Self {
            map_vector: map_vector.clone(),
            dummy_iterator,
            array,
        }
    }
}

impl<'a, T, S, C> MultiArrayViewIter<'a, T, S, AccessOrder, C>
where
    S: AbstractShape<StorageOrder = crate::concept::StorageOrder>,
    C: MultiArrayCollection<T>,
{
    pub fn new_from_view_with_order(
        array: &'a MultiArray<T, S, C>,
        map_vector: &S::MapVectorType,
        _view_shape: &DynShape<Vec, S::StorageOrder>,
        access_order: AccessOrder,
    ) -> Self {
        let iterators = array.shape.map_to_iterator_vector(map_vector);

        let dummy_iterator = DummyAccessIterator::new(&array.shape, iterators, access_order);

        Self {
            map_vector: map_vector.clone(),
            dummy_iterator,
            array,
        }
    }
}

impl<'a, T, S, AO, C> Iterator for MultiArrayViewIter<'a, T, S, AO, C>
where
    S: AbstractShape<StorageOrder = crate::concept::StorageOrder>,
    AO: AccessOrderTrait + AdvancePolicy<S>,
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

impl<'a, T, S, AO, C> Iter for MultiArrayView<'a, T, S, AO, C>
where
    S: AbstractShape<StorageOrder = crate::concept::StorageOrder>,
    AO: AccessOrderTrait + AdvancePolicy<S> + Default,
    C: MultiArrayCollection<T>,
{
    type Iter<'b>
        = MultiArrayViewIter<'b, T, S, AO, C>
    where
        Self: 'b,
        T: 'b;

    fn iter(&self) -> Self::Iter<'_> {
        MultiArrayViewIter::new_from_view(self.array, &self.map_vector, &self.view_shape)
    }
}

/// 多维数组视图枚举迭代器
/// Multi-dimensional array view enumerate iterator
///
/// 遍历多维数组视图，产生 (视图索引, 线性索引, 向量坐标, 元素引用) 四元组。
/// Iterates over a multi-dimensional array view, yielding (view index, linear index, vector coordinate, element reference) quadruples.
///
/// ## 类型参数 / Type Parameters
///
/// - `'a`: 生命周期参数
///   Lifetime parameter
/// - `T`: 元素类型
///   Element type
/// - `S`: 形状类型
///   Shape type
/// - `AO`: 访问顺序类型
///   Access order type
/// - `C`: 存储容器类型
///   Storage container type
pub struct MultiArrayViewEnumerateIter<
    'a,
    T,
    S: AbstractShape<StorageOrder = crate::concept::StorageOrder>,
    AO: AccessOrderTrait,
    C: MultiArrayCollection<T> = std::vec::Vec<T>,
> {
    /// 映射向量
    /// Map vector
    map_vector: S::MapVectorType,

    /// 虚拟迭代器
    /// Dummy iterator
    dummy_iterator: DummyAccessIterator<'a, S, AO>,

    /// 原始数组引用
    /// Reference to the original array
    array: &'a MultiArray<T, S, C>,

    /// 当前索引
    /// Current index
    current_index: usize,
}

impl<'a, T, S, AO, C> MultiArrayViewEnumerateIter<'a, T, S, AO, C>
where
    S: AbstractShape<StorageOrder = crate::concept::StorageOrder>,
    AO: AccessOrderTrait + AdvancePolicy<S>,
    C: MultiArrayCollection<T>,
{
    /// 创建新的枚举迭代器
    /// Create a new enumerate iterator
    ///
    /// ## 参数 / Parameters
    ///
    /// - `view`: 视图引用
    ///   View reference
    pub fn new(view: &'a MultiArrayView<'a, T, S, AO, C>) -> Self {
        let iterators = view.array.shape.map_to_iterator_vector(&view.map_vector);

        let dummy_iterator = DummyAccessIterator::new(&view.array.shape, iterators, AO::default());

        Self {
            map_vector: view.map_vector.clone(),
            dummy_iterator,
            array: view.array,
            current_index: 0,
        }
    }
}

impl<'a, T, S, AO, C> Iterator for MultiArrayViewEnumerateIter<'a, T, S, AO, C>
where
    S: AbstractShape<StorageOrder = crate::concept::StorageOrder>,
    AO: AccessOrderTrait + AdvancePolicy<S>,
    C: MultiArrayCollection<T>,
{
    type Item = (usize, usize, S::VectorType, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(vector) = self.dummy_iterator.next() {
            if let Ok(linear_index) = self.array.shape.index_of(vector) {
                let index = self.current_index;
                let view_vector = self.array.shape.vector_of(linear_index).ok()?;
                let element = &self.array[linear_index];
                self.current_index += 1;

                return Some((index, linear_index, view_vector, element));
            }
        }
        None
    }
}

impl<'a, T, S, AO, C> Index<usize> for MultiArrayView<'a, T, S, AO, C>
where
    S: AbstractShape<StorageOrder = crate::concept::StorageOrder>,
    AO: AccessOrderTrait,
    C: MultiArrayCollection<T>,
{
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        let mut non_index_count = 0;

        for i in self.map_vector.indices() {
            match &self.map_vector[i] {
                MapIndex::Dummy(dummy) => match dummy {
                    DummyIndex::Index(_) => {}
                    _ => {
                        non_index_count += 1;
                        if non_index_count > 1 {
                            panic!(
                                "Cannot use single index when multiple dimensions are not fixed / 当多个维度未固定时无法使用单一索引"
                            );
                        }
                    }
                },
                MapIndex::Map(_) => {
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
            &self.array[index]
        } else if non_index_count == 1 {
            let mut vector = self.array.shape.zero();

            for i in self.map_vector.indices() {
                match &self.map_vector[i] {
                    MapIndex::Dummy(dummy) => match dummy {
                        DummyIndex::Index(val) => {
                            let actual_index = if *val < 0 {
                                let dim_len = self
                                    .array
                                    .shape
                                    .len_of_dimension(i)
                                    .expect("dimension should be valid in Index<usize>");
                                (dim_len as isize + val) as usize
                            } else {
                                *val as usize
                            };
                            vector[i] = actual_index;
                        }
                        _ => {
                            vector[i] = index;
                        }
                    },
                    MapIndex::Map(placeholder) => {
                        let map_dim = placeholder.index;
                        vector[map_dim] = index;
                    }
                }
            }

            let flat_idx = self
                .array
                .shape
                .index_of(&vector)
                .expect("vector index should be valid in Index<usize>");
            &self.array[flat_idx]
        } else {
            panic!(
                "Cannot use single index when multiple dimensions are not fixed / 当多个维度未固定时无法使用单一索引"
            );
        }
    }
}

/// 多维数组视图构建器
/// Multi-dimensional array view builder
///
/// 提供便捷的静态方法来创建多维数组视图。
/// Provides convenient static methods to create multi-dimensional array views.
///
/// ## 类型参数 / Type Parameters
///
/// - `AO`: 访问顺序类型
///   Access order type
///
/// ## 示例 / Examples
///
/// ```ignore
/// // 使用行主序创建视图 / Create view with row-major order
/// let view = MultiArrayViewBuilderRM::new_by_dummy(&array, &dummy_expect![.., ..]);
///
/// // 使用列主序创建视图 / Create view with column-major order
/// let view = MultiArrayViewBuilderCM::new_by_dummy(&array, &dummy_expect![.., ..]);
/// ```
pub struct MultiArrayViewBuilder<AO: AccessOrderTrait> {
    /// 类型标记
    /// Type marker
    _marker: PhantomData<AO>,
}

impl<AO: AccessOrderTrait> MultiArrayViewBuilder<AO> {
    /// 使用虚拟索引向量创建视图
    /// Create a view using dummy index vector
    ///
    /// ## 参数 / Parameters
    ///
    /// - `array`: 原始数组引用
    ///   Reference to the original array
    /// - `dummy_vector`: 虚拟索引向量
    ///   Dummy index vector
    ///
    /// ## 返回值 / Returns
    ///
    /// 返回创建的视图
    /// Returns the created view
    #[inline]
    pub fn new_by_dummy<'a, T, S, C>(
        array: &'a MultiArray<T, S, C>,
        dummy_vector: &S::DummyVectorType,
    ) -> MultiArrayView<'a, T, S, AO, C>
    where
        S: AbstractShape<StorageOrder = crate::concept::StorageOrder>,
        AO: AccessOrderTrait + AdvancePolicy<S>,
        C: MultiArrayCollection<T>,
        S::DummyVectorType: DummyVector,
    {
        MultiArrayView::<'a, T, S, AO, C>::new_by_dummy(array, dummy_vector)
    }

    /// 使用映射向量创建视图
    /// Create a view using map vector
    ///
    /// ## 参数 / Parameters
    ///
    /// - `array`: 原始数组引用
    ///   Reference to the original array
    /// - `map_vector`: 映射向量
    ///   Map vector
    ///
    /// ## 返回值 / Returns
    ///
    /// 成功返回视图，失败返回映射索引错误
    /// Returns the view on success, or mapping index error on failure
    #[inline]
    pub fn new_by_map<'a, T, S, C>(
        array: &'a MultiArray<T, S, C>,
        map_vector: S::MapVectorType,
    ) -> Result<MultiArrayView<'a, T, S, AO, C>, MappingIndexError>
    where
        S: AbstractShape<StorageOrder = crate::concept::StorageOrder>,
        AO: AccessOrderTrait + AdvancePolicy<S>,
        C: MultiArrayCollection<T>,
    {
        MultiArrayView::<'a, T, S, AO, C>::new_by_map(array, map_vector)
    }
}

/// 行主序视图构建器类型别名
/// Row-major view builder type alias
pub type MultiArrayViewBuilderRM = MultiArrayViewBuilder<RowMajor>;

/// 列主序视图构建器类型别名
/// Column-major view builder type alias
pub type MultiArrayViewBuilderCM = MultiArrayViewBuilder<ColumnMajor>;

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
        let shape: Shape<2> = Shape::new([2, 3]);
        let array = MultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![_1, _0];

        let view: MultiArrayView<'_, _, Shape<2>> = MultiArrayView::new_by_map(&array, map_vector)
            .expect("Should be able to create MultiArrayView");

        assert_eq!(view.shape().dimension(), 2);
        assert_eq!(view.shape().len_of_dimension(0).unwrap(), 3);
        assert_eq!(view.shape().len_of_dimension(1).unwrap(), 2);
        assert_eq!(view.len(), 6);
        assert!(!view.is_empty());
    }

    #[test]
    fn test_multi_array_view_with_dummy_index() {
        let shape: Shape<2> = Shape::new([2, 3]);
        let array = MultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![_0, 1];

        let view: MultiArrayView<'_, _, Shape<2>> = MultiArrayView::new_by_map(&array, map_vector)
            .expect("Should be able to create MultiArrayView");

        assert_eq!(view.shape().dimension(), 1);
        assert_eq!(view.shape().len_of_dimension(0).unwrap(), 2);
        assert_eq!(view.len(), 2);
    }

    #[test]
    fn test_multi_array_view_iter_with_mixed_indices() {
        let shape: Shape<3> = Shape::new([2, 3, 4]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = i as i32;
        }

        let map_vector = map_expect![_0, 1, _1];
        let view: MultiArrayView<'_, _, Shape<3>> = MultiArrayView::new_by_map(&array, map_vector)
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
    fn test_multi_array_view_rm_creation() {
        let shape: Shape<2> = Shape::new([2, 3]);
        let array = MultiArrayBuilder::new_with(shape, 0);

        let view = MultiArrayViewBuilderRM::new_by_dummy(&array, &dummy_expect![.., ..]);

        assert_eq!(view.shape().dimension(), 2);
        assert_eq!(view.len(), 6);
        assert_eq!(view.access_order(), AccessOrder::RowMajor);
    }

    #[test]
    fn test_view_by_dummy_chain() {
        use crate::map_index::_2;

        let shape: Shape<3> = Shape::new([2, 3, 4]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        let map_vector = map_expect![_0, _1, _2];
        let view: MultiArrayView<'_, _, Shape<3>> =
            MultiArrayView::new_by_map(&array, map_vector).unwrap();

        let dummy_vector = dummy_expect![0, .., ..];
        let view2 = view.view_by_dummy(&dummy_vector).unwrap();

        assert_eq!(view2.len(), 12);
        let values: std::vec::Vec<i32> = view2.iter().copied().collect();
        assert_eq!(values, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
    }

    #[test]
    fn test_view_by_map_chain() {
        use crate::map_index::_2;

        let shape: Shape<3> = Shape::new([2, 3, 4]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        let map_vector = map_expect![_0, _1, _2];
        let view: MultiArrayView<'_, _, Shape<3>> =
            MultiArrayView::new_by_map(&array, map_vector).unwrap();

        let new_map_vector = map_expect![_2, _1, _0];
        let view2 = view.view_by_map(&new_map_vector).unwrap();

        assert_eq!(view2.shape().len_of_dimension(0).unwrap(), 4);
        assert_eq!(view2.shape().len_of_dimension(1).unwrap(), 3);
        assert_eq!(view2.shape().len_of_dimension(2).unwrap(), 2);
        assert_eq!(view2.len(), 24);
    }

    #[test]
    fn test_view_index_single() {
        let shape: Shape<2> = Shape::new([3, 4]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        let map_vector = map_expect![0, ..];
        let view: MultiArrayView<'_, _, Shape<2>> =
            MultiArrayView::new_by_map(&array, map_vector).unwrap();

        assert_eq!(view.len(), 4);
        assert_eq!(view[0], 1);
        assert_eq!(view[1], 2);
        assert_eq!(view[2], 3);
        assert_eq!(view[3], 4);
    }

    // NOTE: Negative index test commented out due to macro syntax limitations
    // #[test]
    // fn test_view_with_negative_index() {
    //     let shape: Shape<2> = Shape::new([3, 4]);
    //     let mut array = MultiArrayBuilder::new_with(shape, 0);
    //
    //     for i in 0..array.len() {
    //         array[i] = (i + 1) as i32;
    //     }
    //
    //     let map_vector = map_expect![-1, ..];
    //     let view: MultiArrayView<'_, _, Shape<2>> =
    //         MultiArrayView::new_by_map(&array, map_vector).unwrap();
    //
    //     assert_eq!(view.len(), 4);
    //     let values: std::vec::Vec<i32> = view.iter().copied().collect();
    //     assert_eq!(values, vec![9, 10, 11, 12]);
    // }

    #[test]
    fn test_view_with_index_array() {
        let shape: Shape<1> = Shape::new([5]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        let map_vector = map_expect![vec![0, 2, 4]];
        let view: MultiArrayView<'_, _, Shape<1>> =
            MultiArrayView::new_by_map(&array, map_vector).unwrap();

        assert_eq!(view.len(), 3);
        let values: std::vec::Vec<i32> = view.iter().copied().collect();
        assert_eq!(values, vec![1, 3, 5]);
    }

    #[test]
    fn test_view_empty() {
        let shape: Shape<2> = Shape::new([2, 3]);
        let array = MultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![0, 1];
        let view: MultiArrayView<'_, _, Shape<2>> =
            MultiArrayView::new_by_map(&array, map_vector).unwrap();

        assert_eq!(view.len(), 1);
        assert!(!view.is_empty());
    }

    #[test]
    fn test_view_clone() {
        let shape: Shape<2> = Shape::new([2, 3]);
        let array = MultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![_0, _1];
        let view: MultiArrayView<'_, _, Shape<2>> =
            MultiArrayView::new_by_map(&array, map_vector).unwrap();

        let cloned = view.clone();
        assert_eq!(cloned.len(), view.len());
        assert_eq!(cloned.shape().dimension(), view.shape().dimension());
    }

    #[test]
    fn test_view_enumerate() {
        let shape: Shape<2> = Shape::new([2, 3]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        let map_vector = map_expect![_0, _1];
        let view: MultiArrayView<'_, _, Shape<2>> =
            MultiArrayView::new_by_map(&array, map_vector).unwrap();

        let mut count = 0;
        for (idx, linear_idx, vector, value) in view.enumerate() {
            assert_eq!(idx, count);
            assert_eq!(*value, (count + 1) as i32);
            count += 1;
        }
        assert_eq!(count, 6);
    }

    #[test]
    fn test_view_with_range() {
        let shape: Shape<1> = Shape::new([10]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        let map_vector = map_expect![2..6];
        let view: MultiArrayView<'_, _, Shape<1>> =
            MultiArrayView::new_by_map(&array, map_vector).unwrap();

        assert_eq!(view.len(), 4);
        let values: std::vec::Vec<i32> = view.iter().copied().collect();
        assert_eq!(values, vec![3, 4, 5, 6]);
    }

    #[test]
    fn test_view_with_range_inclusive() {
        let shape: Shape<1> = Shape::new([10]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        let map_vector = map_expect![2..=5];
        let view: MultiArrayView<'_, _, Shape<1>> =
            MultiArrayView::new_by_map(&array, map_vector).unwrap();

        assert_eq!(view.len(), 4);
        let values: std::vec::Vec<i32> = view.iter().copied().collect();
        assert_eq!(values, vec![3, 4, 5, 6]);
    }

    #[test]
    fn test_view_map_transpose() {
        let shape: Shape<2> = Shape::new([2, 3]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        let map_vector = map_expect![_1, _0];
        let view: MultiArrayView<'_, _, Shape<2>> =
            MultiArrayView::new_by_map(&array, map_vector).unwrap();

        assert_eq!(view.shape().len_of_dimension(0).unwrap(), 3);
        assert_eq!(view.shape().len_of_dimension(1).unwrap(), 2);
        assert_eq!(view.len(), 6);
    }

    #[test]
    fn test_view_map_repeat_mapping_error() {
        let shape: Shape<3> = Shape::new([2, 3, 4]);
        let array = MultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![_0, _0, _1];
        let result: Result<MultiArrayView<'_, _, Shape<3>>, _> =
            MultiArrayView::new_by_map(&array, map_vector);
        assert!(result.is_err());
    }

    #[test]
    fn test_view_map_non_continuous_mapping_error() {
        let shape: Shape<3> = Shape::new([2, 3, 4]);
        let array = MultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![_0, 0, _2];
        let result: Result<MultiArrayView<'_, _, Shape<3>>, _> =
            MultiArrayView::new_by_map(&array, map_vector);
        assert!(result.is_err());
    }

    #[test]
    fn test_view_clone_deep() {
        let shape: Shape<2> = Shape::new([3, 4]);
        let mut array = MultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        let map_vector = map_expect![_0, _1];
        let view1: MultiArrayView<'_, _, Shape<2>> =
            MultiArrayView::new_by_map(&array, map_vector).unwrap();

        let view2 = view1.clone();
        assert_eq!(view1.len(), view2.len());
        assert_eq!(view1.shape().dimension(), view2.shape().dimension());

        let values1: std::vec::Vec<i32> = view1.iter().copied().collect();
        let values2: std::vec::Vec<i32> = view2.iter().copied().collect();
        assert_eq!(values1, values2);
    }
}
