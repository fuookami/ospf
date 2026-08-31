//! 表面密度单位 / Surface density units
//!
//! 提供表面密度量纲的 SI 单位定义，包括千克每平方米、克每平方米等 / Provides SI unit definitions for surface density dimension, including kilogram per square meter, gram per square meter, etc

use crate::unit::{CTUnit, CTUnitDiv};
use super::area::SquareMeter;
use super::mass::{Gram, Kilogram};

// ============================================================================
// 表面密度单位 / Surface density units
// ============================================================================

define_unit_by!(
    KilogramPerSquareMeter,
    "kilogram per square meter",
    "kg/m²",
    CTUnitDiv<Kilogram, SquareMeter>
);
define_unit_by!(
    GramPerSquareMeter,
    "gram per square meter",
    "g/m²",
    CTUnitDiv<Gram, SquareMeter>
);
