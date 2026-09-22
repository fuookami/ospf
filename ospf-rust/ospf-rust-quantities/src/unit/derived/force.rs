//! 力单位 / Force units
//!
//! 提供力量纲的 SI 单位定义，包括牛顿、千牛、兆牛、千克力等 / Provides SI unit definitions for force dimension, including newton, kilonewton, meganewton, kilogram-force, etc

use super::acceleration::MeterPerSecondSquared;
use super::mass::Kilogram;
use crate::dimension::derived::Force;
use crate::scale::{KILO, MEGA, Scale};
use crate::unit::{CTUnit, CTUnitMul};

// ============================================================================
// SI 力单位 / SI force units
// ============================================================================

define_unit_by!(
    Newton,
    "newton",
    "N",
    CTUnitMul<Kilogram, MeterPerSecondSquared>
);
define_unit!(
    Kilonewton,
    "kilonewton",
    "kN",
    Force,
    &*Newton::SCALE * &*KILO
);
define_unit!(
    Meganewton,
    "meganewton",
    "MN",
    Force,
    &*Newton::SCALE * &*MEGA
);

// ============================================================================
// 公制力单位 / Metric force units
// ============================================================================

// 千克力 / Kilogram force
define_unit!(
    KilogramForce,
    "kilogram force",
    "kgf",
    Force,
    Scale::from_f64(9.80665)
);

// 克力 / Gram force
define_unit!(
    GramForce,
    "gram force",
    "gf",
    Force,
    Scale::from_f64(0.00980665)
);

// 达因 / Dyne (1 g·cm/s²)
define_unit!(Dyne, "dyne", "dyn", Force, Scale::from_f64(0.00001));

// ============================================================================
// 英制力单位 / Imperial force units
// ============================================================================

// 磅力 / Pound force
define_unit!(
    PoundForce,
    "pound force",
    "lbf",
    Force,
    Scale::from_f64(4.4482216152605)
);

