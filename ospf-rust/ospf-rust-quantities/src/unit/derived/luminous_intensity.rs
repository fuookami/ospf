//! 发光强度单位 / Luminous intensity units
//!
//! 提供发光强度量纲的 SI 单位定义，包括坎德拉等 / Provides SI unit definitions for luminous intensity dimension, including candela, etc

use crate::dimension::derived::LuminousIntensity;
use crate::unit::CTUnit;

// ============================================================================
// 发光强度单位 / Luminous intensity units
// ============================================================================

define_unit!(Candela, "candela", "cd", LuminousIntensity);

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::derived::test_support::*;

    #[test]
    fn test_candela_symbol_and_name() {
        // 坎德拉符号为 cd，名称为 candela / Candela symbol is cd, name is candela
        assert_symbol_and_name::<Candela>("cd", "candela", "坎德拉 / candela");
    }

    #[test]
    fn test_candela_is_base_unit_with_unit_scale() {
        // 坎德拉是发光强度的基准单位，比例尺为 1 / Candela is the base unit of luminous intensity with scale 1
        assert_scale_exact(Candela::SCALE.value(), "1", "坎德拉比例尺 / candela scale");
    }

    #[test]
    fn test_candela_dimension_symbol() {
        // 发光强度的量纲符号为 J / Dimension symbol of luminous intensity is J
        assert_dimension_symbol::<Candela>("J", "坎德拉量纲 / candela dimension");
    }

    #[test]
    fn test_candela_instance_symbol_and_name() {
        // 运行时单位实例的符号与名称 / Symbol and name of the runtime unit instance
        assert_instance_symbol_and_name::<Candela>("cd", "candela", "坎德拉实例 / candela instance");
    }

    #[test]
    fn test_candela_differs_from_other_base_dimensions() {
        // 坎德拉与摩尔、开尔文的量纲不同 / Candela differs from mole and kelvin in dimension
        assert_different_dimension::<Candela, crate::unit::derived::Mole>("坎德拉与摩尔 / candela vs mole");
        assert_different_dimension::<Candela, crate::unit::derived::Kelvin>(
            "坎德拉与开尔文 / candela vs kelvin",
        );
    }
}
