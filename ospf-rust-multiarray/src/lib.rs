#![feature(generic_const_exprs)]
#![feature(associated_type_defaults)]
#![feature(coroutines, coroutine_trait)]
#![feature(allocator_api)]
#![feature(trait_alias)]
#![feature(specialization)]
#![cfg_attr(debug_assertions, allow(dead_code, unused, incomplete_features))]

// ==================== 分块存储模块（从 ospf-rust-base 导入）====================
pub use ospf_rust_base::{ChunkedVec, ChunkedVecIter, ChunkedVecIterMut, DEFAULT_CHUNK_SIZE};

// ==================== 运行时 API ====================
pub use concept::{
    AccessKind, AccessOrder, AccessOrderExt, AccessOrderTrait, ColumnMajor, DummyVector,
    DynShapeVector, MapVector, OrderKind, OrderTrait, RowMajor, ShapeVector, StorageKind,
    StorageOrder, StorageOrderExt, StorageOrderTrait, Vector,
};
pub use dummy_index::DummyIndex;
pub use error::*;
pub use index_value::TryIntoIndexValue;
pub use map_index::{
    _0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13, _14, _15, _16, _17, _18, _19, _20,
    MapIndex, PlaceHolder,
};
pub use multi_array::{MultiArray, MultiArrayBuilder, MultiArrayCollection, MultiArrayToView};
pub use multi_array_view::MultiArrayView;
pub use shape::{
    AbstractRTShape, AbstractShape, DynShape, Shape, Shape0, Shape1, Shape2, Shape3, Shape4,
    Shape5, Shape6, Shape7, Shape8, Shape9, Shape10, Shape11, Shape12, Shape13, Shape14, Shape15,
    Shape16, Shape17, Shape18, Shape19, Shape20,
};

// ==================== 类型级 API (STC = Shape-Time Compile) ====================
pub use ct_multi_array::{
    CTMultiArray, CTMultiArrayBuilder, CTMultiArrayIter, CTMultiArrayIterMut, MultiArrayCM,
    MultiArrayRM,
};
pub use ct_multi_array_view::{
    CTMultiArrayView, MultiArrayViewBuilderCM, MultiArrayViewBuilderRM, MultiArrayViewCM,
    MultiArrayViewRM,
};
pub use ct_shape::{
    AbstractCTShape, CTDynShape, CTShape, DynShapeCM, DynShapeRM, ShapeCM, ShapeCM0, ShapeCM1,
    ShapeCM2, ShapeCM3, ShapeCM4, ShapeCM5, ShapeCM6, ShapeCM7, ShapeCM8, ShapeCM9, ShapeCM10,
    ShapeCM11, ShapeCM12, ShapeCM13, ShapeCM14, ShapeCM15, ShapeCM16, ShapeCM17, ShapeCM18,
    ShapeCM19, ShapeCM20, ShapeRM, ShapeRM0, ShapeRM1, ShapeRM2, ShapeRM3, ShapeRM4, ShapeRM5,
    ShapeRM6, ShapeRM7, ShapeRM8, ShapeRM9, ShapeRM10, ShapeRM11, ShapeRM12, ShapeRM13, ShapeRM14,
    ShapeRM15, ShapeRM16, ShapeRM17, ShapeRM18, ShapeRM19, ShapeRM20,
};

// ==================== DataFrame API ====================
pub use data_frame::{
    DataFrame, DataFrameBuilder, DataFrameCM, DataFrameRM, DataFrameView, DataFrameViewCM,
    DataFrameViewRM,
};

// ==================== BlockMultiArray API ====================
pub use block_multi_array::{
    BlockMultiArray, BlockMultiArrayBuilder, BlockMultiArrayCM, BlockMultiArrayRM,
    BlockMultiArrayView, BlockMultiArrayViewCM, BlockMultiArrayViewRM, CTBlockMultiArrayBuilder,
};

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
pub mod ct_multi_array;
pub mod ct_multi_array_view;
pub mod ct_shape;
pub mod data_frame;

#[cfg(test)]
mod tests {
    use cc_traits::Iter;
    use criterion::black_box;
    use crate::{CTMultiArrayBuilder, CTShape, MultiArrayRM, MultiArrayViewBuilderRM, DummyIndex, RowMajor, Shape, MultiArrayBuilder, MultiArrayToView, DynShapeRM};

    #[test]
    fn test() {
        let shape = DynShapeRM::<Vec<usize>, Vec<DummyIndex>>::new(vec![32, 32, 32]);
        let array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new(shape);
        let dummy_vector = dyn_dummy_expect![.., .., ..];
        let view = MultiArrayViewBuilderRM::new_by_dummy(&array, &dummy_vector);

        let mut sum = 0;
        for &val in view.iter() {
            sum += val;
        }
    }
}
