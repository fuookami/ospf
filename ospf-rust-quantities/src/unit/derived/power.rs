//! Power units - 功率单位
//! Power units - SI power units
//!
//! 提供功率量纲的 SI 单位定义，包括瓦特、千瓦、兆瓦、马力等。
//! Provides SI unit definitions for power dimension, including watt, kilowatt, megawatt, horsepower, etc.

use super::energy::Joule;
use super::time::Second;
use crate::dimension::derived::Power;
use crate::scale::{Scale, KILO, MEGA, MILLI};
use crate::unit::{CTUnit, CTUnitDiv};
use once_cell::sync::Lazy;

// ============================================================================
// 功率单位 / Power units
// ============================================================================

define_unit_by!(Watt, "watt", "W", CTUnitDiv<Joule, Second>);
define_unit!(
    Kilowatt,
    "kilowatt",
    "kW",
    Power,
    &*Watt::SCALE * &*KILO
);
define_unit!(
    Megawatt,
    "megawatt",
    "MW",
    Power,
    &*Watt::SCALE * &*MEGA
);
define_unit!(
    Milliwatt,
    "milliwatt",
    "mW",
    Power,
    &*Watt::SCALE * &*MILLI
);
