//! Thermodynamic temperature units - 热力学温度单位
//! Thermodynamic temperature units - SI thermodynamic temperature units
//!
//! 提供热力学温度量纲的 SI 单位定义，包括开尔文等。
//! Provides SI unit definitions for thermodynamic temperature dimension, including kelvin, etc.

use std::str::FromStr;
use bigdecimal::BigDecimal;
use crate::dimension::derived::ThermodynamicTemperature;
use crate::scale::Scale;
use crate::unit::CTUnit;

// ============================================================================
// 热力学温度单位 / Thermodynamic temperature units
// ============================================================================

define_unit!(Kelvin, "kelvin", "K", ThermodynamicTemperature);
define_unit!(
    Celsius,
    "celsius",
    "°C",
    ThermodynamicTemperature,
    Scale::new(),
    offset = BigDecimal::from_str("273.15").expect("无法解析摄氏度偏移值 / Failed to parse Celsius offset value")
);
define_unit!(
    Fahrenheit,
    "fahrenheit",
    "°F",
    ThermodynamicTemperature,
    Scale::from_f64(5.0 / 9.0),
    offset = BigDecimal::from_str("255.37222222222222222222").expect("无法解析华氏度偏移值 / Failed to parse Fahrenheit offset value")
);
define_unit!(
    Rankine,
    "rankine",
    "°R",
    ThermodynamicTemperature,
    Scale::from_f64(5.0 / 9.0)
);
