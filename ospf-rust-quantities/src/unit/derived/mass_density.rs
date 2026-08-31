//! Mass density units - 质量密度单位
//! Mass density units - SI mass density units
//!
//! 提供质量密度量纲的 SI 单位定义，包括千克每立方米、千克每升、克每立方厘米等。
//! Provides SI unit definitions for mass density dimension, including kilogram per cubic meter, kilogram per liter, gram per cubic centimeter, etc.

use super::mass::Kilogram;
use super::volume::CubicMeter;
use crate::dimension::derived::MassDensity;
use crate::scale::Scale;
use crate::unit::physical_unit::CTUnitDiv;
use crate::unit::CTUnit;
use once_cell::sync::Lazy;

// ============================================================================
// 质量密度单位 / Mass density units
// ============================================================================

define_unit_by!(
    KilogramPerCubicMeter,
    "kilogram per cubic meter",
    "kg/m³",
    CTUnitDiv<Kilogram, CubicMeter>
);
define_unit!(
    KilogramPerLiter,
    "kilogram per liter",
    "kg/L",
    MassDensity,
    Scale::from_int(1000)
);
define_unit!(
    GramPerCubicCentimeter,
    "gram per cubic centimeter",
    "g/cm³",
    MassDensity,
    Scale::from_int(1000)
);
