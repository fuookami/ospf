//! 质量密度单位 / Mass density units

use super::mass::{Gram, Kilogram};
use super::volume::{CubicCentimeter, CubicMeter};
use crate::dimension::derived::MassDensity;
use crate::scale::Scale;
use crate::unit::CTUnit;
use crate::unit::physical_unit::CTUnitDiv;

define_unit_by!(
    KilogramPerCubicMeter,
    "kilogram per cubic meter",
    "kg/m^3",
    CTUnitDiv<Kilogram, CubicMeter>
);
define_unit!(
    KilogramPerLiter,
    "kilogram per liter",
    "kg/L",
    MassDensity,
    Scale::from_int(1000)
);
define_unit_by!(
    KilogramPerCubicCentimeter,
    "kilogram per cubic centimeter",
    "kg/cm^3",
    CTUnitDiv<Kilogram, CubicCentimeter>
);
define_unit_by!(
    GramPerCubicCentimeter,
    "gram per cubic centimeter",
    "g/cm^3",
    CTUnitDiv<Gram, CubicCentimeter>
);

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::derived::test_support::*;

    #[test]
    fn test_mass_density_symbols_and_names() {
        // 质量密度单位的符号与名称 / Symbols and names of mass density units
        assert_symbol_and_name::<KilogramPerCubicMeter>("kg/m^3", "kilogram per cubic meter", "千克每立方米 / kilogram per cubic meter");
        assert_symbol_and_name::<KilogramPerLiter>("kg/L", "kilogram per liter", "千克每升 / kilogram per liter");
        assert_symbol_and_name::<KilogramPerCubicCentimeter>("kg/cm^3", "kilogram per cubic centimeter", "千克每立方厘米 / kilogram per cubic centimeter");
        assert_symbol_and_name::<GramPerCubicCentimeter>("g/cm^3", "gram per cubic centimeter", "克每立方厘米 / gram per cubic centimeter");
    }

    #[test]
    fn test_kilogram_per_cubic_meter_scale_is_unit() {
        // 千克每立方米是质量密度的基准单位，比例尺为 1 / Kilogram per cubic meter is the base mass density unit with scale 1
        assert_scale_exact(KilogramPerCubicMeter::SCALE.value(), "1", "千克每立方米比例尺 / kilogram per cubic meter scale");
    }

    #[test]
    fn test_kilogram_per_liter_equals_one_thousand_kilogram_per_cubic_meter() {
        // 1 千克每升等于 1000 千克每立方米 / One kilogram per liter equals 1000 kilograms per cubic meter
        assert_factor_exact::<KilogramPerLiter, KilogramPerCubicMeter>("1000", "千克每升到千克每立方米 / kilogram per liter to kilogram per cubic meter");
    }

    #[test]
    fn test_kilogram_per_cubic_centimeter_equals_one_million_kilogram_per_cubic_meter() {
        // 1 千克每立方厘米等于 1e6 千克每立方米 / One kilogram per cubic centimeter equals 1e6 kilograms per cubic meter
        assert_factor_exact::<KilogramPerCubicCentimeter, KilogramPerCubicMeter>("1000000", "千克每立方厘米到千克每立方米 / kilogram per cubic centimeter to kilogram per cubic meter");
    }

    #[test]
    fn test_gram_per_cubic_centimeter_equals_one_thousand_kilogram_per_cubic_meter() {
        // 1 克每立方厘米等于 1000 千克每立方米 / One gram per cubic centimeter equals 1000 kilograms per cubic meter
        assert_factor_exact::<GramPerCubicCentimeter, KilogramPerCubicMeter>("1000", "克每立方厘米到千克每立方米 / gram per cubic centimeter to kilogram per cubic meter");
    }

    #[test]
    fn test_gram_per_cubic_centimeter_equals_kilogram_per_liter() {
        // 克每立方厘米与千克每升数值相等 / Gram per cubic centimeter and kilogram per liter are numerically equal
        assert_factor_exact::<GramPerCubicCentimeter, KilogramPerLiter>("1", "克每立方厘米到千克每升 / gram per cubic centimeter to kilogram per liter");
    }

    #[test]
    fn test_mass_density_equals_mass_per_volume() {
        // 密度单位应为质量单位除以体积单位，比例尺一致
        // A mass density unit should equal the mass unit divided by the volume unit, with a matching scale
        assert_scale_equals::<KilogramPerCubicMeter, CTUnitDiv<Kilogram, CubicMeter>>("千克每立方米分解 / kilogram per cubic meter decomposition");
        assert_scale_equals::<KilogramPerCubicCentimeter, CTUnitDiv<Kilogram, CubicCentimeter>>("千克每立方厘米分解 / kilogram per cubic centimeter decomposition");
        assert_scale_equals::<GramPerCubicCentimeter, CTUnitDiv<Gram, CubicCentimeter>>("克每立方厘米分解 / gram per cubic centimeter decomposition");
    }

    #[test]
    fn test_mass_density_dimension_symbol() {
        // 质量密度量纲符号为 L^-3·M / Mass density dimension symbol is L^-3·M
        assert_dimension_symbol::<KilogramPerCubicMeter>("L^-3·M", "千克每立方米量纲 / kilogram per cubic meter dimension");
    }

    #[test]
    fn test_mass_density_units_share_dimension() {
        // 所有质量密度单位共享量纲，且与质量、体积不同 / All mass density units share a dimension that differs from mass and volume
        assert_same_dimension::<KilogramPerCubicMeter, GramPerCubicCentimeter>("千克每立方米与克每立方厘米 / kilogram per cubic meter vs gram per cubic centimeter");
        assert_different_dimension::<KilogramPerCubicMeter, Kilogram>("密度与质量 / density vs mass");
        assert_different_dimension::<KilogramPerCubicMeter, CubicMeter>("密度与体积 / density vs volume");
    }

    #[test]
    fn test_mass_density_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<KilogramPerLiter>("kg/L", "kilogram per liter", "千克每升实例 / kilogram per liter instance");
    }
}
