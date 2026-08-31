//! Surface density units - 表面密度单位
//! Surface density units - SI surface density units
//!
//! 提供表面密度量纲的 SI 单位定义，包括千克每平方米、克每平方米等。
//! Provides SI unit definitions for surface density dimension, including kilogram per square meter, gram per square meter, etc.

use super::area::SquareMeter;
use super::mass::{Gram, Kilogram};
use crate::scale::Scale;
use crate::unit::{CTUnit, CTUnitDiv};
use once_cell::sync::Lazy;

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
