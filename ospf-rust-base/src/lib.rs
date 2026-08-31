#![feature(specialization)]
#![feature(entry_insert)]
#![feature(coroutines, coroutine_trait)]
#![cfg_attr(debug_assertions, allow(dead_code, unused, incomplete_features))]

#[macro_use]
extern crate strum;

pub use error::*;
pub use generator_iterator::GeneratorIterator;
pub use indexed_type::{Indexed, IndexGenerator};

pub mod error;
mod generator_iterator;
pub mod indexed_type;
