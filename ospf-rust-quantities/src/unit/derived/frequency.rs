//! Frequency units - 频率单位
//! Frequency units - SI frequency units
//!
//! 提供频率量纲的 SI 单位定义，包括赫兹、千赫、兆赫、吉赫等。
//! Provides SI unit definitions for frequency dimension, including hertz, kilohertz, megahertz, gigahertz, etc.

use crate::dimension::derived::Frequency;
use crate::scale::{GIGA, KILO, MEGA, Scale};
use crate::unit::CTUnit;

// ============================================================================
// 频率单位 / Frequency units
// ============================================================================

define_unit!(Hertz, "hertz", "Hz", Frequency, Scale::new());
define_unit!(Kilohertz, "kilohertz", "kHz", Frequency, KILO.clone());
define_unit!(Megahertz, "megahertz", "MHz", Frequency, MEGA.clone());
define_unit!(Gigahertz, "gigahertz", "GHz", Frequency, GIGA.clone());

// 每小时周期数 / Cycle per hour
define_unit!(
    CyclePerHour,
    "cycle per hour",
    "cph",
    Frequency,
    Scale::from_f64(0.0002777777777777778)
);
