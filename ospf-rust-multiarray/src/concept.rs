//! 核心概念和 trait 定义
//! Core concept and trait definitions
//!
//! 本模块定义了多维数组库的核心抽象：
//! This module defines core abstractions for the multi-dimensional array library:
//!
//! - `StorageOrderTrait`: 存储顺序 trait（行优先/列优先）
//!   Storage order trait (row-major/column-major)
//! - `AccessOrderTrait`: 访问顺序 trait
//!   Access order trait
//! - `Vector`: 通用向量 trait
//!   Generic vector trait
//! - `DynShapeContainer`: 动态形状容器 trait
//!   Dynamic shape container trait

use super::dummy_index::DummyIndex;
use super::map_index::MapIndex;
use ospf_rust_base::collection_traits::{Collection, Len};
use ospf_rust_base::Indices;
use std::alloc::Allocator;
use std::fmt::Debug;
use std::iter::FromIterator;
use std::mem;
use std::ops::{Index, IndexMut};

/// 使用 ospf_rust_base 中的 Vec 作为默认容器类型
/// Use Vec from ospf_rust_base as the default container type
pub use ospf_rust_base::container::Vec;

/// 动态维度标记，表示维度在运行时确定
/// Dynamic dimension marker, indicating dimension is determined at runtime
pub const DYN_DIMENSION: usize = usize::MAX;

/// 存储顺序 trait
/// Storage order trait
///
/// 定义多维数组的存储顺序（行优先或列优先）。
/// Defines the storage order (row-major or column-major) for multi-dimensional arrays.
///
/// ## 类型参数 / Type Parameters
///
/// - `Self::AccessOrder`: 关联的访问顺序类型
///   Associated access order type
pub trait StorageOrderTrait: Default {
    /// 关联的访问顺序类型
    /// Associated access order type
    type AccessOrder: AccessOrderTrait;

    /// 计算给定形状的偏移量和总长度
    /// Calculate offsets and total length for a given shape
    ///
    /// ## 参数 / Parameters
    ///
    /// - `shape`: 维度大小数组
    ///   Array of dimension sizes
    ///
    /// ## 返回值 / Returns
    ///
    /// 返回偏移量数组和总元素数量的元组
    /// Returns a tuple of offset array and total element count
    fn offsets<const DIMENSION: usize>(
        &self,
        shape: &[usize; DIMENSION],
    ) -> ([usize; DIMENSION], usize);

    /// 计算动态形状的偏移量和总长度
    /// Calculate offsets and total length for a dynamic shape
    ///
    /// ## 参数 / Parameters
    ///
    /// - `shape`: 动态形状向量
    ///   Dynamic shape vector
    ///
    /// ## 返回值 / Returns
    ///
    /// 返回偏移量向量和总元素数量的元组
    /// Returns a tuple of offset vector and total element count
    fn dyn_offsets<V: DynShapeVector>(&self, shape: &V) -> (V, usize);

    /// 获取访问顺序
    /// Get the access order
    fn access_order(&self) -> Self::AccessOrder;

    /// 获取运行时的存储顺序值
    /// Get the runtime storage order value
    fn runtime_value(&self) -> StorageOrder;

    /// 获取静态的运行时存储顺序值
    /// Get the static runtime storage order value
    fn runtime_value_static() -> StorageOrder
    where
        Self: Default + Sized,
    {
        Self::default().runtime_value()
    }
}

/// 访问顺序 trait
/// Access order trait
///
/// 定义多维数组的访问顺序。
/// Defines the access order for multi-dimensional arrays.
pub trait AccessOrderTrait: Default {
    /// 获取运行时的访问顺序值
    /// Get the runtime access order value
    fn runtime_value(&self) -> AccessOrder;

    /// 获取静态的运行时访问顺序值
    /// Get the static runtime access order value
    fn runtime_value_static() -> AccessOrder
    where
        Self: Default + Sized,
    {
        Self::default().runtime_value()
    }

