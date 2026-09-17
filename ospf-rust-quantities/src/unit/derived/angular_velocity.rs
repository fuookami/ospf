//! 角速度单位 / Angular velocity units
//!
//! 提供角速度量纲的 SI 单位定义，包括弧度每秒、度每秒等 / Provides SI unit definitions for angular velocity dimension, including radian per second, degree per second, etc

use super::plane_angle::{Degree, Radian};
use super::time::Second;
use crate::unit::CTUnitDiv;
use crate::unit::physical_unit::CTUnit;

// ============================================================================
// 角速度单位 / Angular velocity units
// ============================================================================

define_unit_by!(
    RadianPerSecond,
    "radian per second",
    "rad/s",
    CTUnitDiv<Radian, Second>
);
define_unit_by!(
    DegreePerSecond,
    "degree per second",
    "°/s",
    CTUnitDiv<Degree, Second>
);

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::derived::test_support::*;

    #[test]
    fn test_angular_velocity_symbols_and_names() {
        // 角速度单位的符号与名称 / Symbols and names of angular velocity units
        assert_symbol_and_name::<RadianPerSecond>("rad/s", "radian per second", "弧度每秒 / radian per second");
        assert_symbol_and_name::<DegreePerSecond>("°/s", "degree per second", "度每秒 / degree per second");
    }

    #[test]
    fn test_radian_per_second_scale_is_unit() {
        // 弧度每秒是角速度的基准单位，比例尺为 1 / Radian per second is the base angular velocity unit with scale 1
        assert_scale_exact(RadianPerSecond::SCALE.value(), "1", "弧度每秒比例尺 / radian per second scale");
    }

    #[test]
    fn test_degree_per_second_equals_180_over_pi_radian_per_second() {
        // 1 度每秒等于 180/π 弧度每秒 / One degree per second equals 180/π radians per second
        assert_factor_relative::<RadianPerSecond, DegreePerSecond>(
            "57.295779513082320876798154814105170332405472466564",
            "1e-15",
            "弧度每秒到度每秒 / radian per second to degree per second",
        );
    }

    #[test]
    fn test_angular_velocity_dimension_symbol() {
        // 角速度量纲符号按固定基础量纲顺序生成：先时间后平面角，即 T^-1·φ
        // The angular velocity dimension symbol follows the fixed base dimension order: time then plane angle, i.e. T^-1·φ
        assert_dimension_symbol::<RadianPerSecond>("T^-1·φ", "弧度每秒量纲 / radian per second dimension");
    }

    #[test]
    fn test_angular_velocity_units_share_dimension() {
        // 弧度每秒与度每秒共享角速度量纲 / Radian per second and degree per second share the angular velocity dimension
        assert_same_dimension::<RadianPerSecond, DegreePerSecond>("弧度每秒与度每秒 / radian per second vs degree per second");
        assert_different_dimension::<RadianPerSecond, Radian>("角速度与角度 / angular velocity vs plane angle");
        assert_different_dimension::<RadianPerSecond, crate::unit::derived::MeterPerSecond>(
            "角速度与速度 / angular velocity vs velocity",
        );
    }

    #[test]
    fn test_angular_velocity_equals_angle_per_second() {
        // 角速度单位应为角度单位除以秒，比例尺一致 / An angular velocity unit should equal the angle unit divided by second, with a matching scale
        use crate::unit::{CTUnitDiv, Radian};
        assert_scale_equals::<RadianPerSecond, CTUnitDiv<Radian, Second>>("弧度每秒分解 / radian per second decomposition");
        assert_scale_equals::<DegreePerSecond, CTUnitDiv<Degree, Second>>("度每秒分解 / degree per second decomposition");
    }

    #[test]
    fn test_angular_velocity_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<RadianPerSecond>("rad/s", "radian per second", "弧度每秒实例 / radian per second instance");
        assert_instance_symbol_and_name::<DegreePerSecond>("°/s", "degree per second", "度每秒实例 / degree per second instance");
    }
}
