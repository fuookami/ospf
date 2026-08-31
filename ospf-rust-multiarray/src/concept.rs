//! # Concept - 核心概念特征
//!
//! ## Overview / 概述
//!
//! This module defines the core concepts and traits used throughout the multiarray library.
//! It includes:
//! - Storage order (RowMajor / ColumnMajor) / 存储顺序（行优先/列优先）
//! - Access order (RowMajor / ColumnMajor) / 访问顺序（行优先/列优先）
//! - Vector traits for shapes and indices / 用于形状和索引的向量特征
//!
//! 本模块定义了 multiarray 库中使用的核心概念和特征。
//! 包括：
//! - 存储顺序（行优先/列优先）
//! - 访问顺序（行优先/列优先）
//! - 用于形状和索引的向量特征
//!
//! ## Key Types / 主要类型
//!
//! - `StorageOrder` - Runtime storage order enumeration / 运行时存储顺序枚举
//! - `AccessOrder` - Runtime access order enumeration / 运行时访问顺序枚举
//! - `RowMajor` - Row-major order type / 行优先顺序类型
//! - `ColumnMajor` - Column-major order type / 列优先顺序类型
//!
//! ## Key Traits / 主要特征
//!
//! - `OrderTrait` - Unified trait for order types (compile-time and runtime) / 统一的顺序类型特征（编译时和运行时）
//! - `StorageOrderTrait` - Trait for storage order types / 存储顺序类型的特征
//! - `AccessOrderTrait` - Trait for access order types / 访问顺序类型的特征
//! - `Vector` - Trait for shape vectors / 形状向量的特征
//! - `DummyVector` - Trait for dummy index vectors / 虚拟索引向量的特征
//! - `MapVector` - Trait for map index vectors / 映射索引向量的特征
//! - `ShapeVector` - Trait for shape vectors / 形状向量的特征
//! - `DynShapeVector` - Trait for dynamic shape vectors / 动态形状向量的特征

use super::dummy_index::DummyIndex;
use super::map_index::MapIndex;
use cc_traits::{Collection, Len};
use ospf_rust_base::Indices;
use std::fmt::Debug;
use std::mem;
use std::ops::{Index, IndexMut};

/// Constant representing dynamic (runtime-determined) dimension.
/// 表示动态（运行时确定）维度的常量。
pub const DYN_DIMENSION: usize = usize::MAX;

/// # StorageOrder
///
/// An enumeration representing the storage order of multi-dimensional arrays.
/// Storage order determines how multi-dimensional indices are mapped to linear memory.
///
/// 表示多维数组存储顺序的枚举。
/// 存储顺序决定多维索引如何映射到线性内存。
///
/// ## Variants / 变体
///
/// - `RowMajor` - Row-major order (C-style, last dimension changes fastest) / 行优先顺序（C 风格，最后维度变化最快）
/// - `ColumnMajor` - Column-major order (Fortran-style, first dimension changes fastest) / 列优先顺序（Fortran 风格，第一维度变化最快）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StorageOrder {
    /// Row-major order (default) / 行优先顺序（默认）
    #[default]
    RowMajor,
    /// Column-major order / 列优先顺序
    ColumnMajor,
}

impl StorageOrder {
    /// Calculate offsets for a fixed-dimension shape.
    ///
    /// 计算固定维度形状的偏移量。
    ///
    /// # Type Parameters / 类型参数
    ///
    /// - `const DIMENSION: usize` - The fixed dimension / 固定维度
    ///
    /// # Parameters / 参数
    ///
    /// - `shape` - The shape array / 形状数组
    ///
    /// # Returns / 返回值
    ///
    /// A tuple of (offsets, total_length) / 偏移量和总长度的元组
    pub(crate) fn offsets<const DIMENSION: usize>(
        &self,
        shape: &[usize; DIMENSION],
    ) -> ([usize; DIMENSION], usize) {
        match self {
            StorageOrder::RowMajor => RowMajor::offsets(shape),
            StorageOrder::ColumnMajor => ColumnMajor::offsets(shape),
        }
    }

