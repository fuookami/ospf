//! 形状定义模块
//! Shape definition module
//!
//! 本模块提供多维数组的形状抽象：
//! This module provides shape abstractions for multi-dimensional arrays:
//!
//! - `AbstractShape`: 形状 trait，定义维度、索引计算等核心操作
//!   Shape trait defining core operations like dimension, index calculation
//! - `Shape<N, SO>`: 编译期形状，维度 N 在编译时确定
//!   Compile-time shape with dimension N determined at compile time
//! - `DynShape<C, SO>`: 运行期形状，维度在运行时确定
//!   Runtime shape with dimension determined at runtime

use super::concept::*;
use super::dummy_index::{DummyIndex, DummyIndexIterator, IteratorVector};
use super::error::{DimensionMismatchingError, IndexCalculationError, OutOfShapeError};
use super::map_index::MapIndex;
use cc_traits::Len;
use ospf_rust_base::error::*;
use ospf_rust_base::Indices;
use std::fmt::{Debug, Display, Formatter};
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::mem;
use std::ops::{Index, IndexMut, RangeFull};
use std::result::Result;
use std::sync::OnceLock;

/// 抽象形状 trait
/// Abstract shape trait
///
/// 定义多维数组形状的核心接口。
/// Defines the core interface for multi-dimensional array shapes.
pub trait AbstractShape {
    /// 维度常量
    /// Dimension constant
    const DIMENSION: usize;

    /// 存储顺序类型
    /// Storage order type
    type StorageOrder: StorageOrderTrait;

    /// 向量类型
    /// Vector type
    type VectorType: Vector + Clone;

    /// 虚拟向量类型
    /// Dummy vector type
    type DummyVectorType: DummyVector + Clone;

    /// 映射向量类型
    /// Map vector type
    type MapVectorType: MapVector + Clone;

    /// 形状向量类型
    /// Shape vector type
    type ShapeVectorType: ShapeVector;

    /// 迭代器向量类型
    /// Iterator vector type
    type IteratorVectorType: IteratorVector + Clone;

    /// 带存储顺序的形状类型
    /// Shape type with storage order
    type ShapeWithStorageOrder<NewSO: StorageOrderTrait>: AbstractShape<
            StorageOrder = NewSO,
            VectorType = Self::VectorType,
            DummyVectorType = Self::DummyVectorType,
            MapVectorType = Self::MapVectorType,
            ShapeVectorType = Self::ShapeVectorType,
            IteratorVectorType = Self::IteratorVectorType,
        >;

    /// 创建零向量
    /// Create a zero vector
    fn zero(&self) -> Self::VectorType;

    /// 获取总元素数量
    /// Get total element count
    fn len(&self) -> usize;

    /// 获取维度数量
    /// Get dimension count
    fn dimension(&self) -> usize {
        Self::DIMENSION
    }

    /// 获取向量的维度
    /// Get dimension of a vector
    fn dimension_of(_: &Self::VectorType) -> usize {
        Self::DIMENSION
    }

    /// 获取形状
    /// Get the shape
    fn shape(&self) -> &impl Vector;

    /// 获取偏移量
    /// Get the offsets
    fn offsets(&self) -> &impl Vector;

    /// 获取存储顺序
    /// Get the storage order
    fn storage_order(&self) -> StorageOrder;

    /// 从形状向量创建形状
    /// Create shape from shape vector
    fn from_shape_vector(shape: &Self::ShapeVectorType) -> Self
    where
        Self: Sized;

    /// 从形状向量和存储顺序创建形状
    /// Create shape from shape vector and storage order
    fn from_shape_vector_with_order<NewSO: StorageOrderTrait>(
        shape: &Self::ShapeVectorType,
        order: NewSO,
    ) -> Self::ShapeWithStorageOrder<NewSO>
    where
        Self: Sized;

    /// 转换为指定存储顺序的形状
    /// Convert to shape with specified storage order
    fn with_storage_order<NewSO: StorageOrderTrait>(
        &self,
        order: NewSO,
    ) -> Self::ShapeWithStorageOrder<NewSO>;

    /// 获取指定维度的长度
    /// Get length of specified dimension
    fn len_of_dimension(&self, dimension: usize) -> Result<usize, DimensionMismatchingError> {
        if dimension >= self.dimension() {
            Err(error! {
                DimensionMismatchingError {
                    dimension: self.dimension(),
                    vector_dimension: dimension
                }
            })
        } else {
            Ok(self.shape()[dimension])
        }
    }

    /// 获取指定维度的偏移量
    /// Get offset of specified dimension
    fn offset_of_dimension(&self, dimension: usize) -> Result<usize, DimensionMismatchingError> {
        if dimension >= self.dimension() {
            Err(error! {
                DimensionMismatchingError {
                    dimension: self.dimension(),
                    vector_dimension: dimension
                }
            })
        } else {
            Ok(self.offsets()[dimension])
        }
    }

    /// 将向量转换为线性索引
    /// Convert vector to linear index
    fn index_of(&self, vector: &Self::VectorType) -> Result<usize, IndexCalculationError> {
        let vector_dimension = Self::dimension_of(vector);
        if vector_dimension != self.dimension() {
            Err(IndexCalculationError::DimensionMismatching(error! {
                DimensionMismatchingError {
                    dimension: self.dimension(),
                    vector_dimension: vector_dimension
                }
            }))
        } else {
            let mut index = 0;
            for i in 0..self.dimension() {
                if vector[i] >= self.len_of_dimension(i)? {
                    return Err(IndexCalculationError::OutOfShape(error! {
                        OutOfShapeError {
                            dimension: i,
                            len: self.len_of_dimension(i)?,
                            index: vector[i].cast_signed()
                        }
                    }));
                }
                index += vector[i] * self.offset_of_dimension(i)?;
            }
            Ok(index)
        }
    }

