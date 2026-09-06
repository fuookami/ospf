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
mod dadras;
mod dequan_li;
mod duffing_equation;
mod finance;
mod four_wing;
mod genesio_tesi;
mod hadley;
mod halvorsen;
mod hindmarsh_rose;
mod liu_chen;
mod lorenz;
mod lorenz84;
mod lorenz_attractor;
mod lorenz_mod1;
mod lorenz_mod2;
mod lu_chen_attractor;
mod lu_chen_system;
mod newton_iterate;
mod newton_leipnik;
mod nose_hoover;
mod qi_chen;
mod rabinovich_fabrikant;
mod rayleigh_benard;
mod rossler;
mod rucklidge;
mod sakarya;
mod shimizu_morioka;
mod thomas;
mod thomas_cyclically_symmetric;
mod three_scroll_tsucs1;
mod three_scroll_tsucs2;
mod wang_sun;
mod wimol_banlue;
mod yu_wang;

// Point4-based systems
mod four_scroll_hyper_chaotic;
mod lorenz_stenflo;
mod qi_attractor;

// Point2-based systems
mod arnolds_cat_map;
mod bakers_map;
mod bogdanov_map;
mod brusselator;
mod circuit_chaotic;
mod complex_quadratic;
mod complex_squaring;
mod duffing_map;
mod gingerbreadman;
mod henon;
mod ikeda;
mod kaplan_yorke;
mod kicked_rotator;
mod lotka_volterra;
mod lozi;
mod martin;
mod symplectic;
mod tinkerbell;
mod van_der_pol;

// Scalar maps
mod arnold_tongue;
mod chebyshev_map;
mod circle_map;
mod dyadic;
mod exponential;
mod gauss_iterated;
mod gauss_map;
mod logistic;
mod sine_map;
mod singer;
mod sinus_map;
mod sinusoidal;
mod tent;
mod zaslavskii;

// Special systems
mod double_pendulum;
mod interval_exchange;
mod lorenz96;
mod n_body;

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
pub use dadras::*;
pub use dequan_li::*;
pub use duffing_equation::*;
pub use finance::*;
pub use four_wing::*;
pub use genesio_tesi::*;
pub use hadley::*;
pub use halvorsen::*;
pub use hindmarsh_rose::*;
pub use liu_chen::*;
pub use lorenz::*;
pub use lorenz_attractor::*;
pub use lorenz_mod1::*;
pub use lorenz_mod2::*;
pub use lorenz84::*;
pub use lu_chen_attractor::*;
pub use lu_chen_system::*;
pub use newton_iterate::*;
pub use newton_leipnik::*;
pub use nose_hoover::*;
pub use qi_chen::*;
pub use rabinovich_fabrikant::*;
pub use rayleigh_benard::*;
pub use rossler::*;
pub use rucklidge::*;
pub use sakarya::*;
pub use shimizu_morioka::*;
pub use thomas::*;
pub use thomas_cyclically_symmetric::*;
pub use three_scroll_tsucs1::*;
pub use three_scroll_tsucs2::*;
pub use wang_sun::*;
pub use wimol_banlue::*;
pub use yu_wang::*;

pub use four_scroll_hyper_chaotic::*;
pub use lorenz_stenflo::*;
pub use qi_attractor::*;

pub use arnolds_cat_map::*;
pub use bakers_map::*;
pub use bogdanov_map::*;
pub use brusselator::*;
pub use circuit_chaotic::*;
pub use complex_quadratic::*;
pub use complex_squaring::*;
pub use duffing_map::*;
pub use gingerbreadman::*;
pub use henon::*;
pub use ikeda::*;
pub use kaplan_yorke::*;
pub use kicked_rotator::*;
pub use lotka_volterra::*;
pub use lozi::*;
pub use martin::*;
pub use symplectic::*;
pub use tinkerbell::*;
pub use van_der_pol::*;

pub use arnold_tongue::*;
pub use chebyshev_map::*;
pub use circle_map::*;
pub use dyadic::*;
pub use exponential::*;
pub use gauss_iterated::*;
pub use gauss_map::*;
pub use logistic::*;
pub use sine_map::*;
pub use singer::*;
pub use sinus_map::*;
pub use sinusoidal::*;
pub use tent::*;
pub use zaslavskii::*;

pub use double_pendulum::*;
pub use interval_exchange::*;
pub use lorenz96::*;
pub use n_body::*;
