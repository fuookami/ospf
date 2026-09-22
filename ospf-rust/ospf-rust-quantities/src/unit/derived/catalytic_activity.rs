//! 催化活度单位 / Catalytic activity units
//!
//! 提供催化活度量纲的 SI 单位定义，包括开特、毫开特、微开特、酶单位等 / Provides SI unit definitions for catalytic activity dimension, including katal, millikatal, microkatal, enzyme unit, etc

use super::amount_of_substance::Mole;
use super::time::Second;
use crate::dimension::derived::CatalyticActivity;
use crate::scale::{MICRO, MILLI, Scale};
use crate::unit::{CTUnit, CTUnitDiv};

// ============================================================================
// 催化活度单位 / Catalytic activity units
// ============================================================================

define_unit_by!(Katal, "katal", "kat", CTUnitDiv<Mole, Second>);
define_unit!(
    Millikatal,
    "millikatal",
    "mkat",
    CatalyticActivity,
    &*Katal::SCALE * &*MILLI
);
define_unit!(
    Microkatal,
    "microkatal",
    "μkat",
    CatalyticActivity,
    &*Katal::SCALE * &*MICRO
);
define_unit!(
    EnzymeUnit,
    "enzyme unit",
    "U",
    CatalyticActivity,
    &*Katal::SCALE / &Scale::from_int(60000000)
);

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::derived::test_support::*;

    #[test]
    fn test_catalytic_activity_symbols_and_names() {
        // 催化活度单位的符号与名称 / Symbols and names of catalytic activity units
        assert_symbol_and_name::<Katal>("kat", "katal", "开特 / katal");
        assert_symbol_and_name::<Millikatal>("mkat", "millikatal", "毫开特 / millikatal");
        assert_symbol_and_name::<Microkatal>("μkat", "microkatal", "微开特 / microkatal");
        assert_symbol_and_name::<EnzymeUnit>("U", "enzyme unit", "酶单位 / enzyme unit");
    }

    #[test]
    fn test_katal_is_base_catalytic_activity_unit() {
        // 开特是催化活度的基准单位，比例尺为 1 / Katal is the base catalytic activity unit with scale 1
        assert_scale_exact(Katal::SCALE.value(), "1", "开特比例尺 / katal scale");
    }

    #[test]
    fn test_catalytic_activity_conversion_to_katal() {
        // 催化活度单位到开特的换算系数 / Conversion factors of catalytic activity units to katal
        assert_factor_exact::<Millikatal, Katal>("0.001", "毫开特到开特 / millikatal to katal");
        assert_factor_exact::<Microkatal, Katal>("0.000001", "微开特到开特 / microkatal to katal");
    }

    #[test]
    fn test_enzyme_unit_equals_one_over_sixty_million_katal() {
        // 1 酶单位等于 1/60000000 开特 / One enzyme unit equals 1/60000000 katal
        assert_factor_relative::<EnzymeUnit, Katal>(
            "0.000000016666666666666666666666666666666666666666667",
            "1e-15",
            "酶单位到开特 / enzyme unit to katal",
        );
    }

    #[test]
    fn test_katal_equals_sixty_million_enzyme_unit() {
        // 1 开特等于 6e7 酶单位 / One katal equals 6e7 enzyme units
        assert_factor_relative::<Katal, EnzymeUnit>("60000000", "1e-15", "开特到酶单位 / katal to enzyme unit");
    }

    #[test]
    fn test_catalytic_activity_is_mole_per_second() {
        // 开特应为摩尔除以秒，比例尺一致 / Katal should equal mole divided by second, with a matching scale
        assert_scale_equals::<Katal, CTUnitDiv<Mole, Second>>("开特分解 / katal decomposition");
    }

    #[test]
    fn test_catalytic_activity_dimension_symbol() {
        // 催化活度量纲符号按固定基础量纲顺序生成：先时间后物质的量，即 T^-1·N
        // The catalytic activity dimension symbol follows the fixed base dimension order: time then amount of substance, i.e. T^-1·N
        assert_dimension_symbol::<Katal>("T^-1·N", "开特量纲 / katal dimension");
    }

    #[test]
    fn test_catalytic_activity_units_share_dimension() {
        // 所有催化活度单位共享量纲 / All catalytic activity units share the catalytic activity dimension
        assert_same_dimension::<Katal, EnzymeUnit>("开特与酶单位 / katal vs enzyme unit");
        assert_same_dimension::<Katal, Microkatal>("开特与微开特 / katal vs microkatal");
        assert_different_dimension::<Katal, Mole>("催化活度与物质的量 / catalytic activity vs amount of substance");
        assert_different_dimension::<Katal, super::super::frequency::Hertz>("催化活度与频率 / catalytic activity vs frequency");
    }

    #[test]
    fn test_catalytic_activity_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<EnzymeUnit>("U", "enzyme unit", "酶单位实例 / enzyme unit instance");
        assert_instance_symbol_and_name::<Microkatal>("μkat", "microkatal", "微开特实例 / microkatal instance");
    }
}
