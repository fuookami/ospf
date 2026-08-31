//! Area units - 面积单位
//! Area units - SI area units
//!
//! 提供面积量纲的 SI 单位定义，包括平方米、平方千米、公顷等。
//! Provides SI unit definitions for area dimension, including square meter, square kilometer, hectare, etc.

use super::length::Kilometer;
use crate::scale::Scale;
use crate::unit::physical_unit::CTUnit;
use crate::unit::{CTUnitMul, Meter};
use once_cell::sync::Lazy;

// ============================================================================
// 面积单位 / Area units
// ============================================================================

define_unit_by!(SquareMeter, "square meter", "m²", CTUnitMul<Meter, Meter>);
define_unit_by!(
    SquareKilometer,
    "square kilometer",
    "km²",
    CTUnitMul<Kilometer, Kilometer>
);
