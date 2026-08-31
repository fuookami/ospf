//! 混沌系统与迭代映射。
//! Chaotic systems and iterative maps.

#[macro_use]
mod macros;
mod helpers;

// Point3-based systems
mod aizawa;
mod anishchenko_astakhov;
mod arneodo;
mod biology_chaotic;
mod bouali;
mod burke_shaw;
mod capacitance_equation;
mod chen;
mod chen_celikovsky;
mod chen_lee;
mod chua_attractor;
mod chua_circuit;
mod coullet;
mod coupled_lorenz;
mod lorenz;

// Point2-based systems
mod arnolds_cat_map;
mod bakers_map;
mod bogdanov_map;
mod brusselator;
mod circuit_chaotic;
mod complex_quadratic;
mod complex_squaring;

// Scalar maps
mod arnold_tongue;
mod chebyshev_map;
mod circle_map;
mod gauss_map;

// Special systems
mod double_pendulum;

// Re-exports
pub use aizawa::*;
pub use anishchenko_astakhov::*;
pub use arneodo::*;
pub use biology_chaotic::*;
pub use bouali::*;
pub use burke_shaw::*;
pub use capacitance_equation::*;
pub use chen::*;
pub use chen_celikovsky::*;
pub use chen_lee::*;
pub use chua_attractor::*;
pub use chua_circuit::*;
pub use coullet::*;
pub use coupled_lorenz::*;
pub use lorenz::*;

pub use arnolds_cat_map::*;
pub use bakers_map::*;
pub use bogdanov_map::*;
pub use brusselator::*;
pub use circuit_chaotic::*;
pub use complex_quadratic::*;
pub use complex_squaring::*;

pub use arnold_tongue::*;
pub use chebyshev_map::*;
pub use circle_map::*;
pub use gauss_map::*;

pub use double_pendulum::*;
