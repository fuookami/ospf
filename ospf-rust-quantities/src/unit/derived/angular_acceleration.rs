//! 角加速度单位 / Angular acceleration units
//!
//! 提供角加速度量纲的 SI 单位定义，包括弧度每二次方秒、度每二次方秒等 / Provides SI unit definitions for angular acceleration dimension, including radian per second squared, degree per second squared, etc

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

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::derived::test_support::*;

    #[test]
    fn test_angular_acceleration_symbols_and_names() {
        // 角加速度单位的符号与名称 / Symbols and names of angular acceleration units
        assert_symbol_and_name::<RadianPerSecondSquared>("rad/s²", "radian per second squared", "弧度每二次方秒 / radian per second squared");
        assert_symbol_and_name::<DegreePerSecondSquared>("°/s²", "degree per second squared", "度每二次方秒 / degree per second squared");
    }

    #[test]
    fn test_radian_per_second_squared_scale_is_unit() {
        // 弧度每二次方秒是角加速度的基准单位，比例尺为 1 / Radian per second squared is the base angular acceleration unit with scale 1
        assert_scale_exact(
            RadianPerSecondSquared::SCALE.value(),
            "1",
            "弧度每二次方秒比例尺 / radian per second squared scale",
        );
    }

    #[test]
    fn test_degree_per_second_squared_equals_180_over_pi() {
        // 1 度每二次方秒等于 180/π 弧度每二次方秒 / One degree per second squared equals 180/π radians per second squared
        assert_factor_relative::<RadianPerSecondSquared, DegreePerSecondSquared>(
            "57.295779513082320876798154814105170332405472466564",
            "1e-15",
            "弧度每二次方秒到度每二次方秒 / radian per second squared to degree per second squared",
        );
    }

    #[test]
    fn test_angular_acceleration_dimension_symbol() {
        // 角加速度量纲符号按固定基础量纲顺序生成：先时间后平面角，即 T^-2·φ
        // The angular acceleration dimension symbol follows the fixed base dimension order: time then plane angle, i.e. T^-2·φ
        assert_dimension_symbol::<RadianPerSecondSquared>("T^-2·φ", "弧度每二次方秒量纲 / radian per second squared dimension");
    }

    #[test]
    fn test_angular_acceleration_units_share_dimension() {
        // 两种角加速度单位共享量纲，且与角速度量纲不同
        // The two angular acceleration units share a dimension that differs from angular velocity
        assert_same_dimension::<RadianPerSecondSquared, DegreePerSecondSquared>(
            "弧度每二次方秒与度每二次方秒 / radian per second squared vs degree per second squared",
        );
        assert_different_dimension::<RadianPerSecondSquared, RadianPerSecond>(
            "角加速度与角速度 / angular acceleration vs angular velocity",
        );
    }

    #[test]
    fn test_angular_acceleration_equals_angular_velocity_per_second() {
        // 角加速度单位应为角速度单位除以秒，比例尺一致 / An angular acceleration unit should equal the angular velocity unit divided by second, with a matching scale
        use crate::unit::CTUnitDiv;
        assert_scale_equals::<RadianPerSecondSquared, CTUnitDiv<RadianPerSecond, Second>>(
            "弧度每二次方秒分解 / radian per second squared decomposition",
        );
        assert_scale_equals::<DegreePerSecondSquared, CTUnitDiv<DegreePerSecond, Second>>(
            "度每二次方秒分解 / degree per second squared decomposition",
        );
    }

    #[test]
    fn test_angular_acceleration_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<RadianPerSecondSquared>("rad/s²", "radian per second squared", "弧度每二次方秒实例 / radian per second squared instance");
    }
}
