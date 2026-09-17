//! 能量单位 / Energy units

use super::force::Newton;
use super::length::Meter;
use super::power::Kilowatt;
use super::time::Hour;
use crate::dimension::derived::Energy;
use crate::scale::{GIGA, KILO, MEGA, Scale};
use crate::unit::{CTUnit, CTUnitMul};

define_unit_by!(Joule, "joule", "J", CTUnitMul<Newton, Meter>);
define_unit!(
    Kilojoule,
    "kilojoule",
    "kJ",
    Energy,
    &*Joule::SCALE * &*KILO
);
define_unit!(
    Megajoule,
    "megajoule",
    "MJ",
    Energy,
    &*Joule::SCALE * &*MEGA
);
define_unit!(
    Gigajoule,
    "gigajoule",
    "GJ",
    Energy,
    &*Joule::SCALE * &*GIGA
);

define_unit!(
    ElectronVolt,
    "electronvolt",
    "eV",
    Energy,
    Scale::from_f64(1.602176634e-19)
);
define_unit!(
    Kiloelectronvolt,
    "kiloelectronvolt",
    "keV",
    Energy,
    &*ElectronVolt::SCALE * &*KILO
);
define_unit!(
    Megaelectronvolt,
    "megaelectronvolt",
    "MeV",
    Energy,
    &*ElectronVolt::SCALE * &*MEGA
);
define_unit!(
    Gigaelectronvolt,
    "gigaelectronvolt",
    "GeV",
    Energy,
    &*ElectronVolt::SCALE * &*GIGA
);

define_unit_by!(KilowattHour, "kilowatt-hour", "kWh", CTUnitMul<Kilowatt, Hour>);
define_unit!(
    MegawattHour,
    "megawatt-hour",
    "MWh",
    Energy,
    &*KilowattHour::SCALE * &*KILO
);
define_unit!(
    GigawattHour,
    "gigawatt-hour",
    "GWh",
    Energy,
    &*KilowattHour::SCALE * &*MEGA
);
define_unit!(
    WattHour,
    "watt-hour",
    "Wh",
    Energy,
    &*KilowattHour::SCALE / &*KILO
);

