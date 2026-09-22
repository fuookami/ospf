//! 体积单位 / Volume units
//!
//! 提供体积量纲的 SI 单位定义，包括立方米、升、毫升等 / Provides SI unit definitions for volume dimension, including cubic meter, liter, milliliter, etc

use super::length::{Cetimeter, Decimeter, Foot, Inch, Meter, Millimeter, Yard};
use crate::dimension::derived::Volume;
use crate::scale::Scale;
use crate::unit::{CTUnit, CTUnitMul};

// ============================================================================
// SI 体积单位 / SI volume units
// ============================================================================

define_unit_by!(
    CubicMeter,
    "cubic meter",
    "m³",
    CTUnitMul<Meter, CTUnitMul<Meter, Meter>>
);

define_unit_by!(
    CubicKilometer,
    "cubic kilometer",
    "km³",
    CTUnitMul<super::length::Kilometer, CTUnitMul<super::length::Kilometer, super::length::Kilometer>>
);

define_unit_by!(
    CubicHectometer,
    "cubic hectometer",
    "hm³",
    CTUnitMul<super::length::Hectometer, CTUnitMul<super::length::Hectometer, super::length::Hectometer>>
);

define_unit_by!(
    CubicDecameter,
    "cubic decameter",
    "dam³",
    CTUnitMul<super::length::Decameter, CTUnitMul<super::length::Decameter, super::length::Decameter>>
);

define_unit_by!(
    CubicDecimeter,
    "cubic decimeter",
    "dm³",
    CTUnitMul<Decimeter, CTUnitMul<Decimeter, Decimeter>>
);

define_unit_by!(
    CubicCentimeter,
    "cubic centimeter",
    "cm³",
    CTUnitMul<Cetimeter, CTUnitMul<Cetimeter, Cetimeter>>
);

define_unit_by!(
    CubicMillimeter,
    "cubic millimeter",
    "mm³",
    CTUnitMul<Millimeter, CTUnitMul<Millimeter, Millimeter>>
);

// ============================================================================
// 公制液体单位 / Metric liquid units
// ============================================================================

// 升 / Liter (1 dm³)
define_unit!(
    Liter,
    "liter",
    "L",
    Volume,
    Scale::from_int(1) / Scale::from_int(1000)
);

//  hectoliter / Hectoliter
define_unit!(Hectoliter, "hectoliter", "hL", Volume, Scale::from_f64(0.1));

//  deciliter / Deciliter
define_unit!(
    Deciliter,
    "deciliter",
    "dL",
    Volume,
    Scale::from_f64(0.0001)
);

//  centiliter / Centiliter
define_unit!(
    Centiliter,
    "centiliter",
    "cL",
    Volume,
    Scale::from_f64(0.00001)
);

//  milliliter / Milliliter
define_unit!(
    Milliliter,
    "milliliter",
    "mL",
    Volume,
    Scale::from_f64(0.000001)
);

//  microliter / Microliter
define_unit!(
    Microliter,
    "microliter",
    "μL",
    Volume,
    Scale::from_f64(0.000000001)
);

// ============================================================================
// 英制体积单位 / Imperial volume units
// ============================================================================

// 立方英寸 / Cubic inch
define_unit_by!(
    CubicInch,
    "cubic inch",
    "cu.in",
    CTUnitMul<Inch, CTUnitMul<Inch, Inch>>
);

// 立方英尺 / Cubic foot
define_unit_by!(
    CubicFoot,
    "cubic foot",
    "cu.ft",
    CTUnitMul<Foot, CTUnitMul<Foot, Foot>>
);

// 立方码 / Cubic yard
define_unit_by!(
    CubicYard,
    "cubic yard",
    "cu.yd",
    CTUnitMul<Yard, CTUnitMul<Yard, Yard>>
);

// ============================================================================
// 英制液体单位 / Imperial liquid units
// ============================================================================

// 英制液量盎司 / UK fluid ounce (28.4130625 mL)
define_unit!(
    UKFluidOunce,
    "uk fluid ounce",
    "uk.fl.oz",
    Volume,
    Scale::from_f64(0.0000284130625)
);

// 美制液量盎司 / US fluid ounce (29.5735295625 mL)
define_unit!(
    USFluidOunce,
    "us fluid ounce",
    "us.fl.oz",
    Volume,
    Scale::from_f64(0.0000295735295625)
);