    /// Calculate offsets for a dynamic-dimension shape.
    ///
    /// 计算动态维度形状的偏移量。
    ///
    /// # Type Parameters / 类型参数
    ///
    /// - `V: DynShapeVector` - The vector type / 向量类型
    ///
    /// # Parameters / 参数
    ///
    /// - `shape` - The shape vector / 形状向量
    ///
    /// # Returns / 返回值
    ///
    /// A tuple of (offsets, total_length) / 偏移量和总长度的元组
    pub(crate) fn dyn_offsets<V: DynShapeVector>(&self, shape: &V) -> (V, usize) {
        match self {
            StorageOrder::RowMajor => RowMajor::dyn_offsets(shape),
            StorageOrder::ColumnMajor => ColumnMajor::dyn_offsets(shape),
        }
    }
}

/// # AccessOrder
///
/// An enumeration representing the access order for iterating over array elements.
/// Access order determines the order in which elements are visited during iteration.
///
/// 表示迭代数组元素时访问顺序的枚举。
/// 访问顺序决定迭代期间访问元素的顺序。
///
/// ## Variants / 变体
///
/// - `RowMajor` - Row-major order (last dimension changes fastest) / 行优先顺序（最后维度变化最快）
/// - `ColumnMajor` - Column-major order (first dimension changes fastest) / 列优先顺序（第一维度变化最快）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccessOrder {
    /// Row-major order (default) / 行优先顺序（默认）
    #[default]
    RowMajor,
    /// Column-major order / 列优先顺序
    ColumnMajor,
}

impl AccessOrder {
    /// Convert from StorageOrder to AccessOrder.
    ///
    /// 从 StorageOrder 转换为 AccessOrder。
    ///
    /// # Parameters / 参数
    ///
    /// - `order` - The storage order / 存储顺序
    ///
    /// # Returns / 返回值
    ///
    /// The corresponding access order / 对应的访问顺序
    #[inline]
    pub fn from_storage_order(order: StorageOrder) -> Self {
        match order {
            StorageOrder::RowMajor => AccessOrder::RowMajor,
            StorageOrder::ColumnMajor => AccessOrder::ColumnMajor,
        }
    }
}

/// # StorageOrderTrait
///
/// A trait for types that represent a storage order at compile time.
/// This allows storage order to be encoded in the type system.
///
/// 表示编译时存储顺序的类型的特征。
/// 这允许将存储顺序编码到类型系统中。
pub trait StorageOrderTrait {
    /// Get the runtime StorageOrder value.
    ///
    /// 获取运行时 StorageOrder 值。
    fn runtime_value() -> StorageOrder;

    /// Calculate offsets for a fixed-dimension shape.
    ///
    /// 计算固定维度形状的偏移量。
    fn offsets<const DIMENSION: usize>(shape: &[usize; DIMENSION]) -> ([usize; DIMENSION], usize);

    /// Calculate offsets for a dynamic-dimension shape.
    ///
    /// 计算动态维度形状的偏移量。
    fn dyn_offsets<V: DynShapeVector>(shape: &V) -> (V, usize);
}

/// # RowMajor
///
/// A type representing row-major storage order.
/// In row-major order, elements in the same row (last dimension) are stored contiguously.
/// This is the default order in C/C++ and many other languages.
///
/// 表示行优先存储顺序的类型。
/// 在行优先顺序中，同一行（最后维度）的元素连续存储。
/// 这是 C/C++ 和许多其他语言的默认顺序。
///
/// ## Example / 示例
///
/// For a 2D array with shape [2, 3]:
/// - Row-major layout: [0,0], [0,1], [0,2], [1,0], [1,1], [1,2]
/// - Offsets: [3, 1]
///
/// 对于形状为 [2, 3] 的 2D 数组：
/// - 行优先布局：[0,0], [0,1], [0,2], [1,0], [1,1], [1,2]
/// - 偏移量：[3, 1]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RowMajor;

