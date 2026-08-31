//! Flow rate units - 流量单位
//! Flow rate units - SI flow rate units
//!
//! 提供流量量纲的 SI 单位定义，包括立方米每秒、升每秒、升每分钟等。
//! Provides SI unit definitions for flow rate dimension, including cubic meter per second, liter per second, liter per minute, etc.

use super::time::{Minute, Second};
use super::volume::{CubicMeter, Liter};
use crate::scale::Scale;
use crate::unit::{CTUnit, CTUnitDiv};
use once_cell::sync::Lazy;

// ============================================================================
// 流量单位 / Flow rate units
// ============================================================================

define_unit_by!(
    CubicMeterPerSecond,
    "cubic meter per second",
    "m³/s",
    CTUnitDiv<CubicMeter, Second>
);
define_unit_by!(
    LiterPerSecond,
    "liter per second",
    "L/s",
    CTUnitDiv<Liter, Second>
);
define_unit_by!(
    LiterPerMinute,
    "liter per minute",
    "L/min",
    CTUnitDiv<Liter, Minute>
);
