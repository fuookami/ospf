#![feature(associated_type_defaults)]
#![feature(fn_traits)]
#![feature(unboxed_closures)]
#![feature(specialization)]
#![feature(generators)]
#![feature(generator_trait)]
pub mod algebra;
pub mod combinatorics;
pub mod functional;
pub mod geometry;

pub use algebra::*;
pub use combinatorics::*;
pub use functional::*;
pub use geometry::*;
