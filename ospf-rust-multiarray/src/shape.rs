//! # Shape - 运行时形状类型定义
//!
//! ## Overview / 概述
//!
//! This module provides the `Shape` and `DynShape` types for representing
//! multi-dimensional array shapes at runtime. It defines the `AbstractShape`
//! and `AbstractRTShape` traits that abstract over shape operations.
//!
//! 本模块提供 `Shape` 和 `DynShape` 类型，用于表示运行时多维数组形状。
//! 它定义了 `AbstractShape` 和 `AbstractRTShape` 特征，抽象形状操作。
//!
//! ## Key Types / 主要类型
//!
//! - `Shape<const D: usize>` - Compile-time fixed dimension shape / 编译时固定维度形状
//! - `DynShape` - Runtime dynamic dimension shape / 运行时动态维度形状
//!
//! ## Key Traits / 主要特征
//!
//! - `AbstractShape` - Abstract shape operations / 抽象形状操作
//! - `AbstractRTShape` - Runtime shape operations with storage order / 带存储顺序的运行时形状操作

use super::concept::*;
use super::dummy_index::{DummyIndex, DummyIndexIterator, IteratorVector};
use super::error::{DimensionMismatchingError, IndexCalculationError, OutOfShapeError};
use super::map_index::MapIndex;
use ospf_rust_base::Indices;
use ospf_rust_base::error::*;
use std::fmt::{Debug, Display, Formatter};
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::mem;
use std::ops::{Index, IndexMut, RangeFull};
use std::result::Result;
use std::sync::OnceLock;

/// # AbstractShape Trait
///
/// A trait that defines the abstract interface for multi-dimensional array shapes.
/// It provides methods for converting between multi-dimensional indices (vectors)
/// and linear indices, as well as accessing shape properties.
///
/// 定义多维数组形状抽象接口的特征。
/// 它提供了在多维索引（向量）和线性索引之间转换的方法，以及访问形状属性的方法。
///
/// ## Associated Types / 关联类型
///
/// - `VectorType` - The type used for multi-dimensional indices / 用于多维索引的类型
/// - `DummyVectorType` - The type used for dummy (slicing) indices / 用于虚拟（切片）索引的类型
/// - `MapVectorType` - The type used for map (reordering) indices / 用于映射（重排）索引的类型
/// - `ShapeVectorType` - The type used for shape vectors / 用于形状向量的类型
///
/// ## Implementors / 实现者
///
/// - `Shape<const D: usize>` - Fixed dimension shape / 固定维度形状
/// - `DynShape` - Dynamic dimension shape / 动态维度形状
pub trait AbstractShape {
    /// The compile-time dimension of this shape.
    /// For dynamic shapes, this is `DYN_DIMENSION`.
    ///
    /// 此形状的编译时维度。
    /// 对于动态形状，这是 `DYN_DIMENSION`。
    const DIMENSION: usize;

    /// The type used for multi-dimensional indices.
    ///
    /// 用于多维索引的类型。
    type VectorType: Vector + Clone;

    /// The type used for dummy (slicing) indices.
    ///
    /// 用于虚拟（切片）索引的类型。
    type DummyVectorType: DummyVector + Clone;

    /// The type used for map (reordering) indices.
    ///
    /// 用于映射（重排）索引的类型。
    type MapVectorType: MapVector + Clone;

    /// The type used for shape vectors.
    ///
    /// 用于形状向量的类型。
    type ShapeVectorType: ShapeVector;

    /// The type used for iterator vectors.
    /// Used for storing DummyIndexIterator for each dimension.
    ///
    /// 用于迭代器向量的类型。
    /// 用于存储每个维度的 DummyIndexIterator。
    type IteratorVectorType: IteratorVector + Clone;

    /// Create a zero-initialized vector.
    ///
    /// 创建零初始化的向量。
    ///
    /// # Returns / 返回值
    ///
    /// A vector with all elements set to zero / 所有元素设置为零的向量
    fn zero(&self) -> Self::VectorType;

    /// Get the total number of elements in this shape.
    ///
    /// 获取此形状中的元素总数。
    ///
    /// # Returns / 返回值
    ///
    /// The product of all dimension sizes / 所有维度大小的乘积
    fn len(&self) -> usize;

    /// Get the number of dimensions.
    ///
    /// 获取维度数量。
    ///
    /// # Returns / 返回值
    ///
    /// The dimension count / 维度数量
    fn dimension(&self) -> usize {
        Self::DIMENSION
    }

