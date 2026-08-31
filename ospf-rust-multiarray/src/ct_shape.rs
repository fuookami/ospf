//! # CTShape - 编译时形状类型定义
//!
//! ## Overview / 概述
//!
//! This module provides compile-time shape types with fixed storage order.
//! The storage order is encoded in the type system at compile time, enabling
//! optimizations and type-level guarantees about memory layout.
//!
//! 本模块提供具有固定存储顺序的编译时形状类型。
//! 存储顺序在编译时被编码到类型系统中，实现内存布局的优化和类型级保证。
//!
//! ## Key Types / 主要类型
//!
//! - `CTShape<const D: usize, SO: StorageOrderTrait>` - Compile-time fixed dimension shape / 编译时固定维度形状
//! - `CTDynShape<SO: StorageOrderTrait>` - Compile-time dynamic dimension shape / 编译时动态维度形状
//!
//! ## Type Aliases / 类型别名
//!
//! - `ShapeRM<D>` - Row-major compile-time shape / 行优先编译时形状
//! - `ShapeCM<D>` - Column-major compile-time shape / 列优先编译时形状
//! - `DynShapeRM` - Row-major dynamic shape / 行优先动态形状
//! - `DynShapeCM` - Column-major dynamic shape / 列优先动态形状

use super::concept::*;
use super::dummy_index::{DummyIndex, DummyIndexIterator, IteratorVector};
use super::map_index::MapIndex;
use super::shape::{AbstractRTShape, AbstractShape, DynShape, Shape};
use ospf_rust_base::Indices;
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;
use std::mem;
use std::ops::RangeFull;
use std::sync::OnceLock;

/// # AbstractCTShape Trait
///
/// A trait that extends `AbstractShape` with compile-time storage order information.
/// This allows the storage order to be known at compile time while still providing
/// runtime shape values.
///
/// 扩展 `AbstractShape` 的特征，添加编译时存储顺序信息。
/// 这允许在编译时已知存储顺序，同时仍提供运行时形状值。
///
/// ## Type Parameters / 类型参数
///
/// - `SO: StorageOrderTrait` - The storage order type / 存储顺序类型
///
/// ## Associated Types / 关联类型
///
/// - `RunTimeShapeType` - The corresponding runtime shape type / 对应的运行时形状类型
/// - `StorageOrderType` - The storage order trait type / 存储顺序特征类型
pub trait AbstractCTShape<SO: StorageOrderTrait>: AbstractShape {
    /// The runtime shape type corresponding to this compile-time shape.
    ///
    /// 与此编译时形状对应的运行时形状类型。
    type RunTimeShapeType: AbstractRTShape;

    /// The storage order trait type.
    ///
    /// 存储顺序特征类型。
    type StorageOrderType: StorageOrderTrait;

    /// Convert this compile-time shape to its runtime representation.
    ///
    /// 将此编译时形状转换为其运行时表示。
    ///
    /// # Returns / 返回值
    ///
    /// The runtime shape value / 运行时形状值
    fn runtime_value(&self) -> Self::RunTimeShapeType;
}

/// # CTShape - Compile-Time Shape with Fixed Dimension
///
/// A shape type where the dimension is fixed at compile time and the storage
/// order is encoded in the type system. This provides type-level guarantees
/// about the memory layout.
///
/// 维度在编译时固定且存储顺序编码在类型系统中的形状类型。
/// 这提供了关于内存布局的类型级保证。
///
/// ## Type Parameters / 类型参数
///
/// - `const D: usize` - The fixed dimension / 固定维度
/// - `SO: StorageOrderTrait` - The storage order trait / 存储顺序特征
///
/// ## Fields / 字段
///
/// - `shape` - The size of each dimension / 每个维度的大小
/// - `offsets` - The stride for each dimension / 每个维度的步幅
/// - `len` - Total number of elements / 元素总数
/// - `_marker` - Phantom marker for storage order type / 存储顺序类型的虚拟特征
///
/// ## Example / 示例
///
/// ```rust
/// use ospf_rust_multiarray::*;
///
/// // Create a row-major 2D shape / 创建一个行优先 2D 形状
/// let shape_rm: CTShape<2, RowMajor> = CTShape::new([3, 4]);
///
/// // Using type aliases / 使用类型别名
/// let shape: ShapeRM<2> = ShapeRM::new([3, 4]);
/// let shape_cm: ShapeCM<2> = ShapeCM::new([3, 4]);
/// ```
pub struct CTShape<const D: usize, SO: StorageOrderTrait> {
    /// Dimension sizes / 维度大小
    pub(crate) shape: [usize; D],
    /// Dimension strides (offsets) and total element count - lazy initialized / 维度步幅（偏移量）和元素总数 - 惰性初始化
    pub(crate) offsets_and_len: OnceLock<([usize; D], usize)>,
    /// Phantom marker for storage order type / 存储顺序类型的虚拟特征
    _marker: PhantomData<SO>,
}