    /// 获取默认值
    /// Get the default value
    #[inline]
    fn default_value() -> AccessOrder
    where
        Self: Default + Sized,
    {
        Self::default().runtime_value()
    }
}

/// 行优先存储顺序
/// Row-major storage order
///
/// 在行优先存储中，连续的元素在内存中按行排列。
/// In row-major storage, consecutive elements are arranged by rows in memory.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RowMajor;

impl StorageOrderTrait for RowMajor {
    type AccessOrder = RowMajor;

    /// 计算行优先存储的偏移量
    /// Calculate offsets for row-major storage
    ///
    /// 在行优先存储中，最后一个维度的偏移量为 1，向前递推。
    /// In row-major storage, the last dimension has offset 1, calculated backwards.
    ///
    /// ## 示例 / Example
    ///
    /// 对于形状 [2, 3, 4]：
    /// For shape [2, 3, 4]:
    /// - offsets[2] = 1 (最后一维)
    /// - offsets[1] = 4 (shape[2])
    /// - offsets[0] = 12 (shape[1] * shape[2])
    #[inline]
    fn offsets<const DIMENSION: usize>(
        &self,
        shape: &[usize; DIMENSION],
    ) -> ([usize; DIMENSION], usize) {
        // SAFETY: 安全性说明
        // 我们初始化 offsets 为零，后续会覆盖所有有效维度的值。
        // 对于零维情况，返回全零数组是正确的行为。
        // SAFETY: We initialize offsets to zero, and will overwrite all valid dimension values later.
        // For zero-dimension case, returning an all-zero array is the correct behavior.
        let mut offsets: [usize; DIMENSION] = unsafe { mem::zeroed() };
        let mut len = 1;

        // 处理零维情况 / Handle zero-dimension case
        if shape.len() == 0 {
            return (offsets, 1);
        }

        // 行优先：从后向前计算偏移量
        // Row-major: calculate offsets from back to front
        offsets[shape.len() - 1] = 1;
        for i in (0..(shape.len() - 1)).rev() {
            offsets[i] = offsets[i + 1] * shape[i + 1];
            len *= shape[i + 1];
        }
        len *= shape[0];

        (offsets, len)
    }

    /// 计算行优先存储的动态形状偏移量
    /// Calculate offsets for row-major storage with dynamic shape
    ///
    /// 与 `offsets` 方法功能相同，但支持运行时动态形状。
    /// Same functionality as `offsets`, but supports runtime dynamic shapes.
    ///
    /// ## 类型参数 / Type Parameters
    ///
    /// - `V`: 动态形状向量类型
    ///   Dynamic shape vector type
    #[inline]
    fn dyn_offsets<V: DynShapeVector>(&self, shape: &V) -> (V, usize) {
        // 处理零维情况 / Handle zero-dimension case
        if shape.len() == 0 {
            let offset: V = (0..0).map(|_| 0).collect();
            return (offset, 1);
        }

        let mut offset: V = (0..shape.len()).map(|_| 0).collect();
        let mut len = 1;

        // 行优先：从后向前计算偏移量
        // Row-major: calculate offsets from back to front
        offset[shape.len() - 1] = 1;
        for i in (0..(shape.len() - 1)).rev() {
            offset[i] = offset[i + 1] * shape[i + 1];
            len *= shape[i + 1];
        }
        len *= shape[0];

        (offset, len)
    }

    /// 获取访问顺序（返回自身）
    /// Get access order (returns self)
    #[inline]
    fn access_order(&self) -> RowMajor {
        *self
    }

    /// 获取运行时存储顺序值
    /// Get runtime storage order value
    #[inline]
    fn runtime_value(&self) -> StorageOrder {
        StorageOrder::RowMajor
    }
}

/// 行优先访问顺序的 AccessOrderTrait 实现
/// AccessOrderTrait implementation for row-major access order
impl AccessOrderTrait for RowMajor {
    #[inline]
    fn runtime_value(&self) -> AccessOrder {
        AccessOrder::RowMajor
    }
}

