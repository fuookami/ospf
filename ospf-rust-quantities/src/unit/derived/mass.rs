//! 质量单位 / Mass units
//!
//! 提供质量量纲的 SI 单位定义，包括千克、克、毫克、吨等 / Provides SI unit definitions for mass dimension, including kilogram, gram, milligram, tonne, etc

use crate::dimension::derived::Mass;
use crate::scale::{KILO, MILLI, Scale};
use crate::unit::CTUnit;

// ============================================================================
// SI 质量单位 / SI mass units
// ============================================================================

define_unit!(Kilogram, "kilogram", "kg", Mass);
define_unit!(Gram, "gram", "g", Mass, MILLI.clone());
define_unit!(Hectogram, "hectogram", "hg", Mass, Scale::from_f64(0.1));
define_unit!(
    Milligram,
    "milligram",
    "mg",
    Mass,
    Scale::from_int(1) / Scale::from_int(1000000)
);
define_unit!(
    Microgram,
    "microgram",
    "μg",
    Mass,
    Scale::from_f64(0.000000001)
);
define_unit!(Tonne, "tonne", "t", Mass, KILO.clone());

// ============================================================================
// 公制质量单位 / Metric mass units
// ============================================================================

// 公担 / Kintal (quintal)
define_unit!(Kintal, "kintal", "q", Mass, Scale::from_f64(100.0));

// 克拉 / Carat
define_unit!(Carat, "carat", "ct", Mass, Scale::from_f64(0.0002));

// 点 / Point (2 milligrams)
define_unit!(Point, "point", "pt", Mass, Scale::from_f64(0.000002));

// ============================================================================
// 英制质量单位 / Imperial mass units
// ============================================================================

// 磅 / Pound
define_unit!(Pound, "pound", "lb", Mass, Scale::from_f64(0.45359237));

// 格令 / Grain
define_unit!(Gran, "grain", "gr", Mass, Scale::from_f64(0.00006479891));

// 英吨 / Long ton (2240 pounds)
define_unit!(
    LongTon,
    "long ton",
    "lt",
    Mass,
    Scale::from_f64(1016.0469088)
);

// 美吨 / Short ton (2000 pounds)
define_unit!(
    ShortTon,
    "short ton",
    "st",
    Mass,
    Scale::from_f64(907.18474)
);

// 英石 / Stone (14 pounds)
define_unit!(Stone, "stone", "st", Mass, Scale::from_f64(6.35029318));

// 盎司 / Ounce
define_unit!(Ounce, "ounce", "oz", Mass, Scale::from_f64(0.028349523125));

// 金衡盎司 / Troy ounce
define_unit!(
    TroyOunce,
    "troy ounce",
    "oz.tr",
    Mass,
    Scale::from_f64(0.0311034768)
);

// 打兰 / Dram
define_unit!(
    Dram,
    "dram",
    "dr",
    Mass,
    Scale::from_f64(0.0017718451953125)
);

