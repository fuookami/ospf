//! 长度单位 / Length units
//!
//! 提供长度量纲的 SI 单位定义，包括米、千米、厘米、毫米、微米、纳米等 / Provides SI unit definitions for length dimension, including meter, kilometer, centimeter, millimeter, micrometer, nanometer, etc

use crate::dimension::derived::Length;
use crate::scale::{CENTI, DECA, DECI, HECTO, KILO, MICRO, MILLI, NANO, PICO, Scale};
use crate::unit::CTUnit;

// ============================================================================
// SI 前缀单位 / SI prefix units
// ============================================================================

define_unit!(Meter, "meter", "m", Length);
define_unit!(Kilometer, "kilometer", "km", Length, KILO.clone());
define_unit!(Hectometer, "hectometer", "hm", Length, HECTO.clone());
define_unit!(Decameter, "decameter", "dam", Length, DECA.clone());
define_unit!(Decimeter, "decimeter", "dm", Length, DECI.clone());
define_unit!(Cetimeter, "centimeter", "cm", Length, CENTI.clone());
define_unit_by!(Centimeter, "centimeter", "cm", Cetimeter);
define_unit!(Millimeter, "millimeter", "mm", Length, MILLI.clone());
define_unit!(Micrometer, "micrometer", "μm", Length, MICRO.clone());
define_unit!(Nanometer, "nanometer", "nm", Length, NANO.clone());
define_unit!(Picometer, "picometer", "pm", Length, PICO.clone());

// ============================================================================
// 海里单位 / Nautical mile units
// ============================================================================

// 国际标准海里 / International nautical mile
define_unit!(
    NauticalMile,
    "nautical mile",
    "nmi",
    Length,
    Scale::from_f64(1852.0)
);

// 法国海里 / French nautical mile
define_unit!(
    FRNauticalMile,
    "french nautical mile",
    "fr.nmi",
    Length,
    Scale::from_f64(1853.27)
);

// 英国海里 / UK nautical mile
define_unit!(
    UKNauticalMile,
    "uk nautical mile",
    "uk.nmi",
    Length,
    Scale::from_f64(1854.55)
);

// 俄罗斯海里 / Russian nautical mile
define_unit!(
    RUNauticalMile,
    "russian nautical mile",
    "ru.nmi",
    Length,
    Scale::from_f64(1855.78)
);

// 美国海里 / US nautical mile
define_unit!(
    USNauticalMile,
    "us nautical mile",
    "us.nmi",
    Length,
    Scale::from_f64(1851.01)
);

// ============================================================================
// 英制单位 / Imperial units
// ============================================================================

// 英寸 / Inch
define_unit!(Inch, "inch", "in", Length, Scale::from_f64(0.0254));

// 英尺 / Foot
define_unit!(Foot, "foot", "ft", Length, Scale::from_f64(0.3048));

// 码 / Yard
define_unit!(Yard, "yard", "yd", Length, Scale::from_f64(0.9144));

// 链 / Chain
define_unit!(Chain, "chain", "ch", Length, Scale::from_f64(20.1168));

// 杆 / Rod
define_unit!(Rod, "rod", "rd", Length, Scale::from_f64(5.0292));

// 英里 / Mile
define_unit!(Mile, "mile", "mi", Length, Scale::from_f64(1609.344));

// 英寻 / Fathom
define_unit!(Fathom, "fathom", "fm", Length, Scale::from_f64(1.852));

// 链（海里单位）/ Cable
define_unit!(Cable, "cable", "cab", Length, Scale::from_f64(185.2));

// ============================================================================
// 天文单位 / Astronomical units
// ============================================================================

// 天文单位 / Astronomical unit
define_unit!(
    AstronomicalUnit,
    "astronomical unit",
    "au",
    Length,
    Scale::from_f64(149597870700.0)
);

// 光秒 / Light second
define_unit!(
    LightSecond,
    "light second",
    "lsc",
    Length,
    Scale::from_f64(299792458.0)
);

// 光分 / Light minute
define_unit!(
    LightMinute,
    "light minute",
    "lmn",
    Length,
    Scale::from_f64(17987547480.0)
);

// 光时 / Light hour
define_unit!(
    LightHour,
    "light hour",
    "lhr",
    Length,
    Scale::from_f64(1079252848800.0)
);

// 光日 / Light day
define_unit!(
    LightDay,
    "light day",
    "ldy",
    Length,
    Scale::from_f64(25902068371200.0)
);

// 光年 / Light year
define_unit!(
    LightYear,
    "light year",
    "ly",
    Length,
    Scale::from_f64(9460730472580800.0)
);

