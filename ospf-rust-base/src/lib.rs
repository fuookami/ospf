#![feature(specialization)]
#![feature(coroutines, coroutine_trait)]
#![feature(sync_unsafe_cell)]
#![feature(ptr_as_ref_unchecked)]
#![cfg_attr(debug_assertions, allow(dead_code, unused, incomplete_features))]

#[macro_use]
extern crate strum;

pub use error::*;
pub use generator_iterator::GeneratorIterator;
pub use indexed_type::{Index, ManualIndex, Indexed, ManualIndexed};
pub use iter::*;

pub mod error;
pub mod generator_iterator;
pub mod indexed_type;
pub mod iter;