    /// Get the dimension of a vector.
    ///
    /// 获取向量的维度。
    ///
    /// # Parameters / 参数
    ///
    /// - `_` - The vector to check / 要检查的向量
    ///
    /// # Returns / 返回值
    ///
    /// The dimension of the vector / 向量的维度
    fn dimension_of(_: &Self::VectorType) -> usize {
        Self::DIMENSION
    }

    /// Get the shape vector (dimension sizes).
    ///
    /// 获取形状向量（维度大小）。
    ///
    /// # Returns / 返回值
    ///
    /// A reference to the shape vector / 形状向量的引用
    fn shape(&self) -> &impl Vector;

    /// Get the offsets vector (strides for each dimension).
    ///
    /// 获取偏移向量（每个维度的步幅）。
    ///
    /// # Returns / 返回值
    ///
    /// A reference to the offsets vector / 偏移向量的引用
    fn offsets(&self) -> &impl Vector;

    /// Get the storage order of this shape.
    ///
    /// 获取此形状的存储顺序。
    ///
    /// # Returns / 返回值
    ///
    /// The storage order (RowMajor or ColumnMajor) / 存储顺序（行优先或列优先）
    fn storage_order(&self) -> StorageOrder;

    /// Create a shape from a shape vector.
    ///
    /// 从形状向量创建形状。
    ///
    /// # Parameters / 参数
    ///
    /// - `shape` - The shape vector / 形状向量
    ///
    /// # Returns / 返回值
    ///
    /// A new shape instance / 新的形状实例
    fn from_shape_vector(shape: &Self::ShapeVectorType) -> Self
    where
        Self: Sized;

    /// Get the size of a specific dimension.
    ///
    /// 获取特定维度的大小。
    ///
    /// # Parameters / 参数
    ///
    /// - `dimension` - The dimension index / 维度索引
    ///
    /// # Returns / 返回值
    ///
    /// - `Ok(usize)` - The size of the dimension / 维度的大小
    /// - `Err(DimensionMismatchingError)` - If the dimension is out of bounds / 如果维度越界
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

    /// Get the offset (stride) of a specific dimension.
    ///
    /// 获取特定维度的偏移量（步幅）。
    ///
    /// # Parameters / 参数
    ///
    /// - `dimension` - The dimension index / 维度索引
    ///
    /// # Returns / 返回值
    ///
    /// - `Ok(usize)` - The offset of the dimension / 维度的偏移量
    /// - `Err(DimensionMismatchingError)` - If the dimension is out of bounds / 如果维度越界
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

    /// Convert a multi-dimensional vector to a linear index.
    ///
    /// 将多维向量转换为线性索引。
    ///
    /// # Parameters / 参数
    ///
    /// - `vector` - The multi-dimensional index / 多维索引
    ///
    /// # Returns / 返回值
    ///
    /// - `Ok(usize)` - The linear index / 线性索引
    /// - `Err(IndexCalculationError)` - If the conversion fails / 如果转换失败
    ///
    /// # Errors / 错误
    ///
    /// - `DimensionMismatching` - If the vector dimension doesn't match / 如果向量维度不匹配
    /// - `OutOfShape` - If any index is out of bounds / 如果任何索引越界
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

    /// Convert a linear index to a multi-dimensional vector.
    ///
    /// 将线性索引转换为多维向量。
    ///
    /// # Parameters / 参数
    ///
    /// - `index` - The linear index / 线性索引
    ///
    /// # Returns / 返回值
    ///
    /// - `Ok(VectorType)` - The multi-dimensional vector / 多维向量
    /// - `Err(IndexCalculationError)` - If the conversion fails / 如果转换失败
    fn vector_of(&self, mut index: usize) -> Result<Self::VectorType, IndexCalculationError> {
        let mut vector = self.zero();
        for i in 0..self.dimension() {
            let offset = self.offset_of_dimension(i)?;
            vector[i] = index / offset;
            index = index % offset;
        }
        Ok(vector)
    }

    /// Advance a vector to the next position in lexicographic order.
    ///
    /// 将向量推进到字典序的下一个位置。
    ///
    /// # Parameters / 参数
    ///
    /// - `vector` - The vector to advance (modified in place) / 要推进的向量（原地修改）
    ///
    /// # Returns / 返回值
    ///
    /// - `true` - If the vector was successfully advanced / 如果向量成功推进
    /// - `false` - If the vector has wrapped around to zero / 如果向量已回绕到零
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

