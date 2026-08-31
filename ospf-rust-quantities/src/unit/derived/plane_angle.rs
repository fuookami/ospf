//! Plane angle units - 平面角单位
//! Plane angle units - SI plane angle units (radian, degree, etc.)
//!
//! 提供平面角量纲的 SI 单位定义，包括弧度、度、角分、角秒等。
//! Provides SI unit definitions for plane angle dimension, including radian, degree, arc minute, arc second, etc.

use crate::dimension::derived::PlaneAngle;
use crate::scale::{Scale, SEXAGESIMAL};
use crate::unit::CTUnit;
use once_cell::sync::Lazy;

// ============================================================================
// 平面角单位 / Plane angle units
// ============================================================================

pub static RADIAN_TO_DEGREE: Lazy<Scale> =
    Lazy::new(|| Scale::from_f64(std::f64::consts::PI) / Scale::from_int(180));

define_unit!(Radian, "radian", "rad", PlaneAngle);
define_unit!(Degree, "degree", "°", PlaneAngle, RADIAN_TO_DEGREE.clone());
define_unit!(
    ArcMinute,
    "arc minute",
    "'",
    PlaneAngle,
    &*Degree::SCALE / &*SEXAGESIMAL
);
define_unit!(
    ArcSecond,
    "arc second",
    "\"",
    PlaneAngle,
    &*ArcMinute::SCALE / &*SEXAGESIMAL
);
