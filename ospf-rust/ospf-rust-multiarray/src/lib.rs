#![feature(generic_const_exprs)]
#![feature(associated_type_defaults)]
#![feature(generators, generator_trait)]
#[macro_use]
pub mod dummy_vector;
#[macro_use]
pub mod map_vector;
pub mod multi_array;
pub mod multi_array_view;
pub mod shape;

pub use dummy_vector::DummyIndex;
pub use multi_array::*;
pub use multi_array_view::*;
pub use shape::*;