    /// 将线性索引转换为向量
    /// Convert linear index to vector
    fn vector_of(&self, mut index: usize) -> Result<Self::VectorType, IndexCalculationError> {
        let mut vector = self.zero();
        
        // 根据存储顺序决定遍历方向
        // Determine traversal direction based on storage order
        // RowMajor: 从高维到低维计算 (Calculate from high to low dimension)
        // ColumnMajor: 从低维到高维计算 (Calculate from low to high dimension)
        match self.storage_order() {
            crate::concept::StorageOrder::RowMajor => {
                for i in 0..self.dimension() {
                    let offset = self.offset_of_dimension(i)?;
                    vector[i] = index / offset;
                    index = index % offset;
                }
            }
            crate::concept::StorageOrder::ColumnMajor => {
                for i in (0..self.dimension()).rev() {
                    let offset = self.offset_of_dimension(i)?;
                    vector[i] = index / offset;
                    index = index % offset;
                }
            }
        }
        Ok(vector)
    }

    /// 生成下一个向量（按字典序）
    /// Generate next vector (lexicographic order)
    fn next_vector(&self, vector: &mut Self::VectorType) -> bool {
        let mut carry = false;
        vector[self.dimension() - 1] += 1;

        for i in (0..self.dimension()).rev() {
            if carry {
                vector[i] += 1;
                carry = false;
            }
            if vector[i] == self.len_of_dimension(i).unwrap() {
                vector[i] = 0;
                carry = true;
            }
        }
        !carry
    }

    /// 将负索引转换为实际索引
    /// Convert negative index to actual index
    fn actual_index(&self, dimension: usize, index: isize) -> Option<usize> {
        let len = self.len_of_dimension(dimension).unwrap();
        let len_isize = len.cast_signed();

        if index >= len_isize || index < -len_isize {
            None
        } else {
            let adjusted_index = if index >= 0 { index } else { len_isize + index };
            Some(adjusted_index.cast_unsigned())
        }
    }

    /// 将虚拟向量转换为映射向量
    /// Convert dummy vector to map vector
    fn dummy_to_map_vector(dummy: &Self::DummyVectorType) -> Self::MapVectorType;

    /// 将映射向量转换为虚拟向量
    /// Convert map vector to dummy vector
    fn map_to_dummy_vector(map_vector: &Self::MapVectorType) -> Self::DummyVectorType;

    /// 将虚拟向量转换为迭代器向量
    /// Convert dummy vector to iterator vector
    fn dummy_to_iterator_vector(&self, dummy: &Self::DummyVectorType) -> Self::IteratorVectorType;

    /// 将映射向量转换为迭代器向量
    /// Convert map vector to iterator vector
    fn map_to_iterator_vector(&self, map_vector: &Self::MapVectorType) -> Self::IteratorVectorType;
}

/// 编译期形状结构体
/// Compile-time shape struct
///
/// 维度在编译期确定的形状。
/// Shape with dimension determined at compile time.
pub struct Shape<const D: usize, SO: StorageOrderTrait = StorageOrder> {
    /// 形状数组
    /// Shape array
    pub(crate) shape: [usize; D],

    /// 偏移量和长度的延迟初始化缓存
    /// Lazy-initialized cache for offsets and length
    pub(crate) offsets_and_len: OnceLock<([usize; D], usize)>,

    /// 存储顺序
    /// Storage order
    pub(crate) storage_order: SO,
}

impl<const D: usize, SO: StorageOrderTrait> Debug for Shape<D, SO> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("Shape")
            .field("shape", &self.shape)
            .field("offsets", self.offsets())
            .field("len", &self.len())
            .field("storage_order", &self.storage_order().runtime_value())
            .finish()
    }
}

impl<const D: usize, SO: StorageOrderTrait> Display for Shape<D, SO> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.shape)
    }
}

impl<const D: usize, SO: StorageOrderTrait + Clone> Clone for Shape<D, SO> {
    fn clone(&self) -> Self {
        Self {
            shape: self.shape,
            offsets_and_len: OnceLock::new(),
            storage_order: self.storage_order.clone(),
        }
    }
}

impl<const D: usize, SO: StorageOrderTrait> Index<usize> for Shape<D, SO> {
    type Output = usize;

    fn index(&self, index: usize) -> &Self::Output {
        &self.shape[index]
    }
}

impl<SO: StorageOrderTrait + Default> Shape<1, SO> {
    /// 创建一维形状
    /// Create a 1D shape
    pub fn new_with(shape: usize) -> Self {
        Self::new([shape])
    }
}

impl<const D: usize, SO: StorageOrderTrait> Shape<D, SO> {
    /// 创建新形状
    /// Create a new shape
    pub fn new(shape: [usize; D]) -> Self
    where
        SO: Default,
    {
        Self::new_with_order(shape, SO::default())
    }

    /// 使用指定存储顺序创建形状
    /// Create shape with specified storage order
    pub fn new_with_order(shape: [usize; D], storage_order: SO) -> Self {
        Self {
            shape,
            offsets_and_len: OnceLock::new(),
            storage_order,
        }
    }

    /// 获取或初始化偏移量和长度
    /// Get or initialize offsets and length
    fn get_or_init_offsets_and_len(&self) -> &([usize; D], usize) {
        self.offsets_and_len
            .get_or_init(|| self.storage_order.offsets(&self.shape))
    }
}

impl<const D: usize, SO: StorageOrderTrait> AbstractShape for Shape<D, SO> {
    const DIMENSION: usize = D;
    type StorageOrder = SO;
    type VectorType = [usize; D];
    type DummyVectorType = [DummyIndex; D];
    type MapVectorType = [MapIndex; D];
    type ShapeVectorType = [usize; D];
    type IteratorVectorType = [DummyIndexIterator; D];
    type ShapeWithStorageOrder<NewSO: StorageOrderTrait> = Shape<D, NewSO>;

