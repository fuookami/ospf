//! Length units - 长度单位
//! Length units - SI length units (meter, kilometer, etc.)
//!
//! 提供长度量纲的 SI 单位定义，包括米、千米、厘米、毫米、微米、纳米等。
//! Provides SI unit definitions for length dimension, including meter, kilometer, centimeter, millimeter, micrometer, nanometer, etc.

use crate::dimension::derived::Length;
use crate::scale::{Scale, CENTI, KILO, MICRO, MILLI, NANO};
use crate::unit::CTUnit;
use once_cell::sync::Lazy;

// ============================================================================
// 长度单位 / Length units
// ============================================================================

define_unit!(Meter, "meter", "m", Length);
define_unit!(Kilometer, "kilometer", "km", Length, KILO.clone());
define_unit!(Cetimeter, "centimeter", "cm", Length, CENTI.clone());
define_unit!(Millimeter, "millimeter", "mm", Length, MILLI.clone());
define_unit!(Micrometer, "micrometer", "μm", Length, MICRO.clone());
define_unit!(Nanometer, "nanometer", "nm", Length, NANO.clone());