impl<const D: usize, SO: StorageOrderTrait> Debug for CTShape<D, SO> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("Shape")
            .field("shape", &self.shape)
            .field("offsets", self.offsets())
            .field("len", &self.len())
            .field("storage_order", &SO::runtime_value())
            .finish()
    }
}

impl<const D: usize, SO: StorageOrderTrait> Clone for CTShape<D, SO> {
    fn clone(&self) -> Self {
        Self {
            shape: self.shape,
            offsets_and_len: OnceLock::new(),
            _marker: PhantomData,
        }
    }
}

impl<const D: usize, SO: StorageOrderTrait> CTShape<D, SO> {
    /// Create a new CTShape with the given dimension sizes.
    ///
    /// 使用给定的维度大小创建新的 CTShape。
    ///
    /// # Parameters / 参数
    ///
    /// - `shape` - The size of each dimension / 每个维度的大小
    ///
    /// # Returns / 返回值
    ///
    /// A new CTShape with the specified storage order / 具有指定存储顺序的新 CTShape
    #[inline]
    pub fn new(shape: [usize; D]) -> Self {
        let (offsets, len) = SO::offsets(&shape);

        Self {
            shape,
            offsets_and_len: OnceLock::new(),
            _marker: PhantomData,
        }
    }

    /// Create a CTShape from a runtime Shape.
    ///
    /// 从运行时 Shape 创建 CTShape。
    ///
    /// # Parameters / 参数
    ///
    /// - `runtime_shape` - The runtime shape to convert from / 要转换的运行时形状
    ///
    /// # Returns / 返回值
    ///
    /// A new CTShape with the same dimension sizes / 具有相同维度大小的新 CTShape
    #[inline]
    pub fn from_runtime_shape(runtime_shape: &Shape<D>) -> Self {
        Self::new(runtime_shape.shape.clone())
    }

    /// Get or compute the offsets and len.
    ///
    /// 获取或计算偏移量和长度。
    fn get_or_init_offsets_and_len(&self) -> &([usize; D], usize) {
        self.offsets_and_len
            .get_or_init(|| SO::offsets(&self.shape))
    }
}

impl<const D: usize, SO: StorageOrderTrait> AbstractShape for CTShape<D, SO> {
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
        SO::runtime_value()
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

impl<const D: usize, SO: StorageOrderTrait> AbstractCTShape<SO> for CTShape<D, SO> {
    type RunTimeShapeType = Shape<D>;
    type StorageOrderType = SO;

    fn runtime_value(&self) -> Self::RunTimeShapeType {
        Shape {
            shape: self.shape.clone(),
            offsets_and_len: OnceLock::new(),
            storage_order: SO::runtime_value(),
        }
    }
}

/// # CTDynShape - Compile-Time Dynamic-Dimension Shape
///
/// A shape type where the dimension is determined at runtime but the storage
/// order is encoded in the type system at compile time.
///
/// 维度在运行时确定但存储顺序在编译时编码到类型系统中的形状类型。
///
/// ## Type Parameters / 类型参数
///
/// - `SO: StorageOrderTrait` - The storage order trait / 存储顺序特征
///
/// ## Fields / 字段
///
/// - `shape` - The size of each dimension / 每个维度的大小
/// - `offsets` - The stride for each dimension / 每个维度的步幅
/// - `len` - Total number of elements / 元素总数
/// - `_marker` - Phantom marker for storage order type / 存储顺序类型的虚拟特征
///
/// ## Example / 示例
///
/// ```rust
/// use ospf_rust_multiarray::*;
///
/// // Create a row-major dynamic shape / 创建一个行优先动态形状
/// let shape_rm: CTDynShape<Vec<usize>, Vec<DummyIndex>, RowMajor> = CTDynShape::new(vec![2, 3, 4]);
///
/// // Using type aliases / 使用类型别名
/// let shape: DynShapeRM = DynShapeRM::new(vec![2, 3, 4]);
/// let shape_cm: DynShapeCM = DynShapeCM::new(vec![2, 3, 4]);
/// ```
pub struct CTDynShape<V: DynShapeVector, DV: DummyVector, SO: StorageOrderTrait> {
    /// Dimension sizes / 维度大小
    pub(crate) shape: V,
    /// Dimension strides (offsets) and total element count - lazy initialized / 维度步幅（偏移量）和元素总数 - 惰性初始化
    pub(crate) offsets_and_len: OnceLock<(V, usize)>,
    /// Phantom marker for storage order type / 存储顺序类型的虚拟特征
    _marker: PhantomData<(DV, SO)>,
}

impl<V: DynShapeVector + Display, DV: DummyVector, SO: StorageOrderTrait> Display
    for CTDynShape<V, DV, SO>
{
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.shape)
    }
}

