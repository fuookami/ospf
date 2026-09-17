//! 扭矩单位 / Torque units
//!
//! 提供扭矩量纲的 SI 单位定义，扭矩与能量同量纲，包括牛顿米等 / Provides SI unit definitions for torque dimension, torque has the same dimension as energy, including newton meter, etc

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

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::derived::test_support::*;

    // 由 f64 构造的比例尺无法用十进制精确表示，统一使用相对容差比较。
    // Scales built from f64 cannot be represented exactly in decimal, so a relative tolerance is used.
    const FUZZ: &str = "1e-12";

    #[test]
    fn test_torque_symbol_and_name_are_not_swapped() {
        // 牛顿米的名称与符号不应互换 / The name and symbol of newton meter must not be swapped
        assert_symbol_and_name::<NewtonMeter>("N·m", "newton meter", "牛顿米 / newton meter");
        assert_name::<NewtonMeter>("newton meter", "牛顿米名称 / newton meter name");
        assert_symbol::<NewtonMeter>("N·m", "牛顿米符号 / newton meter symbol");
    }

    #[test]
    fn test_kilogram_force_meter_symbol_and_name() {
        // 千克力米的符号与名称 / Symbol and name of kilogram-force meter
        assert_symbol_and_name::<KilogramForceMeter>("kgf·m", "kilogram-force meter", "千克力米 / kilogram-force meter");
    }

    #[test]
    fn test_newton_meter_scale_is_unit() {
        // 牛顿米是扭矩的基准单位，比例尺为 1 / Newton meter is the base torque unit with scale 1
        assert_scale_exact(NewtonMeter::SCALE.value(), "1", "牛顿米比例尺 / newton meter scale");
    }

    #[test]
    fn test_kilogram_force_meter_equals_9_80665_newton_meter() {
        // 1 千克力米等于 9.80665 牛顿米 / One kilogram-force meter equals 9.80665 newton meters
        assert_factor_relative::<KilogramForceMeter, NewtonMeter>("9.80665", FUZZ, "千克力米到牛顿米 / kilogram-force meter to newton meter");
    }

    #[test]
    fn test_newton_meter_is_newton_times_meter() {
        // 牛顿米应为牛顿乘以米，比例尺一致 / Newton meter should equal newton times meter, with a matching scale
        assert_scale_equals::<NewtonMeter, CTUnitMul<Newton, Meter>>("牛顿米分解 / newton meter decomposition");
        assert_scale_equals::<KilogramForceMeter, CTUnitMul<KilogramForce, Meter>>(
            "千克力米分解 / kilogram-force meter decomposition",
        );
    }

    #[test]
    fn test_torque_dimension_symbol() {
        // 扭矩量纲符号为 L^2·M·T^-2 / Torque dimension symbol is L^2·M·T^-2
        assert_dimension_symbol::<NewtonMeter>("L^2·M·T^-2", "牛顿米量纲 / newton meter dimension");
    }

    #[test]
    fn test_torque_shares_dimension_with_energy() {
        // 扭矩与能量同量纲，但与力不同 / Torque shares the energy dimension and differs from force
        assert_same_dimension::<NewtonMeter, super::super::energy::Joule>("牛顿米与焦耳 / newton meter vs joule");
        assert_different_dimension::<NewtonMeter, Newton>("牛顿米与牛顿 / newton meter vs newton");
        assert_different_dimension::<NewtonMeter, Meter>("牛顿米与米 / newton meter vs meter");
    }

    #[test]
    fn test_torque_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<NewtonMeter>("N·m", "newton meter", "牛顿米实例 / newton meter instance");
    }
}