    /// Convert a possibly negative index to an actual positive index.
    ///
    /// 将可能为负的索引转换为实际的正索引。
    ///
    /// # Parameters / 参数
    ///
    /// - `dimension` - The dimension to check / 要检查的维度
    /// - `index` - The possibly negative index / 可能为负的索引
    ///
    /// # Returns / 返回值
    ///
    /// - `Some(usize)` - The actual positive index / 实际的正索引
    /// - `None` - If the index is out of bounds / 如果索引越界
    ///
    /// # Note / 注意
    ///
    /// Negative indices count from the end: -1 is the last element.
    /// 负索引从末尾计数：-1 是最后一个元素。
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

    /// Convert a dummy vector to a map vector.
    /// Each dummy index is wrapped in `MapIndex::Dummy`.
    ///
    /// 将虚拟向量转换为映射向量。
    /// 每个虚拟索引被包装为 `MapIndex::Dummy`。
    ///
    /// # Parameters / 参数
    ///
    /// - `dummy` - The dummy vector to convert / 要转换的虚拟向量
    ///
    /// # Returns / 返回值
    ///
    /// A map vector with each element wrapped in `MapIndex::Dummy` / 每个元素包装为 `MapIndex::Dummy` 的映射向量
    fn dummy_to_map_vector(dummy: &Self::DummyVectorType) -> Self::MapVectorType;

    fn map_to_dummy_vector(map_vector: &Self::MapVectorType) -> Self::DummyVectorType;

    /// Convert a dummy vector to an iterator vector.
    /// Each dummy index is converted to its corresponding iterator.
    ///
    /// 将虚拟向量转换为迭代器向量。
    /// 每个虚拟索引被转换为其对应的迭代器。
    ///
    /// # Parameters / 参数
    ///
    /// - `dummy` - The dummy vector to convert / 要转换的虚拟向量
    ///
    /// # Returns / 返回值
    ///
    /// An iterator vector with each element converted to DummyIndexIterator / 每个元素转换为 DummyIndexIterator 的迭代器向量
    fn dummy_to_iterator_vector(&self, dummy: &Self::DummyVectorType) -> Self::IteratorVectorType;

    /// Convert a map vector to an iterator vector.
    /// Each MapIndex::Dummy is converted to its corresponding iterator.
    /// MapIndex::Map is treated as a full range.
    ///
    /// 将映射向量转换为迭代器向量。
    /// 每个 MapIndex::Dummy 被转换为其对应的迭代器。
    /// MapIndex::Map 被视为完整范围。
    ///
    /// # Parameters / 参数
    ///
    /// - `map_vector` - The map vector to convert / 要转换的映射向量
    ///
    /// # Returns / 返回值
    ///
    /// An iterator vector / 迭代器向量
    fn map_to_iterator_vector(&self, map_vector: &Self::MapVectorType) -> Self::IteratorVectorType;
}

/// # AbstractRTShape Trait
///
/// A trait that extends `AbstractShape` with runtime-specific operations,
/// particularly for handling storage order conversions.
///
/// 扩展 `AbstractShape` 的特征，添加运行时特定操作，
/// 特别是处理存储顺序转换。
///
/// ## Supertrait / 父特征
///
/// - `AbstractShape` - Base shape operations / 基础形状操作
pub trait AbstractRTShape: AbstractShape {
    /// Create a shape from a shape vector with a specific storage order.
    ///
    /// 从形状向量和特定存储顺序创建形状。
    ///
    /// # Parameters / 参数
    ///
    /// - `shape` - The shape vector / 形状向量
    /// - `order` - The storage order / 存储顺序
    ///
    /// # Returns / 返回值
    ///
    /// A new shape instance with the specified storage order / 具有指定存储顺序的新形状实例
    fn from_shape_vector_with_order(shape: &Self::ShapeVectorType, order: StorageOrder) -> Self
    where
        Self: Sized;

    /// Create a new shape with a different storage order.
    ///
    /// 创建具有不同存储顺序的新形状。
    ///
    /// # Parameters / 参数
    ///
    /// - `order` - The new storage order / 新的存储顺序
    ///
    /// # Returns / 返回值
    ///
    /// A new shape with the specified storage order / 具有指定存储顺序的新形状
    fn with_storage_order(&self, order: StorageOrder) -> Self
    where
        Self: Sized;
}