// Kotlin 命名兼容别名 / Kotlin naming compatibility alias
pub type Ton = Tonne;

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
    fn test_si_mass_symbols_and_names() {
        // SI 质量单位的符号与名称 / Symbols and names of SI mass units
        assert_symbol_and_name::<Kilogram>("kg", "kilogram", "千克 / kilogram");
        assert_symbol_and_name::<Gram>("g", "gram", "克 / gram");
        assert_symbol_and_name::<Hectogram>("hg", "hectogram", "百克 / hectogram");
        assert_symbol_and_name::<Milligram>("mg", "milligram", "毫克 / milligram");
        assert_symbol_and_name::<Microgram>("μg", "microgram", "微克 / microgram");
        assert_symbol_and_name::<Tonne>("t", "tonne", "吨 / tonne");
    }

    #[test]
    fn test_si_mass_conversion_to_kilogram() {
        // SI 质量单位到千克的换算系数 / Conversion factors of SI mass units to kilogram
        assert_factor_exact::<Kilogram, Kilogram>("1", "千克到千克 / kilogram to kilogram");
        assert_factor_exact::<Gram, Kilogram>("0.001", "克到千克 / gram to kilogram");
        assert_factor_relative::<Hectogram, Kilogram>("0.1", FUZZ, "百克到千克 / hectogram to kilogram");
        assert_factor_exact::<Milligram, Kilogram>("0.000001", "毫克到千克 / milligram to kilogram");
        assert_factor_relative::<Microgram, Kilogram>("1e-9", FUZZ, "微克到千克 / microgram to kilogram");
        assert_factor_exact::<Tonne, Kilogram>("1000", "吨到千克 / tonne to kilogram");
    }

    #[test]
    fn test_si_mass_internal_ratios() {
        // SI 质量单位之间的整数比例关系 / Integer ratios between SI mass units
        assert_factor_relative::<Hectogram, Gram>("100", FUZZ, "百克到克 / hectogram to gram");
        assert_factor_relative::<Kilogram, Gram>("1000", FUZZ, "千克到克 / kilogram to gram");
        assert_factor_relative::<Gram, Milligram>("1000", FUZZ, "克到毫克 / gram to milligram");
        assert_factor_relative::<Kilogram, Tonne>("0.001", FUZZ, "千克到吨 / kilogram to tonne");
    }

    #[test]
    fn test_metric_mass_symbols_and_names() {
        // 公制质量单位的符号与名称 / Symbols and names of metric mass units
        assert_symbol_and_name::<Kintal>("q", "kintal", "公担 / kintal");
        assert_symbol_and_name::<Carat>("ct", "carat", "克拉 / carat");
        assert_symbol_and_name::<Point>("pt", "point", "点 / point");
    }

    #[test]
    fn test_metric_mass_conversion_to_kilogram() {
        // 公制质量单位到千克的换算系数 / Conversion factors of metric mass units to kilogram
        assert_factor_relative::<Kintal, Kilogram>("100", FUZZ, "公担到千克 / kintal to kilogram");
        assert_factor_relative::<Carat, Kilogram>("0.0002", FUZZ, "克拉到千克 / carat to kilogram");
        assert_factor_relative::<Point, Kilogram>("0.000002", FUZZ, "点到千克 / point to kilogram");
    }

    #[test]
    fn test_carat_is_two_hundred_milligram() {
        // 1 克拉等于 200 毫克 / One carat equals 200 milligrams
        assert_factor_relative::<Carat, Milligram>("200", FUZZ, "克拉到毫克 / carat to milligram");
    }

    #[test]
    fn test_point_is_two_milligram() {
        // 1 点等于 2 毫克 / One point equals two milligrams
        assert_factor_relative::<Point, Milligram>("2", FUZZ, "点到毫克 / point to milligram");
    }

    #[test]
    fn test_imperial_mass_symbols_and_names() {
        // 英制质量单位的符号与名称 / Symbols and names of imperial mass units
        assert_symbol_and_name::<Pound>("lb", "pound", "磅 / pound");
        assert_symbol_and_name::<Gran>("gr", "grain", "格令 / grain");
        assert_symbol_and_name::<LongTon>("lt", "long ton", "英吨 / long ton");
        assert_symbol_and_name::<ShortTon>("st", "short ton", "美吨 / short ton");
        assert_symbol_and_name::<Stone>("st", "stone", "英石 / stone");
        assert_symbol_and_name::<Ounce>("oz", "ounce", "盎司 / ounce");
        assert_symbol_and_name::<TroyOunce>("oz.tr", "troy ounce", "金衡盎司 / troy ounce");
        assert_symbol_and_name::<Dram>("dr", "dram", "打兰 / dram");
    }

    #[test]
    fn test_imperial_mass_conversion_to_kilogram() {
        // 英制质量单位到千克的换算系数 / Conversion factors of imperial mass units to kilogram
        assert_factor_relative::<Pound, Kilogram>("0.45359237", FUZZ, "磅到千克 / pound to kilogram");
        assert_factor_relative::<Gran, Kilogram>("0.00006479891", FUZZ, "格令到千克 / grain to kilogram");
        assert_factor_relative::<LongTon, Kilogram>("1016.0469088", FUZZ, "英吨到千克 / long ton to kilogram");
        assert_factor_relative::<ShortTon, Kilogram>("907.18474", FUZZ, "美吨到千克 / short ton to kilogram");
        assert_factor_relative::<Stone, Kilogram>("6.35029318", FUZZ, "英石到千克 / stone to kilogram");
        assert_factor_relative::<Ounce, Kilogram>("0.028349523125", FUZZ, "盎司到千克 / ounce to kilogram");
        assert_factor_relative::<TroyOunce, Kilogram>("0.0311034768", FUZZ, "金衡盎司到千克 / troy ounce to kilogram");
        assert_factor_relative::<Dram, Kilogram>("0.0017718451953125", FUZZ, "打兰到千克 / dram to kilogram");
    }

    #[test]
    fn test_imperial_mass_internal_ratios() {
        // 英制质量单位之间的比例关系 / Ratios between imperial mass units
        assert_factor_relative::<Pound, Ounce>("16", FUZZ, "磅到盎司 / pound to ounce");
        assert_factor_relative::<Stone, Pound>("14", FUZZ, "英石到磅 / stone to pound");
        assert_factor_relative::<LongTon, Pound>("2240", FUZZ, "英吨到磅 / long ton to pound");
        assert_factor_relative::<ShortTon, Pound>("2000", FUZZ, "美吨到磅 / short ton to pound");
        assert_factor_relative::<Pound, Gran>("7000", FUZZ, "磅到格令 / pound to grain");
        assert_factor_relative::<Ounce, Dram>("16", FUZZ, "盎司到打兰 / ounce to dram");
        assert_factor_relative::<TroyOunce, Gran>("480", FUZZ, "金衡盎司到格令 / troy ounce to grain");
    }

    #[test]
    fn test_tonne_alias_matches_tonne() {
        // Ton 是 Tonne 的兼容别名，比例尺完全相同 / Ton is an alias of Tonne with an identical scale
        assert_scale_equals::<Ton, Tonne>("吨别名 / ton alias");
        assert_factor_exact::<Ton, Tonne>("1", "吨别名换算 / ton alias factor");
    }

    #[test]
    fn test_mass_units_share_mass_dimension() {
        // 所有质量单位共享质量量纲 / All mass units share the mass dimension
        assert_same_dimension::<Kilogram, Pound>("千克与磅 / kilogram vs pound");
        assert_same_dimension::<Kilogram, Tonne>("千克与吨 / kilogram vs tonne");
        assert_different_dimension::<Kilogram, crate::unit::derived::Meter>("千克与米 / kilogram vs meter");
        assert_different_dimension::<Kilogram, crate::unit::derived::Second>("千克与秒 / kilogram vs second");
    }

    #[test]
    fn test_mass_dimension_symbol_and_domain() {
        // 质量量纲符号为 M，取值域为连续 / Mass dimension symbol is M and the domain is continuous
        assert_dimension_symbol::<Kilogram>("M", "千克量纲 / kilogram dimension");
        assert_domain::<Kilogram>(
            crate::dimension::derived_quantity::QuantityDomain::Continuous,
            "千克取值域 / kilogram domain",
        );
    }

    #[test]
    fn test_kilogram_is_base_unit_with_unit_scale() {
        // 千克是质量的基准单位，比例尺为 1 / Kilogram is the base unit of mass with scale 1
        assert_scale_exact(Kilogram::SCALE.value(), "1", "千克比例尺 / kilogram scale");
    }

    #[test]
    fn test_mass_unit_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<Pound>("lb", "pound", "磅实例 / pound instance");
        assert_instance_symbol_and_name::<Tonne>("t", "tonne", "吨实例 / tonne instance");
    }
}
