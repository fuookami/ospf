use crate::{
    CTMultiArray, CTMultiArrayBuilder, ColumnMajor, MultiArray, MultiArrayBuilder, MultiArrayCM,
    MultiArrayRM, MultiArrayView, MultiArrayViewCM, MultiArrayViewRM, RowMajor
};
use ospf_rust_base::ChunkedVec;

pub type BlockMultiArray<T, S> = MultiArray<T, S, ChunkedVec<T>>;
pub type BlockMultiArrayBuilder = MultiArrayBuilder;
pub type BlockMultiArrayView<'a, T, S> = MultiArrayView<'a, T, S, ChunkedVec<T>>;

pub type BlockMultiArrayRM<T, S> = MultiArrayRM<T, S, ChunkedVec<T>>;
pub type BlockMultiArrayCM<T, S> = MultiArrayCM<T, S, ChunkedVec<T>>;
pub type CTBlockMultiArrayBuilder = CTMultiArrayBuilder;
pub type BlockMultiArrayViewRM<'a, T, S> = MultiArrayViewRM<'a, T, S, ChunkedVec<T>>;
pub type BlockMultiArrayViewCM<'a, T, S> = MultiArrayViewCM<'a, T, S, ChunkedVec<T>>;