/// # Shape - Fixed-Dimension Runtime Shape
///
/// A shape type with a compile-time fixed dimension. The dimension `D` is
/// specified as a const generic parameter.
///
/// 具有编译时固定维度的形状类型。维度 `D` 作为 const 泛型参数指定。
///
/// ## Type Parameters / 类型参数
///
/// - `const D: usize` - The fixed dimension / 固定维度
///
/// ## Fields / 字段
///
/// - `shape` - The size of each dimension / 每个维度的大小
/// - `offsets_and_len` - The strides and total count (lazy initialized) / 步幅和总数（惰性初始化）
/// - `storage_order` - The storage order / 存储顺序
///
/// ## Example / 示例
///
/// ```rust
/// use ospf_rust_multiarray::*;
///
/// // Create a 2D shape (3x4) / 创建一个 2D 形状 (3x4)
/// let shape = Shape::new([3, 4]);
///
/// // Create with specific storage order / 使用特定存储顺序创建
/// let shape_col = Shape::new_with_order([3, 4], StorageOrder::ColumnMajor);
/// ```
pub struct Shape<const D: usize> {
    /// Dimension sizes / 维度大小
    pub(crate) shape: [usize; D],
    /// Dimension strides (offsets) and total element count - lazy initialized / 维度步幅（偏移量）和元素总数 - 惰性初始化
    pub(crate) offsets_and_len: OnceLock<([usize; D], usize)>,
    /// Storage order / 存储顺序
    pub(crate) storage_order: StorageOrder,
}

impl<const D: usize> Debug for Shape<D> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("Shape")
            .field("shape", &self.shape)
            .field("offsets", self.offsets())
            .field("len", &self.len())
            .field("storage_order", &self.storage_order)
            .finish()
    }
}

impl<const D: usize> Display for Shape<D> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.shape)
    }
}

impl<const D: usize> Clone for Shape<D> {
    fn clone(&self) -> Self {
        Self {
            shape: self.shape,
            offsets_and_len: OnceLock::new(),
            storage_order: self.storage_order,
        }
    }
}

impl<const D: usize> Index<usize> for Shape<D> {
    type Output = usize;

    fn index(&self, index: usize) -> &Self::Output {
        &self.shape[index]
    }
}

impl Shape<1> {
    /// Create a 1-dimensional shape.
    ///
    /// 创建一维形状。
    ///
    /// # Parameters / 参数
    ///
    /// - `shape` - The size of the dimension / 维度的大小
    ///
    /// # Returns / 返回值
    ///
    /// A new Shape<1> / 新的 Shape<1>
    pub fn new_with(shape: usize) -> Self {
        Self::new([shape])
    }
}

impl<const D: usize> Shape<D> {
    /// Create a new shape with default (RowMajor) storage order.
    ///
    /// 使用默认（行优先）存储顺序创建新形状。
    ///
    /// # Parameters / 参数
    ///
    /// - `shape` - The size of each dimension / 每个维度的大小
    ///
    /// # Returns / 返回值
    ///
    /// A new Shape / 新的 Shape
    pub fn new(shape: [usize; D]) -> Self {
        Self::new_with_order(shape, StorageOrder::RowMajor)
    }

    /// Create a new shape with a specific storage order.
    ///
    /// 使用特定存储顺序创建新形状。
    ///
    /// # Parameters / 参数
    ///
    /// - `shape` - The size of each dimension / 每个维度的大小
    /// - `storage_order` - The storage order / 存储顺序
    ///
    /// # Returns / 返回值
    ///
    /// A new Shape / 新的 Shape
    pub fn new_with_order(shape: [usize; D], storage_order: StorageOrder) -> Self {
        Self {
            shape,
            offsets_and_len: OnceLock::new(),
            storage_order,
        }
    }

    /// Get or compute the offsets and len.
    ///
    /// 获取或计算偏移量和长度。
    fn get_or_init_offsets_and_len(&self) -> &([usize; D], usize) {
        self.offsets_and_len
            .get_or_init(|| self.storage_order.offsets(&self.shape))
    }
}

