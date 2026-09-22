//! 立体角单位 / Solid angle units
//!
//! 提供立体角量纲的 SI 单位定义，包括球面度等 / Provides SI unit definitions for solid angle dimension, including steradian, etc

use crate::dimension::derived::SolidAngle;
use crate::scale::Scale;
use crate::unit::CTUnit;

// ============================================================================
// 立体角单位 / Solid angle units
// ============================================================================

// 球面度 / Steradian
define_unit!(Steradian, "steradian", "sr", SolidAngle);

// 平方度 / Square degree (1 sr = (180/π)² square degrees)
define_unit!(
    SquareDegree,
    "square degree",
    "deg²",
    SolidAngle,
    Scale::from_f64(0.00030461741978670857)
);

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::derived::test_support::*;

    #[test]
    fn test_solid_angle_symbols_and_names() {
        // 立体角单位的符号与名称 / Symbols and names of solid angle units
        assert_symbol_and_name::<Steradian>("sr", "steradian", "球面度 / steradian");
        assert_symbol_and_name::<SquareDegree>("deg²", "square degree", "平方度 / square degree");
    }

    #[test]
    fn test_steradian_is_base_unit_with_unit_scale() {
        // 球面度是立体角的基准单位，比例尺为 1 / Steradian is the base unit of solid angle with scale 1
        assert_scale_exact(Steradian::SCALE.value(), "1", "球面度比例尺 / steradian scale");
    }

    #[test]
    fn test_square_degree_to_steradian() {
        // 1 平方度约等于 0.00030461741978670857 球面度 / One square degree is about 0.00030461741978670857 steradian
        assert_factor_relative::<SquareDegree, Steradian>(
            "0.00030461741978670857",
            "1e-15",
            "平方度到球面度 / square degree to steradian",
        );
    }

    #[test]
    fn test_square_degree_is_reciprocal_of_radian_to_degree_squared() {
        // 平方度应为 (π/180)² 球面度 / A square degree should be (π/180)² steradians
        assert_factor_relative::<SquareDegree, Steradian>(
            "0.00030461741978670856889968576730609584046760573983",
            "1e-15",
            "平方度到球面度理论值 / square degree to steradian theoretical value",
        );
    }

    #[test]
    fn test_solid_angle_dimension_symbol() {
        // 立体角量纲符号为 Ω / Solid angle dimension symbol is Ω
        assert_dimension_symbol::<Steradian>("Ω", "球面度量纲 / steradian dimension");
    }

    #[test]
    fn test_solid_angle_units_share_dimension() {
        // 球面度与平方度共享立体角量纲 / Steradian and square degree share the solid angle dimension
        assert_same_dimension::<Steradian, SquareDegree>("球面度与平方度 / steradian vs square degree");
        assert_different_dimension::<Steradian, crate::unit::derived::Radian>(
            "立体角与平面角 / solid angle vs plane angle",
        );
    }

    #[test]
    fn test_solid_angle_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<Steradian>("sr", "steradian", "球面度实例 / steradian instance");
    }
}
