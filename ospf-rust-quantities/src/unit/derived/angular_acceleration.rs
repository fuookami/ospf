//! Angular acceleration units - 角加速度单位
//! Angular acceleration units - SI angular acceleration units
//!
//! 提供角加速度量纲的 SI 单位定义，包括弧度每二次方秒、度每二次方秒等。
//! Provides SI unit definitions for angular acceleration dimension, including radian per second squared, degree per second squared, etc.

use super::angular_velocity::{DegreePerSecond, RadianPerSecond};
use super::time::Second;
use crate::unit::CTUnitDiv;
use crate::unit::physical_unit::CTUnit;

// ============================================================================
// 角加速度单位 / Angular acceleration units
// ============================================================================

define_unit_by!(
    RadianPerSecondSquared,
    "radian per second squared",
    "rad/s²",
    CTUnitDiv<RadianPerSecond, Second>
);
define_unit_by!(
    DegreePerSecondSquared,
    "degree per second squared",
    "°/s²",
    CTUnitDiv<DegreePerSecond, Second>
);
