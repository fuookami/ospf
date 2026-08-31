//! Thermodynamic temperature units - 热力学温度单位
//! Thermodynamic temperature units - SI thermodynamic temperature units
//!
//! 提供热力学温度量纲的 SI 单位定义，包括开尔文等。
//! Provides SI unit definitions for thermodynamic temperature dimension, including kelvin, etc.

use crate::dimension::derived::ThermodynamicTemperature;
use crate::unit::CTUnit;

// ============================================================================
// 热力学温度单位 / Thermodynamic temperature units
// ============================================================================

define_unit!(Kelvin, "kelvin", "K", ThermodynamicTemperature);