define_unit!(Calorie, "calorie", "cal", Energy, Scale::from_f64(4.184));
define_unit!(
    Kilocalorie,
    "kilocalorie",
    "kcal",
    Energy,
    &*Calorie::SCALE * &*KILO
);
define_unit!(
    BritishThermalUnit,
    "british thermal unit",
    "BTU",
    Energy,
    Scale::from_f64(1055.06)
);
define_unit!(Erg, "erg", "erg", Energy, Scale::from_f64(1e-7));

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
    fn test_si_energy_symbols_and_names() {
        // SI 能量单位的符号与名称 / Symbols and names of SI energy units
        assert_symbol_and_name::<Joule>("J", "joule", "焦耳 / joule");
        assert_symbol_and_name::<Kilojoule>("kJ", "kilojoule", "千焦 / kilojoule");
        assert_symbol_and_name::<Megajoule>("MJ", "megajoule", "兆焦 / megajoule");
        assert_symbol_and_name::<Gigajoule>("GJ", "gigajoule", "吉焦 / gigajoule");
    }

    #[test]
    fn test_si_energy_conversion_to_joule() {
        // SI 能量单位到焦耳的换算系数 / Conversion factors of SI energy units to joule
        assert_factor_exact::<Joule, Joule>("1", "焦耳到焦耳 / joule to joule");
        assert_factor_exact::<Kilojoule, Joule>("1000", "千焦到焦耳 / kilojoule to joule");
        assert_factor_exact::<Megajoule, Joule>("1000000", "兆焦到焦耳 / megajoule to joule");
        assert_factor_exact::<Gigajoule, Joule>("1000000000", "吉焦到焦耳 / gigajoule to joule");
        assert_factor_exact::<Megajoule, Kilojoule>("1000", "兆焦到千焦 / megajoule to kilojoule");
        assert_factor_exact::<Gigajoule, Megajoule>("1000", "吉焦到兆焦 / gigajoule to megajoule");
    }

    #[test]
    fn test_electron_volt_symbols_and_names() {
        // 电子伏特单位的符号与名称 / Symbols and names of electronvolt units
        assert_symbol_and_name::<ElectronVolt>("eV", "electronvolt", "电子伏特 / electronvolt");
        assert_symbol_and_name::<Kiloelectronvolt>("keV", "kiloelectronvolt", "千电子伏特 / kiloelectronvolt");
        assert_symbol_and_name::<Megaelectronvolt>("MeV", "megaelectronvolt", "兆电子伏特 / megaelectronvolt");
        assert_symbol_and_name::<Gigaelectronvolt>("GeV", "gigaelectronvolt", "吉电子伏特 / gigaelectronvolt");
    }

    #[test]
    fn test_electron_volt_conversions() {
        // 电子伏特单位到焦耳的换算系数 / Conversion factors of electronvolt units to joule
        assert_factor_relative::<ElectronVolt, Joule>("1.602176634e-19", FUZZ, "电子伏特到焦耳 / electronvolt to joule");
        assert_factor_relative::<Kiloelectronvolt, Joule>("1.602176634e-16", FUZZ, "千电子伏特到焦耳 / kiloelectronvolt to joule");
        assert_factor_relative::<Megaelectronvolt, Joule>("1.602176634e-13", FUZZ, "兆电子伏特到焦耳 / megaelectronvolt to joule");
        assert_factor_relative::<Gigaelectronvolt, Joule>("1.602176634e-10", FUZZ, "吉电子伏特到焦耳 / gigaelectronvolt to joule");
        assert_factor_relative::<Kiloelectronvolt, ElectronVolt>("1000", FUZZ, "千电子伏特到电子伏特 / kiloelectronvolt to electronvolt");
        assert_factor_relative::<Megaelectronvolt, Kiloelectronvolt>("1000", FUZZ, "兆电子伏特到千电子伏特 / megaelectronvolt to kiloelectronvolt");
        assert_factor_relative::<Gigaelectronvolt, Megaelectronvolt>("1000", FUZZ, "吉电子伏特到兆电子伏特 / gigaelectronvolt to megaelectronvolt");
    }

    #[test]
    fn test_watt_hour_symbols_and_names() {
        // 瓦时单位的符号与名称 / Symbols and names of watt-hour units
        assert_symbol_and_name::<WattHour>("Wh", "watt-hour", "瓦时 / watt-hour");
        assert_symbol_and_name::<KilowattHour>("kWh", "kilowatt-hour", "千瓦时 / kilowatt-hour");
        assert_symbol_and_name::<MegawattHour>("MWh", "megawatt-hour", "兆瓦时 / megawatt-hour");
        assert_symbol_and_name::<GigawattHour>("GWh", "gigawatt-hour", "吉瓦时 / gigawatt-hour");
    }

    #[test]
    fn test_watt_hour_equals_three_thousand_six_hundred_joule() {
        // 1 瓦时等于 3600 焦耳 / One watt-hour equals 3600 joules
        assert_factor_exact::<WattHour, Joule>("3600", "瓦时到焦耳 / watt-hour to joule");
    }

    #[test]
    fn test_kilowatt_hour_equals_three_point_six_megajoule() {
        // 1 千瓦时等于 3.6e6 焦耳 / One kilowatt-hour equals 3.6e6 joules
        assert_factor_exact::<KilowattHour, Joule>("3600000", "千瓦时到焦耳 / kilowatt-hour to joule");
        assert_factor_exact::<KilowattHour, WattHour>("1000", "千瓦时到瓦时 / kilowatt-hour to watt-hour");
        assert_factor_exact::<MegawattHour, KilowattHour>("1000", "兆瓦时到千瓦时 / megawatt-hour to kilowatt-hour");
        assert_factor_exact::<GigawattHour, MegawattHour>("1000", "吉瓦时到兆瓦时 / gigawatt-hour to megawatt-hour");
    }

    #[test]
    fn test_kilowatt_hour_matches_kilowatt_times_hour() {
        // 千瓦时应等于千瓦乘以小时，比例尺一致 / Kilowatt-hour should equal kilowatt times hour, with a matching scale
        assert_scale_equals::<KilowattHour, CTUnitMul<Kilowatt, Hour>>("千瓦时分解 / kilowatt-hour decomposition");
    }

    #[test]
    fn test_calorie_symbols_and_names() {
        // 卡路里单位的符号与名称 / Symbols and names of calorie units
        assert_symbol_and_name::<Calorie>("cal", "calorie", "卡路里 / calorie");
        assert_symbol_and_name::<Kilocalorie>("kcal", "kilocalorie", "千卡 / kilocalorie");
        assert_symbol_and_name::<BritishThermalUnit>("BTU", "british thermal unit", "英热单位 / british thermal unit");
        assert_symbol_and_name::<Erg>("erg", "erg", "尔格 / erg");
    }

    #[test]
    fn test_calorie_equals_4_184_joule() {
        // 1 卡路里等于 4.184 焦耳 / One calorie equals 4.184 joules
        assert_factor_relative::<Calorie, Joule>("4.184", FUZZ, "卡路里到焦耳 / calorie to joule");
    }

    #[test]
    fn test_kilocalorie_equals_one_thousand_calorie() {
        // 1 千卡等于 1000 卡路里 / One kilocalorie equals 1000 calories
        assert_factor_relative::<Kilocalorie, Calorie>("1000", FUZZ, "千卡到卡路里 / kilocalorie to calorie");
    }

    #[test]
    fn test_british_thermal_unit_conversions() {
        // 1 英热单位等于 1055.06 焦耳 / One british thermal unit equals 1055.06 joules
        assert_factor_relative::<BritishThermalUnit, Joule>("1055.06", FUZZ, "英热单位到焦耳 / british thermal unit to joule");
    }

    #[test]
    fn test_erg_equals_1e_minus_7_joule() {
        // 1 尔格等于 1e-7 焦耳 / One erg equals 1e-7 joules
        assert_factor_relative::<Erg, Joule>("1e-7", FUZZ, "尔格到焦耳 / erg to joule");
    }

    #[test]
    fn test_energy_dimension_symbol() {
        // 能量量纲符号为 L^2·M·T^-2 / Energy dimension symbol is L^2·M·T^-2
        assert_dimension_symbol::<Joule>("L^2·M·T^-2", "焦耳量纲 / joule dimension");
    }

    #[test]
    fn test_energy_units_share_dimension() {
        // 所有能量单位共享能量量纲 / All energy units share the energy dimension
        assert_same_dimension::<Joule, Calorie>("焦耳与卡路里 / joule vs calorie");
        assert_same_dimension::<Joule, KilowattHour>("焦耳与千瓦时 / joule vs kilowatt-hour");
        assert_same_dimension::<Joule, ElectronVolt>("焦耳与电子伏特 / joule vs electronvolt");
        assert_different_dimension::<Joule, Newton>("能量与力 / energy vs force");
        assert_different_dimension::<Joule, super::super::power::Watt>("能量与功率 / energy vs power");
    }

    #[test]
    fn test_energy_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<KilowattHour>("kWh", "kilowatt-hour", "千瓦时实例 / kilowatt-hour instance");
        assert_instance_symbol_and_name::<Kilocalorie>("kcal", "kilocalorie", "千卡实例 / kilocalorie instance");
    }
}