    #[inline]
    fn zero(&self) -> Self::VectorType {
        unsafe { mem::zeroed() }
    }

    #[inline]
    fn len(&self) -> usize {
        self.get_or_init_offsets_and_len().1
    }

    #[inline]
    fn shape(&self) -> &impl Vector {
        &self.shape
    }

    #[inline]
    fn offsets(&self) -> &impl Vector {
        &self.get_or_init_offsets_and_len().0
    }

    #[inline]
    fn storage_order(&self) -> StorageOrder {
        self.storage_order.runtime_value()
    }

    #[inline]
    fn from_shape_vector(shape: &Self::ShapeVectorType) -> Self {
        Self::new(*shape)
    }

    #[inline]
    fn from_shape_vector_with_order<NewSO: StorageOrderTrait>(
        shape: &Self::ShapeVectorType,
        order: NewSO,
    ) -> Shape<D, NewSO> {
        Shape::new_with_order(*shape, order)
    }

    #[inline]
    fn with_storage_order<NewSO: StorageOrderTrait>(&self, order: NewSO) -> Shape<D, NewSO> {
        Shape {
            shape: self.shape,
            offsets_and_len: OnceLock::new(),
            storage_order: order,
        }
    }

    #[inline]
    fn dummy_to_map_vector(dummy: &Self::DummyVectorType) -> Self::MapVectorType {
        core::array::from_fn(|i| MapIndex::Dummy(dummy[i].clone()))
    }

    #[inline]
    fn map_to_dummy_vector(map_vector: &Self::MapVectorType) -> Self::DummyVectorType {
        core::array::from_fn(|i| match &map_vector[i] {
            MapIndex::Dummy(dummy) => dummy.clone(),
            MapIndex::Map(_) => DummyIndex::Range(Box::new(RangeFull)),
        })
    }

    #[inline]
    fn dummy_to_iterator_vector(&self, dummy: &Self::DummyVectorType) -> Self::IteratorVectorType {
        core::array::from_fn(|i| dummy[i].iterator_of(self, i))
    }

    #[inline]
    fn map_to_iterator_vector(&self, map_vector: &Self::MapVectorType) -> Self::IteratorVectorType {
        use std::ops::RangeFull;
        core::array::from_fn(|i| match &map_vector[i] {
            MapIndex::Dummy(dummy) => dummy.iterator_of(self, i),
            MapIndex::Map(_) => DummyIndex::Range(Box::new(RangeFull)).iterator_of(self, i),
        })
    }
}

// ============================================================================
// 编译期形状类型别名
// Compile-time shape type aliases
// ============================================================================

/// 0 维形状
/// 0-dimensional shape
pub type Shape0<SO = StorageOrder> = Shape<0, SO>;
/// 1 维形状
/// 1-dimensional shape
pub type Shape1<SO = StorageOrder> = Shape<1, SO>;
/// 2 维形状
/// 2-dimensional shape
pub type Shape2<SO = StorageOrder> = Shape<2, SO>;
/// 3 维形状
/// 3-dimensional shape
pub type Shape3<SO = StorageOrder> = Shape<3, SO>;
/// 4 维形状
/// 4-dimensional shape
pub type Shape4<SO = StorageOrder> = Shape<4, SO>;
/// 5 维形状
/// 5-dimensional shape
pub type Shape5<SO = StorageOrder> = Shape<5, SO>;
/// 6 维形状
/// 6-dimensional shape
pub type Shape6<SO = StorageOrder> = Shape<6, SO>;
/// 7 维形状
/// 7-dimensional shape
pub type Shape7<SO = StorageOrder> = Shape<7, SO>;
/// 8 维形状
/// 8-dimensional shape
pub type Shape8<SO = StorageOrder> = Shape<8, SO>;
/// 9 维形状
/// 9-dimensional shape
pub type Shape9<SO = StorageOrder> = Shape<9, SO>;
/// 10 维形状
/// 10-dimensional shape
pub type Shape10<SO = StorageOrder> = Shape<10, SO>;
/// 11 维形状
/// 11-dimensional shape
pub type Shape11<SO = StorageOrder> = Shape<11, SO>;
/// 12 维形状
/// 12-dimensional shape
pub type Shape12<SO = StorageOrder> = Shape<12, SO>;
/// 13 维形状
/// 13-dimensional shape
pub type Shape13<SO = StorageOrder> = Shape<13, SO>;
/// 14 维形状
/// 14-dimensional shape
pub type Shape14<SO = StorageOrder> = Shape<14, SO>;
/// 15 维形状
/// 15-dimensional shape
pub type Shape15<SO = StorageOrder> = Shape<15, SO>;
/// 16 维形状
/// 16-dimensional shape
pub type Shape16<SO = StorageOrder> = Shape<16, SO>;
/// 17 维形状
/// 17-dimensional shape
pub type Shape17<SO = StorageOrder> = Shape<17, SO>;
/// 18 维形状
/// 18-dimensional shape
pub type Shape18<SO = StorageOrder> = Shape<18, SO>;
/// 19 维形状
/// 19-dimensional shape
pub type Shape19<SO = StorageOrder> = Shape<19, SO>;
/// 20 维形状
/// 20-dimensional shape
pub type Shape20<SO = StorageOrder> = Shape<20, SO>;

/// 动态形状结构体
/// Dynamic shape struct
///
/// 维度在运行时确定的形状。
/// Shape with dimension determined at runtime.
pub struct DynShape<C: DynShapeContainer = Vec, SO: StorageOrderTrait = StorageOrder> {
    /// 形状向量
    /// Shape vector
    pub(crate) shape: <C as DynShapeContainer>::Type<usize>,

    /// 偏移量和长度的延迟初始化缓存
    /// Lazy-initialized cache for offsets and length
    pub(crate) offsets_and_len: OnceLock<(<C as DynShapeContainer>::Type<usize>, usize)>,

