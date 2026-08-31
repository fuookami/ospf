//! # ospf-rust-multiarray
//!
//! 高性能、泛型的 Rust 多维数组库，支持编译期和运行期形状。
//! A high-performance, generic multi-dimensional array library for Rust with compile-time and runtime shape support.
//!
//! ## 核心特性 / Core Features
//!
//! - **泛型多维数组 / Generic Multi-Dimensional Arrays**: 支持编译期 (`Shape<N>`) 和运行期 (`DynShape`) 维度定义
//!   Support for both compile-time and runtime dimensionality
//! - **灵活的存储顺序 / Flexible Storage Order**: 行优先和列优先存储顺序
//!   Row-major and column-major storage orders
//! - **数组视图 / Array Views**: 零拷贝视图，支持切片和映射操作
//!   Zero-copy views with slicing and mapping support
//! - **块数组 / Block Arrays**: 大数组分块存储
//!   Chunked storage for large arrays
//! - **数据框 / DataFrame**: 带命名列的表格数据结构
//!   Tabular data structure with named columns

#![feature(generic_const_exprs)]
#![feature(associated_type_defaults)]
#![feature(coroutines, coroutine_trait)]
#![feature(allocator_api)]
#![feature(trait_alias)]
#![feature(specialization)]
#![cfg_attr(debug_assertions, allow(dead_code, unused, incomplete_features))]

pub use ospf_rust_base::{ChunkedVec, ChunkedVecIter, ChunkedVecIterMut, DEFAULT_CHUNK_SIZE};

pub use concept::{
    AccessOrder, AccessOrderTrait, ColumnMajor, DummyVector, DynShapeContainer, DynShapeVector,
    MapVector, RowMajor, ShapeVector, StorageOrder, StorageOrderTrait, Vector,
};
pub use dummy_index::DummyIndex;
pub use error::*;
pub use index_value::TryIntoIndexValue;
pub use map_index::{
    _0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13, _14, _15, _16, _17, _18, _19, _20,
    MapIndex, PlaceHolder,
};
pub use multi_array::{
    DynMultiArray, MultiArray, MultiArray1, MultiArray2, MultiArray3, MultiArray4,
    MultiArrayBuilder, MultiArrayCollection, MultiArrayEnumerateWithOrderIter, MultiArrayToView,
    MultiArrayWithOrderIter, multi_array_1, multi_array_2, multi_array_3, multi_array_of_shape,
};
pub use multi_array_view::{MultiArrayView, MultiArrayViewBuilderCM, MultiArrayViewBuilderRM};

pub use shape::{
    AbstractShape, DynShape, Shape, Shape0, Shape1, Shape2, Shape3, Shape4, Shape5, Shape6, Shape7,
    Shape8, Shape9, Shape10, Shape11, Shape12, Shape13, Shape14, Shape15, Shape16, Shape17,
    Shape18, Shape19, Shape20, ShapeAccessOrderExt, ShapeIndicesIter,
};

pub use data_frame::{
    DataFrame, DataFrame2, DataFrameBuilder, DataFrameRowsBuilder, DataFrameView,
    data_frame_from_rows, data_frame_of,
};

pub use block_multi_array::{
    BlockDynMultiArray, BlockMultiArray, BlockMultiArray1, BlockMultiArray2, BlockMultiArray3,
    BlockMultiArray4, BlockMultiArrayBuilder, BlockMultiArrayView, CTBlockMultiArrayBuilder,
};

pub use fast_sum::{FastCumSum, FastSum, SumError};
pub use list_ext::{List2, List2Ext, List3, List3Ext};
pub use map_ext::{
    MapAllValuesExt, MapMultiArrayExt, MapMultiArrayMutExt, MultiMap2ArrayExt,
    MultiMap2ArrayMutExt, MultiMap3ArrayExt, MultiMap3ArrayMutExt, MultiMap4ArrayExt,
    MultiMap4ArrayMutExt,
};
pub use multimap::{MultiMap2, MultiMap3, MultiMap4};

pub mod concept;
#[macro_use]
pub mod dummy_index;
pub mod error;
pub mod index_value;
#[macro_use]
pub mod map_index;
pub mod multi_array;
pub mod multi_array_view;
pub mod shape;

pub mod block_multi_array;
pub mod data_frame;
pub mod einsum;
pub mod fast_sum;
pub mod list_ext;
pub mod map_ext;
pub mod multimap;

#[cfg(test)]
mod tests {
    use crate::concept::RowMajor;
    use crate::multi_array::MultiArrayToView;
    use crate::shape::AbstractShape;
    use crate::{DummyIndex, DynShape, MultiArray, MultiArrayBuilder, Shape};
    use cc_traits::Iter;

    #[test]
    fn test() {
        let shape: DynShape = DynShape::new(vec![32, 32, 32]);
        let array: MultiArray<i32, _> = MultiArrayBuilder::new(shape);
        let dummy_vector = dyn_dummy_expect![.., .., ..];

        let view = array.view(&dummy_vector).unwrap();

        let mut sum = 0;
        for &val in view.iter() {
            sum += val;
        }
    }

    #[test]
    fn test_unified_shape() {
        let rt_shape: Shape<2> = Shape::new([3, 4]);
        assert_eq!(rt_shape.len(), 12);

        let rm_shape: Shape<2, RowMajor> = Shape::new([3, 4]);
        assert_eq!(rm_shape.len(), 12);
    }
}
