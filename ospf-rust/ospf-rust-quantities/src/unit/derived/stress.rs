//! 压力/应力单位 / Pressure/Stress units

use super::area::SquareMeter;
use super::force::Newton;
use crate::dimension::derived::Pressure;
use crate::scale::{KILO, MEGA, Scale};
use crate::unit::{CTUnit, CTUnitDiv};

define_unit_by!(
    PascalStress,
    "pascal",
    "Pa",
    CTUnitDiv<Newton, SquareMeter>
);
define_unit!(
    KilopascalStress,
    "kilopascal",
    "kPa",
    Pressure,
    &*PascalStress::SCALE * &*KILO
);
define_unit!(
    MegapascalStress,
    "megapascal",
    "MPa",
    Pressure,
    &*PascalStress::SCALE * &*MEGA
);
define_unit!(
    PoundForcePerSquareInch,
    "pound-force per square inch",
    "psi",
    Pressure,
    Scale::from_f64(6894.757)
);
define_unit!(
    PoundForcePerSquareFoot,
    "pound-force per square foot",
    "psf",
    Pressure,
    Scale::from_f64(47.88025898)
);
define_unit!(
    KilogramForcePerSquareCentimeter,
    "kilogram-force per square centimeter",
    "kgf/cm^2",
    Pressure,
    Scale::from_f64(98066.5)
);
define_unit!(
    KilogramForcePerSquareMeter,
    "kilogram-force per square meter",
    "kgf/m^2",
    Pressure,
    Scale::from_f64(9.80665)
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
    fn test_stress_symbols_and_names() {
        // 应力单位的符号与名称 / Symbols and names of stress units
        assert_symbol_and_name::<PascalStress>("Pa", "pascal", "帕斯卡应力 / pascal stress");
        assert_symbol_and_name::<KilopascalStress>("kPa", "kilopascal", "千帕应力 / kilopascal stress");
        assert_symbol_and_name::<MegapascalStress>("MPa", "megapascal", "兆帕应力 / megapascal stress");
        assert_symbol_and_name::<PoundForcePerSquareInch>("psi", "pound-force per square inch", "每平方英寸磅力 / pound-force per square inch");
        assert_symbol_and_name::<PoundForcePerSquareFoot>("psf", "pound-force per square foot", "每平方英尺磅力 / pound-force per square foot");
        assert_symbol_and_name::<KilogramForcePerSquareCentimeter>("kgf/cm^2", "kilogram-force per square centimeter", "每平方厘米千克力 / kilogram-force per square centimeter");
        assert_symbol_and_name::<KilogramForcePerSquareMeter>("kgf/m^2", "kilogram-force per square meter", "每平方米千克力 / kilogram-force per square meter");
    }

    #[test]
    fn test_stress_si_conversion_to_pascal_stress() {
        // SI 应力单位到帕斯卡应力的换算系数 / Conversion factors of SI stress units to pascal stress
        assert_factor_exact::<PascalStress, PascalStress>("1", "帕斯卡应力到自身 / pascal stress to itself");
        assert_factor_exact::<KilopascalStress, PascalStress>("1000", "千帕应力到帕斯卡应力 / kilopascal stress to pascal stress");
        assert_factor_exact::<MegapascalStress, PascalStress>("1000000", "兆帕应力到帕斯卡应力 / megapascal stress to pascal stress");
        assert_factor_exact::<MegapascalStress, KilopascalStress>("1000", "兆帕应力到千帕应力 / megapascal stress to kilopascal stress");
    }

    #[test]
    fn test_pascal_stress_matches_pressure_pascal() {
        // 应力帕斯卡与压力帕斯卡应为同一单位 / Pascal stress and pressure pascal should be the same unit
        assert_scale_equals::<PascalStress, super::super::pressure::Pascal>("应力帕斯卡与压力帕斯卡 / pascal stress vs pascal pressure");
        assert_scale_equals::<KilopascalStress, super::super::pressure::Kilopascal>("千帕应力与千帕压力 / kilopascal stress vs kilopascal pressure");
        assert_scale_equals::<MegapascalStress, super::super::pressure::Megapascal>("兆帕应力与兆帕压力 / megapascal stress vs megapascal pressure");
    }

    #[test]
    fn test_psi_to_pascal_stress() {
        // 1 psi 等于 6894.757 帕斯卡 / One psi equals 6894.757 pascals
        assert_factor_relative::<PoundForcePerSquareInch, PascalStress>("6894.757", FUZZ, "psi 到帕斯卡 / psi to pascal");
    }

    #[test]
    fn test_pascal_stress_converts_to_psi() {
        // 1 帕斯卡约等于 1/6894.757 psi；psi 常量取整到 6894.757，故使用 1e-6 容差
        // One pascal is about 1/6894.757 psi; the psi constant is rounded to 6894.757, so a 1e-6 tolerance is used
        assert_factor_relative::<PascalStress, PoundForcePerSquareInch>(
            "0.00014503773773020924",
            "1e-6",
            "帕斯卡到 psi / pascal to psi",
        );
    }

    #[test]
    fn test_psf_is_144th_of_psi() {
        // 1 psi 应约为 144 psf（1 平方英尺 = 144 平方英寸）；常量取整带来约 1e-7 的相对偏差
        // One psi should be about 144 psf (one square foot equals 144 square inches); rounded constants introduce about a 1e-7 relative deviation
        assert_factor_relative::<PoundForcePerSquareInch, PoundForcePerSquareFoot>(
            "144",
            "1e-6",
            "psi 到 psf / psi to psf",
        );
    }

    #[test]
    fn test_kilogram_force_per_square_centimeter_conversions() {
        // 千克力度量应约等于 98066.5 帕斯卡，且为每平方米值的 10000 倍
        // Kilogram force per square centimeter should be about 98066.5 pascals and 10000 times the per square meter value
        assert_factor_relative::<KilogramForcePerSquareCentimeter, PascalStress>("98066.5", FUZZ, "千克力每平方厘米到帕斯卡 / kgf per square centimeter to pascal");
        assert_factor_relative::<KilogramForcePerSquareCentimeter, KilogramForcePerSquareMeter>(
            "10000",
            FUZZ,
            "千克力每平方厘米到每平方米 / kgf per square centimeter to per square meter",
        );
    }

    #[test]
    fn test_stress_units_share_pressure_dimension() {
        // 应力单位共享压力量纲，且与力、能量量纲不同
        // Stress units share the pressure dimension and differ from force and energy
        assert_same_dimension::<PascalStress, PoundForcePerSquareInch>("帕斯卡应力与 psi / pascal stress vs psi");
        assert_same_dimension::<PascalStress, super::super::pressure::Pascal>("应力与压力 / stress vs pressure");
        assert_different_dimension::<PascalStress, Newton>("应力与力 / stress vs force");
        assert_different_dimension::<PascalStress, super::super::energy::Joule>("应力与能量 / stress vs energy");
    }

    #[test]
    fn test_stress_dimension_symbol() {
        // 应力量纲符号与压力一致：L^-1·M·T^-2 / Stress dimension symbol matches pressure: L^-1·M·T^-2
        assert_dimension_symbol::<PascalStress>("L^-1·M·T^-2", "帕斯卡应力量纲 / pascal stress dimension");
    }

    #[test]
    fn test_stress_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<MegapascalStress>("MPa", "megapascal", "兆帕应力实例 / megapascal stress instance");
        assert_instance_symbol_and_name::<PoundForcePerSquareInch>("psi", "pound-force per square inch", "psi 实例 / psi instance");
    }
}
