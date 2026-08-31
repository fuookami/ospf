//! 电阻单位 / Resistance units
//!
//! 提供电阻量纲的 SI 单位定义，包括欧姆、千欧、兆欧等 / Provides SI unit definitions for resistance dimension, including ohm, kiloohm, megaohm, etc

use super::electrical::{Ampere, Volt};
use crate::dimension::derived::Resistance;
use crate::scale::{KILO, MEGA};
use crate::unit::{CTUnit, CTUnitDiv};

// ============================================================================
// 电阻单位 / Resistance units
// ============================================================================

define_unit_by!(Ohm, "ohm", "Ω", CTUnitDiv<Volt, Ampere>);
define_unit!(Kiloohm, "kiloohm", "kΩ", Resistance, &*Ohm::SCALE * &*KILO);
define_unit!(Megaohm, "megaohm", "MΩ", Resistance, &*Ohm::SCALE * &*MEGA);
