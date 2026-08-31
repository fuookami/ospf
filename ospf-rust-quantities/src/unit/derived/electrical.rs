//! Electrical units - 电学单位
//! Electrical units - SI electrical units
//!
//! 提供电学量纲的 SI 单位定义，包括安培、伏特、欧姆、库仑、法拉、亨利等。
//! Provides SI unit definitions for electrical dimensions, including ampere, volt, ohm, coulomb, farad, henry, etc.

use super::power::Watt;
use super::time::Second;
use crate::dimension::derived::{ElectricCharge, ElectricCurrent, ElectricPotential};
use crate::scale::{Scale, KILO, MICRO, MILLI};
use crate::unit::{CTUnit, CTUnitDiv, CTUnitMul};
use once_cell::sync::Lazy;

// ============================================================================
// 电流单位 / Electric current units
// ============================================================================

define_unit!(Ampere, "ampere", "A", ElectricCurrent);
define_unit!(
    Milliampere,
    "milliampere",
    "mA",
    ElectricCurrent,
    &*Ampere::SCALE * &*MILLI
);
define_unit!(
    MicroAmpere,
    "microampere",
    "μA",
    ElectricCurrent,
    &*Ampere::SCALE * &*MICRO
);

// ============================================================================
// 电荷单位 / Electric charge units
// ============================================================================

define_unit_by!(Coulomb, "coulomb", "C", CTUnitMul<Ampere, Second>);
define_unit!(
    MilliCoulomb,
    "millicoulomb",
    "mC",
    ElectricCharge,
    &*Coulomb::SCALE * &*MILLI
);

// ============================================================================
// 电压单位 / Electric potential units
// ============================================================================

define_unit_by!(Volt, "volt", "V", CTUnitDiv<Watt, Ampere>);
define_unit!(
    MilliVolt,
    "millivolt",
    "mV",
    ElectricPotential,
    &*Volt::SCALE * &*MILLI
);
define_unit!(
    KiloVolt,
    "kilovolt",
    "kV",
    ElectricPotential,
    &*Volt::SCALE * &*KILO
);