/// 列优先存储顺序
/// Column-major storage order
///
/// 在列优先存储中，连续的元素在内存中按列排列。
/// In column-major storage, consecutive elements are arranged by columns in memory.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ColumnMajor;

impl StorageOrderTrait for ColumnMajor {
    type AccessOrder = ColumnMajor;

    /// 计算列优先存储的偏移量
    /// Calculate offsets for column-major storage
    ///
    /// 在列优先存储中，第一个维度的偏移量为 1，向后递推。
    /// In column-major storage, the first dimension has offset 1, calculated forwards.
    ///
    /// ## 示例 / Example
    ///
    /// 对于形状 [2, 3, 4]：
    /// For shape [2, 3, 4]:
    /// - offsets[0] = 1 (第一维)
    /// - offsets[1] = 2 (shape[0])
    /// - offsets[2] = 6 (shape[0] * shape[1])
    #[inline]
    fn offsets<const DIMENSION: usize>(
        &self,
        shape: &[usize; DIMENSION],
    ) -> ([usize; DIMENSION], usize) {
        // SAFETY: 安全性说明
        // 我们初始化 offsets 为零，后续会覆盖所有有效维度的值。
        // 对于零维情况，返回全零数组是正确的行为。
        // SAFETY: We initialize offsets to zero, and will overwrite all valid dimension values later.
        // For zero-dimension case, returning an all-zero array is the correct behavior.
        let mut offsets: [usize; DIMENSION] = unsafe { mem::zeroed() };
        let mut len = 1;

        // 处理零维情况 / Handle zero-dimension case
        if shape.len() == 0 {
            return (offsets, 1);
        }

        // 列优先：从前向后计算偏移量
        // Column-major: calculate offsets from front to back
        offsets[0] = 1;
        for i in 1..shape.len() {
            offsets[i] = offsets[i - 1] * shape[i - 1];
            len *= shape[i - 1];
        }
        len *= shape[shape.len() - 1];

        (offsets, len)
    }

    /// 计算列优先存储的动态形状偏移量
    /// Calculate offsets for column-major storage with dynamic shape
    ///
    /// 与 `offsets` 方法功能相同，但支持运行时动态形状。
    /// Same functionality as `offsets`, but supports runtime dynamic shapes.
    ///
    /// ## 类型参数 / Type Parameters
    ///
    /// - `V`: 动态形状向量类型
    ///   Dynamic shape vector type
    #[inline]
    fn dyn_offsets<V: DynShapeVector>(&self, shape: &V) -> (V, usize) {
        // 处理零维情况 / Handle zero-dimension case
        if shape.len() == 0 {
            let offset: V = (0..0).map(|_| 0).collect();
            return (offset, 1);
        }

        let mut offsets: V = (0..shape.len()).map(|_| 0).collect();
        let mut len = 1;

        // 列优先：从前向后计算偏移量
        // Column-major: calculate offsets from front to back
        offsets[0] = 1;
        for i in 1..shape.len() {
            offsets[i] = offsets[i - 1] * shape[i - 1];
            len *= shape[i - 1];
        }
        len *= shape[shape.len() - 1];

        (offsets, len)
    }

    /// 获取访问顺序（返回自身）
    /// Get access order (returns self)
    #[inline]
    fn access_order(&self) -> ColumnMajor {
        *self
    }

    /// 获取运行时存储顺序值
    /// Get runtime storage order value
    #[inline]
    fn runtime_value(&self) -> StorageOrder {
        StorageOrder::ColumnMajor
    }
}

/// 列优先访问顺序的 AccessOrderTrait 实现
/// AccessOrderTrait implementation for column-major access order
impl AccessOrderTrait for ColumnMajor {
    #[inline]
    fn runtime_value(&self) -> AccessOrder {
        AccessOrder::ColumnMajor
    }
}

