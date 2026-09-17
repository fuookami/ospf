//! 动量单位 / Momentum units
//!
//! 提供动量量纲的 SI 单位定义，包括千克米每秒等 / Provides SI unit definitions for momentum dimension, including kilogram meter per second, etc

use super::length::Meter;
use super::mass::Kilogram;
use super::time::Second;
use crate::unit::{CTUnit, CTUnitDiv, CTUnitMul};

// ============================================================================
// 动量单位 / Momentum units
// ============================================================================

define_unit_by!(
    KilogramMeterPerSecond,
    "kilogram meter per second",
    "kg·m/s",
    CTUnitDiv<CTUnitMul<Kilogram, Meter>, Second>
);

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::derived::test_support::*;

    #[test]
    fn test_momentum_symbol_and_name() {
        // 动量单位的符号与名称 / Symbol and name of the momentum unit
        assert_symbol_and_name::<KilogramMeterPerSecond>("kg·m/s", "kilogram meter per second", "千克米每秒 / kilogram meter per second");
    }

    #[test]
    fn test_momentum_scale_is_unit() {
        // 动量基准单位比例尺为 1 / The base momentum unit has scale 1
        assert_scale_exact(KilogramMeterPerSecond::SCALE.value(), "1", "千克米每秒比例尺 / kilogram meter per second scale");
    }

    #[test]
    fn test_momentum_is_kilogram_meter_per_second() {
        // 动量单位应为千克米除以秒，比例尺一致 / Momentum should equal kilogram meter divided by second, with a matching scale
        use crate::unit::CTUnitDiv;
        assert_scale_equals::<KilogramMeterPerSecond, CTUnitDiv<CTUnitMul<Kilogram, Meter>, Second>>(
            "千克米每秒分解 / kilogram meter per second decomposition",
        );
    }

    #[test]
    fn test_momentum_equals_newton_second() {
        // 1 kg·m/s 等于 1 N·s / One kilogram meter per second equals one newton second
        use crate::unit::derived::force::Newton;
        assert_scale_equals::<KilogramMeterPerSecond, CTUnitMul<Newton, Second>>("动量与牛顿秒 / momentum vs newton second");
    }

    #[test]
    fn test_momentum_dimension_symbol() {
        // 动量量纲符号为 L·M·T^-1 / Momentum dimension symbol is L·M·T^-1
        assert_dimension_symbol::<KilogramMeterPerSecond>("L·M·T^-1", "千克米每秒量纲 / kilogram meter per second dimension");
    }

    #[test]
    fn test_momentum_differs_from_mass_and_velocity() {
        // 动量与质量、速度量纲不同 / Momentum differs from mass and velocity in dimension
        assert_different_dimension::<KilogramMeterPerSecond, Kilogram>("动量与质量 / momentum vs mass");
        assert_different_dimension::<KilogramMeterPerSecond, super::super::velocity::MeterPerSecond>(
            "动量与速度 / momentum vs velocity",
        );
    }

    #[test]
    fn test_momentum_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<KilogramMeterPerSecond>("kg·m/s", "kilogram meter per second", "千克米每秒实例 / kilogram meter per second instance");
    }
}
