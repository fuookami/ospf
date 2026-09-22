//! 物质的量单位 / Amount of substance units
//!
//! 提供物质的量量纲的 SI 单位定义，包括摩尔等 / Provides SI unit definitions for amount of substance dimension, including mole, etc

use crate::dimension::derived::AmountOfSubstance;
use crate::unit::physical_unit::CTUnit;

// ============================================================================
// 物质的量单位 / Amount of substance units
// ============================================================================

define_unit!(Mole, "mole", "mol", AmountOfSubstance);

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::derived::test_support::*;

    #[test]
    fn test_mole_symbol_and_name() {
        // 摩尔符号为 mol，名称为 mole / Mole symbol is mol, name is mole
        assert_symbol_and_name::<Mole>("mol", "mole", "摩尔 / mole");
    }

    #[test]
    fn test_mole_is_base_unit_with_unit_scale() {
        // 摩尔是物质的量的基准单位，比例尺为 1 / Mole is the base unit of amount of substance with scale 1
        assert_scale_exact(Mole::SCALE.value(), "1", "摩尔比例尺 / mole scale");
        assert_factor_exact::<Mole, Mole>("1", "摩尔到摩尔 / mole to mole");
    }

    #[test]
    fn test_mole_dimension_symbol() {
        // 物质的量的量纲符号为 N / Dimension symbol of amount of substance is N
        assert_dimension_symbol::<Mole>("N", "摩尔量纲 / mole dimension");
    }

    #[test]
    fn test_mole_domain_is_continuous() {
        // 摩尔为连续量 / Mole is a continuous quantity
        assert_domain::<Mole>(
            crate::dimension::derived_quantity::QuantityDomain::Continuous,
            "摩尔取值域 / mole domain",
        );
    }

    #[test]
    fn test_mole_instance_symbol_and_name() {
        // 运行时单位实例的符号与名称 / Symbol and name of the runtime unit instance
        assert_instance_symbol_and_name::<Mole>("mol", "mole", "摩尔实例 / mole instance");
    }

    #[test]
    fn test_mole_differs_from_other_base_dimensions() {
        // 摩尔与长度、时间的量纲不同 / Mole differs in dimension from length and time
        assert_different_dimension::<Mole, crate::unit::derived::Meter>("摩尔与米 / mole vs meter");
        assert_different_dimension::<Mole, crate::unit::derived::Second>("摩尔与秒 / mole vs second");
    }
}
