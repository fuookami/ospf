//! 表面密度单位 / Surface density units
//!
//! 提供表面密度量纲的 SI 单位定义，包括千克每平方米、克每平方米等 / Provides SI unit definitions for surface density dimension, including kilogram per square meter, gram per square meter, etc

use super::area::SquareMeter;
use super::mass::{Gram, Kilogram};
use crate::unit::{CTUnit, CTUnitDiv};

// ============================================================================
// 表面密度单位 / Surface density units
// ============================================================================

define_unit_by!(
    KilogramPerSquareMeter,
    "kilogram per square meter",
    "kg/m²",
    CTUnitDiv<Kilogram, SquareMeter>
);
define_unit_by!(
    GramPerSquareMeter,
    "gram per square meter",
    "g/m²",
    CTUnitDiv<Gram, SquareMeter>
);

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::derived::test_support::*;

    #[test]
    fn test_surface_density_symbols_and_names() {
        // 表面密度单位的符号与名称 / Symbols and names of surface density units
        assert_symbol_and_name::<KilogramPerSquareMeter>("kg/m²", "kilogram per square meter", "千克每平方米 / kilogram per square meter");
        assert_symbol_and_name::<GramPerSquareMeter>("g/m²", "gram per square meter", "克每平方米 / gram per square meter");
    }

    #[test]
    fn test_kilogram_per_square_meter_scale_is_unit() {
        // 千克每平方米是表面密度的基准单位，比例尺为 1 / Kilogram per square meter is the base surface density unit with scale 1
        assert_scale_exact(KilogramPerSquareMeter::SCALE.value(), "1", "千克每平方米比例尺 / kilogram per square meter scale");
    }

    #[test]
    fn test_gram_per_square_meter_equals_0_001_kilogram_per_square_meter() {
        // 1 克每平方米等于 0.001 千克每平方米 / One gram per square meter equals 0.001 kilograms per square meter
        assert_factor_exact::<GramPerSquareMeter, KilogramPerSquareMeter>("0.001", "克每平方米到千克每平方米 / gram per square meter to kilogram per square meter");
    }

    #[test]
    fn test_surface_density_equals_mass_per_area() {
        // 表面密度单位应为质量单位除以面积单位，比例尺一致
        // A surface density unit should equal the mass unit divided by the area unit, with a matching scale
        assert_scale_equals::<KilogramPerSquareMeter, CTUnitDiv<Kilogram, SquareMeter>>("千克每平方米分解 / kilogram per square meter decomposition");
        assert_scale_equals::<GramPerSquareMeter, CTUnitDiv<Gram, SquareMeter>>("克每平方米分解 / gram per square meter decomposition");
    }

    #[test]
    fn test_surface_density_dimension_symbol() {
        // 表面密度量纲符号为 L^-2·M / Surface density dimension symbol is L^-2·M
        assert_dimension_symbol::<KilogramPerSquareMeter>("L^-2·M", "千克每平方米量纲 / kilogram per square meter dimension");
    }

    #[test]
    fn test_surface_density_units_share_dimension() {
        // 两种表面密度单位共享量纲，且与质量密度不同 / Both surface density units share a dimension that differs from mass density
        assert_same_dimension::<KilogramPerSquareMeter, GramPerSquareMeter>("千克每平方米与克每平方米 / kilogram per square meter vs gram per square meter");
        assert_different_dimension::<KilogramPerSquareMeter, super::super::mass_density::KilogramPerCubicMeter>(
            "表面密度与质量密度 / surface density vs mass density",
        );
        assert_different_dimension::<KilogramPerSquareMeter, SquareMeter>("表面密度与面积 / surface density vs area");
    }

    #[test]
    fn test_surface_density_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<GramPerSquareMeter>("g/m²", "gram per square meter", "克每平方米实例 / gram per square meter instance");
    }
}
