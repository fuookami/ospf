//! 面积单位 / Area units

use super::length::{
    Cetimeter, Chain, Decimeter, Foot, Inch, Kilometer, Meter, Mile, Millimeter, Rod, Yard,
};
use crate::dimension::derived::Area;
use crate::scale::Scale;
use crate::unit::CTUnitMul;
use crate::unit::physical_unit::CTUnit;

define_unit_by!(
    SquareMillimeter,
    "square millimeter",
    "mm^2",
    CTUnitMul<Millimeter, Millimeter>
);
define_unit_by!(
    SquareCentimeter,
    "square centimeter",
    "cm^2",
    CTUnitMul<Cetimeter, Cetimeter>
);
define_unit_by!(
    SquareDecimeter,
    "square decimeter",
    "dm^2",
    CTUnitMul<Decimeter, Decimeter>
);
define_unit_by!(SquareMeter, "square meter", "m^2", CTUnitMul<Meter, Meter>);
define_unit_by!(
    SquareKilometer,
    "square kilometer",
    "km^2",
    CTUnitMul<Kilometer, Kilometer>
);

define_unit!(Are, "are", "are", Area, Scale::from_f64(100.0));
define_unit!(Hectare, "hectare", "ha", Area, Scale::from_f64(10000.0));

define_unit_by!(
    SquareInch,
    "square inch",
    "sq.in",
    CTUnitMul<Inch, Inch>
);
define_unit_by!(
    SquareFoot,
    "square foot",
    "sq.ft",
    CTUnitMul<Foot, Foot>
);
define_unit_by!(
    SquareYard,
    "square yard",
    "sq.yd",
    CTUnitMul<Yard, Yard>
);
define_unit_by!(
    SquareChain,
    "square chain",
    "sq.ch",
    CTUnitMul<Chain, Chain>
);
define_unit_by!(
    SquareRod,
    "square rod",
    "sq.rd",
    CTUnitMul<Rod, Rod>
);
define_unit_by!(
    SquareMile,
    "square mile",
    "sq.mi",
    CTUnitMul<Mile, Mile>
);

