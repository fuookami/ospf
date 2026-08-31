#![feature(generic_const_exprs)]
#![feature(associated_type_defaults)]
#![feature(coroutines, coroutine_trait)]
#![feature(allocator_api)]
#![cfg_attr(debug_assertions, allow(dead_code, unused, incomplete_features))]

pub use dummy_index::*;
pub use index_vector::*;
pub use map_index::*;
pub use multi_array::*;
pub use multi_array_view::*;
pub use shape::*;

#[macro_use]
pub mod dummy_index;
#[macro_use]
pub mod map_index;
mod error;
pub mod index_vector;
pub mod multi_array;
pub mod multi_array_view;
pub mod shape;