impl<V: DynShapeVector, DV: DummyVector, SO: StorageOrderTrait> Debug for CTDynShape<V, DV, SO> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("DynShape")
            .field("shape", &self.shape)
            .field("offsets", self.offsets())
            .field("len", &self.len())
            .field("storage_order", &SO::runtime_value())
            .finish()
    }
}

impl<V: DynShapeVector, DV: DummyVector, SO: StorageOrderTrait> Clone for CTDynShape<V, DV, SO> {
    fn clone(&self) -> Self {
        Self {
            shape: self.shape.indices().map(|i| self.shape[i]).collect(),
            offsets_and_len: OnceLock::new(),
            _marker: PhantomData,
        }
    }
}

impl<V: DynShapeVector, DV: DummyVector, SO: StorageOrderTrait> CTDynShape<V, DV, SO> {
    /// Create a new CTDynShape with the given dimension sizes.
    ///
    /// 使用给定的维度大小创建新的 CTDynShape。
    ///
    /// # Parameters / 参数
    ///
    /// - `shape` - The size of each dimension / 每个维度的大小
    ///
    /// # Returns / 返回值
    ///
    /// A new CTDynShape with the specified storage order / 具有指定存储顺序的新 CTDynShape
    #[inline]
    pub fn new(shape: V) -> Self {
        let (offsets, len) = SO::dyn_offsets(&shape);

        Self {
            shape,
            offsets_and_len: OnceLock::new(),
            _marker: PhantomData,
        }
    }

    /// Create a CTDynShape from a runtime DynShape.
    ///
    /// 从运行时 DynShape 创建 CTDynShape。
    ///
    /// # Parameters / 参数
    ///
    /// - `runtime_shape` - The runtime shape to convert from / 要转换的运行时形状
    ///
    /// # Returns / 返回值
    ///
    /// A new CTDynShape with the same dimension sizes / 具有相同维度大小的新 CTDynShape
    #[inline]
    pub fn from_runtime_shape(runtime_shape: &DynShape) -> Self {
        Self::new(
            (0..runtime_shape.dimension())
                .map(|i| runtime_shape[i])
                .collect(),
        )
    }

    /// Get or compute the offsets and len.
    ///
    /// 获取或计算偏移量和长度。
    fn get_or_init_offsets_and_len(&self) -> &(V, usize) {
        self.offsets_and_len
            .get_or_init(|| SO::dyn_offsets(&self.shape))
    }
}

impl<V: DynShapeVector, DV: DummyVector, SO: StorageOrderTrait> AbstractShape
    for CTDynShape<V, DV, SO>
{
    const DIMENSION: usize = DYN_DIMENSION;
    type VectorType = Vec<usize>;
    type DummyVectorType = Vec<DummyIndex>;
    type MapVectorType = Vec<MapIndex>;
    type ShapeVectorType = Vec<usize>;
    type IteratorVectorType = Vec<DummyIndexIterator>;

    #[inline]
    fn zero(&self) -> Self::VectorType {
        vec![0; self.shape.len()]
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
        SO::runtime_value()
    }

    #[inline]
    fn from_shape_vector(shape: &Self::ShapeVectorType) -> Self
    where
        Self: Sized,
    {
        Self::new(shape.indices().map(|i| i).collect())
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
        dummy.iter().enumerate().map(|(i, d)| d.iterator_of(self, i)).collect()
    }

    #[inline]
    fn map_to_iterator_vector(&self, map_vector: &Self::MapVectorType) -> Self::IteratorVectorType {
        use std::ops::RangeFull;
        map_vector.iter().enumerate().map(|(i, m)| match m {
            MapIndex::Dummy(dummy) => dummy.iterator_of(self, i),
            MapIndex::Map(_) => DummyIndex::Range(Box::new(RangeFull)).iterator_of(self, i),
        }).collect()
    }
}

