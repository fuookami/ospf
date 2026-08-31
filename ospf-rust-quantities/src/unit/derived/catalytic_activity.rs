//! 催化活度单位 / Catalytic activity units
//!
//! 提供催化活度量纲的 SI 单位定义，包括开特、毫开特、微开特、酶单位等 / Provides SI unit definitions for catalytic activity dimension, including katal, millikatal, microkatal, enzyme unit, etc

use crate::dimension::derived::CatalyticActivity;
use crate::scale::{MICRO, MILLI, Scale};
use crate::unit::{CTUnit, CTUnitDiv};
use super::amount_of_substance::Mole;
use super::time::Second;

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
