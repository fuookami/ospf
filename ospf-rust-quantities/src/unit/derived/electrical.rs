//! 电学单位 / Electrical units

use super::power::Watt;
use super::time::{Hour, Second};
use crate::dimension::derived::{Capacitance, ElectricCharge, ElectricCurrent, ElectricPotential};
use crate::scale::{KILO, MEGA, MICRO, MILLI, NANO, PICO};
use crate::unit::{CTUnit, CTUnitDiv, CTUnitMul};

define_unit!(Ampere, "ampere", "A", ElectricCurrent);
define_unit!(
    Milliampere,
    "milliampere",
    "mA",
    ElectricCurrent,
    &*Ampere::SCALE * &*MILLI
);
define_unit!(
    Microampere,
    "microampere",
    "uA",
    ElectricCurrent,
    &*Ampere::SCALE * &*MICRO
);
define_unit!(
    Kiloampere,
    "kiloampere",
    "kA",
    ElectricCurrent,
    &*Ampere::SCALE * &*KILO
);

define_unit_by!(Coulomb, "coulomb", "C", CTUnitMul<Ampere, Second>);
define_unit!(
    Millicoulomb,
    "millicoulomb",
    "mC",
    ElectricCharge,
    &*Coulomb::SCALE * &*MILLI
);
define_unit!(
    Microcoulomb,
    "microcoulomb",
    "uC",
    ElectricCharge,
    &*Coulomb::SCALE * &*MICRO
);
define_unit!(
    Kilocoulomb,
    "kilocoulomb",
    "kC",
    ElectricCharge,
    &*Coulomb::SCALE * &*KILO
);
define_unit_by!(AmpereSecond, "ampere-second", "As", CTUnitMul<Ampere, Second>);
define_unit_by!(
    MilliampereHour,
    "milliampere-hour",
    "mAh",
    CTUnitMul<Milliampere, Hour>
);
define_unit_by!(
    MicroampereHour,
    "microampere-hour",
    "uAh",
    CTUnitMul<Microampere, Hour>
);
define_unit_by!(
    AmpereHour,
    "ampere-hour",
    "Ah",
    CTUnitMul<Ampere, Hour>
);
define_unit_by!(
    KiloampereHour,
    "kiloampere-hour",
    "kAh",
    CTUnitMul<Kiloampere, Hour>
);

define_unit_by!(Volt, "volt", "V", CTUnitDiv<Watt, Ampere>);
define_unit!(
    Microvolt,
    "microvolt",
    "uV",
    ElectricPotential,
    &*Volt::SCALE * &*MICRO
);
define_unit!(
    Millivolt,
    "millivolt",
    "mV",
    ElectricPotential,
    &*Volt::SCALE * &*MILLI
);
define_unit!(
    Kilovolt,
    "kilovolt",
    "kV",
    ElectricPotential,
    &*Volt::SCALE * &*KILO
);
define_unit!(
    Megavolt,
    "megavolt",
    "MV",
    ElectricPotential,
    &*Volt::SCALE * &*MEGA
);

