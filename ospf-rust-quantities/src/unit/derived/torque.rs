//! Torque units - 扭矩单位
//! Torque units - SI torque units
//!
//! 提供扭矩量纲的 SI 单位定义，扭矩与能量同量纲，包括牛顿米等。
//! Provides SI unit definitions for torque dimension, torque has the same dimension as energy, including newton meter, etc.

use super::force::{KilogramForce, Newton};
use super::length::Meter;
use crate::unit::{CTUnit, CTUnitMul};

// ============================================================================
// 扭矩单位 / Torque units
// ============================================================================

define_unit_by!(
    NewtonMeter,
    "newton meter",
    "N·m",
    CTUnitMul<Newton, Meter>
);
define_unit_by!(
    KilogramForceMeter,
    "kilogram-force meter",
    "kgf·m",
    CTUnitMul<KilogramForce, Meter>
);