impl<V: DynShapeVector, DV: DummyVector, SO: StorageOrderTrait> AbstractCTShape<SO>
    for CTDynShape<V, DV, SO>
{
    type RunTimeShapeType = DynShape;
    type StorageOrderType = SO;

    fn runtime_value(&self) -> Self::RunTimeShapeType {
        DynShape {
            shape: self.shape.indices().map(|i| self.shape[i]).collect(),
            offsets_and_len: OnceLock::new(),
            storage_order: SO::runtime_value(),
            _marker: PhantomData,
        }
    }
}

/// Type aliases for row-major compile-time shapes.
/// 行优先编译时形状的类型别名。
pub type ShapeRM<const D: usize> = CTShape<{ D }, RowMajor>;
/// Type aliases for column-major compile-time shapes.
/// 列优先编译时形状的类型别名。
pub type ShapeCM<const D: usize> = CTShape<{ D }, ColumnMajor>;
/// Type alias for row-major compile-time dynamic shape.
/// 行优先编译时动态形状的类型别名。
pub type DynShapeRM<V = Vec<usize>, DV = Vec<DummyIndex>> = CTDynShape<V, DV, RowMajor>;
/// Type alias for column-major compile-time dynamic shape.
/// 列优先编译时动态形状的类型别名。
pub type DynShapeCM<V = Vec<usize>, DV = Vec<DummyIndex>> = CTDynShape<V, DV, ColumnMajor>;

/// Type aliases for row-major shapes with specific dimensions (0-20).
/// 具有特定维度（0-20）的行优先形状的类型别名。
pub type ShapeRM0 = ShapeRM<0>;
pub type ShapeRM1 = ShapeRM<1>;
pub type ShapeRM2 = ShapeRM<2>;
pub type ShapeRM3 = ShapeRM<3>;
pub type ShapeRM4 = ShapeRM<4>;
pub type ShapeRM5 = ShapeRM<5>;
pub type ShapeRM6 = ShapeRM<6>;
pub type ShapeRM7 = ShapeRM<7>;
pub type ShapeRM8 = ShapeRM<8>;
pub type ShapeRM9 = ShapeRM<9>;
pub type ShapeRM10 = ShapeRM<10>;
pub type ShapeRM11 = ShapeRM<11>;
pub type ShapeRM12 = ShapeRM<12>;
pub type ShapeRM13 = ShapeRM<13>;
pub type ShapeRM14 = ShapeRM<14>;
pub type ShapeRM15 = ShapeRM<15>;
pub type ShapeRM16 = ShapeRM<16>;
pub type ShapeRM17 = ShapeRM<17>;
pub type ShapeRM18 = ShapeRM<18>;
pub type ShapeRM19 = ShapeRM<19>;
pub type ShapeRM20 = ShapeRM<20>;

/// Type aliases for column-major shapes with specific dimensions (0-20).
/// 具有特定维度（0-20）的列优先形状的类型别名。
pub type ShapeCM0 = ShapeCM<0>;
pub type ShapeCM1 = ShapeCM<1>;
pub type ShapeCM2 = ShapeCM<2>;
pub type ShapeCM3 = ShapeCM<3>;
pub type ShapeCM4 = ShapeCM<4>;
pub type ShapeCM5 = ShapeCM<5>;
pub type ShapeCM6 = ShapeCM<6>;
pub type ShapeCM7 = ShapeCM<7>;
pub type ShapeCM8 = ShapeCM<8>;
pub type ShapeCM9 = ShapeCM<9>;
pub type ShapeCM10 = ShapeCM<10>;
pub type ShapeCM11 = ShapeCM<11>;
pub type ShapeCM12 = ShapeCM<12>;
pub type ShapeCM13 = ShapeCM<13>;
pub type ShapeCM14 = ShapeCM<14>;
pub type ShapeCM15 = ShapeCM<15>;
pub type ShapeCM16 = ShapeCM<16>;
pub type ShapeCM17 = ShapeCM<17>;
pub type ShapeCM18 = ShapeCM<18>;
pub type ShapeCM19 = ShapeCM<19>;
pub type ShapeCM20 = ShapeCM<20>;
