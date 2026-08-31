//! 压力单位 / Pressure units
//!
//! 提供压力量纲的 SI 单位定义，包括帕斯卡、千帕、兆帕、巴等 / Provides SI unit definitions for pressure dimension, including pascal, kilopascal, megapascal, bar, etc

use super::area::SquareMeter;
use super::force::Newton;
use crate::dimension::derived::Pressure;
use crate::scale::{HECTO, KILO, MEGA, MILLI, Scale};
use crate::unit::{CTUnit, CTUnitDiv};

// ============================================================================
// 压力单位 / Pressure units
// ============================================================================

define_unit_by!(Pascal, "pascal", "Pa", CTUnitDiv<Newton, SquareMeter>);
define_unit!(
    Hectopascal,
    "hectopascal",
    "hPa",
    Pressure,
    &*Pascal::SCALE * &*HECTO
);
define_unit!(
    Kilopascal,
    "kilopascal",
    "kPa",
    Pressure,
    &*Pascal::SCALE * &*KILO
);
define_unit!(
    Megapascal,
    "megapascal",
    "MPa",
    Pressure,
    &*Pascal::SCALE * &*MEGA
);
define_unit!(
    Bar,
    "bar",
    "bar",
    Pressure,
    &*Pascal::SCALE * &Scale::from_int(100000)
);
define_unit!(
    StandardAtmosphericPressure,
    "standard atmospheric pressure",
    "atm",
    Pressure,
    &*Pascal::SCALE * &Scale::from_int(101325)
);
define_unit!(
    MeterMercury,
    "meter mercury",
    "mHg",
    Pressure,
    &*Pascal::SCALE * &Scale::from_f64(133322.387415)
);
define_unit!(
    MillimeterMercury,
    "millimeter mercury",
    "mmHg",
    Pressure,
    &*MeterMercury::SCALE * &*MILLI
);
define_unit!(
    InchOfMercury,
    "inch of mercury",
    "inHg",
    Pressure,
    &*Pascal::SCALE * &Scale::from_f64(3386.38815789)
);
define_unit!(
    Millibar,
    "millibar",
    "mbar",
    Pressure,
    &*Bar::SCALE * &*MILLI
);
