//! 电阻单位 / Resistance units
//!
//! 提供电阻量纲的 SI 单位定义，包括欧姆、千欧、兆欧等 / Provides SI unit definitions for resistance dimension, including ohm, kiloohm, megaohm, etc

use super::electrical::{Ampere, Volt};
use crate::dimension::derived::Resistance;
use crate::scale::{KILO, MEGA};
use crate::unit::{CTUnit, CTUnitDiv};

// ============================================================================
// 电阻单位 / Resistance units
// ============================================================================

define_unit_by!(Ohm, "ohm", "Ω", CTUnitDiv<Volt, Ampere>);
define_unit!(Kiloohm, "kiloohm", "kΩ", Resistance, &*Ohm::SCALE * &*KILO);
define_unit!(Megaohm, "megaohm", "MΩ", Resistance, &*Ohm::SCALE * &*MEGA);

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::derived::test_support::*;

    #[test]
    fn test_resistance_symbols_and_names() {
        // 电阻单位的符号与名称 / Symbols and names of resistance units
        assert_symbol_and_name::<Ohm>("Ω", "ohm", "欧姆 / ohm");
        assert_symbol_and_name::<Kiloohm>("kΩ", "kiloohm", "千欧 / kiloohm");
        assert_symbol_and_name::<Megaohm>("MΩ", "megaohm", "兆欧 / megaohm");
    }

    #[test]
    fn test_ohm_is_base_resistance_unit() {
        // 欧姆是电阻的基准单位，比例尺为 1 / Ohm is the base resistance unit with scale 1
        assert_scale_exact(Ohm::SCALE.value(), "1", "欧姆比例尺 / ohm scale");
    }

    #[test]
    fn test_megaohm_equals_one_million_ohm() {
        // 1 兆欧等于 1e6 欧姆 / One megaohm equals 1e6 ohms
        assert_factor_exact::<Megaohm, Ohm>("1000000", "兆欧到欧姆 / megaohm to ohm");
    }

    #[test]
    fn test_megaohm_equals_one_thousand_kiloohm() {
        // 1 兆欧等于 1000 千欧 / One megaohm equals 1000 kiloohms
        assert_factor_exact::<Megaohm, Kiloohm>("1000", "兆欧到千欧 / megaohm to kiloohm");
    }

    #[test]
    fn test_kiloohm_equals_one_thousand_ohm() {
        // 1 千欧等于 1000 欧姆 / One kiloohm equals 1000 ohms
        assert_factor_exact::<Kiloohm, Ohm>("1000", "千欧到欧姆 / kiloohm to ohm");
    }

    #[test]
    fn test_ohm_is_volt_per_ampere() {
        // 欧姆应为伏特除以安培，比例尺一致 / Ohm should equal volt divided by ampere, with a matching scale
        assert_scale_equals::<Ohm, CTUnitDiv<Volt, Ampere>>("欧姆分解 / ohm decomposition");
    }

    #[test]
    fn test_resistance_dimension_symbol() {
        // 电阻量纲符号为 L^2·M·T^-3·I^-2 / Resistance dimension symbol is L^2·M·T^-3·I^-2
        assert_dimension_symbol::<Ohm>("L^2·M·T^-3·I^-2", "欧姆量纲 / ohm dimension");
    }

    #[test]
    fn test_resistance_units_share_dimension() {
        // 所有电阻单位共享电阻量纲 / All resistance units share the resistance dimension
        assert_same_dimension::<Ohm, Megaohm>("欧姆与兆欧 / ohm vs megaohm");
        assert_different_dimension::<Ohm, Volt>("电阻与电压 / resistance vs voltage");
        assert_different_dimension::<Ohm, Ampere>("电阻与电流 / resistance vs current");
    }

    #[test]
    fn test_resistance_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<Ohm>("Ω", "ohm", "欧姆实例 / ohm instance");
        assert_instance_symbol_and_name::<Megaohm>("MΩ", "megaohm", "兆欧实例 / megaohm instance");
    }
}
