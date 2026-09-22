//! 压力单位 / Pressure units
//!
//! 提供压力量纲的 SI 单位定义，包括帕斯卡、千帕、兆帕、巴等 / Provides SI unit definitions for pressure dimension, including pascal, kilopascal, megapascal, bar, etc

use super::area::SquareMeter;
use super::force::Newton;
use crate::dimension::derived::Pressure;
use crate::scale::{HECTO, KILO, MEGA, MILLI, Scale};
use crate::unit::{CTUnit, CTUnitDiv};

// ============================================================================
// 压力单位 / Pressure units
// ============================================================================

define_unit_by!(Pascal, "pascal", "Pa", CTUnitDiv<Newton, SquareMeter>);
define_unit!(
    Hectopascal,
    "hectopascal",
    "hPa",
    Pressure,
    &*Pascal::SCALE * &*HECTO
);
define_unit!(
    Kilopascal,
    "kilopascal",
    "kPa",
    Pressure,
    &*Pascal::SCALE * &*KILO
);
define_unit!(
    Megapascal,
    "megapascal",
    "MPa",
    Pressure,
    &*Pascal::SCALE * &*MEGA
);
define_unit!(
    Bar,
    "bar",
    "bar",
    Pressure,
    &*Pascal::SCALE * &Scale::from_int(100000)
);
define_unit!(
    StandardAtmosphericPressure,
    "standard atmospheric pressure",
    "atm",
    Pressure,
    &*Pascal::SCALE * &Scale::from_int(101325)
);
define_unit!(
    MeterMercury,
    "meter mercury",
    "mHg",
    Pressure,
    &*Pascal::SCALE * &Scale::from_f64(133322.387415)
);
define_unit!(
    MillimeterMercury,
    "millimeter mercury",
    "mmHg",
    Pressure,
    &*MeterMercury::SCALE * &*MILLI
);
define_unit!(
    InchOfMercury,
    "inch of mercury",
    "inHg",
    Pressure,
    &*Pascal::SCALE * &Scale::from_f64(3386.38815789)
);
define_unit!(
    Millibar,
    "millibar",
    "mbar",
    Pressure,
    &*Bar::SCALE * &*MILLI
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
    fn test_pressure_symbols_and_names() {
        // 压力单位的符号与名称 / Symbols and names of pressure units
        assert_symbol_and_name::<Pascal>("Pa", "pascal", "帕斯卡 / pascal");
        assert_symbol_and_name::<Hectopascal>("hPa", "hectopascal", "百帕 / hectopascal");
        assert_symbol_and_name::<Kilopascal>("kPa", "kilopascal", "千帕 / kilopascal");
        assert_symbol_and_name::<Megapascal>("MPa", "megapascal", "兆帕 / megapascal");
        assert_symbol_and_name::<Bar>("bar", "bar", "巴 / bar");
        assert_symbol_and_name::<StandardAtmosphericPressure>("atm", "standard atmospheric pressure", "标准大气压 / standard atmospheric pressure");
        assert_symbol_and_name::<MeterMercury>("mHg", "meter mercury", "米汞柱 / meter mercury");
        assert_symbol_and_name::<MillimeterMercury>("mmHg", "millimeter mercury", "毫米汞柱 / millimeter mercury");
        assert_symbol_and_name::<InchOfMercury>("inHg", "inch of mercury", "英寸汞柱 / inch of mercury");
        assert_symbol_and_name::<Millibar>("mbar", "millibar", "毫巴 / millibar");
    }

    #[test]
    fn test_megapascal_equals_one_million_pascal() {
        // 1 兆帕等于 1e6 帕斯卡 / One megapascal equals 1e6 pascals
        assert_factor_exact::<Megapascal, Pascal>("1000000", "兆帕到帕斯卡 / megapascal to pascal");
        assert_factor_exact::<Megapascal, Kilopascal>("1000", "兆帕到千帕 / megapascal to kilopascal");
        assert_factor_exact::<Megapascal, Hectopascal>("10000", "兆帕到百帕 / megapascal to hectopascal");
    }

    #[test]
    fn test_si_pressure_conversion_to_pascal() {
        // SI 压力单位到帕斯卡的换算系数 / Conversion factors of SI pressure units to pascal
        assert_factor_exact::<Pascal, Pascal>("1", "帕斯卡到帕斯卡 / pascal to pascal");
        assert_factor_exact::<Hectopascal, Pascal>("100", "百帕到帕斯卡 / hectopascal to pascal");
        assert_factor_exact::<Kilopascal, Pascal>("1000", "千帕到帕斯卡 / kilopascal to pascal");
    }

    #[test]
    fn test_bar_equals_one_hundred_thousand_pascal() {
        // 1 巴等于 100000 帕斯卡 / One bar equals 100000 pascals
        assert_factor_exact::<Bar, Pascal>("100000", "巴到帕斯卡 / bar to pascal");
        assert_factor_exact::<Bar, Kilopascal>("100", "巴到千帕 / bar to kilopascal");
    }

    #[test]
    fn test_millibar_equals_one_thousandth_bar() {
        // 1 毫巴等于 0.001 巴 / One millibar equals 0.001 bar
        assert_factor_exact::<Millibar, Bar>("0.001", "毫巴到巴 / millibar to bar");
        assert_factor_exact::<Millibar, Pascal>("100", "毫巴到帕斯卡 / millibar to pascal");
    }

    #[test]
    fn test_standard_atmospheric_pressure_equals_101325_pascal() {
        // 1 标准大气压等于 101325 帕斯卡 / One standard atmosphere equals 101325 pascals
        assert_factor_exact::<StandardAtmosphericPressure, Pascal>("101325", "标准大气压到帕斯卡 / standard atmosphere to pascal");
    }

    #[test]
    fn test_mercury_column_pressure_units() {
        // 汞柱压力单位的换算关系 / Conversion relations of mercury column pressure units
        assert_factor_relative::<MeterMercury, Pascal>("133322.387415", FUZZ, "米汞柱到帕斯卡 / meter mercury to pascal");
        assert_factor_relative::<MillimeterMercury, MeterMercury>("0.001", FUZZ, "毫米汞柱到米汞柱 / millimeter mercury to meter mercury");
        assert_factor_relative::<MillimeterMercury, Pascal>("133.322387415", FUZZ, "毫米汞柱到帕斯卡 / millimeter mercury to pascal");
        assert_factor_relative::<InchOfMercury, Pascal>("3386.38815789", FUZZ, "英寸汞柱到帕斯卡 / inch of mercury to pascal");
    }

    #[test]
    fn test_inch_of_mercury_is_25_4_millimeter_mercury() {
        // 1 英寸汞柱应约等于 25.4 毫米汞柱；两个常量均只保留有限有效数字，故使用 1e-6 容差
        // One inch of mercury should be about 25.4 millimeter mercury; both constants keep limited significant digits, so a 1e-6 tolerance is used
        assert_factor_relative::<InchOfMercury, MillimeterMercury>("25.4", "1e-6", "英寸汞柱到毫米汞柱 / inch of mercury to millimeter mercury");
    }

    #[test]
    fn test_pressure_dimension_symbol() {
        // 压力量纲符号为 L^-1·M·T^-2 / Pressure dimension symbol is L^-1·M·T^-2
        assert_dimension_symbol::<Pascal>("L^-1·M·T^-2", "帕斯卡量纲 / pascal dimension");
    }

    #[test]
    fn test_pressure_units_share_dimension() {
        // 所有压力单位共享压力量纲 / All pressure units share the pressure dimension
        assert_same_dimension::<Pascal, Bar>("帕斯卡与巴 / pascal vs bar");
        assert_same_dimension::<Pascal, InchOfMercury>("帕斯卡与英寸汞柱 / pascal vs inch of mercury");
        assert_different_dimension::<Pascal, Newton>("压力与力 / pressure vs force");
        assert_different_dimension::<Pascal, super::super::area::SquareMeter>("压力与面积 / pressure vs area");
    }

    #[test]
    fn test_pressure_is_newton_per_square_meter() {
        // 帕斯卡应为牛顿每平方米，比例尺一致 / Pascal should equal newton per square meter, with a matching scale
        use crate::unit::CTUnitDiv;
        assert_scale_equals::<Pascal, CTUnitDiv<Newton, super::super::area::SquareMeter>>(
            "帕斯卡分解 / pascal decomposition",
        );
    }

    #[test]
    fn test_pressure_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<Megapascal>("MPa", "megapascal", "兆帕实例 / megapascal instance");
        assert_instance_symbol_and_name::<Millibar>("mbar", "millibar", "毫巴实例 / millibar instance");
    }
}
