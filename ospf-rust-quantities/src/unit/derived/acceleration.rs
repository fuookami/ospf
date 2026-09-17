//! 加速度单位 / Acceleration units
//!
//! 提供加速度量纲的 SI 单位定义，包括米每二次方秒、标准重力加速度等 / Provides SI unit definitions for acceleration dimension, including meter per second squared, standard gravity, etc

use super::length::{Cetimeter, Foot, Inch, Kilometer, Meter};
use super::time::Second;
use crate::dimension::derived::Acceleration;
use crate::scale::Scale;
use crate::unit::physical_unit::CTUnit;
use crate::unit::{CTUnitDiv, CTUnitMul};

// ============================================================================
// SI 加速度单位 / SI acceleration units
// ============================================================================

// 米每二次方秒 / Meter per second squared
define_unit_by!(
    MeterPerSecondSquared,
    "meter per second squared",
    "m/s²",
    CTUnitDiv<Meter, CTUnitMul<Second, Second>>
);

// 厘米每二次方秒 / Centimeter per second squared
define_unit_by!(
    CentimeterPerSecondSquared,
    "centimeter per second squared",
    "cm/s²",
CTUnitDiv<Cetimeter, CTUnitMul<Second, Second>>
);

// 千米每二次方秒 / Kilometer per second squared
define_unit_by!(
    KilometerPerSecondSquared,
    "kilometer per second squared",
    "km/s²",
    CTUnitDiv<Kilometer, CTUnitMul<Second, Second>>
);

// ============================================================================
// 英制加速度单位 / Imperial acceleration units
// ============================================================================

// 英寸每二次方秒 / Inch per second squared
define_unit_by!(
    InchPerSecondSquared,
    "inch per second squared",
    "in/s²",
    CTUnitDiv<Inch, CTUnitMul<Second, Second>>
);

// 英尺每二次方秒 / Foot per second squared
define_unit_by!(
    FootPerSecondSquared,
    "foot per second squared",
    "ft/s²",
    CTUnitDiv<Foot, CTUnitMul<Second, Second>>
);

// ============================================================================
// 特殊加速度单位 / Special acceleration units
// ============================================================================

// 标准重力加速度 / Standard gravity (9.80665 m/s²)
define_unit!(
    StandardGravity,
    "standard gravity",
    "g",
    Acceleration,
    Scale::from_f64(9.80665)
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
    fn test_acceleration_symbols_and_names() {
        // 加速度单位的符号与名称 / Symbols and names of acceleration units
        assert_symbol_and_name::<MeterPerSecondSquared>("m/s²", "meter per second squared", "米每二次方秒 / meter per second squared");
        assert_symbol_and_name::<CentimeterPerSecondSquared>("cm/s²", "centimeter per second squared", "厘米每二次方秒 / centimeter per second squared");
        assert_symbol_and_name::<KilometerPerSecondSquared>("km/s²", "kilometer per second squared", "千米每二次方秒 / kilometer per second squared");
        assert_symbol_and_name::<InchPerSecondSquared>("in/s²", "inch per second squared", "英寸每二次方秒 / inch per second squared");
        assert_symbol_and_name::<FootPerSecondSquared>("ft/s²", "foot per second squared", "英尺每二次方秒 / foot per second squared");
        assert_symbol_and_name::<StandardGravity>("g", "standard gravity", "标准重力加速度 / standard gravity");
    }

    #[test]
    fn test_acceleration_conversion_to_meter_per_second_squared() {
        // 加速度单位到米每二次方秒的换算系数 / Conversion factors of acceleration units to meter per second squared
        assert_factor_exact::<MeterPerSecondSquared, MeterPerSecondSquared>("1", "米每二次方秒到自身 / meter per second squared to itself");
        assert_factor_exact::<CentimeterPerSecondSquared, MeterPerSecondSquared>("0.01", "厘米每二次方秒到米每二次方秒 / centimeter per second squared to meter per second squared");
        assert_factor_exact::<KilometerPerSecondSquared, MeterPerSecondSquared>("1000", "千米每二次方秒到米每二次方秒 / kilometer per second squared to meter per second squared");
        assert_factor_relative::<InchPerSecondSquared, MeterPerSecondSquared>("0.0254", FUZZ, "英寸每二次方秒到米每二次方秒 / inch per second squared to meter per second squared");
        assert_factor_relative::<FootPerSecondSquared, MeterPerSecondSquared>("0.3048", FUZZ, "英尺每二次方秒到米每二次方秒 / foot per second squared to meter per second squared");
    }

    #[test]
    fn test_standard_gravity_equals_9_80665() {
        // 标准重力加速度等于 9.80665 米每二次方秒 / Standard gravity equals 9.80665 meters per second squared
        assert_factor_relative::<StandardGravity, MeterPerSecondSquared>("9.80665", FUZZ, "标准重力加速度到米每二次方秒 / standard gravity to meter per second squared");
    }

    #[test]
    fn test_acceleration_equals_velocity_per_second() {
        // 加速度单位应为速度单位除以秒，比例尺一致 / An acceleration unit should equal the velocity unit divided by second, with a matching scale
        use crate::unit::{CTUnitDiv, KilometerPerSecond, MeterPerSecond, Second};
        assert_scale_equals::<MeterPerSecondSquared, CTUnitDiv<MeterPerSecond, Second>>("米每二次方秒分解 / meter per second squared decomposition");
        assert_scale_equals::<KilometerPerSecondSquared, CTUnitDiv<KilometerPerSecond, Second>>("千米每二次方秒分解 / kilometer per second squared decomposition");
    }

    #[test]
    fn test_acceleration_dimension_symbol() {
        // 加速度量纲符号为 L·T^-2 / Acceleration dimension symbol is L·T^-2
        assert_dimension_symbol::<MeterPerSecondSquared>("L·T^-2", "米每二次方秒量纲 / meter per second squared dimension");
    }

    #[test]
    fn test_acceleration_units_share_dimension() {
        // 所有加速度单位共享加速度量纲 / All acceleration units share the acceleration dimension
        assert_same_dimension::<MeterPerSecondSquared, StandardGravity>("米每二次方秒与标准重力 / meter per second squared vs standard gravity");
        assert_different_dimension::<MeterPerSecondSquared, super::super::velocity::MeterPerSecond>("加速度与速度 / acceleration vs velocity");
        assert_different_dimension::<MeterPerSecondSquared, Meter>("加速度与长度 / acceleration vs length");
    }

    #[test]
    fn test_acceleration_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<FootPerSecondSquared>("ft/s²", "foot per second squared", "英尺每二次方秒实例 / foot per second squared instance");
        assert_instance_symbol_and_name::<StandardGravity>("g", "standard gravity", "标准重力加速度实例 / standard gravity instance");
    }
}
