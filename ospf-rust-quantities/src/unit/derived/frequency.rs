//! Frequency units - 频率单位
//! Frequency units - SI frequency units
//!
//! 提供频率量纲的 SI 单位定义，包括赫兹、千赫、兆赫、吉赫等。
//! Provides SI unit definitions for frequency dimension, including hertz, kilohertz, megahertz, gigahertz, etc.

use crate::dimension::derived::Frequency;
use crate::scale::{Scale, GIGA, KILO, MEGA};
use crate::unit::CTUnit;
use once_cell::sync::Lazy;

// ============================================================================
// 频率单位 / Frequency units
// ============================================================================

define_unit!(Hertz, "hertz", "Hz", Frequency, Scale::new());
define_unit!(KiloHertz, "kilohertz", "kHz", Frequency, KILO.clone());
define_unit!(MegaHertz, "megahertz", "MHz", Frequency, MEGA.clone());
define_unit!(GigaHertz, "gigahertz", "GHz", Frequency, GIGA.clone());