impl StorageOrderTrait for RowMajor {
    #[inline]
    fn runtime_value() -> StorageOrder {
        StorageOrder::RowMajor
    }

    #[inline]
    fn offsets<const DIMENSION: usize>(shape: &[usize; DIMENSION]) -> ([usize; DIMENSION], usize) {
        let mut offsets: [usize; DIMENSION] = unsafe { mem::zeroed() };
        let mut len = 1;

        offsets[shape.len() - 1] = 1;
        for i in (0..(shape.len() - 1)).rev() {
            offsets[i] = offsets[i + 1] * shape[i + 1];
            len *= shape[i + 1];
        }
        len *= shape[0];

        (offsets, len)
    }

    #[inline]
    fn dyn_offsets<V: DynShapeVector>(shape: &V) -> (V, usize) {
        if shape.len() == 0 {
            let offset: V = (0..0).map(|_| 0).collect();
            return (offset, 1);
        }

        let mut offset: V = (0..shape.len()).map(|_| 0).collect();
        let mut len = 1;

        offset[shape.len() - 1] = 1;
        for i in (0..(shape.len() - 1)).rev() {
            offset[i] = offset[i + 1] * shape[i + 1];
            len *= shape[i + 1];
        }
        len *= shape[0];

        (offset, len)
    }
}

/// # ColumnMajor
///
/// A type representing column-major storage order.
/// In column-major order, elements in the same column (first dimension) are stored contiguously.
/// This is the default order in Fortran and MATLAB.
///
/// 表示列优先存储顺序的类型。
/// 在列优先顺序中，同一列（第一维度）的元素连续存储。
/// 这是 Fortran 和 MATLAB 的默认顺序。
///
/// ## Example / 示例
///
/// For a 2D array with shape [2, 3]:
/// - Column-major layout: [0,0], [1,0], [0,1], [1,1], [0,2], [1,2]
/// - Offsets: [1, 2]
///
/// 对于形状为 [2, 3] 的 2D 数组：
/// - 列优先布局：[0,0], [1,0], [0,1], [1,1], [0,2], [1,2]
/// - 偏移量：[1, 2]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ColumnMajor;

impl StorageOrderTrait for ColumnMajor {
    #[inline]
    fn runtime_value() -> StorageOrder {
        StorageOrder::ColumnMajor
    }

    #[inline]
    fn offsets<const DIMENSION: usize>(shape: &[usize; DIMENSION]) -> ([usize; DIMENSION], usize) {
        let mut offsets: [usize; DIMENSION] = unsafe { mem::zeroed() };
        let mut len = 1;

        offsets[0] = 1;
        for i in 1..shape.len() {
            offsets[i] = offsets[i - 1] * shape[i - 1];
            len *= shape[i - 1];
        }
        len *= shape[shape.len() - 1];

        (offsets, len)
    }

    #[inline]
    fn dyn_offsets<V: DynShapeVector>(shape: &V) -> (V, usize) {
        if shape.len() == 0 {
            let offset: V = (0..0).map(|_| 0).collect();
            return (offset, 1);
        }

        let mut offsets: V = (0..shape.len()).map(|_| 0).collect();
        let mut len = 1;

        offsets[0] = 1;
        for i in 1..shape.len() {
            offsets[i] = offsets[i - 1] * shape[i - 1];
            len *= shape[i - 1];
        }
        len *= shape[shape.len() - 1];

        (offsets, len)
    }
}

/// # AccessOrderTrait
///
/// A trait for types that represent an access order at compile time.
///
/// 表示编译时访问顺序的类型的特征。
pub trait AccessOrderTrait {
    /// Get the runtime AccessOrder value.
    ///
    /// 获取运行时 AccessOrder 值。
    fn runtime_value() -> AccessOrder;
}

impl AccessOrderTrait for RowMajor {
    #[inline]
    fn runtime_value() -> AccessOrder {
        AccessOrder::RowMajor
    }
}

