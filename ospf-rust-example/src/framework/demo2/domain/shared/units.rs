//! 运行期单位定义 / Runtime unit definitions
//!
//! 对齐 Kotlin Quantity<Flt64> 用法，使用运行期单位。
//! 提供集中式单位转换 helper，避免 model 文件中散落的 `.value` 直接提取。
//! Provides centralized unit conversion helpers to avoid scattered `.value` extraction in model files.

use std::error::Error;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::{CTUnit, Unit};
use ospf_rust_quantities::unit::derived::{Kilogram, Meter};

/// 重量单位 / Weight unit (千克)
pub fn weight_unit() -> Unit {
    Kilogram::INSTANT.clone()
}

/// 长度单位 / Length unit (米)
pub fn length_unit() -> Unit {
    Meter::INSTANT.clone()
}

/// 创建重量物理量 / Create weight quantity
pub fn weight(value: f64) -> Quantity<f64, Unit> {
    Quantity::new(value, weight_unit())
}

/// 创建长度物理量 / Create length quantity
pub fn length(value: f64) -> Quantity<f64, Unit> {
    Quantity::new(value, length_unit())
}

/// 从物理量提取数值 / Extract value from quantity
pub fn to_f64(q: &Quantity<f64, Unit>) -> f64 {
    q.value
}

/// 从数值创建物理量 / Create quantity from value (using weight unit by default)
pub fn from_f64(value: f64) -> Quantity<f64, Unit> {
    weight(value)
}

/// 将物理量转换到指定单位并提取数值
/// Convert quantity to the specified unit and extract the f64 value
///
/// 如果量纲不匹配或转换失败，返回错误。
/// Returns an error if dimensions don't match or conversion fails.
pub fn quantity_value_in_unit(
    quantity: &Quantity<f64, Unit>,
    unit: &Unit,
) -> Result<f64, Box<dyn Error>> {
    let converted = quantity.to_unit(unit).map_err(|e| {
        Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", e))) as Box<dyn std::error::Error>
    })?;
    Ok(converted.value)
}

/// 将可选物理量转换到指定单位并提取数值，None 时返回默认值
/// Convert optional quantity to the specified unit and extract value, returning default for None
pub fn quantity_value_in_unit_or_default(
    quantity: Option<&Quantity<f64, Unit>>,
    unit: &Unit,
    default: f64,
) -> Result<f64, Box<dyn Error>> {
    match quantity {
        Some(q) => quantity_value_in_unit(q, unit),
        None => Ok(default),
    }
}