/// 存储顺序枚举
/// Storage order enum
///
/// 表示运行时的存储顺序。
/// Represents the storage order at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StorageOrder {
    /// 行优先存储（默认）
    /// Row-major storage (default)
    #[default]
    RowMajor,

    /// 列优先存储
    /// Column-major storage
    ColumnMajor,
}

/// StorageOrder 枚举的 StorageOrderTrait 实现
/// StorageOrderTrait implementation for StorageOrder enum
///
/// 该实现通过模式匹配委托给具体的存储顺序类型。
/// This implementation delegates to specific storage order types via pattern matching.
impl StorageOrderTrait for StorageOrder {
    type AccessOrder = AccessOrder;

    /// 计算偏移量，根据存储顺序委托给对应实现
    /// Calculate offsets, delegating to the corresponding implementation based on storage order
    fn offsets<const DIMENSION: usize>(
        &self,
        shape: &[usize; DIMENSION],
    ) -> ([usize; DIMENSION], usize) {
        match self {
            StorageOrder::RowMajor => RowMajor.offsets(shape),
            StorageOrder::ColumnMajor => ColumnMajor.offsets(shape),
        }
    }

    /// 计算动态形状偏移量，根据存储顺序委托给对应实现
    /// Calculate dynamic shape offsets, delegating to the corresponding implementation
    fn dyn_offsets<V: DynShapeVector>(&self, shape: &V) -> (V, usize) {
        match self {
            StorageOrder::RowMajor => RowMajor.dyn_offsets(shape),
            StorageOrder::ColumnMajor => ColumnMajor.dyn_offsets(shape),
        }
    }

    /// 获取访问顺序，根据存储顺序返回对应的访问顺序
    /// Get access order, returning the corresponding access order based on storage order
    fn access_order(&self) -> Self::AccessOrder {
        match self {
            StorageOrder::RowMajor => AccessOrder::RowMajor,
            StorageOrder::ColumnMajor => AccessOrder::ColumnMajor,
        }
    }

    /// 获取运行时存储顺序值（返回自身）
    /// Get runtime storage order value (returns self)
    #[inline]
    fn runtime_value(&self) -> StorageOrder {
        *self
    }
}

/// 访问顺序枚举
/// Access order enum
///
/// 表示运行时的访问顺序。
/// Represents the access order at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccessOrder {
    /// 行优先访问（默认）
    /// Row-major access (default)
    #[default]
    RowMajor,

    /// 列优先访问
    /// Column-major access
    ColumnMajor,
}

/// AccessOrder 枚举的 AccessOrderTrait 实现
/// AccessOrderTrait implementation for AccessOrder enum
impl AccessOrderTrait for AccessOrder {
    /// 获取运行时访问顺序值（返回自身）
    /// Get runtime access order value (returns self)
    #[inline]
    fn runtime_value(&self) -> AccessOrder {
        *self
    }
}

impl AccessOrder {
    /// 从存储顺序创建访问顺序
    /// Create access order from storage order
    #[inline]
    pub fn from_storage_order(storage_order: StorageOrder) -> Self {
        match storage_order {
            StorageOrder::RowMajor => AccessOrder::RowMajor,
            StorageOrder::ColumnMajor => AccessOrder::ColumnMajor,
        }
    }
}

/// 向量 trait 别名
/// Vector trait alias
///
/// 定义向量必须实现的基本接口。
/// Defines the basic interface that vectors must implement.
pub trait Vector = Collection<Item = usize>
    + Len
    + Indices
    + Index<usize, Output = usize>
    + IndexMut<usize, Output = usize>
    + Debug
    + PartialEq<[usize]>;

/// 虚拟向量 trait 别名
/// Dummy vector trait alias
///
/// 用于虚拟索引的向量类型。
/// Vector type used for dummy indexing.
pub trait DummyVector = Collection<Item = DummyIndex>
    + Len
    + Indices
    + Index<usize, Output = DummyIndex>
    + IndexMut<usize, Output = DummyIndex>
    + Debug;