// 秒差距 / Parsec
define_unit!(
    Parsec,
    "parsec",
    "pc",
    Length,
    Scale::from_f64(30856775814913673.0)
);

// 千秒差距 / Kiloparsec
define_unit!(
    Kiloparsec,
    "kiloparsec",
    "kpc",
    Length,
    Scale::from_f64(3.0856775814913673e19)
);

// 百万秒差距 / Megaparsec
define_unit!(
    Megaparsec,
    "megaparsec",
    "Mpc",
    Length,
    Scale::from_f64(3.0856775814913673e22)
);

// 十亿秒差距 / Gigaparsec
define_unit!(
    Gigaparsec,
    "gigaparsec",
    "Gpc",
    Length,
    Scale::from_f64(3.0856775814913673e25)
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
    fn test_si_prefix_length_symbols_and_names() {
        // SI 前缀长度单位的符号与名称 / Symbols and names of SI prefix length units
        assert_symbol_and_name::<Meter>("m", "meter", "米 / meter");
        assert_symbol_and_name::<Kilometer>("km", "kilometer", "千米 / kilometer");
        assert_symbol_and_name::<Hectometer>("hm", "hectometer", "百米 / hectometer");
        assert_symbol_and_name::<Decameter>("dam", "decameter", "十米 / decameter");
        assert_symbol_and_name::<Decimeter>("dm", "decimeter", "分米 / decimeter");
        assert_symbol_and_name::<Cetimeter>("cm", "centimeter", "厘米 / centimeter");
        assert_symbol_and_name::<Centimeter>("cm", "centimeter", "厘米别名 / centimeter alias");
        assert_symbol_and_name::<Millimeter>("mm", "millimeter", "毫米 / millimeter");
        assert_symbol_and_name::<Micrometer>("μm", "micrometer", "微米 / micrometer");
        assert_symbol_and_name::<Nanometer>("nm", "nanometer", "纳米 / nanometer");
        assert_symbol_and_name::<Picometer>("pm", "picometer", "皮米 / picometer");
    }

    #[test]
    fn test_si_prefix_length_conversion_to_meter() {
        // SI 前缀长度单位到米的换算系数 / Conversion factors of SI prefix length units to meter
        assert_factor_exact::<Meter, Meter>("1", "米到米 / meter to meter");
        assert_factor_exact::<Kilometer, Meter>("1000", "千米到米 / kilometer to meter");
        assert_factor_exact::<Hectometer, Meter>("100", "百米到米 / hectometer to meter");
        assert_factor_exact::<Decameter, Meter>("10", "十米到米 / decameter to meter");
        assert_factor_exact::<Decimeter, Meter>("0.1", "分米到米 / decimeter to meter");
        assert_factor_exact::<Centimeter, Meter>("0.01", "厘米到米 / centimeter to meter");
        assert_factor_exact::<Cetimeter, Meter>("0.01", "厘米别名到米 / centimeter alias to meter");
        assert_factor_exact::<Millimeter, Meter>("0.001", "毫米到米 / millimeter to meter");
        assert_factor_exact::<Micrometer, Meter>("0.000001", "微米到米 / micrometer to meter");
        assert_factor_exact::<Nanometer, Meter>("1e-9", "纳米到米 / nanometer to meter");
        assert_factor_exact::<Picometer, Meter>("1e-12", "皮米到米 / picometer to meter");
    }

    #[test]
    fn test_centimeter_alias_matches_original() {
        // Centimeter 是 Cetimeter 的兼容别名，比例尺完全相同
        // Centimeter is a compatibility alias of Cetimeter with an identical scale
        assert_scale_equals::<Centimeter, Cetimeter>("厘米别名 / centimeter alias");
        assert_factor_exact::<Centimeter, Cetimeter>("1", "厘米别名换算 / centimeter alias factor");
    }

    #[test]
    fn test_nautical_mile_units() {
        // 各国海里的符号与名称 / Symbols and names of national nautical miles
        assert_symbol_and_name::<NauticalMile>("nmi", "nautical mile", "国际海里 / international nautical mile");
        assert_symbol_and_name::<FRNauticalMile>(
            "fr.nmi",
            "french nautical mile",
            "法国海里 / french nautical mile",
        );
        assert_symbol_and_name::<UKNauticalMile>(
            "uk.nmi",
            "uk nautical mile",
            "英国海里 / uk nautical mile",
        );
        assert_symbol_and_name::<RUNauticalMile>(
            "ru.nmi",
            "russian nautical mile",
            "俄罗斯海里 / russian nautical mile",
        );
        assert_symbol_and_name::<USNauticalMile>(
            "us.nmi",
            "us nautical mile",
            "美国海里 / us nautical mile",
        );
    }

    #[test]
    fn test_nautical_mile_conversion_to_meter() {
        // 各国海里到米的换算系数 / Conversion factors of national nautical miles to meter
        assert_factor_exact::<NauticalMile, Meter>("1852", "国际海里到米 / international nautical mile to meter");
        assert_factor_relative::<FRNauticalMile, Meter>("1853.27", FUZZ, "法国海里到米 / french nautical mile to meter");
        assert_factor_relative::<UKNauticalMile, Meter>("1854.55", FUZZ, "英国海里到米 / uk nautical mile to meter");
        assert_factor_relative::<RUNauticalMile, Meter>("1855.78", FUZZ, "俄罗斯海里到米 / russian nautical mile to meter");
        assert_factor_relative::<USNauticalMile, Meter>("1851.01", FUZZ, "美国海里到米 / us nautical mile to meter");
    }

    #[test]
    fn test_imperial_length_symbols_and_names() {
        // 英制长度单位的符号与名称 / Symbols and names of imperial length units
        assert_symbol_and_name::<Inch>("in", "inch", "英寸 / inch");
        assert_symbol_and_name::<Foot>("ft", "foot", "英尺 / foot");
        assert_symbol_and_name::<Yard>("yd", "yard", "码 / yard");
        assert_symbol_and_name::<Chain>("ch", "chain", "链 / chain");
        assert_symbol_and_name::<Rod>("rd", "rod", "杆 / rod");
        assert_symbol_and_name::<Mile>("mi", "mile", "英里 / mile");
        assert_symbol_and_name::<Fathom>("fm", "fathom", "英寻 / fathom");
        assert_symbol_and_name::<Cable>("cab", "cable", "缆长 / cable");
    }

    #[test]
    fn test_imperial_length_conversion_to_meter() {
        // 英制长度单位到米的换算系数 / Conversion factors of imperial length units to meter
        assert_factor_relative::<Inch, Meter>("0.0254", FUZZ, "英寸到米 / inch to meter");
        assert_factor_relative::<Foot, Meter>("0.3048", FUZZ, "英尺到米 / foot to meter");
        assert_factor_relative::<Yard, Meter>("0.9144", FUZZ, "码到米 / yard to meter");
        assert_factor_relative::<Chain, Meter>("20.1168", FUZZ, "链到米 / chain to meter");
        assert_factor_relative::<Rod, Meter>("5.0292", FUZZ, "杆到米 / rod to meter");
        assert_factor_relative::<Mile, Meter>("1609.344", FUZZ, "英里到米 / mile to meter");
        assert_factor_relative::<Fathom, Meter>("1.852", FUZZ, "英寻到米 / fathom to meter");
        assert_factor_relative::<Cable, Meter>("185.2", FUZZ, "缆长到米 / cable to meter");
    }

    #[test]
    fn test_imperial_length_internal_ratios() {
        // 英制长度单位之间的整数比例关系 / Integer ratios between imperial length units
        assert_factor_relative::<Foot, Inch>("12", FUZZ, "英尺到英寸 / foot to inch");
        assert_factor_relative::<Yard, Foot>("3", FUZZ, "码到英尺 / yard to foot");
        assert_factor_relative::<Yard, Inch>("36", FUZZ, "码到英寸 / yard to inch");
        assert_factor_relative::<Chain, Yard>("22", FUZZ, "链到码 / chain to yard");
        assert_factor_relative::<Rod, Yard>("5.5", FUZZ, "杆到码 / rod to yard");
        assert_factor_relative::<Mile, Foot>("5280", FUZZ, "英里到英尺 / mile to foot");
        assert_factor_relative::<Mile, Yard>("1760", FUZZ, "英里到码 / mile to yard");
        assert_factor_relative::<Mile, Chain>("80", FUZZ, "英里到链 / mile to chain");
        assert_factor_relative::<Mile, Rod>("320", FUZZ, "英里到杆 / mile to rod");
    }

    #[test]
    fn test_astronomical_length_symbols_and_names() {
        // 天文长度单位的符号与名称 / Symbols and names of astronomical length units
        assert_symbol_and_name::<AstronomicalUnit>("au", "astronomical unit", "天文单位 / astronomical unit");
        assert_symbol_and_name::<LightSecond>("lsc", "light second", "光秒 / light second");
        assert_symbol_and_name::<LightMinute>("lmn", "light minute", "光分 / light minute");
        assert_symbol_and_name::<LightHour>("lhr", "light hour", "光时 / light hour");
        assert_symbol_and_name::<LightDay>("ldy", "light day", "光日 / light day");
        assert_symbol_and_name::<LightYear>("ly", "light year", "光年 / light year");
        assert_symbol_and_name::<Parsec>("pc", "parsec", "秒差距 / parsec");
        assert_symbol_and_name::<Kiloparsec>("kpc", "kiloparsec", "千秒差距 / kiloparsec");
        assert_symbol_and_name::<Megaparsec>("Mpc", "megaparsec", "百万秒差距 / megaparsec");
        assert_symbol_and_name::<Gigaparsec>("Gpc", "gigaparsec", "十亿秒差距 / gigaparsec");
    }

    #[test]
    fn test_light_travel_distance_conversion_to_meter() {
        // 光速行程单位到米的换算系数 / Conversion factors of light travel units to meter
        assert_factor_exact::<LightSecond, Meter>("299792458", "光秒到米 / light second to meter");
        assert_factor_exact::<LightMinute, Meter>("17987547480", "光分到米 / light minute to meter");
        assert_factor_exact::<LightHour, Meter>("1079252848800", "光时到米 / light hour to meter");
        assert_factor_exact::<LightDay, Meter>("25902068371200", "光日到米 / light day to meter");
        assert_factor_exact::<LightYear, Meter>("9460730472580800", "光年到米 / light year to meter");
    }

    #[test]
    fn test_light_travel_distance_is_sixty_based() {
        // 光分、光时、光日为 60 进制递进 / Light minute, hour and day advance in base 60
        assert_factor_exact::<LightMinute, LightSecond>("60", "光分到光秒 / light minute to light second");
        assert_factor_exact::<LightHour, LightMinute>("60", "光时到光分 / light hour to light minute");
        assert_factor_exact::<LightDay, LightHour>("24", "光日到光时 / light day to light hour");
    }

    #[test]
    fn test_parsec_series_ratio_is_one_thousand() {
        // 千秒差距、百万秒差距、十亿秒差距之间为 1000 倍关系
        // Kiloparsec, megaparsec and gigaparsec are related by a factor of 1000
        assert_factor_relative::<Kiloparsec, Parsec>("1000", FUZZ, "千秒差距到秒差距 / kiloparsec to parsec");
        assert_factor_relative::<Megaparsec, Kiloparsec>("1000", FUZZ, "百万秒差距到千秒差距 / megaparsec to kiloparsec");
        assert_factor_relative::<Gigaparsec, Megaparsec>("1000", FUZZ, "十亿秒差距到百万秒差距 / gigaparsec to megaparsec");
    }

    #[test]
    fn test_parsec_is_close_to_astronomical_unit_ratio() {
        // 1 秒差距约等于 206264.8 天文单位（天文常数定义值）
        // One parsec is about 206264.8 astronomical units (by astronomical constant definition)
        assert_factor_relative::<Parsec, AstronomicalUnit>(
            "206264.80624709636",
            "1e-6",
            "秒差距到天文单位 / parsec to astronomical unit",
        );
    }

    #[test]
    fn test_astronomical_unit_is_exactly_defined() {
        // 天文单位按定义精确等于 149597870700 米 / The astronomical unit is exactly 149597870700 meters by definition
        assert_scale_exact(
            AstronomicalUnit::SCALE.value(),
            "149597870700",
            "天文单位比例尺 / astronomical unit scale",
        );
    }

    #[test]
    fn test_length_units_share_length_dimension() {
        // 所有长度单位共享长度量纲 / All length units share the length dimension
        assert_same_dimension::<Meter, Mile>("米与英里 / meter vs mile");
        assert_same_dimension::<Meter, Parsec>("米与秒差距 / meter vs parsec");
        assert_different_dimension::<Meter, crate::unit::derived::Second>("米与秒 / meter vs second");
        assert_different_dimension::<Meter, crate::unit::derived::Kilogram>("米与千克 / meter vs kilogram");
    }

    #[test]
    fn test_length_dimension_symbol_and_domain() {
        // 长度量纲符号为 L，取值域为连续 / Length dimension symbol is L and the domain is continuous
        assert_dimension_symbol::<Meter>("L", "米量纲 / meter dimension");
        assert_domain::<Meter>(
            crate::dimension::derived_quantity::QuantityDomain::Continuous,
            "米取值域 / meter domain",
        );
    }

    #[test]
    fn test_length_unit_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<Kilometer>("km", "kilometer", "千米实例 / kilometer instance");
        assert_instance_symbol_and_name::<Nanometer>("nm", "nanometer", "纳米实例 / nanometer instance");
    }
}