define_unit!(Acre, "acre", "acre", Area, Scale::from_f64(4046.8564224));

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
    fn test_si_area_symbols_and_names() {
        // SI 面积单位的符号与名称 / Symbols and names of SI area units
        assert_symbol_and_name::<SquareMillimeter>("mm^2", "square millimeter", "平方毫米 / square millimeter");
        assert_symbol_and_name::<SquareCentimeter>("cm^2", "square centimeter", "平方厘米 / square centimeter");
        assert_symbol_and_name::<SquareDecimeter>("dm^2", "square decimeter", "平方分米 / square decimeter");
        assert_symbol_and_name::<SquareMeter>("m^2", "square meter", "平方米 / square meter");
        assert_symbol_and_name::<SquareKilometer>("km^2", "square kilometer", "平方千米 / square kilometer");
    }

    #[test]
    fn test_si_area_conversion_to_square_meter() {
        // SI 面积单位到平方米的换算系数 / Conversion factors of SI area units to square meter
        assert_factor_exact::<SquareMeter, SquareMeter>("1", "平方米到平方米 / square meter to square meter");
        assert_factor_exact::<SquareMillimeter, SquareMeter>("0.000001", "平方毫米到平方米 / square millimeter to square meter");
        assert_factor_exact::<SquareCentimeter, SquareMeter>("0.0001", "平方厘米到平方米 / square centimeter to square meter");
        assert_factor_exact::<SquareDecimeter, SquareMeter>("0.01", "平方分米到平方米 / square decimeter to square meter");
        assert_factor_exact::<SquareKilometer, SquareMeter>("1000000", "平方千米到平方米 / square kilometer to square meter");
    }

    #[test]
    fn test_si_area_prefix_squares_length_prefix() {
        // 面积单位比例尺应为对应长度单位的平方 / An area scale should be the square of the matching length scale
        assert_factor_exact::<SquareKilometer, SquareMeter>("1000000", "平方千米面积因子 / square kilometer area factor");
        assert_scale_exact(
            SquareKilometer::SCALE.value(),
            "1000000",
            "平方千米比例尺 / square kilometer scale",
        );
        assert_scale_exact(
            SquareCentimeter::SCALE.value(),
            "0.0001",
            "平方厘米比例尺 / square centimeter scale",
        );
    }

    #[test]
    fn test_metric_land_area_symbols_and_names() {
        // 公制地积单位的符号与名称 / Symbols and names of metric land area units
        assert_symbol_and_name::<Are>("are", "are", "公亩 / are");
        assert_symbol_and_name::<Hectare>("ha", "hectare", "公顷 / hectare");
    }

    #[test]
    fn test_are_equals_one_hundred_square_meter() {
        // 1 公亩等于 100 平方米 / One are equals 100 square meters
        assert_factor_relative::<Are, SquareMeter>("100", FUZZ, "公亩到平方米 / are to square meter");
    }

    #[test]
    fn test_hectare_equals_ten_thousand_square_meter() {
        // 1 公顷等于 10000 平方米 / One hectare equals 10000 square meters
        assert_factor_relative::<Hectare, SquareMeter>("10000", FUZZ, "公顷到平方米 / hectare to square meter");
        assert_factor_relative::<Hectare, Are>("100", FUZZ, "公顷到公亩 / hectare to are");
    }

    #[test]
    fn test_imperial_area_symbols_and_names() {
        // 英制面积单位的符号与名称 / Symbols and names of imperial area units
        assert_symbol_and_name::<SquareInch>("sq.in", "square inch", "平方英寸 / square inch");
        assert_symbol_and_name::<SquareFoot>("sq.ft", "square foot", "平方英尺 / square foot");
        assert_symbol_and_name::<SquareYard>("sq.yd", "square yard", "平方码 / square yard");
        assert_symbol_and_name::<SquareChain>("sq.ch", "square chain", "平方链 / square chain");
        assert_symbol_and_name::<SquareRod>("sq.rd", "square rod", "平方杆 / square rod");
        assert_symbol_and_name::<SquareMile>("sq.mi", "square mile", "平方英里 / square mile");
        assert_symbol_and_name::<Acre>("acre", "acre", "英亩 / acre");
    }

    #[test]
    fn test_imperial_area_conversion_to_square_meter() {
        // 英制面积单位到平方米的换算系数 / Conversion factors of imperial area units to square meter
        assert_factor_relative::<SquareInch, SquareMeter>("0.00064516", FUZZ, "平方英寸到平方米 / square inch to square meter");
        assert_factor_relative::<SquareFoot, SquareMeter>("0.09290304", FUZZ, "平方英尺到平方米 / square foot to square meter");
        assert_factor_relative::<SquareYard, SquareMeter>("0.83612736", FUZZ, "平方码到平方米 / square yard to square meter");
        assert_factor_relative::<SquareChain, SquareMeter>("404.68564224", FUZZ, "平方链到平方米 / square chain to square meter");
        assert_factor_relative::<SquareRod, SquareMeter>("25.29285264", FUZZ, "平方杆到平方米 / square rod to square meter");
        assert_factor_relative::<SquareMile, SquareMeter>("2589988.110336", FUZZ, "平方英里到平方米 / square mile to square meter");
        assert_factor_relative::<Acre, SquareMeter>("4046.8564224", FUZZ, "英亩到平方米 / acre to square meter");
    }

    #[test]
    fn test_imperial_area_internal_ratios() {
        // 英制面积单位之间的整数比例关系 / Integer ratios between imperial area units
        assert_factor_relative::<SquareFoot, SquareInch>("144", FUZZ, "平方英尺到平方英寸 / square foot to square inch");
        assert_factor_relative::<SquareYard, SquareFoot>("9", FUZZ, "平方码到平方英尺 / square yard to square foot");
        assert_factor_relative::<SquareMile, SquareYard>("3097600", FUZZ, "平方英里到平方码 / square mile to square yard");
        assert_factor_relative::<Acre, SquareYard>("4840", FUZZ, "英亩到平方码 / acre to square yard");
        assert_factor_relative::<Acre, SquareChain>("10", FUZZ, "英亩到平方链 / acre to square chain");
    }

    #[test]
    fn test_area_dimension_symbol_and_domain() {
        // 面积量纲符号为 L^2，取值域为连续 / Area dimension symbol is L^2 and the domain is continuous
        assert_dimension_symbol::<SquareMeter>("L^2", "平方米量纲 / square meter dimension");
        assert_domain::<SquareMeter>(
            crate::dimension::derived_quantity::QuantityDomain::Continuous,
            "平方米取值域 / square meter domain",
        );
    }

    #[test]
    fn test_area_units_share_dimension() {
        // 所有面积单位共享面积量纲 / All area units share the area dimension
        assert_same_dimension::<SquareMeter, Acre>("平方米与英亩 / square meter vs acre");
        assert_same_dimension::<SquareMeter, SquareMile>("平方米与平方英里 / square meter vs square mile");
        assert_different_dimension::<SquareMeter, crate::unit::derived::Meter>("平方米与米 / square meter vs meter");
        assert_different_dimension::<SquareMeter, crate::unit::derived::CubicMeter>(
            "平方米与立方米 / square meter vs cubic meter",
        );
    }

    #[test]
    fn test_area_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<Hectare>("ha", "hectare", "公顷实例 / hectare instance");
        assert_instance_symbol_and_name::<SquareMile>("sq.mi", "square mile", "平方英里实例 / square mile instance");
    }
}