/// 映射向量 trait 别名
/// Map vector trait alias
///
/// 用于映射索引的向量类型。
/// Vector type used for map indexing.
pub trait MapVector = Collection<Item = MapIndex>
    + Len
    + Indices
    + Index<usize, Output = MapIndex>
    + IndexMut<usize, Output = MapIndex>
    + Debug;

/// 形状向量 trait 别名
/// Shape vector trait alias
///
/// 用于表示形状的向量类型。
/// Vector type used to represent shapes.
pub trait ShapeVector = Collection<Item = usize>
    + Len
    + Indices
    + Index<usize, Output = usize>
    + IndexMut<usize, Output = usize>
    + Debug
    + PartialEq<[usize]>;

/// 动态形状向量 trait 别名
/// Dynamic shape vector trait alias
///
/// 支持运行时动态形状的向量类型。
/// Vector type supporting runtime dynamic shapes.
pub trait DynShapeVector = Collection<Item = usize>
    + Len
    + Indices
    + Index<usize, Output = usize>
    + IndexMut<usize, Output = usize>
    + FromIterator<usize>
    + Debug
    + PartialEq<[usize]>;

/// 动态形状容器 trait
/// Dynamic shape container trait
///
/// 定义动态形状容器的要求。
/// Defines requirements for dynamic shape containers.
pub trait DynShapeContainer {
    /// 不带分配器的类型
    /// Type without allocator
    type Type<T>: Collection<Item = T>
        + Len
        + Indices
        + Index<usize, Output = T>
        + IndexMut<usize, Output = T>
        + FromIterator<T>
        + Clone
        + Debug
        + PartialEq<[T]>
    where
        T: Clone + Debug + PartialEq;

    /// 带分配器的类型
    /// Type with allocator
    type TypeWithAllocator<T, A: Allocator + Clone>: Collection<Item = T>
        + Len
        + Indices
        + Index<usize, Output = T>
        + IndexMut<usize, Output = T>
        + FromIterator<T>
        + Clone
        + Debug
        + PartialEq<[T]>
    where
        T: Clone + Debug + PartialEq;
}

/// DynShapeContainer trait 的 Vec 实现
/// DynShapeContainer trait implementation for Vec
///
/// 使用标准库的 `std::vec::Vec` 作为容器类型。
/// Uses standard library's `std::vec::Vec` as the container type.
///
/// ## 说明 / Notes
///
/// 由于 `std::vec::Vec` 的分配器是类型参数而非运行时参数，
/// 因此 `Type` 和 `TypeWithAllocator` 都映射到 `std::vec::Vec<T>`。
/// Since `std::vec::Vec`'s allocator is a type parameter rather than a runtime parameter,
/// both `Type` and `TypeWithAllocator` map to `std::vec::Vec<T>`.
impl DynShapeContainer for Vec {
    type Type<T>
        = std::vec::Vec<T>
    where
        T: Clone + Debug + PartialEq;
    type TypeWithAllocator<T, A: Allocator + Clone>
        = std::vec::Vec<T>
    where
        T: Clone + Debug + PartialEq;
}

/// 数组相等性宏
/// Array equality macro
///
/// 用于比较数组与切片是否相等。
/// Used to compare if an array equals a slice.
#[macro_export]
macro_rules! array_eq {
    ($a:expr, [$($elem:expr),*]) => {
        $a == [$($elem),*].as_slice()
    };

    (assert_eq!($a:expr, [$($elem:expr),*])) => {
        assert_eq!($a, [$($elem),*].as_slice())
    };
}

/// 数组相等性断言宏
/// Array equality assertion macro
///
/// 断言数组与切片相等。
/// Assert that an array equals a slice.
#[macro_export]
macro_rules! assert_array_eq {
     ($a:expr, [$($elem:expr),*]) => {
        assert_eq!($a, [$($elem),*].as_slice())
    };
}

pub use array_eq;
pub use assert_array_eq;
