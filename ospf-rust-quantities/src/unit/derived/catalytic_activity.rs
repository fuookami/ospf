//! Catalytic activity units - 催化活度单位
//! Catalytic activity units - SI catalytic activity units
//!
//! 提供催化活度量纲的 SI 单位定义，包括开特、毫开特、微开特、酶单位等。
//! Provides SI unit definitions for catalytic activity dimension, including katal, millikatal, microkatal, enzyme unit, etc.

use super::amount_of_substance::Mole;
use super::time::Second;
use crate::dimension::derived::CatalyticActivity;
use crate::scale::{Scale, MICRO, MILLI};
use crate::unit::{CTUnit, CTUnitDiv};
use once_cell::sync::Lazy;

// ============================================================================
// 催化活度单位 / Catalytic activity units
// ============================================================================

define_unit_by!(Katal, "katal", "kat", CTUnitDiv<Mole, Second>);
define_unit!(
    Millikatal,
    "millikatal",
    "mkat",
    CatalyticActivity,
    &*Katal::SCALE * &*MILLI
);
define_unit!(
    Microkatal,
    "microkatal",
    "μkat",
    CatalyticActivity,
    &*Katal::SCALE * &*MICRO
);
define_unit!(
    EnzymeUnit,
    "enzyme unit",
    "U",
    CatalyticActivity,
    &*Katal::SCALE / &Scale::from_int(60000000)
);