impl<const D: usize> AbstractShape for Shape<D> {
    const DIMENSION: usize = D;
    type VectorType = [usize; D];
    type DummyVectorType = [DummyIndex; D];
    type MapVectorType = [MapIndex; D];
    type ShapeVectorType = [usize; D];
    type IteratorVectorType = [DummyIndexIterator; D];

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
        self.storage_order
    }

    #[inline]
    fn from_shape_vector(shape: &Self::ShapeVectorType) -> Self {
        Self::new(*shape)
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

impl<const D: usize> AbstractRTShape for Shape<D> {
    fn from_shape_vector_with_order(shape: &Self::ShapeVectorType, order: StorageOrder) -> Self {
        Self::new_with_order(*shape, order)
    }

    fn with_storage_order(&self, order: StorageOrder) -> Self {
        if self.storage_order == order {
            return self.clone();
        }

        // 创建新形状，偏移量和长度将惰性计算 / Create new shape, offsets and len will be lazily computed
        Self {
            shape: self.shape,
            offsets_and_len: OnceLock::new(),
            storage_order: order,
        }
    }
}

/// Type aliases for common fixed dimensions.
/// 常见固定维度的类型别名。
pub type Shape0 = Shape<0>;
pub type Shape1 = Shape<1>;
pub type Shape2 = Shape<2>;
pub type Shape3 = Shape<3>;
pub type Shape4 = Shape<4>;
pub type Shape5 = Shape<5>;
pub type Shape6 = Shape<6>;
pub type Shape7 = Shape<7>;
pub type Shape8 = Shape<8>;
pub type Shape9 = Shape<9>;
pub type Shape10 = Shape<10>;
pub type Shape11 = Shape<11>;
pub type Shape12 = Shape<12>;
pub type Shape13 = Shape<13>;
pub type Shape14 = Shape<14>;
pub type Shape15 = Shape<15>;
pub type Shape16 = Shape<16>;
pub type Shape17 = Shape<17>;
pub type Shape18 = Shape<18>;
pub type Shape19 = Shape<19>;
pub type Shape20 = Shape<20>;

/// # DynShape - Dynamic-Dimension Runtime Shape
///
/// A shape type with runtime-determined dimension. The dimension can vary
/// and is not known at compile time.
///
/// 具有运行时确定维度的形状类型。维度可以变化，在编译时未知。
///
/// ## Type Parameters / 类型参数
///
/// - `V: DynShapeVector` - The vector type for shape and offsets (default: `Vec<usize>`) / 形状和偏移的向量类型（默认：`Vec<usize>`）
/// - `DV: DummyVector` - The dummy vector type (default: `Vec<DummyIndex>`) / 虚拟向量类型（默认：`Vec<DummyIndex>`）
///
/// ## Fields / 字段
///
/// - `shape` - The size of each dimension / 每个维度的大小
/// - `offsets_and_len` - The strides and total count (lazy initialized) / 步幅和总数（惰性初始化）
/// - `storage_order` - The storage order / 存储顺序
/// - `_marker` - Phantom marker for type safety / 类型安全的虚拟特征
///
/// ## Example / 示例
///
/// ```rust
/// use ospf_rust_multiarray::*;
///
/// // Create using constructor / 使用构造函数创建
/// let shape = DynShape::<Vec<usize>, Vec<DummyIndex>>::new(vec![2, 3, 4]);
///
/// // Create using macro / 使用宏创建
/// let shape = dyn_shape![2, 3, 4];
/// ```
pub struct DynShape<V: DynShapeVector = Vec<usize>, DV: DummyVector = Vec<DummyIndex>> {
    /// Dimension sizes / 维度大小
    pub(crate) shape: V,
    /// Dimension strides (offsets) and total element count - lazy initialized / 维度步幅（偏移量）和元素总数 - 惰性初始化
    pub(crate) offsets_and_len: OnceLock<(V, usize)>,
    /// Storage order / 存储顺序
    pub(crate) storage_order: StorageOrder,
    /// Phantom marker for type parameter DV / 类型参数 DV 的虚拟特征
    pub(crate) _marker: PhantomData<DV>,
}

/// # dyn_shape! Macro
///
/// A convenience macro for creating `DynShape` instances.
///
/// 用于创建 `DynShape` 实例的便捷宏。
///
/// ## Usage / 用法
///
/// ```rust
/// use ospf_rust_multiarray::*;
///
/// // Create a 2D shape / 创建一个 2D 形状
/// let shape = dyn_shape![3, 4];
///
/// // Create a 3D shape / 创建一个 3D 形状
/// let shape = dyn_shape![2, 3, 4];
/// ```
#[macro_export]
macro_rules! dyn_shape {
    [$($shape:expr),*] => {
        DynShape::<Vec<usize>, Vec<DummyIndex>>::new(vec![$($shape),*])
    };

    (vec![$($shape:expr),*]) => {
        DynShape::<Vec<usize>, Vec<DummyIndex>>::new(vec![$($shape),*])
    }
}

impl<V: DynShapeVector + Display, DV: DummyVector> Display for DynShape<V, DV> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.shape)
    }
}

