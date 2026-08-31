//! 导出单位 / Derived units
//!
//! 按量纲分类组织的 SI 导出单位，提供运行时和编译时两种表示方式 / SI derived units organized by dimension, providing both runtime and compile-time representations
//!
//! # 分类 / Categories
//! - 基本物理量：长度、质量、时间、电流、温度、物质的量、发光强度、信息量、角度
//! - 几何量：面积、体积
//! - 运动学量：速度、加速度、角速度、角加速度
//! - 动力学量：力、能量、功率、压力、扭矩
//! - 电磁学量：电荷、电压、电阻、电容、电感
//! - 光学量：光通量、照度、亮度

use super::concept::UnitTrait;
use super::physical_unit::CTUnit;
use crate::dimension::derived::DimLess;
use crate::dimension::derived_quantity::CTDerivedQuantity;
use crate::scale::Scale;
use bigdecimal::BigDecimal;
use once_cell::sync::Lazy;

#[macro_use]
mod macros;

// ============================================================================
// 模块声明 / Module declarations
// ============================================================================

// 基本物理量 / Base physical quantities
pub mod amount_of_substance;
pub mod information;
pub mod length;
pub mod luminous_intensity;
pub mod mass;
pub mod plane_angle;
pub mod solid_angle;
pub mod thermodynamic_temperature;
pub mod time;

// 导出几何量 / Derived geometric quantities
pub mod area;
pub mod volume;

// 导出运动学量 / Derived kinematic quantities
pub mod acceleration;
pub mod angular_acceleration;
pub mod angular_velocity;
pub mod velocity;

// 导出动力学量 / Derived dynamic quantities
pub mod energy;
pub mod force;
pub mod momentum;
pub mod power;
pub mod pressure;
pub mod stress;
pub mod torque;

// 导出材料性质量 / Derived material properties
pub mod flow_rate;
pub mod mass_density;
pub mod surface_density;

// 导出电磁学量 / Derived electromagnetic quantities
pub mod electrical;
pub mod resistance;

// 导出波动与周期量 / Derived wave and periodic quantities
pub mod bandwidth;
pub mod frequency;
pub mod wavenumber;

// 导出化学量 / Derived chemical quantities
pub mod catalytic_activity;

// ============================================================================
// 重导出基本单位类型 / Re-export base unit types
// ============================================================================

// 基本物理量 / Base physical quantities
pub use amount_of_substance::*;
pub use information::*;
pub use length::*;
pub use luminous_intensity::*;
pub use mass::*;
pub use plane_angle::*;
pub use solid_angle::*;
pub use thermodynamic_temperature::*;
pub use time::*;

// 导出几何量 / Derived geometric quantities
pub use area::*;
pub use volume::*;

// 导出运动学量 / Derived kinematic quantities
pub use acceleration::*;
pub use angular_acceleration::*;
pub use angular_velocity::*;
pub use velocity::*;

// 导出动力学量 / Derived dynamic quantities
pub use energy::*;
pub use force::*;
pub use momentum::*;
pub use power::*;
pub use pressure::*;
pub use stress::*;
pub use torque::*;

// 导出材料性质量 / Derived material properties
pub use flow_rate::*;
pub use mass_density::*;
pub use surface_density::*;

// 导出电磁学量 / Derived electromagnetic quantities
pub use electrical::*;
pub use resistance::*;

// 导出波动与周期量 / Derived wave and periodic quantities
pub use bandwidth::*;
pub use frequency::*;
pub use wavenumber::*;

// 导出化学量 / Derived chemical quantities
pub use catalytic_activity::*;

// ============================================================================
// 特殊单位类型 / Special unit types
// ============================================================================

/// 无量纲单位 / Dimensionless unit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct None;

impl UnitTrait for None {
    type Dimension = DimLess;

    fn symbol(&self) -> &'static str {
        "1"
    }

    fn name(&self) -> &'static str {
        "None"
    }

    fn dimension_symbol(&self) -> String {
        DimLess::INSTANT.symbol().to_string()
    }

    fn scale_value(&self) -> BigDecimal {
        BigDecimal::from(1)
    }
}

impl CTUnit for None {
    const NAME: &'static str = "None";
    const SYMBOL: &'static str = "1";
    const SCALE: Lazy<Scale> = Lazy::new(|| Scale::new());
    type Dimension = DimLess;
}

/// Kotlin 命名兼容别名 / Kotlin naming compatibility alias
pub type NoneUnit = None;
