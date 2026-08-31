#![feature(specialization)]
#![feature(coroutines, coroutine_trait)]
#![feature(sync_unsafe_cell)]
#![feature(trait_alias)]
#![feature(associated_type_defaults)]
#![feature(allocator_api)]
#![cfg_attr(
    debug_assertions,
    allow(dead_code, unused, incomplete_features, static_mut_refs)
)]

#[macro_use]
extern crate strum;

pub use cloneable_function::*;
pub use collection::*;
pub use container::*;
pub use error::*;
pub use generator_iterator::*;
pub use indexed_type::{Indexed, ManualIndexed};
pub use iter::*;

#[macro_use]
pub mod error;
pub mod generator_iterator;
#[macro_use]
pub mod indexed_type;
pub mod iter;
#[macro_use]
pub mod cloneable_function;
pub mod chunked_collection;
pub mod collection;
pub mod container;

pub use chunked_collection::{ChunkedVec, ChunkedVecIter, ChunkedVecIterMut, DEFAULT_CHUNK_SIZE};
