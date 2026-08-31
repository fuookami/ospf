//! Mass units - 质量单位
//! Mass units - SI mass units (kilogram, gram, etc.)
//!
//! 提供质量量纲的 SI 单位定义，包括千克、克、毫克、吨等。
//! Provides SI unit definitions for mass dimension, including kilogram, gram, milligram, tonne, etc.

use crate::dimension::derived::Mass;
use crate::scale::{Scale, KILO, MILLI};
use crate::unit::CTUnit;
use once_cell::sync::Lazy;

// ============================================================================
// 质量单位 / Mass units
// ============================================================================

define_unit!(Kilogram, "kilogram", "kg", Mass);
define_unit!(Gram, "gram", "g", Mass, MILLI.clone());
define_unit!(
    MilliGram,
    "milligram",
    "mg",
    Mass,
    Scale::from_int(1) / Scale::from_int(1000000)
);
define_unit!(Tonne, "tonne", "t", Mass, KILO.clone());
