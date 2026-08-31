//! 平面角单位 / Plane angle units
//!
//! 提供平面角量纲的 SI 单位定义，包括弧度、度、角分、角秒等 / Provides SI unit definitions for plane angle dimension, including radian, degree, arc minute, arc second, etc

use once_cell::sync::Lazy;
use crate::dimension::derived::PlaneAngle;
use crate::scale::{SEXAGESIMAL, Scale};
use crate::unit::CTUnit;

// ============================================================================
// 平面角单位 / Plane angle units
// ============================================================================

// 弧度 / Radian
define_unit!(Radian, "radian", "rad", PlaneAngle);

// 毫弧度 / Milliradian
define_unit!(
    Milliradian,
    "milliradian",
    "mrad",
    PlaneAngle,
    Scale::from_f64(0.001)
);

// 度 / Degree
pub static RADIAN_TO_DEGREE: Lazy<Scale> =
    Lazy::new(|| Scale::from_f64(std::f64::consts::PI) / Scale::from_int(180));

define_unit!(Degree, "degree", "°", PlaneAngle, RADIAN_TO_DEGREE.clone());

// 角分 / Arc minute
define_unit!(
    ArcMinute,
    "arc minute",
    "'",
    PlaneAngle,
    &*Degree::SCALE / &*SEXAGESIMAL
);

// 角秒 / Arc second
define_unit!(
    ArcSecond,
    "arc second",
    "\"",
    PlaneAngle,
    &*ArcMinute::SCALE / &*SEXAGESIMAL
);

// 周角 / Round angle (360 degrees = 2π radians)
define_unit!(
    RoundAngle,
    "round angle",
    "round angle",
    PlaneAngle,
    Scale::from_f64(std::f64::consts::PI * 2.0)
);

// 直角 / Right angle (90 degrees = π/2 radians)
define_unit!(
    RightAngle,
    "right angle",
    "right angle",
    PlaneAngle,
    Scale::from_f64(std::f64::consts::PI / 2.0)
);

// 梯度 / Gradian (gon)
define_unit!(
    Gradian,
    "gradian",
    "gon",
    PlaneAngle,
    Scale::from_f64(std::f64::consts::PI / 200.0)
);