// 英制加仑 / UK gallon (160 UK fluid ounces)
define_unit!(
    UKGallon,
    "uk gallon",
    "uk.gal",
    Volume,
    Scale::from_f64(0.00454609)
);

// 美制加仑 / US gallon (128 US fluid ounces)
define_unit!(
    USGallon,
    "us gallon",
    "us.gal",
    Volume,
    Scale::from_f64(0.00378541178)
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
    fn test_cubic_length_symbols_and_names() {
        // 立方长度单位的符号与名称 / Symbols and names of cubic length units
        assert_symbol_and_name::<CubicMeter>("m³", "cubic meter", "立方米 / cubic meter");
        assert_symbol_and_name::<CubicKilometer>("km³", "cubic kilometer", "立方千米 / cubic kilometer");
        assert_symbol_and_name::<CubicHectometer>("hm³", "cubic hectometer", "立方百米 / cubic hectometer");
        assert_symbol_and_name::<CubicDecameter>("dam³", "cubic decameter", "立方十米 / cubic decameter");
        assert_symbol_and_name::<CubicDecimeter>("dm³", "cubic decimeter", "立方分米 / cubic decimeter");
        assert_symbol_and_name::<CubicCentimeter>("cm³", "cubic centimeter", "立方厘米 / cubic centimeter");
        assert_symbol_and_name::<CubicMillimeter>("mm³", "cubic millimeter", "立方毫米 / cubic millimeter");
    }

    #[test]
    fn test_cubic_kilometer_equals_one_billion_cubic_meter() {
        // 1 立方千米等于 1e9 立方米 / One cubic kilometer equals 1e9 cubic meters
        assert_factor_exact::<CubicKilometer, CubicMeter>("1000000000", "立方千米到立方米 / cubic kilometer to cubic meter");
        assert_scale_exact(
            CubicKilometer::SCALE.value(),
            "1000000000",
            "立方千米比例尺 / cubic kilometer scale",
        );
    }

    #[test]
    fn test_cubic_hectometer_equals_one_million_cubic_meter() {
        // 1 立方百米等于 1e6 立方米 / One cubic hectometer equals 1e6 cubic meters
        assert_factor_exact::<CubicHectometer, CubicMeter>("1000000", "立方百米到立方米 / cubic hectometer to cubic meter");
    }

    #[test]
    fn test_cubic_decameter_equals_one_thousand_cubic_meter() {
        // 1 立方十米等于 1000 立方米 / One cubic decameter equals 1000 cubic meters
        assert_factor_exact::<CubicDecameter, CubicMeter>("1000", "立方十米到立方米 / cubic decameter to cubic meter");
    }

    #[test]
    fn test_cubic_prefix_series_to_cubic_meter() {
        // 立方单位到立方米的换算系数 / Conversion factors of cubic units to cubic meter
        assert_factor_exact::<CubicMeter, CubicMeter>("1", "立方米到立方米 / cubic meter to cubic meter");
        assert_factor_exact::<CubicDecimeter, CubicMeter>("0.001", "立方分米到立方米 / cubic decimeter to cubic meter");
        assert_factor_exact::<CubicCentimeter, CubicMeter>("0.000001", "立方厘米到立方米 / cubic centimeter to cubic meter");
        assert_factor_exact::<CubicMillimeter, CubicMeter>("1e-9", "立方毫米到立方米 / cubic millimeter to cubic meter");
    }

    #[test]
    fn test_cubic_volume_is_length_prefix_cubed() {
        // 立方体积单位应为长度前缀的三次方 / A cubic volume scale should be the length prefix cubed
        assert_factor_exact::<CubicMeter, CubicDecimeter>("1000", "立方米到立方分米 / cubic meter to cubic decimeter");
        assert_factor_exact::<CubicDecimeter, CubicCentimeter>("1000", "立方分米到立方厘米 / cubic decimeter to cubic centimeter");
        assert_factor_exact::<CubicCentimeter, CubicMillimeter>("1000", "立方厘米到立方毫米 / cubic centimeter to cubic millimeter");
    }

    #[test]
    fn test_metric_liquid_symbols_and_names() {
        // 公制液体单位的符号与名称 / Symbols and names of metric liquid units
        assert_symbol_and_name::<Liter>("L", "liter", "升 / liter");
        assert_symbol_and_name::<Hectoliter>("hL", "hectoliter", "百升 / hectoliter");
        assert_symbol_and_name::<Deciliter>("dL", "deciliter", "分升 / deciliter");
        assert_symbol_and_name::<Centiliter>("cL", "centiliter", "厘升 / centiliter");
        assert_symbol_and_name::<Milliliter>("mL", "milliliter", "毫升 / milliliter");
        assert_symbol_and_name::<Microliter>("μL", "microliter", "微升 / microliter");
    }

    #[test]
    fn test_liter_conversions_to_cubic_meter() {
        // 公制液体单位到立方米的换算系数 / Conversion factors of metric liquid units to cubic meter
        assert_factor_exact::<Liter, CubicMeter>("0.001", "升到立方米 / liter to cubic meter");
        assert_factor_relative::<Hectoliter, CubicMeter>("0.1", FUZZ, "百升到立方米 / hectoliter to cubic meter");
        assert_factor_relative::<Deciliter, CubicMeter>("0.0001", FUZZ, "分升到立方米 / deciliter to cubic meter");
        assert_factor_relative::<Centiliter, CubicMeter>("0.00001", FUZZ, "厘升到立方米 / centiliter to cubic meter");
        assert_factor_relative::<Milliliter, CubicMeter>("0.000001", FUZZ, "毫升到立方米 / milliliter to cubic meter");
        assert_factor_relative::<Microliter, CubicMeter>("1e-9", FUZZ, "微升到立方米 / microliter to cubic meter");
    }

    #[test]
    fn test_liter_equals_cubic_decimeter() {
        // 1 升等于 1 立方分米 / One liter equals one cubic decimeter
        assert_factor_exact::<Liter, CubicDecimeter>("1", "升到立方分米 / liter to cubic decimeter");
    }

    #[test]
    fn test_liquid_prefix_series_to_liter() {
        // 液体单位之间按 10 递进 / Liquid units advance by powers of ten
        assert_factor_relative::<Hectoliter, Liter>("100", FUZZ, "百升到升 / hectoliter to liter");
        assert_factor_relative::<Liter, Deciliter>("10", FUZZ, "升到分升 / liter to deciliter");
        assert_factor_relative::<Deciliter, Centiliter>("10", FUZZ, "分升到厘升 / deciliter to centiliter");
        assert_factor_relative::<Centiliter, Milliliter>("10", FUZZ, "厘升到毫升 / centiliter to milliliter");
        assert_factor_relative::<Milliliter, Microliter>("1000", FUZZ, "毫升到微升 / milliliter to microliter");
    }

    #[test]
    fn test_imperial_cubic_symbols_and_names() {
        // 英制立方单位的符号与名称 / Symbols and names of imperial cubic units
        assert_symbol_and_name::<CubicInch>("cu.in", "cubic inch", "立方英寸 / cubic inch");
        assert_symbol_and_name::<CubicFoot>("cu.ft", "cubic foot", "立方英尺 / cubic foot");
        assert_symbol_and_name::<CubicYard>("cu.yd", "cubic yard", "立方码 / cubic yard");
    }

    #[test]
    fn test_cubic_yard_equals_twenty_seven_cubic_foot() {
        // 1 立方码等于 27 立方英尺 / One cubic yard equals 27 cubic feet
        assert_factor_relative::<CubicYard, CubicFoot>("27", FUZZ, "立方码到立方英尺 / cubic yard to cubic foot");
        assert_factor_relative::<CubicFoot, CubicInch>("1728", FUZZ, "立方英尺到立方英寸 / cubic foot to cubic inch");
    }

    #[test]
    fn test_imperial_cubic_conversion_to_cubic_meter() {
        // 英制立方单位到立方米的换算系数 / Conversion factors of imperial cubic units to cubic meter
        assert_factor_relative::<CubicInch, CubicMeter>("0.000016387064", FUZZ, "立方英寸到立方米 / cubic inch to cubic meter");
        assert_factor_relative::<CubicFoot, CubicMeter>("0.028316846592", FUZZ, "立方英尺到立方米 / cubic foot to cubic meter");
        assert_factor_relative::<CubicYard, CubicMeter>("0.764554857984", FUZZ, "立方码到立方米 / cubic yard to cubic meter");
    }

    #[test]
    fn test_imperial_liquid_symbols_and_names() {
        // 英制液体单位的符号与名称 / Symbols and names of imperial liquid units
        assert_symbol_and_name::<UKFluidOunce>("uk.fl.oz", "uk fluid ounce", "英制液量盎司 / uk fluid ounce");
        assert_symbol_and_name::<USFluidOunce>("us.fl.oz", "us fluid ounce", "美制液量盎司 / us fluid ounce");
        assert_symbol_and_name::<UKGallon>("uk.gal", "uk gallon", "英制加仑 / uk gallon");
        assert_symbol_and_name::<USGallon>("us.gal", "us gallon", "美制加仑 / us gallon");
    }

    #[test]
    fn test_uk_fluid_ounce_equals_28_4130625_milliliter() {
        // 1 英制液量盎司等于 28.4130625 毫升 / One UK fluid ounce equals 28.4130625 milliliters
        assert_factor_relative::<UKFluidOunce, Milliliter>("28.4130625", FUZZ, "英制液量盎司到毫升 / uk fluid ounce to milliliter");
    }

    #[test]
    fn test_us_fluid_ounce_equals_29_5735295625_milliliter() {
        // 1 美制液量盎司等于 29.5735295625 毫升 / One US fluid ounce equals 29.5735295625 milliliters
        assert_factor_relative::<USFluidOunce, Milliliter>("29.5735295625", FUZZ, "美制液量盎司到毫升 / us fluid ounce to milliliter");
    }

    #[test]
    fn test_uk_gallon_equals_4_54609_liter() {
        // 1 英制加仑等于 4.54609 升 / One UK gallon equals 4.54609 liters
        assert_factor_relative::<UKGallon, Liter>("4.54609", FUZZ, "英制加仑到升 / uk gallon to liter");
    }

    #[test]
    fn test_us_gallon_equals_3_78541178_liter() {
        // 1 美制加仑等于 3.78541178 升 / One US gallon equals 3.78541178 liters
        assert_factor_relative::<USGallon, Liter>("3.78541178", FUZZ, "美制加仑到升 / us gallon to liter");
    }

    #[test]
    fn test_uk_gallon_equals_one_hundred_sixty_uk_fluid_ounce() {
        // 1 英制加仑等于 160 英制液量盎司 / One UK gallon equals 160 UK fluid ounces
        assert_factor_relative::<UKGallon, UKFluidOunce>("160", FUZZ, "英制加仑到英制液量盎司 / uk gallon to uk fluid ounce");
    }

    #[test]
    fn test_us_gallon_equals_one_hundred_twenty_eight_us_fluid_ounce() {
        // 1 美制加仑等于 128 美制液量盎司（常量仅保留 9 位有效数字，故使用 1e-8 容差）
        // One US gallon equals 128 US fluid ounces (the constant keeps only 9 significant digits, so a 1e-8 tolerance is used)
        assert_factor_relative::<USGallon, USFluidOunce>("128", "1e-8", "美制加仑到美制液量盎司 / us gallon to us fluid ounce");
    }

    #[test]
    fn test_volume_dimension_symbol_and_domain() {
        // 体积量纲符号为 L^3，取值域为连续 / Volume dimension symbol is L^3 and the domain is continuous
        assert_dimension_symbol::<CubicMeter>("L^3", "立方米量纲 / cubic meter dimension");
        assert_domain::<CubicMeter>(
            crate::dimension::derived_quantity::QuantityDomain::Continuous,
            "立方米取值域 / cubic meter domain",
        );
    }

    #[test]
    fn test_volume_units_share_dimension() {
        // 所有体积单位共享体积量纲 / All volume units share the volume dimension
        assert_same_dimension::<CubicMeter, Liter>("立方米与升 / cubic meter vs liter");
        assert_same_dimension::<CubicMeter, USGallon>("立方米与美制加仑 / cubic meter vs us gallon");
        assert_different_dimension::<CubicMeter, crate::unit::derived::SquareMeter>(
            "立方米与平方米 / cubic meter vs square meter",
        );
        assert_different_dimension::<CubicMeter, crate::unit::derived::Meter>(
            "立方米与米 / cubic meter vs meter",
        );
    }

    #[test]
    fn test_volume_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<Liter>("L", "liter", "升实例 / liter instance");
        assert_instance_symbol_and_name::<CubicKilometer>("km³", "cubic kilometer", "立方千米实例 / cubic kilometer instance");
    }
}
