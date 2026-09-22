//! 平面角单位 / Plane angle units
//!
//! 提供平面角量纲的 SI 单位定义，包括弧度、度、角分、角秒等 / Provides SI unit definitions for plane angle dimension, including radian, degree, arc minute, arc second, etc

use crate::dimension::derived::PlaneAngle;
use crate::scale::{SEXAGESIMAL, Scale};
use crate::unit::CTUnit;
use once_cell::sync::Lazy;

// ============================================================================
// 平面角单位 / Plane angle units
// ============================================================================

// 弧度 / Radian
define_unit!(Radian, "radian", "rad", PlaneAngle);

// 毫弧度 / Milliradian
define_unit!(
    Milliradian,
    "milliradian",
    "mrad",
    PlaneAngle,
    Scale::from_f64(0.001)
);

// 度 / Degree
pub static RADIAN_TO_DEGREE: Lazy<Scale> =
    Lazy::new(|| Scale::from_f64(std::f64::consts::PI) / Scale::from_int(180));

define_unit!(Degree, "degree", "°", PlaneAngle, RADIAN_TO_DEGREE.clone());

// 角分 / Arc minute
define_unit!(
    ArcMinute,
    "arc minute",
    "'",
    PlaneAngle,
    &*Degree::SCALE / &*SEXAGESIMAL
);

// 角秒 / Arc second
define_unit!(
    ArcSecond,
    "arc second",
    "\"",
    PlaneAngle,
    &*ArcMinute::SCALE / &*SEXAGESIMAL
);

// 周角 / Round angle (360 degrees = 2π radians)
define_unit!(
    RoundAngle,
    "round angle",
    "round angle",
    PlaneAngle,
    Scale::from_f64(std::f64::consts::PI * 2.0)
);

// 直角 / Right angle (90 degrees = π/2 radians)
define_unit!(
    RightAngle,
    "right angle",
    "right angle",
    PlaneAngle,
    Scale::from_f64(std::f64::consts::PI / 2.0)
);

// 梯度 / Gradian (gon)
define_unit!(
    Gradian,
    "gradian",
    "gon",
    PlaneAngle,
    Scale::from_f64(std::f64::consts::PI / 200.0)
);

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::derived::test_support::*;

    // π 由 f64 常量构造，无法用十进制精确表示，统一使用相对容差比较。
    // Since π is built from an f64 constant, it cannot be represented exactly in decimal, so a relative tolerance is used.
    const PI_FUZZ: &str = "1e-15";

    #[test]
    fn test_plane_angle_symbols_and_names() {
        // 平面角单位的符号与名称 / Symbols and names of plane angle units
        assert_symbol_and_name::<Radian>("rad", "radian", "弧度 / radian");
        assert_symbol_and_name::<Milliradian>("mrad", "milliradian", "毫弧度 / milliradian");
        assert_symbol_and_name::<Degree>("°", "degree", "度 / degree");
        assert_symbol_and_name::<ArcMinute>("'", "arc minute", "角分 / arc minute");
        assert_symbol_and_name::<ArcSecond>("\"", "arc second", "角秒 / arc second");
        assert_symbol_and_name::<RoundAngle>("round angle", "round angle", "周角 / round angle");
        assert_symbol_and_name::<RightAngle>("right angle", "right angle", "直角 / right angle");
        assert_symbol_and_name::<Gradian>("gon", "gradian", "梯度 / gradian");
    }

    #[test]
    fn test_radian_is_base_unit_with_unit_scale() {
        // 弧度是平面角的基准单位，比例尺为 1 / Radian is the base unit of plane angle with scale 1
        assert_scale_exact(Radian::SCALE.value(), "1", "弧度比例尺 / radian scale");
    }

    #[test]
    fn test_milliradian_is_one_thousandth_radian() {
        // 1 毫弧度等于 0.001 弧度 / One milliradian equals 0.001 radian
        assert_factor_relative::<Milliradian, Radian>("0.001", PI_FUZZ, "毫弧度到弧度 / milliradian to radian");
    }

    #[test]
    fn test_degree_is_pi_over_180_radian() {
        // 1 度等于 π/180 弧度 / One degree equals π/180 radian
        assert_factor_relative::<Degree, Radian>(
            "0.0174532925199432957692369076848861271344287188854172545609719144",
            PI_FUZZ,
            "度到弧度 / degree to radian",
        );
    }

    #[test]
    fn test_seconds_sexagesimal_subdivision() {
        // 角分、角秒为 60 进制细分 / Arc minute and arc second subdivide in base 60
        assert_factor_relative::<Degree, ArcMinute>("60", PI_FUZZ, "度到角分 / degree to arc minute");
        assert_factor_relative::<ArcMinute, ArcSecond>("60", PI_FUZZ, "角分到角秒 / arc minute to arc second");
        assert_factor_relative::<Degree, ArcSecond>("3600", PI_FUZZ, "度到角秒 / degree to arc second");
    }

    #[test]
    fn test_full_turn_and_right_angle() {
        // 周角为 360 度，直角为 90 度 / A full turn is 360 degrees and a right angle is 90 degrees
        assert_factor_relative::<RoundAngle, Degree>("360", PI_FUZZ, "周角到度 / round angle to degree");
        assert_factor_relative::<RightAngle, Degree>("90", PI_FUZZ, "直角到度 / right angle to degree");
        assert_factor_relative::<RoundAngle, RightAngle>("4", PI_FUZZ, "周角到直角 / round angle to right angle");
    }

    #[test]
    fn test_full_turn_is_two_pi_radian() {
        // 周角等于 2π 弧度 / A full turn equals 2π radians
        assert_factor_relative::<RoundAngle, Radian>(
            "6.2831853071795864769252867665590057683943387987502116419498891846",
            PI_FUZZ,
            "周角到弧度 / round angle to radian",
        );
    }

    #[test]
    fn test_gradian_is_nine_tenths_degree() {
        // 1 梯度等于 0.9 度 / One gradian equals 0.9 degree
        assert_factor_relative::<Gradian, Degree>("0.9", PI_FUZZ, "梯度到度 / gradian to degree");
        assert_factor_relative::<RoundAngle, Gradian>("400", PI_FUZZ, "周角到梯度 / round angle to gradian");
    }

    #[test]
    fn test_plane_angle_units_share_dimension() {
        // 所有平面角单位共享平面角量纲 / All plane angle units share the plane angle dimension
        assert_same_dimension::<Radian, Degree>("弧度与度 / radian vs degree");
        assert_same_dimension::<Radian, Gradian>("弧度与梯度 / radian vs gradian");
        assert_different_dimension::<Radian, crate::unit::derived::Steradian>(
            "平面角与立体角 / plane angle vs solid angle",
        );
    }

    #[test]
    fn test_plane_angle_dimension_symbol() {
        // 平面角量纲符号为 φ / Plane angle dimension symbol is φ
        assert_dimension_symbol::<Radian>("φ", "弧度量纲 / radian dimension");
    }

    #[test]
    fn test_plane_angle_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<Degree>("°", "degree", "度实例 / degree instance");
        assert_instance_symbol_and_name::<ArcSecond>("\"", "arc second", "角秒实例 / arc second instance");
    }
}