impl AccessOrderTrait for ColumnMajor {
    #[inline]
    fn runtime_value() -> AccessOrder {
        AccessOrder::ColumnMajor
    }
}

// ============================================================================
// Unified Order Trait System
// 统一的顺序特征系统
// ============================================================================

/// # Sealed Trait Module
///
/// Private module implementing the sealed trait pattern to prevent external
/// implementations of `OrderTrait` and `OrderKind`.
///
/// 私有模块，实现 sealed trait 模式，防止外部实现 `OrderTrait` 和 `OrderKind`。
mod sealed {
    use super::{AccessOrder, ColumnMajor, RowMajor, StorageOrder};

    /// Sealed trait for preventing external implementations.
    /// 防止外部实现的 sealed trait。
    pub trait Sealed {}

    // Implement Sealed for marker types
    // 为标记类型实现 Sealed
    impl Sealed for super::StorageKind {}
    impl Sealed for super::AccessKind {}

    // Implement Sealed for compile-time order types
    // 为编译时顺序类型实现 Sealed
    impl Sealed for RowMajor {}
    impl Sealed for ColumnMajor {}

    // Implement Sealed for runtime order types
    // 为运行时顺序类型实现 Sealed
    impl Sealed for StorageOrder {}
    impl Sealed for AccessOrder {}
}

/// # OrderKind
///
/// A trait for marker types that distinguish between storage order and access order.
/// Used as an associated type in `OrderTrait` to specify the kind of order.
///
/// 用于区分存储顺序和访问顺序的标记类型的特征。
/// 作为 `OrderTrait` 的关联类型，用于指定顺序的种类。
///
/// ## Types / 类型
///
/// - `StorageKind` - Marks types as storage order / 将类型标记为存储顺序
/// - `AccessKind` - Marks types as access order / 将类型标记为访问顺序
pub trait OrderKind: sealed::Sealed + Clone + Copy + Debug + PartialEq + Eq {}

/// # StorageKind
///
/// Marker type for storage order.
/// Used in `OrderTrait::Kind` to indicate a storage order type.
///
/// 存储顺序的标记类型。
/// 在 `OrderTrait::Kind` 中用于指示存储顺序类型。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StorageKind;

/// # AccessKind
///
/// Marker type for access order.
/// Used in `OrderTrait::Kind` to indicate an access order type.
///
/// 访问顺序的标记类型。
/// 在 `OrderTrait::Kind` 中用于指示访问顺序类型。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AccessKind;

impl OrderKind for StorageKind {}
impl OrderKind for AccessKind {}

/// # OrderTrait
///
/// A unified trait for order types that works with both compile-time types
/// (`RowMajor`, `ColumnMajor`) and runtime enumerations (`StorageOrder`, `AccessOrder`).
///
/// 统一的顺序类型特征，适用于编译时类型（`RowMajor`、`ColumnMajor`）
/// 和运行时枚举（`StorageOrder`、`AccessOrder`）。
///
/// ## Design / 设计
///
/// This trait provides a common interface for querying order properties,
/// while the `Kind` associated type distinguishes between storage and access orders.
///
/// 此特征提供了查询顺序属性的通用接口，而 `Kind` 关联类型区分存储顺序和访问顺序。
///
/// ## Examples / 示例
///
/// ```ignore
/// use ospf_rust_multiarray::concept::{OrderTrait, RowMajor, StorageOrder};
///
/// // Compile-time type / 编译时类型
/// assert!(RowMajor.is_row_major());
///
/// // Runtime enumeration / 运行时枚举
/// let order = StorageOrder::RowMajor;
/// assert!(order.is_row_major());
/// ```
pub trait OrderTrait: sealed::Sealed + Clone + Copy + Debug + PartialEq + Eq {
    /// The kind of order (storage or access).
    /// 顺序的种类（存储或访问）。
    type Kind: OrderKind;

    /// Returns `true` if this is a row-major order.
    /// 如果是行优先顺序，返回 `true`。
    fn is_row_major(&self) -> bool;

