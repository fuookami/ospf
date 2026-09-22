//! 速度单位 / Velocity units
//!
//! 提供速度量纲的 SI 单位定义，包括米每秒、千米每小时等 / Provides SI unit definitions for velocity dimension, including meter per second, kilometer per hour, etc

use super::length::{
    Cetimeter, FRNauticalMile, Foot, Inch, Kilometer, Meter, Mile, NauticalMile, RUNauticalMile,
    UKNauticalMile, USNauticalMile,
};
use super::time::{Hour, Second};
use crate::dimension::derived::Velocity;
use crate::scale::Scale;
use crate::unit::{CTUnit, CTUnitDiv};

// ============================================================================
// SI 速度单位 / SI velocity units
// ============================================================================

define_unit_by!(
    MeterPerSecond,
    "meter per second",
    "m/s",
    CTUnitDiv<Meter, Second>
);

define_unit_by!(
    CentimeterPerSecond,
    "centimeter per second",
    "cm/s",
    CTUnitDiv<Cetimeter, Second>
);

define_unit_by!(
    KilometerPerSecond,
    "kilometer per second",
    "km/s",
    CTUnitDiv<Kilometer, Second>
);
define_unit_by!(
    KilometersPerSecond,
    "kilometers per second",
    "km/s",
    KilometerPerSecond
);

define_unit_by!(
    KilometerPerHour,
    "kilometer per hour",
    "km/h",
    CTUnitDiv<Kilometer, Hour>
);

// ============================================================================
// 英制速度单位 / Imperial velocity units
// ============================================================================

// 英寸每秒 / Inch per second
define_unit_by!(
    InchPerSecond,
    "inch per second",
    "ips",
    CTUnitDiv<Inch, Second>
);

// 英尺每秒 / Foot per second
define_unit_by!(
    FootPerSecond,
    "foot per second",
    "fps",
    CTUnitDiv<Foot, Second>
);

// 英里每小时 / Mile per hour
define_unit_by!(
    MilePerHour,
    "mile per hour",
    "mph",
    CTUnitDiv<Mile, Hour>
);

// ============================================================================
// 特殊速度单位 / Special velocity units
// ============================================================================

// 节 / Knot (nautical mile per hour)
define_unit_by!(
    Knot,
    "knot",
    "kn",
    CTUnitDiv<NauticalMile, Hour>
);
define_unit_by!(
    FRKnot,
    "fr knot",
    "fr.kn",
    CTUnitDiv<FRNauticalMile, Hour>
);
define_unit_by!(
    UKKnot,
    "uk knot",
    "uk.kn",
    CTUnitDiv<UKNauticalMile, Hour>
);
define_unit_by!(
    RUKnot,
    "ru knot",
    "ru.kn",
    CTUnitDiv<RUNauticalMile, Hour>
);
define_unit_by!(
    USKnot,
    "us knot",
    "us.kn",
    CTUnitDiv<USNauticalMile, Hour>
);

// 马赫 / Mach (speed of sound, approximately 340.3 m/s)
define_unit!(Mach, "mach", "ma", Velocity, Scale::from_f64(340.3));