// 千磅力 / Kilopound force
define_unit!(
    KilopoundForce,
    "kilopound force",
    "klbf",
    Force,
    Scale::from_f64(4448.2216152605)
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
    fn test_si_force_symbols_and_names() {
        // SI 力单位的符号与名称 / Symbols and names of SI force units
        assert_symbol_and_name::<Newton>("N", "newton", "牛顿 / newton");
        assert_symbol_and_name::<Kilonewton>("kN", "kilonewton", "千牛 / kilonewton");
        assert_symbol_and_name::<Meganewton>("MN", "meganewton", "兆牛 / meganewton");
    }

    #[test]
    fn test_kilonewton_equals_one_thousand_newton() {
        // 1 千牛等于 1000 牛顿 / One kilonewton equals 1000 newtons
        assert_factor_exact::<Kilonewton, Newton>("1000", "千牛到牛顿 / kilonewton to newton");
    }

    #[test]
    fn test_meganewton_equals_one_million_newton() {
        // 1 兆牛等于 1e6 牛顿 / One meganewton equals 1e6 newtons
        assert_factor_exact::<Meganewton, Newton>("1000000", "兆牛到牛顿 / meganewton to newton");
        assert_factor_exact::<Meganewton, Kilonewton>("1000", "兆牛到千牛 / meganewton to kilonewton");
    }

    #[test]
    fn test_newton_is_base_force_unit() {
        // 牛顿是力的基准单位，比例尺为 1 / Newton is the base force unit with scale 1
        assert_scale_exact(Newton::SCALE.value(), "1", "牛顿比例尺 / newton scale");
        assert_factor_exact::<Newton, Newton>("1", "牛顿到牛顿 / newton to newton");
    }

    #[test]
    fn test_metric_force_symbols_and_names() {
        // 公制力单位的符号与名称 / Symbols and names of metric force units
        assert_symbol_and_name::<KilogramForce>("kgf", "kilogram force", "千克力 / kilogram force");
        assert_symbol_and_name::<GramForce>("gf", "gram force", "克力 / gram force");
        assert_symbol_and_name::<Dyne>("dyn", "dyne", "达因 / dyne");
    }

    #[test]
    fn test_metric_force_conversion_to_newton() {
        // 公制力单位到牛顿的换算系数 / Conversion factors of metric force units to newton
        assert_factor_relative::<KilogramForce, Newton>("9.80665", FUZZ, "千克力到牛顿 / kilogram force to newton");
        assert_factor_relative::<GramForce, Newton>("0.00980665", FUZZ, "克力到牛顿 / gram force to newton");
        assert_factor_relative::<Dyne, Newton>("0.00001", FUZZ, "达因到牛顿 / dyne to newton");
    }

    #[test]
    fn test_gram_force_is_one_thousandth_kilogram_force() {
        // 1 克力等于 0.001 千克力 / One gram force equals 0.001 kilogram force
        assert_factor_relative::<GramForce, KilogramForce>("0.001", FUZZ, "克力到千克力 / gram force to kilogram force");
    }

    #[test]
    fn test_kilogram_force_equals_standard_gravity_newton() {
        // 1 千克力等于标准重力加速度下的 1 千克重量 / One kilogram force equals one kilogram under standard gravity
        assert_scale_equals::<KilogramForce, super::super::acceleration::StandardGravity>(
            "千克力与标准重力加速度 / kilogram force vs standard gravity",
        );
    }

    #[test]
    fn test_dyne_is_gram_centimeter_per_second_squared() {
        // 1 达因等于 1e-5 牛顿 / One dyne equals 1e-5 newtons
        assert_factor_relative::<Dyne, Kilonewton>("0.00000001", FUZZ, "达因到千牛 / dyne to kilonewton");
    }

    #[test]
    fn test_imperial_force_symbols_and_names() {
        // 英制力单位的符号与名称 / Symbols and names of imperial force units
        assert_symbol_and_name::<PoundForce>("lbf", "pound force", "磅力 / pound force");
        assert_symbol_and_name::<KilopoundForce>("klbf", "kilopound force", "千磅力 / kilopound force");
    }

    #[test]
    fn test_pound_force_equals_4_4482216152605_newton() {
        // 1 磅力等于 4.4482216152605 牛顿 / One pound force equals 4.4482216152605 newtons
        assert_factor_relative::<PoundForce, Newton>("4.4482216152605", FUZZ, "磅力到牛顿 / pound force to newton");
    }

    #[test]
    fn test_kilopound_force_equals_one_thousand_pound_force() {
        // 1 千磅力等于 1000 磅力 / One kilopound force equals 1000 pound force
        assert_factor_relative::<KilopoundForce, PoundForce>("1000", FUZZ, "千磅力到磅力 / kilopound force to pound force");
    }

    #[test]
    fn test_force_dimension_symbol() {
        // 力量纲符号为 L·M·T^-2 / Force dimension symbol is L·M·T^-2
        assert_dimension_symbol::<Newton>("L·M·T^-2", "牛顿量纲 / newton dimension");
    }

    #[test]
    fn test_force_dimension_and_domain() {
        // 力为连续量，且与质量、加速度量纲不同 / Force is continuous and differs from mass and acceleration
        assert_domain::<Newton>(
            crate::dimension::derived_quantity::QuantityDomain::Continuous,
            "牛顿取值域 / newton domain",
        );
        assert_different_dimension::<Newton, crate::unit::derived::Kilogram>("力与质量 / force vs mass");
        assert_different_dimension::<Newton, super::super::acceleration::MeterPerSecondSquared>(
            "力与加速度 / force vs acceleration",
        );
    }

    #[test]
    fn test_force_units_share_dimension() {
        // 所有力单位共享力量纲 / All force units share the force dimension
        assert_same_dimension::<Newton, PoundForce>("牛顿与磅力 / newton vs pound force");
        assert_same_dimension::<Newton, Dyne>("牛顿与达因 / newton vs dyne");
        assert_same_dimension::<Newton, Meganewton>("牛顿与兆牛 / newton vs meganewton");
    }

    #[test]
    fn test_force_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<Kilonewton>("kN", "kilonewton", "千牛实例 / kilonewton instance");
        assert_instance_symbol_and_name::<PoundForce>("lbf", "pound force", "磅力实例 / pound force instance");
    }
}