    /// Returns `true` if this is a column-major order.
    /// 如果是列优先顺序，返回 `true`。
    #[inline]
    fn is_column_major(&self) -> bool {
        !self.is_row_major()
    }
}

// ============================================================================
// OrderTrait implementations for compile-time types
// 为编译时类型实现 OrderTrait
// ============================================================================

impl OrderTrait for RowMajor {
    type Kind = StorageKind;

    #[inline]
    fn is_row_major(&self) -> bool {
        true
    }

    #[inline]
    fn is_column_major(&self) -> bool {
        false
    }
}

impl OrderTrait for ColumnMajor {
    type Kind = StorageKind;

    #[inline]
    fn is_row_major(&self) -> bool {
        false
    }

    #[inline]
    fn is_column_major(&self) -> bool {
        true
    }
}

// ============================================================================
// OrderTrait implementations for runtime enumerations
// 为运行时枚举实现 OrderTrait
// ============================================================================

impl OrderTrait for StorageOrder {
    type Kind = StorageKind;

    #[inline]
    fn is_row_major(&self) -> bool {
        matches!(self, StorageOrder::RowMajor)
    }
}

impl OrderTrait for AccessOrder {
    type Kind = AccessKind;

    #[inline]
    fn is_row_major(&self) -> bool {
        matches!(self, AccessOrder::RowMajor)
    }
}

/// # StorageOrderExt
///
/// Extension trait for runtime storage order types that provides additional utility methods.
///
/// 运行时存储顺序类型的扩展特征，提供额外的实用方法。
pub trait StorageOrderExt: OrderTrait<Kind = StorageKind> {
    /// Get the opposite order (RowMajor <-> ColumnMajor).
    /// 获取相反的顺序（行优先 <-> 列优先）。
    fn opposite(&self) -> Self;
}

impl StorageOrderExt for StorageOrder {
    #[inline]
    fn opposite(&self) -> Self {
        match self {
            StorageOrder::RowMajor => StorageOrder::ColumnMajor,
            StorageOrder::ColumnMajor => StorageOrder::RowMajor,
        }
    }
}

/// # AccessOrderExt
///
/// Extension trait for runtime access order types that provides additional utility methods.
///
/// 运行时访问顺序类型的扩展特征，提供额外的实用方法。
pub trait AccessOrderExt: OrderTrait<Kind = AccessKind> {
    /// Get the opposite order (RowMajor <-> ColumnMajor).
    /// 获取相反的顺序（行优先 <-> 列优先）。
    fn opposite(&self) -> Self;
}

impl AccessOrderExt for AccessOrder {
    #[inline]
    fn opposite(&self) -> Self {
        match self {
            AccessOrder::RowMajor => AccessOrder::ColumnMajor,
            AccessOrder::ColumnMajor => AccessOrder::RowMajor,
        }
    }
}

/// # Vector Trait
///
/// A trait for vectors used in shape operations.
/// It combines various collection traits needed for shape manipulation.
///
/// 用于形状操作的向量的特征。
/// 它结合了形状操作所需的各种集合特征。
pub trait Vector = Collection<Item = usize>
    + Len
    + Indices
    + Index<usize, Output = usize>
    + IndexMut<usize, Output = usize>
    + Debug
    + PartialEq<[usize]>;

/// # DummyVector Trait
///
/// A trait for vectors of dummy indices.
/// Used for specifying slicing operations on array dimensions.
///
/// 虚拟索引向量的特征。
/// 用于指定数组维度的切片操作。
pub trait DummyVector = Collection<Item = DummyIndex>
    + Len
    + Indices
    + Index<usize, Output = DummyIndex>
    + IndexMut<usize, Output = DummyIndex>
    + Debug;

/// # MapVector Trait
///
/// A trait for vectors of map indices.
/// Used for specifying dimension mapping and reordering operations.
///
/// 映射索引向量的特征。
/// 用于指定维度映射和重排操作。
pub trait MapVector = Collection<Item = MapIndex>
    + Len
    + Indices
    + Index<usize, Output = MapIndex>
    + IndexMut<usize, Output = MapIndex>
    + Debug;