    /// 存储顺序
    /// Storage order
    pub(crate) storage_order: SO,
}

/// 创建动态形状的宏
/// Macro for creating dynamic shapes
#[macro_export]
macro_rules! dyn_shape {
    [$($shape:expr),*] => {
        DynShape::<Vec, StorageOrder>::new(vec![$($shape),*])
    };

    (vec![$($shape:expr),*]) => {
        DynShape::<Vec, StorageOrder>::new(vec![$($shape),*])
    }
}

impl<C: DynShapeContainer, SO: StorageOrderTrait> Display for DynShape<C, SO>
where
    <C as DynShapeContainer>::Type<usize>: Display,
{
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.shape)
    }
}

impl<C: DynShapeContainer, SO: StorageOrderTrait> Debug for DynShape<C, SO> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("DynShape")
            .field("shape", &self.shape)
            .field("offsets", self.offsets())
            .field("len", &self.len())
            .field("storage_order", &self.storage_order().runtime_value())
            .finish()
    }
}

impl<C: DynShapeContainer, SO: StorageOrderTrait + Clone> Clone for DynShape<C, SO> {
    fn clone(&self) -> Self {
        Self {
            shape: self.shape.indices().map(|i| self.shape[i]).collect(),
            offsets_and_len: OnceLock::new(),
            storage_order: self.storage_order.clone(),
        }
    }
}

impl<C: DynShapeContainer, SO: StorageOrderTrait> Index<usize> for DynShape<C, SO> {
    type Output = usize;

    fn index(&self, index: usize) -> &Self::Output {
        &self.shape[index]
    }
}

impl<C: DynShapeContainer, SO: StorageOrderTrait> DynShape<C, SO> {
    /// 创建新动态形状
    /// Create a new dynamic shape
    pub fn new(shape: <C as DynShapeContainer>::Type<usize>) -> Self
    where
        SO: Default,
    {
        Self::new_with_order(shape, SO::default())
    }

    /// 使用指定存储顺序创建动态形状
    /// Create dynamic shape with specified storage order
    pub fn new_with_order(shape: <C as DynShapeContainer>::Type<usize>, storage_order: SO) -> Self {
        Self {
            shape,
            offsets_and_len: OnceLock::new(),
            storage_order,
        }
    }

    /// 获取或初始化偏移量和长度
    /// Get or initialize offsets and length
    fn get_or_init_offsets_and_len(&self) -> &(<C as DynShapeContainer>::Type<usize>, usize) {
        self.offsets_and_len
            .get_or_init(|| self.storage_order.dyn_offsets(&self.shape))
    }
}

impl<C: DynShapeContainer, SO: StorageOrderTrait> AbstractShape for DynShape<C, SO> {
    const DIMENSION: usize = DYN_DIMENSION;
    type StorageOrder = SO;
    type VectorType = <C as DynShapeContainer>::Type<usize>;
    type DummyVectorType = <C as DynShapeContainer>::Type<DummyIndex>;
    type MapVectorType = <C as DynShapeContainer>::Type<MapIndex>;
    type ShapeVectorType = <C as DynShapeContainer>::Type<usize>;
    type IteratorVectorType = <C as DynShapeContainer>::Type<DummyIndexIterator>;
    type ShapeWithStorageOrder<NewSO: StorageOrderTrait> = DynShape<C, NewSO>;

    #[inline]
    fn zero(&self) -> Self::VectorType {
        (0..self.shape.len()).map(|_| 0).collect()
    }

    #[inline]
    fn len(&self) -> usize {
        self.get_or_init_offsets_and_len().1
    }

    #[inline]
    fn dimension(&self) -> usize {
        self.shape.len()
    }

    #[inline]
    fn dimension_of(vector: &Self::VectorType) -> usize {
        vector.len()
    }

    #[inline]
    fn shape(&self) -> &impl Vector {
        &self.shape
    }

    #[inline]
    fn offsets(&self) -> &impl Vector {
        &self.get_or_init_offsets_and_len().0
    }

    #[inline]
    fn storage_order(&self) -> StorageOrder {
        self.storage_order.runtime_value()
    }

    #[inline]
    fn from_shape_vector(shape: &Self::ShapeVectorType) -> Self {
        let shape_vec: <C as DynShapeContainer>::Type<usize> =
            shape.indices().map(|i| shape[i]).collect();
        Self::new(shape_vec)
    }

    #[inline]
    fn from_shape_vector_with_order<NewSO: StorageOrderTrait>(
        shape: &Self::ShapeVectorType,
        order: NewSO,
    ) -> DynShape<C, NewSO> {
        let shape_vec: <C as DynShapeContainer>::Type<usize> =
            shape.indices().map(|i| shape[i]).collect();
        DynShape::<C, NewSO>::new_with_order(shape_vec, order)
    }

    #[inline]
    fn with_storage_order<NewSO: StorageOrderTrait>(&self, order: NewSO) -> DynShape<C, NewSO> {
        DynShape {
            shape: self.shape.indices().map(|i| self.shape[i]).collect(),
            offsets_and_len: OnceLock::new(),
            storage_order: order,
        }
    }

    #[inline]
    fn len_of_dimension(&self, dimension: usize) -> Result<usize, DimensionMismatchingError> {
        if dimension >= self.shape.len() {
            Err(error! {
                DimensionMismatchingError {
                    dimension: self.shape.len(),
                    vector_dimension: dimension
                }
            })
        } else {
            Ok(self.shape[dimension])
        }
    }

    #[inline]
    fn offset_of_dimension(&self, dimension: usize) -> Result<usize, DimensionMismatchingError> {
        let offsets: &_ = &self.get_or_init_offsets_and_len().0;
        if dimension >= offsets.len() {
            Err(error! {
                DimensionMismatchingError {
                    dimension: offsets.len(),
                    vector_dimension: dimension
                }
            })
        } else {
            Ok(offsets[dimension])
        }
    }