// 光速 / Light speed
define_unit!(
    LightSpeed,
    "light speed",
    "c",
    Velocity,
    Scale::from_f64(299792458.0)
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
    fn test_si_velocity_symbols_and_names() {
        // SI 速度单位的符号与名称 / Symbols and names of SI velocity units
        assert_symbol_and_name::<MeterPerSecond>("m/s", "meter per second", "米每秒 / meter per second");
        assert_symbol_and_name::<CentimeterPerSecond>("cm/s", "centimeter per second", "厘米每秒 / centimeter per second");
        assert_symbol_and_name::<KilometerPerSecond>("km/s", "kilometer per second", "千米每秒 / kilometer per second");
        assert_symbol_and_name::<KilometersPerSecond>("km/s", "kilometers per second", "千米每秒别名 / kilometers per second alias");
        assert_symbol_and_name::<KilometerPerHour>("km/h", "kilometer per hour", "千米每小时 / kilometer per hour");
    }

    #[test]
    fn test_si_velocity_conversion_to_meter_per_second() {
        // SI 速度单位到米每秒的换算系数 / Conversion factors of SI velocity units to meter per second
        assert_factor_exact::<MeterPerSecond, MeterPerSecond>("1", "米每秒到米每秒 / meter per second to meter per second");
        assert_factor_exact::<CentimeterPerSecond, MeterPerSecond>("0.01", "厘米每秒到米每秒 / centimeter per second to meter per second");
        assert_factor_exact::<KilometerPerSecond, MeterPerSecond>("1000", "千米每秒到米每秒 / kilometer per second to meter per second");
    }

    #[test]
    fn test_kilometer_per_hour_equals_five_over_eighteen_meter_per_second() {
        // 1 千米每小时等于 1/3.6 米每秒 / One kilometer per hour equals 1/3.6 meters per second
        assert_factor_relative::<KilometerPerHour, MeterPerSecond>(
            "0.27777777777777777777777777777777777777777777777778",
            FUZZ,
            "千米每小时到米每秒 / kilometer per hour to meter per second",
        );
    }

    #[test]
    fn test_kilometers_per_second_alias() {
        // KilometersPerSecond 是 KilometerPerSecond 的等价单位 / KilometersPerSecond is an equivalent unit of KilometerPerSecond
        assert_scale_equals::<KilometersPerSecond, KilometerPerSecond>("千米每秒别名 / kilometers per second alias");
        assert_factor_exact::<KilometersPerSecond, KilometerPerSecond>("1", "千米每秒别名换算 / kilometers per second alias factor");
    }

    #[test]
    fn test_imperial_velocity_symbols_and_names() {
        // 英制速度单位的符号与名称 / Symbols and names of imperial velocity units
        assert_symbol_and_name::<InchPerSecond>("ips", "inch per second", "英寸每秒 / inch per second");
        assert_symbol_and_name::<FootPerSecond>("fps", "foot per second", "英尺每秒 / foot per second");
        assert_symbol_and_name::<MilePerHour>("mph", "mile per hour", "英里每小时 / mile per hour");
    }

    #[test]
    fn test_imperial_velocity_conversion_to_meter_per_second() {
        // 英制速度单位到米每秒的换算系数 / Conversion factors of imperial velocity units to meter per second
        assert_factor_relative::<InchPerSecond, MeterPerSecond>("0.0254", FUZZ, "英寸每秒到米每秒 / inch per second to meter per second");
        assert_factor_relative::<FootPerSecond, MeterPerSecond>("0.3048", FUZZ, "英尺每秒到米每秒 / foot per second to meter per second");
        assert_factor_relative::<MilePerHour, MeterPerSecond>("0.44704", FUZZ, "英里每小时到米每秒 / mile per hour to meter per second");
        assert_factor_relative::<MilePerHour, FootPerSecond>(
            "1.4666666666666666666666666666666666666666666666667",
            FUZZ,
            "英里每小时到英尺每秒 / mile per hour to foot per second",
        );
    }

    #[test]
    fn test_knot_symbols_and_names() {
        // 节的符号与名称 / Symbols and names of knot units
        assert_symbol_and_name::<Knot>("kn", "knot", "节 / knot");
        assert_symbol_and_name::<FRKnot>("fr.kn", "fr knot", "法国节 / fr knot");
        assert_symbol_and_name::<UKKnot>("uk.kn", "uk knot", "英国节 / uk knot");
        assert_symbol_and_name::<RUKnot>("ru.kn", "ru knot", "俄罗斯节 / ru knot");
        assert_symbol_and_name::<USKnot>("us.kn", "us knot", "美国节 / us knot");
    }

    #[test]
    fn test_knot_equals_nautical_mile_per_hour() {
        // 1 节等于每小时 1 国际海里，即 1/3600 海里每秒
        // One knot equals one international nautical mile per hour, that is 1/3600 nautical mile per second
        assert_factor_relative::<Knot, MeterPerSecond>(
            "0.51444444444444444444444444444444444444444444444444",
            FUZZ,
            "节到米每秒 / knot to meter per second",
        );
        // 节为速度单位（L·T^-1），海里为长度单位（L），两者量纲不同不可换算
        // Knot is a velocity unit (L·T^-1) while nautical mile is a length unit (L); they are not inter-convertible
        assert!(Knot::conversion_factor_to::<NauticalMile>().is_none());
        use crate::unit::CTUnitDiv;
        assert_scale_equals::<Knot, CTUnitDiv<NauticalMile, Hour>>("国际节分解 / international knot decomposition");
        assert_factor_exact::<NauticalMile, Meter>("1852", "海里到米 / nautical mile to meter");
    }

    #[test]
    fn test_national_knots_follow_national_nautical_miles() {
        // 各国节单位定义为对应海里除以小时 / Each national knot is defined as the corresponding nautical mile divided by hour
        assert_scale_equals::<Knot, CTUnitDiv<NauticalMile, Hour>>("国际节分解 / international knot decomposition");
        assert_scale_equals::<FRKnot, CTUnitDiv<FRNauticalMile, Hour>>("法国节分解 / fr knot decomposition");
        assert_scale_equals::<UKKnot, CTUnitDiv<UKNauticalMile, Hour>>("英国节分解 / uk knot decomposition");
        assert_scale_equals::<RUKnot, CTUnitDiv<RUNauticalMile, Hour>>("俄罗斯节分解 / ru knot decomposition");
        assert_scale_equals::<USKnot, CTUnitDiv<USNauticalMile, Hour>>("美国节分解 / us knot decomposition");
        assert_factor_relative::<FRKnot, MeterPerSecond>(
            "0.51479722222222222222222222222222222222222222222222",
            FUZZ,
            "法国节到米每秒 / fr knot to meter per second",
        );
        assert_factor_relative::<UKKnot, MeterPerSecond>(
            "0.51515277777777777777777777777777777777777777777778",
            FUZZ,
            "英国节到米每秒 / uk knot to meter per second",
        );
        assert_factor_relative::<RUKnot, MeterPerSecond>(
            "0.51549444444444444444444444444444444444444444444444",
            FUZZ,
            "俄罗斯节到米每秒 / ru knot to meter per second",
        );
        assert_factor_relative::<USKnot, MeterPerSecond>(
            "0.51416944444444444444444444444444444444444444444444",
            FUZZ,
            "美国节到米每秒 / us knot to meter per second",
        );
    }

    #[test]
    fn test_special_velocity_symbols_and_names() {
        // 特殊速度单位的符号与名称 / Symbols and names of special velocity units
        assert_symbol_and_name::<Mach>("ma", "mach", "马赫 / mach");
        assert_symbol_and_name::<LightSpeed>("c", "light speed", "光速 / light speed");
    }

    #[test]
    fn test_mach_equals_340_point_3_meter_per_second() {
        // 马赫取近似音速 340.3 米每秒 / Mach uses an approximate speed of sound of 340.3 meters per second
        assert_factor_relative::<Mach, MeterPerSecond>("340.3", FUZZ, "马赫到米每秒 / mach to meter per second");
    }

    #[test]
    fn test_light_speed_equals_exact_vacuum_light_speed() {
        // 光速等于真空光速 299792458 米每秒 / Light speed equals the vacuum speed of light 299792458 meters per second
        assert_factor_exact::<LightSpeed, MeterPerSecond>("299792458", "光速到米每秒 / light speed to meter per second");
    }

    #[test]
    fn test_velocity_dimension_symbol_and_domain() {
        // 速度量纲符号为 L·T^-1，取值域为连续 / Velocity dimension symbol is L·T^-1 and the domain is continuous
        assert_dimension_symbol::<MeterPerSecond>("L·T^-1", "米每秒量纲 / meter per second dimension");
        assert_domain::<MeterPerSecond>(
            crate::dimension::derived_quantity::QuantityDomain::Continuous,
            "米每秒取值域 / meter per second domain",
        );
    }

    #[test]
    fn test_velocity_units_share_dimension() {
        // 所有速度单位共享速度量纲 / All velocity units share the velocity dimension
        assert_same_dimension::<MeterPerSecond, Knot>("米每秒与节 / meter per second vs knot");
        assert_same_dimension::<MeterPerSecond, LightSpeed>("米每秒与光速 / meter per second vs light speed");
        assert_different_dimension::<MeterPerSecond, Meter>("米每秒与米 / meter per second vs meter");
        assert_different_dimension::<MeterPerSecond, super::super::acceleration::MeterPerSecondSquared>(
            "米每秒与米每二次方秒 / meter per second vs meter per second squared",
        );
    }

    #[test]
    fn test_velocity_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<KilometerPerHour>("km/h", "kilometer per hour", "千米每小时实例 / kilometer per hour instance");
        assert_instance_symbol_and_name::<Knot>("kn", "knot", "节实例 / knot instance");
    }
}
