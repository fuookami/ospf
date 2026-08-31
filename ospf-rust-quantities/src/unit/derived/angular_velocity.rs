//! Angular velocity units - 角速度单位
//! Angular velocity units - SI angular velocity units
//!
//! 提供角速度量纲的 SI 单位定义，包括弧度每秒、度每秒等。
//! Provides SI unit definitions for angular velocity dimension, including radian per second, degree per second, etc.

use crate::unit::CTUnitDiv;
use crate::unit::physical_unit::CTUnit;
use super::plane_angle::{Degree, Radian};
use super::time::Second;

// ============================================================================
// 角速度单位 / Angular velocity units
// ============================================================================

define_unit_by!(
    RadianPerSecond,
    "radian per second",
    "rad/s",
    CTUnitDiv<Radian, Second>
);
define_unit_by!(
    DegreePerSecond,
    "degree per second",
    "°/s",
    CTUnitDiv<Degree, Second>
);