impl<V: DynShapeVector, DV: DummyVector> Debug for DynShape<V, DV> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("DynShape")
            .field("shape", &self.shape)
            .field("offsets", self.offsets())
            .field("len", &self.len())
            .field("storage_order", &self.storage_order)
            .finish()
    }
}

impl<V: DynShapeVector, DV: DummyVector> Clone for DynShape<V, DV> {
    fn clone(&self) -> Self {
        Self {
            shape: self.shape.indices().map(|i| self.shape[i]).collect(),
            offsets_and_len: OnceLock::new(),
            storage_order: self.storage_order,
            _marker: PhantomData,
        }
    }
}

impl<V: DynShapeVector, DV: DummyVector> Index<usize> for DynShape<V, DV> {
    type Output = usize;

    fn index(&self, index: usize) -> &Self::Output {
        &self.shape[index]
    }
}

impl<V: DynShapeVector, DV: DummyVector> DynShape<V, DV> {
    /// Create a new DynShape with default (RowMajor) storage order.
    ///
    /// 使用默认（行优先）存储顺序创建新的 DynShape。
    ///
    /// # Parameters / 参数
    ///
    /// - `shape` - The size of each dimension / 每个维度的大小
    ///
    /// # Returns / 返回值
    ///
    /// A new DynShape / 新的 DynShape
    pub fn new(shape: V) -> Self {
        Self::new_with_order(shape, StorageOrder::RowMajor)
    }

    /// Create a new DynShape with a specific storage order.
    ///
    /// 使用特定存储顺序创建新的 DynShape。
    ///
    /// # Parameters / 参数
    ///
    /// - `shape` - The size of each dimension / 每个维度的大小
    /// - `storage_order` - The storage order / 存储顺序
    ///
    /// # Returns / 返回值
    ///
    /// A new DynShape / 新的 DynShape
    pub fn new_with_order(shape: V, storage_order: StorageOrder) -> Self {
        Self {
            shape,
            offsets_and_len: OnceLock::new(),
            storage_order,
            _marker: PhantomData,
        }
    }

    /// Get or compute the offsets and len.
    ///
    /// 获取或计算偏移量和长度。
    fn get_or_init_offsets_and_len(&self) -> &(V, usize) {
        self.offsets_and_len
            .get_or_init(|| self.storage_order.dyn_offsets(&self.shape))
    }
}

impl<V: DynShapeVector, DV: DummyVector> AbstractShape for DynShape<V, DV> {
    const DIMENSION: usize = DYN_DIMENSION;
    type VectorType = Vec<usize>;
    type DummyVectorType = Vec<DummyIndex>;
    type MapVectorType = Vec<MapIndex>;
    type ShapeVectorType = Vec<usize>;
    type IteratorVectorType = Vec<DummyIndexIterator>;

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
        self.storage_order
    }

    #[inline]
    fn from_shape_vector(shape: &Self::ShapeVectorType) -> Self {
        let shape_vec: V = shape.iter().cloned().collect();
        Self::new(shape_vec)
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
        let offsets = &self.get_or_init_offsets_and_len().0;
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
        dummy.iter().map(|d| MapIndex::Dummy(d.clone())).collect()
    }

    #[inline]
    fn map_to_dummy_vector(map_vector: &Self::MapVectorType) -> Self::DummyVectorType {
        map_vector
            .iter()
            .map(|d| match d {
                MapIndex::Dummy(dummy) => dummy.clone(),
                MapIndex::Map(_) => DummyIndex::Range(Box::new(RangeFull)),
            })
            .collect()
    }

    #[inline]
    fn dummy_to_iterator_vector(&self, dummy: &Self::DummyVectorType) -> Self::IteratorVectorType {
        dummy
            .iter()
            .enumerate()
            .map(|(i, d)| d.iterator_of(self, i))
            .collect()
    }

    #[inline]
    fn map_to_iterator_vector(&self, map_vector: &Self::MapVectorType) -> Self::IteratorVectorType {
        use std::ops::RangeFull;
        map_vector
            .iter()
            .enumerate()
            .map(|(i, m)| match m {
                MapIndex::Dummy(dummy) => dummy.iterator_of(self, i),
                MapIndex::Map(_) => DummyIndex::Range(Box::new(RangeFull)).iterator_of(self, i),
            })
            .collect()
    }
}

