//! 功率单位 / Power units
//!
//! 提供功率量纲的 SI 单位定义，包括瓦特、千瓦、兆瓦、马力等 / Provides SI unit definitions for power dimension, including watt, kilowatt, megawatt, horsepower, etc

use super::energy::Joule;
use super::time::Second;
use crate::dimension::derived::Power;
use crate::scale::{KILO, MEGA, MILLI};
use crate::unit::{CTUnit, CTUnitDiv};

// ============================================================================
// 功率单位 / Power units
// ============================================================================

define_unit_by!(Watt, "watt", "W", CTUnitDiv<Joule, Second>);
define_unit!(Kilowatt, "kilowatt", "kW", Power, &*Watt::SCALE * &*KILO);
define_unit!(Megawatt, "megawatt", "MW", Power, &*Watt::SCALE * &*MEGA);
define_unit!(Milliwatt, "milliwatt", "mW", Power, &*Watt::SCALE * &*MILLI);
define_unit_by!(JoulePerSecond, "joule per second", "J/s", Watt);
define_unit_by!(
    NewtonMeterPerSecond,
    "newton meter per second",
    "N.m/s",
    Watt
);
define_unit!(
    Horsepower,
    "horsepower",
    "ps",
    Power,
    &*Watt::SCALE * &crate::scale::Scale::from_int(735)
);
define_unit!(
    UKHorsepower,
    "uk horsepower",
    "uk.ps",
    Power,
    &*Watt::SCALE * &crate::scale::Scale::from_int(550)
);

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::derived::test_support::*;

    #[test]
    fn test_power_symbols_and_names() {
        // 功率单位的符号与名称 / Symbols and names of power units
        assert_symbol_and_name::<Watt>("W", "watt", "瓦特 / watt");
        assert_symbol_and_name::<Kilowatt>("kW", "kilowatt", "千瓦 / kilowatt");
        assert_symbol_and_name::<Megawatt>("MW", "megawatt", "兆瓦 / megawatt");
        assert_symbol_and_name::<Milliwatt>("mW", "milliwatt", "毫瓦 / milliwatt");
        assert_symbol_and_name::<JoulePerSecond>("J/s", "joule per second", "焦耳每秒 / joule per second");
        assert_symbol_and_name::<NewtonMeterPerSecond>("N.m/s", "newton meter per second", "牛顿米每秒 / newton meter per second");
        assert_symbol_and_name::<Horsepower>("ps", "horsepower", "公制马力 / horsepower");
        assert_symbol_and_name::<UKHorsepower>("uk.ps", "uk horsepower", "英制马力 / uk horsepower");
    }

    #[test]
    fn test_watt_is_base_power_unit() {
        // 瓦特是功率的基准单位，比例尺为 1 / Watt is the base power unit with scale 1
        assert_scale_exact(Watt::SCALE.value(), "1", "瓦特比例尺 / watt scale");
    }

    #[test]
    fn test_power_conversion_to_watt() {
        // 功率单位到瓦特的换算系数 / Conversion factors of power units to watt
        assert_factor_exact::<Kilowatt, Watt>("1000", "千瓦到瓦特 / kilowatt to watt");
        assert_factor_exact::<Megawatt, Watt>("1000000", "兆瓦到瓦特 / megawatt to watt");
        assert_factor_exact::<Milliwatt, Watt>("0.001", "毫瓦到瓦特 / milliwatt to watt");
    }

    #[test]
    fn test_power_prefix_series() {
        // 功率单位按 1000 递进 / Power units advance by 1000
        assert_factor_exact::<Megawatt, Kilowatt>("1000", "兆瓦到千瓦 / megawatt to kilowatt");
        assert_factor_exact::<Kilowatt, Milliwatt>("1000000", "千瓦到毫瓦 / kilowatt to milliwatt");
    }

    #[test]
    fn test_joule_per_second_equals_watt() {
        // 1 焦耳每秒等于 1 瓦特 / One joule per second equals one watt
        assert_factor_exact::<JoulePerSecond, Watt>("1", "焦耳每秒到瓦特 / joule per second to watt");
        assert_factor_exact::<NewtonMeterPerSecond, Watt>("1", "牛顿米每秒到瓦特 / newton meter per second to watt");
    }

    #[test]
    fn test_horsepower_definitions() {
        // 公制马力按 735 瓦定义，英制马力按 550 瓦定义
        // Metric horsepower is defined as 735 watts and UK horsepower as 550 watts
        assert_factor_exact::<Horsepower, Watt>("735", "公制马力到瓦特 / horsepower to watt");
        assert_factor_exact::<UKHorsepower, Watt>("550", "英制马力到瓦特 / uk horsepower to watt");
    }

    #[test]
    fn test_power_is_joule_per_second() {
        // 瓦特应为焦耳除以秒，比例尺一致 / Watt should equal joule divided by second, with a matching scale
        use crate::unit::CTUnitDiv;
        assert_scale_equals::<Watt, CTUnitDiv<Joule, super::super::time::Second>>("瓦特分解 / watt decomposition");
    }

    #[test]
    fn test_power_dimension_symbol() {
        // 功率量纲符号为 L^2·M·T^-3 / Power dimension symbol is L^2·M·T^-3
        assert_dimension_symbol::<Watt>("L^2·M·T^-3", "瓦特量纲 / watt dimension");
    }

    #[test]
    fn test_power_units_share_dimension() {
        // 所有功率单位共享功率量纲 / All power units share the power dimension
        assert_same_dimension::<Watt, Horsepower>("瓦特与公制马力 / watt vs horsepower");
        assert_same_dimension::<Watt, Milliwatt>("瓦特与毫瓦 / watt vs milliwatt");
        assert_different_dimension::<Watt, Joule>("功率与能量 / power vs energy");
        assert_different_dimension::<Watt, super::super::force::Newton>("功率与力 / power vs force");
    }

    #[test]
    fn test_power_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<Megawatt>("MW", "megawatt", "兆瓦实例 / megawatt instance");
        assert_instance_symbol_and_name::<Milliwatt>("mW", "milliwatt", "毫瓦实例 / milliwatt instance");
    }
}