    #[inline]
    fn dummy_to_map_vector(dummy: &Self::DummyVectorType) -> Self::MapVectorType {
        dummy
            .indices()
            .map(|i| MapIndex::Dummy(dummy[i].clone()))
            .collect()
    }

    #[inline]
    fn map_to_dummy_vector(map_vector: &Self::MapVectorType) -> Self::DummyVectorType {
        map_vector
            .indices()
            .map(|i| match &map_vector[i] {
                MapIndex::Dummy(dummy) => dummy.clone(),
                MapIndex::Map(_) => DummyIndex::Range(Box::new(RangeFull)),
            })
            .collect()
    }

    #[inline]
    fn dummy_to_iterator_vector(&self, dummy: &Self::DummyVectorType) -> Self::IteratorVectorType {
        dummy
            .indices()
            .map(|i| dummy[i].iterator_of(self, i))
            .collect()
    }

    #[inline]
    fn map_to_iterator_vector(&self, map_vector: &Self::MapVectorType) -> Self::IteratorVectorType {
        use std::ops::RangeFull;
        map_vector
            .indices()
            .map(|i| match &map_vector[i] {
                MapIndex::Dummy(dummy) => dummy.iterator_of(self, i),
                MapIndex::Map(_) => DummyIndex::Range(Box::new(RangeFull)).iterator_of(self, i),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shape_creation() {
        let shape1: Shape<1> = Shape::new([5]);
        assert_array_eq!(shape1.shape(), [5]);
        assert_array_eq!(shape1.offsets(), [1]);
        assert_eq!(shape1.len(), 5);
        assert_eq!(shape1.dimension(), 1);

        let shape2: Shape<2> = Shape::new([3, 4]);
        assert_array_eq!(shape2.shape(), [3, 4]);
        assert_array_eq!(shape2.offsets(), [4, 1]);
        assert_eq!(shape2.len(), 12);
        assert_eq!(shape2.dimension(), 2);

        let shape3: Shape<3> = Shape::new([2, 3, 4]);
        assert_array_eq!(shape3.shape(), [2, 3, 4]);
        assert_array_eq!(shape3.offsets(), [12, 4, 1]);
        assert_eq!(shape3.len(), 24);
        assert_eq!(shape3.dimension(), 3);
    }

    #[test]
    fn test_shape1_new_with() {
        let shape: Shape1 = Shape1::new_with(10);
        assert_array_eq!(shape.shape(), [10]);
        assert_eq!(shape.len(), 10);
    }

    #[test]
    fn test_compile_time_shape_row_major() {
        let shape: Shape2<RowMajor> = Shape2::new([3, 4]);
        assert_array_eq!(shape.shape(), [3, 4]);
        assert_array_eq!(shape.offsets(), [4, 1]);
        assert_eq!(shape.len(), 12);
        assert_eq!(shape.storage_order(), StorageOrder::RowMajor);
    }

    #[test]
    fn test_compile_time_shape_column_major() {
        let shape: Shape2<ColumnMajor> = Shape2::new([3, 4]);
        assert_array_eq!(shape.shape(), [3, 4]);
        assert_array_eq!(shape.offsets(), [1, 3]);
        assert_eq!(shape.len(), 12);
        assert_eq!(shape.storage_order(), StorageOrder::ColumnMajor);
    }

    #[test]
    fn test_dyn_shape_creation() {
        let dyn_shape1 = dyn_shape![5];
        assert_array_eq!(dyn_shape1.shape(), [5]);
        assert_array_eq!(dyn_shape1.offsets(), [1]);
        assert_eq!(dyn_shape1.len(), 5);
        assert_eq!(dyn_shape1.dimension(), 1);

        let dyn_shape2 = dyn_shape![3, 4];
        assert_array_eq!(dyn_shape2.shape(), [3, 4]);
        assert_array_eq!(dyn_shape2.offsets(), [4, 1]);
        assert_eq!(dyn_shape2.len(), 12);
        assert_eq!(dyn_shape2.dimension(), 2);

        let dyn_shape3 = dyn_shape![2, 3, 4];
        assert_array_eq!(dyn_shape3.shape(), [2, 3, 4]);
        assert_array_eq!(dyn_shape3.offsets(), [12, 4, 1]);
        assert_eq!(dyn_shape3.len(), 24);
        assert_eq!(dyn_shape3.dimension(), 3);
    }

    #[test]
    fn test_dyn_shape_row_major() {
        let shape: DynShape<Vec, RowMajor> = DynShape::new(vec![3, 4]);
        assert_array_eq!(shape.shape(), [3, 4]);
        assert_array_eq!(shape.offsets(), [4, 1]);
        assert_eq!(shape.len(), 12);
        assert_eq!(shape.storage_order(), StorageOrder::RowMajor);
    }

    #[test]
    fn test_dyn_shape_column_major() {
        let shape: DynShape<Vec, ColumnMajor> = DynShape::new(vec![3, 4]);
        assert_array_eq!(shape.shape(), [3, 4]);
        assert_array_eq!(shape.offsets(), [1, 3]);
        assert_eq!(shape.len(), 12);
        assert_eq!(shape.storage_order(), StorageOrder::ColumnMajor);
    }

    #[test]
    fn test_index_calculation() {
        let shape: Shape<2> = Shape::new([3, 4]);

        assert_eq!(shape.index_of(&[0, 0]).unwrap(), 0);
        assert_eq!(shape.index_of(&[0, 1]).unwrap(), 1);
        assert_eq!(shape.index_of(&[0, 2]).unwrap(), 2);
        assert_eq!(shape.index_of(&[0, 3]).unwrap(), 3);
        assert_eq!(shape.index_of(&[1, 0]).unwrap(), 4);
        assert_eq!(shape.index_of(&[1, 1]).unwrap(), 5);
        assert_eq!(shape.index_of(&[2, 3]).unwrap(), 11);

        let shape3d: Shape<3> = Shape::new([2, 3, 4]);
        assert_eq!(shape3d.index_of(&[0, 0, 0]).unwrap(), 0);
        assert_eq!(shape3d.index_of(&[0, 0, 1]).unwrap(), 1);
        assert_eq!(shape3d.index_of(&[0, 1, 0]).unwrap(), 4);
        assert_eq!(shape3d.index_of(&[1, 2, 3]).unwrap(), 23);
    }

    #[test]
    fn test_vector_calculation() {
        let shape: Shape<2> = Shape::new([3, 4]);

        assert_eq!(shape.vector_of(0).unwrap(), [0, 0]);
        assert_eq!(shape.vector_of(1).unwrap(), [0, 1]);
        assert_eq!(shape.vector_of(2).unwrap(), [0, 2]);
        assert_eq!(shape.vector_of(3).unwrap(), [0, 3]);
        assert_eq!(shape.vector_of(4).unwrap(), [1, 0]);
        assert_eq!(shape.vector_of(5).unwrap(), [1, 1]);
        assert_eq!(shape.vector_of(11).unwrap(), [2, 3]);

        let shape3d: Shape<3> = Shape::new([2, 3, 4]);
        assert_eq!(shape3d.vector_of(0).unwrap(), [0, 0, 0]);
        assert_eq!(shape3d.vector_of(1).unwrap(), [0, 0, 1]);
        assert_eq!(shape3d.vector_of(4).unwrap(), [0, 1, 0]);
        assert_eq!(shape3d.vector_of(23).unwrap(), [1, 2, 3]);
    }

    #[test]
    fn test_index_vector_inverse() {
        let shape: Shape<3> = Shape::new([3, 4, 5]);

        for i in 0..shape.len() {
            let vector = shape.vector_of(i).unwrap();
            let calculated_index = shape.index_of(&vector).unwrap();
            assert_eq!(
                i, calculated_index,
                "Failed at i={}, vector={:?}",
                i, vector
            );
        }

        let dyn_shape2 = dyn_shape![3, 4, 5];

        for i in 0..dyn_shape2.len() {
            let vector = dyn_shape2.vector_of(i).unwrap();
            let calculated_index = dyn_shape2.index_of(&vector).unwrap();
            assert_eq!(
                i, calculated_index,
                "Failed at i={}, vector={:?}",
                i, vector
            );
        }
    }

    #[test]
    fn test_next_vector() {
        let shape: Shape<2> = Shape::new([2, 3]);

        let mut all_vectors: std::vec::Vec<[usize; 2]> = std::vec::Vec::new();
        let mut v = shape.zero();

        all_vectors.push(v.clone());

        while shape.next_vector(&mut v) {
            all_vectors.push(v.clone());
        }

        assert_eq!(all_vectors.len(), 6);
        assert_eq!(all_vectors[0], [0, 0]);
        assert_eq!(all_vectors[1], [0, 1]);
        assert_eq!(all_vectors[2], [0, 2]);
        assert_eq!(all_vectors[3], [1, 0]);
        assert_eq!(all_vectors[4], [1, 1]);
        assert_eq!(all_vectors[5], [1, 2]);

        let dyn_shape2 = dyn_shape![2, 3];
        let mut dyn_v = dyn_shape2.zero();
        let mut dyn_count = 1;

        while dyn_shape2.next_vector(&mut dyn_v) {
            dyn_count += 1;
        }

        assert_eq!(dyn_count, 6);
    }

    #[test]
    fn test_dimension_mismatch_error() {
        let dyn_shape = dyn_shape![3, 4];

        let wrong_dim_vec_small = vec![0];
        let result = dyn_shape.index_of(&wrong_dim_vec_small);
        assert!(result.is_err());

        if let Err(IndexCalculationError::DimensionMismatching(_)) = result {
        } else {
            panic!("Expected DimensionMismatching error for small vector");
        }

        let wrong_dim_vec_large = vec![0, 0, 0];
        let result = dyn_shape.index_of(&wrong_dim_vec_large);
        assert!(result.is_err());

        if let Err(IndexCalculationError::DimensionMismatching(_)) = result {
        } else {
            panic!("Expected DimensionMismatching error for large vector");
        }
    }

    #[test]
    fn test_out_of_shape_error() {
        let shape: Shape<2> = Shape::new([3, 4]);

        let out_of_bounds_vector = [3, 0];
        let result = shape.index_of(&out_of_bounds_vector);
        assert!(result.is_err());

        if let Err(IndexCalculationError::OutOfShape(_)) = result {
        } else {
            panic!("Expected OutOfShape error");
        }

        let result = shape.actual_index(0, -1);
        assert!(result.is_some());

        let result = shape.actual_index(0, -4);
        assert!(result.is_none());
    }

    #[test]
    fn test_len_of_dimension() {
        let shape = dyn_shape![2, 3, 4];

        assert_eq!(shape.len_of_dimension(0).unwrap(), 2);
        assert_eq!(shape.len_of_dimension(1).unwrap(), 3);
        assert_eq!(shape.len_of_dimension(2).unwrap(), 4);

        let result = shape.len_of_dimension(3);
        assert!(result.is_err());

        let dyn_shape2 = dyn_shape![2, 3, 4];
        assert_eq!(dyn_shape2.len_of_dimension(0).unwrap(), 2);
        assert_eq!(dyn_shape2.len_of_dimension(1).unwrap(), 3);
        assert_eq!(dyn_shape2.len_of_dimension(2).unwrap(), 4);
    }

    #[test]
    fn test_offset_of_dimension() {
        let shape: Shape<3> = Shape::new([2, 3, 4]);

        assert_eq!(shape.offset_of_dimension(0).unwrap(), 12);
        assert_eq!(shape.offset_of_dimension(1).unwrap(), 4);
        assert_eq!(shape.offset_of_dimension(2).unwrap(), 1);

        let dyn_shape = dyn_shape![2, 3, 4];
        assert_eq!(dyn_shape.offset_of_dimension(0).unwrap(), 12);
        assert_eq!(dyn_shape.offset_of_dimension(1).unwrap(), 4);
        assert_eq!(dyn_shape.offset_of_dimension(2).unwrap(), 1);
    }

    #[test]
    fn test_type_aliases() {
        let shape1: Shape<1> = Shape::new([5]);
        assert_eq!(shape1.dimension(), 1);

        let shape2: Shape<2> = Shape::new([3, 4]);
        assert_eq!(shape2.dimension(), 2);

        let shape3: Shape<3> = Shape::new([2, 3, 4]);
        assert_eq!(shape3.dimension(), 3);

        let shape_rm2: Shape<2, RowMajor> = Shape::new([3, 4]);
        assert_eq!(shape_rm2.dimension(), 2);
        assert_eq!(shape_rm2.storage_order(), StorageOrder::RowMajor);

        let shape_cm2: Shape<2, ColumnMajor> = Shape::new([3, 4]);
        assert_eq!(shape_cm2.dimension(), 2);
        assert_eq!(shape_cm2.storage_order(), StorageOrder::ColumnMajor);
    }

    #[test]
    fn test_zero_vector() {
        let shape: Shape<3> = Shape::new([2, 3, 4]);
        let zero = shape.zero();
        assert_eq!(zero, [0, 0, 0]);

        let dyn_shape2 = dyn_shape![2, 3, 4];
        let dyn_zero = dyn_shape2.zero();
        assert_eq!(dyn_zero, vec![0, 0, 0]);
    }

    #[test]
    fn test_actual_index_calculation() {
        let shape: Shape<1> = Shape::new([5]);

        assert_eq!(shape.actual_index(0, 0), Some(0));
        assert_eq!(shape.actual_index(0, 2), Some(2));
        assert_eq!(shape.actual_index(0, 4), Some(4));
        assert_eq!(shape.actual_index(0, 5), None);

        assert_eq!(shape.actual_index(0, -1), Some(4));
        assert_eq!(shape.actual_index(0, -2), Some(3));
        assert_eq!(shape.actual_index(0, -5), Some(0));
        assert_eq!(shape.actual_index(0, -6), None);
    }

    #[test]
    fn test_storage_order_row_major() {
        let shape = Shape::new_with_order([2, 3], StorageOrder::RowMajor);

        assert_eq!(shape.storage_order(), StorageOrder::RowMajor);
        assert_eq!(shape.len(), 6);

        assert_eq!(shape.offset_of_dimension(0).unwrap(), 3);
        assert_eq!(shape.offset_of_dimension(1).unwrap(), 1);

        assert_eq!(shape.index_of(&[0, 0]).unwrap(), 0);
        assert_eq!(shape.index_of(&[0, 1]).unwrap(), 1);
        assert_eq!(shape.index_of(&[0, 2]).unwrap(), 2);
        assert_eq!(shape.index_of(&[1, 0]).unwrap(), 3);
        assert_eq!(shape.index_of(&[1, 1]).unwrap(), 4);
        assert_eq!(shape.index_of(&[1, 2]).unwrap(), 5);
    }

    #[test]
    fn test_storage_order_column_major() {
        let shape = Shape::new_with_order([2, 3], StorageOrder::ColumnMajor);

        assert_eq!(shape.storage_order(), StorageOrder::ColumnMajor);
        assert_eq!(shape.len(), 6);

        assert_eq!(shape.offset_of_dimension(0).unwrap(), 1);
        assert_eq!(shape.offset_of_dimension(1).unwrap(), 2);

        assert_eq!(shape.index_of(&[0, 0]).unwrap(), 0);
        assert_eq!(shape.index_of(&[1, 0]).unwrap(), 1);
        assert_eq!(shape.index_of(&[0, 1]).unwrap(), 2);
        assert_eq!(shape.index_of(&[1, 1]).unwrap(), 3);
        assert_eq!(shape.index_of(&[0, 2]).unwrap(), 4);
        assert_eq!(shape.index_of(&[1, 2]).unwrap(), 5);
    }

    #[test]
    fn test_storage_order_3d_row_major() {
        let shape = Shape::new_with_order([2, 3, 4], StorageOrder::RowMajor);

        assert_eq!(shape.storage_order(), StorageOrder::RowMajor);
        assert_eq!(shape.len(), 24);

        assert_eq!(shape.offset_of_dimension(0).unwrap(), 12);
        assert_eq!(shape.offset_of_dimension(1).unwrap(), 4);
        assert_eq!(shape.offset_of_dimension(2).unwrap(), 1);

        assert_eq!(shape.index_of(&[0, 0, 0]).unwrap(), 0);
        assert_eq!(shape.index_of(&[0, 0, 1]).unwrap(), 1);
        assert_eq!(shape.index_of(&[0, 1, 0]).unwrap(), 4);
        assert_eq!(shape.index_of(&[1, 0, 0]).unwrap(), 12);
    }

    #[test]
    fn test_storage_order_3d_column_major() {
        let shape = Shape::new_with_order([2, 3, 4], StorageOrder::ColumnMajor);

        assert_eq!(shape.storage_order(), StorageOrder::ColumnMajor);
        assert_eq!(shape.len(), 24);

        assert_eq!(shape.offset_of_dimension(0).unwrap(), 1);
        assert_eq!(shape.offset_of_dimension(1).unwrap(), 2);
        assert_eq!(shape.offset_of_dimension(2).unwrap(), 6);

        assert_eq!(shape.index_of(&[0, 0, 0]).unwrap(), 0);
        assert_eq!(shape.index_of(&[1, 0, 0]).unwrap(), 1);
        assert_eq!(shape.index_of(&[0, 1, 0]).unwrap(), 2);
        assert_eq!(shape.index_of(&[0, 0, 1]).unwrap(), 6);
    }

    #[test]
    fn test_with_storage_order() {
        let shape_row = Shape::new_with_order([2, 3], StorageOrder::RowMajor);
        let shape_col = shape_row.with_storage_order(StorageOrder::ColumnMajor);

        assert_eq!(shape_col.storage_order(), StorageOrder::ColumnMajor);
        assert_eq!(shape_col.len(), 6);
        assert_eq!(shape_col.shape()[0], 2);
        assert_eq!(shape_col.shape()[1], 3);

        assert_eq!(shape_col.offset_of_dimension(0).unwrap(), 1);
        assert_eq!(shape_col.offset_of_dimension(1).unwrap(), 2);
    }

    #[test]
    fn test_dyn_storage_order_row_major() {
        let dyn_shape = DynShape::<Vec, RowMajor>::new(vec![2, 3]);

        assert_eq!(dyn_shape.storage_order(), StorageOrder::RowMajor);
        assert_eq!(dyn_shape.len(), 6);
        assert_eq!(dyn_shape.offset_of_dimension(0).unwrap(), 3);
        assert_eq!(dyn_shape.offset_of_dimension(1).unwrap(), 1);
    }

    #[test]
    fn test_dyn_storage_order_column_major() {
        let dyn_shape = DynShape::<Vec, ColumnMajor>::new(vec![2, 3]);

        assert_eq!(dyn_shape.storage_order(), StorageOrder::ColumnMajor);
        assert_eq!(dyn_shape.len(), 6);
        assert_eq!(dyn_shape.offset_of_dimension(0).unwrap(), 1);
        assert_eq!(dyn_shape.offset_of_dimension(1).unwrap(), 2);
    }

    #[test]
    fn test_dyn_with_storage_order() {
        let dyn_shape_row = DynShape::<Vec, RowMajor>::new(vec![2, 3]);
        let dyn_shape_col = dyn_shape_row.with_storage_order(ColumnMajor);

        assert_eq!(dyn_shape_col.storage_order(), StorageOrder::ColumnMajor);
        assert_eq!(dyn_shape_col.len(), 6);
        assert_eq!(dyn_shape_col.offset_of_dimension(0).unwrap(), 1);
        assert_eq!(dyn_shape_col.offset_of_dimension(1).unwrap(), 2);
    }

    #[test]
    fn test_shape_zero_dimension() {
        let shape: Shape<0> = Shape::new([]);
        assert_eq!(shape.dimension(), 0);
        assert_eq!(shape.len(), 1);
        assert_eq!(shape.shape().len(), 0);
    }

    #[test]
    fn test_shape_large_size() {
        let shape: Shape<2> = Shape::new([1000, 1000]);
        assert_eq!(shape.len(), 1_000_000);
        assert_eq!(shape.dimension(), 2);
        assert_eq!(shape.shape()[0], 1000);
        assert_eq!(shape.shape()[1], 1000);
    }

    #[test]
    fn test_dyn_shape_modify() {
        let mut dyn_shape = DynShape::<Vec, StorageOrder>::new(vec![2, 3, 4]);
        assert_eq!(dyn_shape.len(), 24);
        assert_eq!(dyn_shape.dimension(), 3);

        dyn_shape.shape[0] = 5;
        assert_eq!(dyn_shape.shape()[0], 5);
    }

    #[test]
    fn test_next_vector_complete_traversal() {
        let shape: Shape<3> = Shape::new([2, 3, 2]);
        let mut v = shape.zero();
        let mut count = 0;

        loop {
            count += 1;
            if !shape.next_vector(&mut v) {
                break;
            }
        }

        assert_eq!(count, 12);
    }

    #[test]
    fn test_offset_of_dimension_error() {
        let shape: Shape<2> = Shape::new([3, 4]);
        let result = shape.offset_of_dimension(5);
        assert!(result.is_err());
    }

    #[test]
    fn test_actual_index_edge_cases() {
        let shape: Shape<2> = Shape::new([3, 4]);

        assert_eq!(shape.actual_index(0, 0), Some(0));
        assert_eq!(shape.actual_index(0, -1), Some(2));
        assert_eq!(shape.actual_index(0, -3), Some(0));
        assert_eq!(shape.actual_index(0, -4), None);
        assert_eq!(shape.actual_index(1, -5), None);

        assert_eq!(shape.actual_index(1, 0), Some(0));
        assert_eq!(shape.actual_index(1, 3), Some(3));
        assert_eq!(shape.actual_index(1, 4), None);
    }

    #[test]
    fn test_shape_clone() {
        let shape1: Shape<2, RowMajor> = Shape::new([3, 4]);
        let shape2 = shape1.clone();

        assert_eq!(shape1.shape()[0], shape2.shape()[0]);
        assert_eq!(shape1.shape()[1], shape2.shape()[1]);
        assert_eq!(shape1.len(), shape2.len());
    }

    #[test]
    fn test_dyn_shape_clone() {
        let dyn_shape1 = dyn_shape![2, 3, 4];
        let dyn_shape2 = dyn_shape1.clone();

        assert_eq!(dyn_shape1.shape()[0], dyn_shape2.shape()[0]);
        assert_eq!(dyn_shape1.len(), dyn_shape2.len());
    }

    #[test]
    fn test_shape_debug_display() {
        let shape: Shape<2> = Shape::new([3, 4]);
        let debug_str = format!("{:?}", shape);
        assert!(debug_str.contains("Shape"));
        assert!(debug_str.contains("[3, 4]"));

        let display_str = format!("{}", shape);
        assert!(display_str.contains("[3, 4]"));
    }
}