/// # ShapeVector Trait
///
/// A trait for vectors representing array shapes.
///
/// 表示数组形状的向量的特征。
pub trait ShapeVector = Collection<Item = usize>
    + Len
    + Indices
    + Index<usize, Output = usize>
    + IndexMut<usize, Output = usize>
    + Debug
    + PartialEq<[usize]>;

/// # DynShapeVector Trait
///
/// A trait for vectors representing dynamic array shapes.
/// Extends ShapeVector with FromIterator for runtime construction.
///
/// 表示动态数组形状的向量的特征。
/// 扩展 ShapeVector，添加 FromIterator 用于运行时构造。
pub trait DynShapeVector = Collection<Item = usize>
    + Len
    + Indices
    + Index<usize, Output = usize>
    + IndexMut<usize, Output = usize>
    + FromIterator<usize>
    + Debug
    + PartialEq<[usize]>;

/// # array_eq! Macro
///
/// A macro for comparing arrays for equality.
///
/// 用于比较数组相等性的宏。
#[macro_export]
macro_rules! array_eq {
    ($a:expr, [$($elem:expr),*]) => {
        $a == [$($elem),*].as_slice()
    };

    (assert_eq!($a:expr, [$($elem:expr),*])) => {
        assert_eq!($a, [$($elem),*].as_slice())
    };
}

/// # assert_array_eq! Macro
///
/// A macro for asserting array equality in tests.
///
/// 用于在测试中断言数组相等性的宏。
#[macro_export]
macro_rules! assert_array_eq {
     ($a:expr, [$($elem:expr),*]) => {
        assert_eq!($a, [$($elem),*].as_slice())
    };
}

pub use array_eq;
pub use assert_array_eq;