define_unit_by!(Farad, "farad", "F", CTUnitDiv<Coulomb, Volt>);
define_unit!(
    Millifarad,
    "millifarad",
    "mF",
    Capacitance,
    &*Farad::SCALE * &*MILLI
);
define_unit!(
    Microfarad,
    "microfarad",
    "uF",
    Capacitance,
    &*Farad::SCALE * &*MICRO
);
define_unit!(
    Nanofarad,
    "nanofarad",
    "nF",
    Capacitance,
    &*Farad::SCALE * &*NANO
);
define_unit!(
    Picofarad,
    "picofarad",
    "pF",
    Capacitance,
    &*Farad::SCALE * &*PICO
);

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::derived::test_support::*;

    #[test]
    fn test_electric_current_symbols_and_names() {
        // 电流单位的符号与名称 / Symbols and names of electric current units
        assert_symbol_and_name::<Ampere>("A", "ampere", "安培 / ampere");
        assert_symbol_and_name::<Milliampere>("mA", "milliampere", "毫安 / milliampere");
        assert_symbol_and_name::<Microampere>("uA", "microampere", "微安 / microampere");
        assert_symbol_and_name::<Kiloampere>("kA", "kiloampere", "千安 / kiloampere");
    }

    #[test]
    fn test_electric_current_conversion_to_ampere() {
        // 电流单位到安培的换算系数 / Conversion factors of electric current units to ampere
        assert_factor_exact::<Ampere, Ampere>("1", "安培到安培 / ampere to ampere");
        assert_factor_exact::<Milliampere, Ampere>("0.001", "毫安到安培 / milliampere to ampere");
        assert_factor_exact::<Microampere, Ampere>("0.000001", "微安到安培 / microampere to ampere");
        assert_factor_exact::<Kiloampere, Ampere>("1000", "千安到安培 / kiloampere to ampere");
    }

    #[test]
    fn test_electric_charge_symbols_and_names() {
        // 电荷单位的符号与名称 / Symbols and names of electric charge units
        assert_symbol_and_name::<Coulomb>("C", "coulomb", "库仑 / coulomb");
        assert_symbol_and_name::<Millicoulomb>("mC", "millicoulomb", "毫库仑 / millicoulomb");
        assert_symbol_and_name::<Microcoulomb>("uC", "microcoulomb", "微库仑 / microcoulomb");
        assert_symbol_and_name::<Kilocoulomb>("kC", "kilocoulomb", "千库仑 / kilocoulomb");
        assert_symbol_and_name::<AmpereSecond>("As", "ampere-second", "安培秒 / ampere-second");
    }

    #[test]
    fn test_electric_charge_conversion_to_coulomb() {
        // 电荷单位到库仑的换算系数 / Conversion factors of electric charge units to coulomb
        assert_factor_exact::<Coulomb, Coulomb>("1", "库仑到库仑 / coulomb to coulomb");
        assert_factor_exact::<Millicoulomb, Coulomb>("0.001", "毫库仑到库仑 / millicoulomb to coulomb");
        assert_factor_exact::<Microcoulomb, Coulomb>("0.000001", "微库仑到库仑 / microcoulomb to coulomb");
        assert_factor_exact::<Kilocoulomb, Coulomb>("1000", "千库仑到库仑 / kilocoulomb to coulomb");
    }

    #[test]
    fn test_coulomb_equals_ampere_second() {
        // 1 库仑等于 1 安培秒 / One coulomb equals one ampere-second
        assert_factor_exact::<Coulomb, AmpereSecond>("1", "库仑到安培秒 / coulomb to ampere-second");
        assert_scale_equals::<Coulomb, CTUnitMul<Ampere, Second>>("库仑分解 / coulomb decomposition");
    }

    #[test]
    fn test_ampere_hour_symbols_and_names() {
        // 安时单位的符号与名称 / Symbols and names of ampere-hour units
        assert_symbol_and_name::<MilliampereHour>("mAh", "milliampere-hour", "毫安时 / milliampere-hour");
        assert_symbol_and_name::<MicroampereHour>("uAh", "microampere-hour", "微安时 / microampere-hour");
        assert_symbol_and_name::<AmpereHour>("Ah", "ampere-hour", "安时 / ampere-hour");
        assert_symbol_and_name::<KiloampereHour>("kAh", "kiloampere-hour", "千安时 / kiloampere-hour");
    }

    #[test]
    fn test_ampere_hour_conversions() {
        // 安时单位到库仑的换算系数：1 Ah = 3600 C / Conversion factors of ampere-hour units to coulomb: 1 Ah = 3600 C
        assert_factor_exact::<AmpereHour, Coulomb>("3600", "安时到库仑 / ampere-hour to coulomb");
        assert_factor_exact::<MilliampereHour, Coulomb>("3.6", "毫安时到库仑 / milliampere-hour to coulomb");
        assert_factor_exact::<MicroampereHour, Coulomb>("0.0036", "微安时到库仑 / microampere-hour to coulomb");
        assert_factor_exact::<KiloampereHour, Coulomb>("3600000", "千安时到库仑 / kiloampere-hour to coulomb");
    }

    #[test]
    fn test_ampere_hour_prefix_series() {
        // 安时单位按 1000 递进 / Ampere-hour units advance by 1000
        assert_factor_exact::<AmpereHour, MilliampereHour>("1000", "安时到毫安时 / ampere-hour to milliampere-hour");
        assert_factor_exact::<MilliampereHour, MicroampereHour>("1000", "毫安时到微安时 / milliampere-hour to microampere-hour");
        assert_factor_exact::<KiloampereHour, AmpereHour>("1000", "千安时到安时 / kiloampere-hour to ampere-hour");
    }

    #[test]
    fn test_ampere_hour_is_ampere_times_hour() {
        // 安时应为安培乘以小时，比例尺一致 / Ampere-hour should equal ampere times hour, with a matching scale
        assert_scale_equals::<AmpereHour, CTUnitMul<Ampere, Hour>>("安时分解 / ampere-hour decomposition");
        assert_scale_equals::<MilliampereHour, CTUnitMul<Milliampere, Hour>>("毫安时分解 / milliampere-hour decomposition");
    }

    #[test]
    fn test_electric_charge_differs_from_current() {
        // 电荷与电流量纲不同，电荷与安时量纲相同 / Electric charge differs from current and matches ampere-hour
        assert_different_dimension::<Coulomb, Ampere>("电荷与电流 / charge vs current");
        assert_same_dimension::<Coulomb, AmpereHour>("电荷与安时 / charge vs ampere-hour");
    }

    #[test]
    fn test_electric_potential_symbols_and_names() {
        // 电势单位的符号与名称 / Symbols and names of electric potential units
        assert_symbol_and_name::<Volt>("V", "volt", "伏特 / volt");
        assert_symbol_and_name::<Microvolt>("uV", "microvolt", "微伏 / microvolt");
        assert_symbol_and_name::<Millivolt>("mV", "millivolt", "毫伏 / millivolt");
        assert_symbol_and_name::<Kilovolt>("kV", "kilovolt", "千伏 / kilovolt");
        assert_symbol_and_name::<Megavolt>("MV", "megavolt", "兆伏 / megavolt");
    }

    #[test]
    fn test_electric_potential_conversion_to_volt() {
        // 电势单位到伏特的换算系数 / Conversion factors of electric potential units to volt
        assert_factor_exact::<Volt, Volt>("1", "伏特到伏特 / volt to volt");
        assert_factor_exact::<Microvolt, Volt>("0.000001", "微伏到伏特 / microvolt to volt");
        assert_factor_exact::<Millivolt, Volt>("0.001", "毫伏到伏特 / millivolt to volt");
        assert_factor_exact::<Kilovolt, Volt>("1000", "千伏到伏特 / kilovolt to volt");
        assert_factor_exact::<Megavolt, Volt>("1000000", "兆伏到伏特 / megavolt to volt");
    }

    #[test]
    fn test_capacitance_symbols_and_names() {
        // 电容单位的符号与名称 / Symbols and names of capacitance units
        assert_symbol_and_name::<Farad>("F", "farad", "法拉 / farad");
        assert_symbol_and_name::<Millifarad>("mF", "millifarad", "毫法拉 / millifarad");
        assert_symbol_and_name::<Microfarad>("uF", "microfarad", "微法拉 / microfarad");
        assert_symbol_and_name::<Nanofarad>("nF", "nanofarad", "纳法拉 / nanofarad");
        assert_symbol_and_name::<Picofarad>("pF", "picofarad", "皮法拉 / picofarad");
    }

    #[test]
    fn test_capacitance_conversion_to_farad() {
        // 电容单位到法拉的换算系数 / Conversion factors of capacitance units to farad
        assert_factor_exact::<Farad, Farad>("1", "法拉到法拉 / farad to farad");
        assert_factor_exact::<Millifarad, Farad>("0.001", "毫法拉到法拉 / millifarad to farad");
        assert_factor_exact::<Microfarad, Farad>("0.000001", "微法拉到法拉 / microfarad to farad");
        assert_factor_exact::<Nanofarad, Farad>("1e-9", "纳法拉到法拉 / nanofarad to farad");
        assert_factor_exact::<Picofarad, Farad>("1e-12", "皮法拉到法拉 / picofarad to farad");
    }

    #[test]
    fn test_electrical_dimension_symbols() {
        // 各电学量的量纲符号（按 L、M、T、I 的固定顺序生成）
        // Dimension symbols of the electrical quantities (generated in the fixed order L, M, T, I)
        assert_dimension_symbol::<Ampere>("I", "安培量纲 / ampere dimension");
        assert_dimension_symbol::<Coulomb>("T·I", "库仑量纲 / coulomb dimension");
        assert_dimension_symbol::<Volt>("L^2·M·T^-3·I^-1", "伏特量纲 / volt dimension");
        assert_dimension_symbol::<Farad>("L^-2·M^-1·T^4·I^2", "法拉量纲 / farad dimension");
    }

    #[test]
    fn test_electrical_units_share_dimension_within_quantity() {
        // 同类电学量共享量纲，不同类之间不同 / Units of the same electrical quantity share a dimension, different ones do not
        assert_same_dimension::<Ampere, Kiloampere>("安培与千安 / ampere vs kiloampere");
        assert_same_dimension::<Coulomb, AmpereHour>("库仑与安时 / coulomb vs ampere-hour");
        assert_same_dimension::<Volt, Millivolt>("伏特与毫伏 / volt vs millivolt");
        assert_same_dimension::<Farad, Picofarad>("法拉与皮法拉 / farad vs picofarad");
        assert_different_dimension::<Ampere, Coulomb>("安培与库仑 / ampere vs coulomb");
        assert_different_dimension::<Volt, Farad>("伏特与法拉 / volt vs farad");
        assert_different_dimension::<Coulomb, Volt>("库仑与伏特 / coulomb vs volt");
    }

    #[test]
    fn test_volt_is_watt_per_ampere() {
        // 伏特应为瓦特除以安培，比例尺一致 / Volt should equal watt divided by ampere, with a matching scale
        assert_scale_equals::<Volt, CTUnitDiv<Watt, Ampere>>("伏特分解 / volt decomposition");
    }

    #[test]
    fn test_farad_is_coulomb_per_volt() {
        // 法拉应为库仑除以伏特，比例尺一致 / Farad should equal coulomb divided by volt, with a matching scale
        assert_scale_equals::<Farad, CTUnitDiv<Coulomb, Volt>>("法拉分解 / farad decomposition");
    }

    #[test]
    fn test_electrical_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<AmpereHour>("Ah", "ampere-hour", "安时实例 / ampere-hour instance");
        assert_instance_symbol_and_name::<Picofarad>("pF", "picofarad", "皮法拉实例 / picofarad instance");
    }
}