impl<V: DynShapeVector, DV: DummyVector> AbstractRTShape for DynShape<V, DV> {
    fn from_shape_vector_with_order(shape: &Self::ShapeVectorType, order: StorageOrder) -> Self {
        let shape_vec: V = shape.indices().map(|i| shape[i]).collect();
        Self::new_with_order(shape_vec, order)
    }

    fn with_storage_order(&self, order: StorageOrder) -> Self
    where
        Self: Sized,
    {
        if self.storage_order == order {
            return self.clone();
        }

        Self {
            shape: self.shape.indices().map(|i| self.shape[i]).collect(),
            offsets_and_len: OnceLock::new(),
            storage_order: order,
            _marker: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shape_creation() {
        let shape1 = Shape::new([5]);
        assert_array_eq!(shape1.shape(), [5]);
        assert_array_eq!(shape1.shape(), [5]);
        assert_array_eq!(shape1.offsets(), [1]);
        assert_eq!(shape1.len(), 5);
        assert_eq!(shape1.dimension(), 1);

        let shape2 = Shape::new([3, 4]);
        assert_array_eq!(shape2.shape(), [3, 4]);
        assert_array_eq!(shape2.offsets(), [4, 1]);
        assert_eq!(shape2.len(), 12);
        assert_eq!(shape2.dimension(), 2);

        let shape3 = Shape::new([2, 3, 4]);
        assert_array_eq!(shape3.shape(), [2, 3, 4]);
        assert_array_eq!(shape3.offsets(), [12, 4, 1]);
        assert_eq!(shape3.len(), 24);
        assert_eq!(shape3.dimension(), 3);
    }

    #[test]
    fn test_shape1_new_with() {
        let shape = Shape1::new_with(10);
        assert_array_eq!(shape.shape(), [10]);
        assert_eq!(shape.len(), 10);
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
    fn test_index_calculation() {
        let shape = Shape::new([3, 4]);

        assert_eq!(shape.index_of(&[0, 0]).unwrap(), 0);
        assert_eq!(shape.index_of(&[0, 1]).unwrap(), 1);
        assert_eq!(shape.index_of(&[0, 2]).unwrap(), 2);
        assert_eq!(shape.index_of(&[0, 3]).unwrap(), 3);
        assert_eq!(shape.index_of(&[1, 0]).unwrap(), 4);
        assert_eq!(shape.index_of(&[1, 1]).unwrap(), 5);
        assert_eq!(shape.index_of(&[2, 3]).unwrap(), 11);

        let shape3d = Shape::new([2, 3, 4]);
        assert_eq!(shape3d.index_of(&[0, 0, 0]).unwrap(), 0);
        assert_eq!(shape3d.index_of(&[0, 0, 1]).unwrap(), 1);
        assert_eq!(shape3d.index_of(&[0, 1, 0]).unwrap(), 4);
        assert_eq!(shape3d.index_of(&[1, 2, 3]).unwrap(), 23);
    }

    #[test]
    fn test_vector_calculation() {
        let shape = Shape::new([3, 4]);

        assert_eq!(shape.vector_of(0).unwrap(), [0, 0]);
        assert_eq!(shape.vector_of(1).unwrap(), [0, 1]);
        assert_eq!(shape.vector_of(2).unwrap(), [0, 2]);
        assert_eq!(shape.vector_of(3).unwrap(), [0, 3]);
        assert_eq!(shape.vector_of(4).unwrap(), [1, 0]);
        assert_eq!(shape.vector_of(5).unwrap(), [1, 1]);
        assert_eq!(shape.vector_of(11).unwrap(), [2, 3]);

        let shape3d = Shape::new([2, 3, 4]);
        assert_eq!(shape3d.vector_of(0).unwrap(), [0, 0, 0]);
        assert_eq!(shape3d.vector_of(1).unwrap(), [0, 0, 1]);
        assert_eq!(shape3d.vector_of(4).unwrap(), [0, 1, 0]);
        assert_eq!(shape3d.vector_of(23).unwrap(), [1, 2, 3]);
    }

    #[test]
    fn test_index_vector_inverse() {
        let shape = Shape::new([3, 4, 5]);

        for i in 0..shape.len() {
            let vector = shape.vector_of(i).unwrap();
            let calculated_index = shape.index_of(&vector).unwrap();
            assert_eq!(
                i, calculated_index,
                "Failed at i={}, vector={:?}",
                i, vector
            );
        }

        let dyn_shape = dyn_shape![3, 4, 5];

        for i in 0..dyn_shape.len() {
            let vector = dyn_shape.vector_of(i).unwrap();
            let calculated_index = dyn_shape.index_of(&vector).unwrap();
            assert_eq!(
                i, calculated_index,
                "Failed at i={}, vector={:?}",
                i, vector
            );
        }
    }

    #[test]
    fn test_next_vector() {
        let shape = Shape::new([2, 3]);

        let mut all_vectors = Vec::new();
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

        let dyn_shape = dyn_shape![2, 3];
        let mut dyn_v = dyn_shape.zero();
        let mut dyn_count = 1;

        while dyn_shape.next_vector(&mut dyn_v) {
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
            // correct
        } else {
            panic!("Expected DimensionMismatching error for small vector");
        }

        let wrong_dim_vec_large = vec![0, 0, 0];
        let result = dyn_shape.index_of(&wrong_dim_vec_large);
        assert!(result.is_err());

        if let Err(IndexCalculationError::DimensionMismatching(_)) = result {
            // correct
        } else {
            panic!("Expected DimensionMismatching error for large vector");
        }
    }

    #[test]
    fn test_out_of_shape_error() {
        let shape = Shape::new([3, 4]);

        let out_of_bounds_vector = [3, 0];
        let result = shape.index_of(&out_of_bounds_vector);
        assert!(result.is_err());

        if let Err(IndexCalculationError::OutOfShape(_)) = result {
            // correct
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

        let dyn_shape = dyn_shape![2, 3, 4];
        assert_eq!(dyn_shape.len_of_dimension(0).unwrap(), 2);
        assert_eq!(dyn_shape.len_of_dimension(1).unwrap(), 3);
        assert_eq!(dyn_shape.len_of_dimension(2).unwrap(), 4);
    }

    #[test]
    fn test_offset_of_dimension() {
        let shape = Shape::new([2, 3, 4]);

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
        let shape1: Shape1 = Shape::new([5]);
        assert_eq!(shape1.dimension(), 1);

        let shape2: Shape2 = Shape::new([3, 4]);
        assert_eq!(shape2.dimension(), 2);

        let shape3: Shape3 = Shape::new([2, 3, 4]);
        assert_eq!(shape3.dimension(), 3);
    }

    #[test]
    fn test_zero_vector() {
        let shape = Shape::new([2, 3, 4]);
        let zero = shape.zero();
        assert_eq!(zero, [0, 0, 0]);

        let dyn_shape = dyn_shape![2, 3, 4];
        let dyn_zero = dyn_shape.zero();
        assert_eq!(dyn_zero, vec![0, 0, 0]);
    }

    #[test]
    fn test_actual_index_calculation() {
        let shape = Shape::new([5]);

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
        let dyn_shape = DynShape::<Vec<usize>, Vec<DummyIndex>>::new_with_order(
            vec![2, 3],
            StorageOrder::RowMajor,
        );

        assert_eq!(dyn_shape.storage_order(), StorageOrder::RowMajor);
        assert_eq!(dyn_shape.len(), 6);
        assert_eq!(dyn_shape.offset_of_dimension(0).unwrap(), 3);
        assert_eq!(dyn_shape.offset_of_dimension(1).unwrap(), 1);
    }

    #[test]
    fn test_dyn_storage_order_column_major() {
        let dyn_shape = DynShape::<Vec<usize>, Vec<DummyIndex>>::new_with_order(
            vec![2, 3],
            StorageOrder::ColumnMajor,
        );

        assert_eq!(dyn_shape.storage_order(), StorageOrder::ColumnMajor);
        assert_eq!(dyn_shape.len(), 6);
        assert_eq!(dyn_shape.offset_of_dimension(0).unwrap(), 1);
        assert_eq!(dyn_shape.offset_of_dimension(1).unwrap(), 2);
    }

    #[test]
    fn test_dyn_with_storage_order() {
        let dyn_shape_row = DynShape::<Vec<usize>, Vec<DummyIndex>>::new_with_order(
            vec![2, 3],
            StorageOrder::RowMajor,
        );
        let dyn_shape_col = dyn_shape_row.with_storage_order(StorageOrder::ColumnMajor);

        assert_eq!(dyn_shape_col.storage_order(), StorageOrder::ColumnMajor);
        assert_eq!(dyn_shape_col.len(), 6);
        assert_eq!(dyn_shape_col.offset_of_dimension(0).unwrap(), 1);
        assert_eq!(dyn_shape_col.offset_of_dimension(1).unwrap(), 2);
    }
}