// ============================================================================
// Unit Tests / 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // OrderTrait tests for compile-time types
    // 为编译时类型测试 OrderTrait
    // ==========================================================================

    #[test]
    fn test_row_major_order_trait() {
        let order = RowMajor;

        // Test OrderTrait methods / 测试 OrderTrait 方法
        assert!(OrderTrait::is_row_major(&order));
        assert!(!OrderTrait::is_column_major(&order));
    }

    #[test]
    fn test_column_major_order_trait() {
        let order = ColumnMajor;

        // Test OrderTrait methods / 测试 OrderTrait 方法
        assert!(!OrderTrait::is_row_major(&order));
        assert!(OrderTrait::is_column_major(&order));
    }

    // ==========================================================================
    // OrderTrait tests for runtime enumerations
    // 为运行时枚举测试 OrderTrait
    // ==========================================================================

    #[test]
    fn test_storage_order_enum_row_major() {
        let order = StorageOrder::RowMajor;

        // Test OrderTrait methods / 测试 OrderTrait 方法
        assert!(OrderTrait::is_row_major(&order));
        assert!(!OrderTrait::is_column_major(&order));

        // Test StorageOrderExt / 测试 StorageOrderExt
        assert_eq!(StorageOrderExt::opposite(&order), StorageOrder::ColumnMajor);
    }

    #[test]
    fn test_storage_order_enum_column_major() {
        let order = StorageOrder::ColumnMajor;

        // Test OrderTrait methods / 测试 OrderTrait 方法
        assert!(!OrderTrait::is_row_major(&order));
        assert!(OrderTrait::is_column_major(&order));

        // Test StorageOrderExt / 测试 StorageOrderExt
        assert_eq!(StorageOrderExt::opposite(&order), StorageOrder::RowMajor);
    }

    #[test]
    fn test_access_order_enum_row_major() {
        let order = AccessOrder::RowMajor;

        // Test OrderTrait methods / 测试 OrderTrait 方法
        assert!(OrderTrait::is_row_major(&order));
        assert!(!OrderTrait::is_column_major(&order));

        // Test AccessOrderExt / 测试 AccessOrderExt
        assert_eq!(AccessOrderExt::opposite(&order), AccessOrder::ColumnMajor);
    }

    #[test]
    fn test_access_order_enum_column_major() {
        let order = AccessOrder::ColumnMajor;

        // Test OrderTrait methods / 测试 OrderTrait 方法
        assert!(!OrderTrait::is_row_major(&order));
        assert!(OrderTrait::is_column_major(&order));

        // Test AccessOrderExt / 测试 AccessOrderExt
        assert_eq!(AccessOrderExt::opposite(&order), AccessOrder::RowMajor);
    }

    // ==========================================================================
    // Kind type tests
    // 类型种类测试
    // ==========================================================================

    #[test]
    fn test_order_kind_storage() {
        // Verify that RowMajor and ColumnMajor have StorageKind
        // 验证 RowMajor 和 ColumnMajor 具有 StorageKind
        fn assert_storage_kind<T: OrderTrait<Kind = StorageKind>>() {}
        assert_storage_kind::<RowMajor>();
        assert_storage_kind::<ColumnMajor>();
        assert_storage_kind::<StorageOrder>();
    }

    #[test]
    fn test_order_kind_access() {
        // Verify that AccessOrder has AccessKind
        // 验证 AccessOrder 具有 AccessKind
        fn assert_access_kind<T: OrderTrait<Kind = AccessKind>>() {}
        assert_access_kind::<AccessOrder>();
    }

    // ==========================================================================
    // Generic function tests
    // 泛型函数测试
    // ==========================================================================

    /// Generic function that works with any storage order type
    /// 适用于任何存储顺序类型的泛型函数
    fn process_storage_order<O: OrderTrait<Kind = StorageKind>>(order: &O) -> bool {
        order.is_row_major()
    }

    /// Generic function that works with any access order type
    /// 适用于任何访问顺序类型的泛型函数
    fn process_access_order<O: OrderTrait<Kind = AccessKind>>(order: &O) -> bool {
        order.is_row_major()
    }

    #[test]
    fn test_generic_function_with_compile_time_type() {
        // Compile-time types / 编译时类型
        assert!(process_storage_order(&RowMajor));
        assert!(!process_storage_order(&ColumnMajor));
    }

    #[test]
    fn test_generic_function_with_runtime_enum() {
        // Runtime enumerations / 运行时枚举
        assert!(process_storage_order(&StorageOrder::RowMajor));
        assert!(!process_storage_order(&StorageOrder::ColumnMajor));
        assert!(process_access_order(&AccessOrder::RowMajor));
        assert!(!process_access_order(&AccessOrder::ColumnMajor));
    }

    // ==========================================================================
    // Backward compatibility tests
    // 向后兼容性测试
    // ==========================================================================

    #[test]
    fn test_storage_order_trait_still_works() {
        // Verify StorageOrderTrait still works
        // 验证 StorageOrderTrait 仍然工作
        assert_eq!(<RowMajor as StorageOrderTrait>::runtime_value(), StorageOrder::RowMajor);
        assert_eq!(<ColumnMajor as StorageOrderTrait>::runtime_value(), StorageOrder::ColumnMajor);
    }

    #[test]
    fn test_access_order_trait_still_works() {
        // Verify AccessOrderTrait still works
        // 验证 AccessOrderTrait 仍然工作
        assert_eq!(<RowMajor as AccessOrderTrait>::runtime_value(), AccessOrder::RowMajor);
        assert_eq!(<ColumnMajor as AccessOrderTrait>::runtime_value(), AccessOrder::ColumnMajor);
    }

    #[test]
    fn test_access_order_from_storage_order() {
        // Verify conversion still works
        // 验证转换仍然工作
        assert_eq!(
            AccessOrder::from_storage_order(StorageOrder::RowMajor),
            AccessOrder::RowMajor
        );
        assert_eq!(
            AccessOrder::from_storage_order(StorageOrder::ColumnMajor),
            AccessOrder::ColumnMajor
        );
    }
}
