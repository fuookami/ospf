//! 信息量单位 / Information units
//!
//! 提供信息量量纲的单位定义，包括比特、字节、千比特、兆比特等 / Provides unit definitions for information dimension, including bit, byte, kilobit, megabit, etc

use crate::dimension::derived::Information;
use crate::dimension::derived_quantity::QuantityDomain;
use crate::scale::{EXA, GIGA, KILO, MEGA, OCTAL, PETA, Scale, TERA};
use crate::unit::CTUnit;
use bigdecimal::BigDecimal;
use once_cell::sync::Lazy;

// ============================================================================
// 信息量单位 / Information units
// ============================================================================

static INFO_RADIX: Lazy<Scale> = Lazy::new(|| Scale::from_int(2).pow(&BigDecimal::from(10)));

define_unit!(Bit, "bit", "bit", Information);
define_unit!(
    Kilobit,
    "kilobit",
    "kilobit",
    Information,
    KILO.clone(),
    domain = QuantityDomain::Continuous
);
define_unit!(
    Megabit,
    "megabit",
    "megabit",
    Information,
    MEGA.clone(),
    domain = QuantityDomain::Continuous
);
define_unit!(
    Gigabit,
    "gigabit",
    "gigabit",
    Information,
    GIGA.clone(),
    domain = QuantityDomain::Continuous
);
define_unit!(
    Terabit,
    "terabit",
    "terabit",
    Information,
    TERA.clone(),
    domain = QuantityDomain::Continuous
);
define_unit!(
    Petabit,
    "petabit",
    "petabit",
    Information,
    PETA.clone(),
    domain = QuantityDomain::Continuous
);
define_unit!(
    Exabit,
    "exabit",
    "exabit",
    Information,
    EXA.clone(),
    domain = QuantityDomain::Continuous
);

define_unit!(Byte, "byte", "B", Information, OCTAL.clone());
define_unit!(
    Kibibyte,
    "kibibyte",
    "KiB",
    Information,
    &*Byte::SCALE * &*INFO_RADIX,
    domain = QuantityDomain::Continuous
);
define_unit!(
    Kilobyte,
    "kilobyte",
    "kB",
    Information,
    &*Byte::SCALE * &*KILO,
    domain = QuantityDomain::Continuous
);
define_unit!(
    Mebibyte,
    "mebibyte",
    "MiB",
    Information,
    &*Kibibyte::SCALE * &*INFO_RADIX,
    domain = QuantityDomain::Continuous
);
define_unit!(
    Megabyte,
    "megabyte",
    "MB",
    Information,
    &*Byte::SCALE * &*MEGA,
    domain = QuantityDomain::Continuous
);
define_unit!(
    Gibibyte,
    "gibibyte",
    "GiB",
    Information,
    &*Mebibyte::SCALE * &*INFO_RADIX,
    domain = QuantityDomain::Continuous
);
define_unit!(
    Gigabyte,
    "gigabyte",
    "GB",
    Information,
    &*Byte::SCALE * &*GIGA,
    domain = QuantityDomain::Continuous
);
define_unit!(
    Terabyte,
    "terabyte",
    "TB",
    Information,
    &*Byte::SCALE * &*TERA,
    domain = QuantityDomain::Continuous
);
define_unit!(
    Petabyte,
    "petabyte",
    "PB",
    Information,
    &*Byte::SCALE * &*PETA,
    domain = QuantityDomain::Continuous
);
define_unit!(
    Exabyte,
    "exabyte",
    "EB",
    Information,
    &*Byte::SCALE * &*EXA,
    domain = QuantityDomain::Continuous
);
define_unit!(
    Tebibyte,
    "tebibyte",
    "TiB",
    Information,
    &*Gibibyte::SCALE * &*INFO_RADIX,
    domain = QuantityDomain::Continuous
);

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::derived::test_support::*;

    #[test]
    fn test_information_decimal_prefix_symbols_and_names() {
        // 十进制信息量前缀单位的符号与名称 / Symbols and names of decimal prefix information units
        assert_symbol_and_name::<Bit>("bit", "bit", "比特 / bit");
        assert_symbol_and_name::<Kilobit>("kilobit", "kilobit", "千比特 / kilobit");
        assert_symbol_and_name::<Megabit>("megabit", "megabit", "兆比特 / megabit");
        assert_symbol_and_name::<Gigabit>("gigabit", "gigabit", "吉比特 / gigabit");
        assert_symbol_and_name::<Terabit>("terabit", "terabit", "太比特 / terabit");
        assert_symbol_and_name::<Petabit>("petabit", "petabit", "拍比特 / petabit");
        assert_symbol_and_name::<Exabit>("exabit", "exabit", "艾比特 / exabit");
        assert_symbol_and_name::<Byte>("B", "byte", "字节 / byte");
        assert_symbol_and_name::<Kilobyte>("kB", "kilobyte", "千字节 / kilobyte");
        assert_symbol_and_name::<Megabyte>("MB", "megabyte", "兆字节 / megabyte");
        assert_symbol_and_name::<Gigabyte>("GB", "gigabyte", "吉字节 / gigabyte");
        assert_symbol_and_name::<Terabyte>("TB", "terabyte", "太字节 / terabyte");
        assert_symbol_and_name::<Petabyte>("PB", "petabyte", "拍字节 / petabyte");
        assert_symbol_and_name::<Exabyte>("EB", "exabyte", "艾字节 / exabyte");
    }

    #[test]
    fn test_information_binary_prefix_symbols_and_names() {
        // 二进制信息量前缀单位的符号与名称 / Symbols and names of binary prefix information units
        assert_symbol_and_name::<Kibibyte>("KiB", "kibibyte", "基比字节 / kibibyte");
        assert_symbol_and_name::<Mebibyte>("MiB", "mebibyte", "梅比字节 / mebibyte");
        assert_symbol_and_name::<Gibibyte>("GiB", "gibibyte", "吉比字节 / gibibyte");
        assert_symbol_and_name::<Tebibyte>("TiB", "tebibyte", "特比字节 / tebibyte");
    }

    #[test]
    fn test_bit_is_base_unit_with_unit_scale() {
        // 比特是信息量的基准单位，比例尺为 1 / Bit is the base unit of information with scale 1
        assert_scale_exact(Bit::SCALE.value(), "1", "比特比例尺 / bit scale");
    }

    #[test]
    fn test_byte_equals_eight_bit() {
        // 1 字节等于 8 比特 / One byte equals eight bits
        assert_factor_exact::<Byte, Bit>("8", "字节到比特 / byte to bit");
    }

    #[test]
    fn test_kilobyte_follows_decimal_prefix() {
        // 千字节使用十进制前缀：1 kB = 1000 B = 8000 bit（二进制 1024 由 Ki bibyte 承担）
        // Kilobyte uses the decimal prefix: 1 kB = 1000 B = 8000 bit (the binary 1024 is covered by kibibyte)
        assert_factor_exact::<Kilobyte, Byte>("1000", "千字节到字节 / kilobyte to byte");
        assert_factor_exact::<Kilobyte, Bit>("8000", "千字节到比特 / kilobyte to bit");
    }

    #[test]
    fn test_kibibyte_equals_1024_byte_and_8192_bit() {
        // 1 基比字节等于 1024 字节 / 8192 比特 / One kibibyte equals 1024 bytes / 8192 bits
        assert_factor_exact::<Kibibyte, Byte>("1024", "基比字节到字节 / kibibyte to byte");
        assert_factor_exact::<Kibibyte, Bit>("8192", "基比字节到比特 / kibibyte to bit");
    }

    #[test]
    fn test_kilobit_equals_one_thousand_bit() {
        // 1 千比特等于 1000 比特（十进制前缀）/ One kilobit equals 1000 bits (decimal prefix)
        assert_factor_exact::<Kilobit, Bit>("1000", "千比特到比特 / kilobit to bit");
    }

    #[test]
    fn test_decimal_information_prefix_series() {
        // 十进制比特与字节前缀按 1000 递进 / Decimal bit and byte prefixes advance by 1000
        assert_factor_exact::<Megabit, Kilobit>("1000", "兆比特到千比特 / megabit to kilobit");
        assert_factor_exact::<Gigabit, Megabit>("1000", "吉比特到兆比特 / gigabit to megabit");
        assert_factor_exact::<Terabit, Gigabit>("1000", "太比特到吉比特 / terabit to gigabit");
        assert_factor_exact::<Petabit, Terabit>("1000", "拍比特到太比特 / petabit to terabit");
        assert_factor_exact::<Exabit, Petabit>("1000", "艾比特到拍比特 / exabit to petabit");
        assert_factor_exact::<Megabyte, Kilobyte>("1000", "兆字节到千字节 / megabyte to kilobyte");
        assert_factor_exact::<Gigabyte, Megabyte>("1000", "吉字节到兆字节 / gigabyte to megabyte");
        assert_factor_exact::<Terabyte, Gigabyte>("1000", "太字节到吉字节 / terabyte to gigabyte");
        assert_factor_exact::<Petabyte, Terabyte>("1000", "拍字节到太字节 / petabyte to terabyte");
        assert_factor_exact::<Exabyte, Petabyte>("1000", "艾字节到拍字节 / exabyte to petabyte");
    }

    #[test]
    fn test_binary_information_prefix_series() {
        // 二进制字节前缀按 1024 递进 / Binary byte prefixes advance by 1024
        assert_factor_exact::<Kibibyte, Byte>("1024", "基比字节到字节 / kibibyte to byte");
        assert_factor_exact::<Mebibyte, Kibibyte>("1024", "梅比字节到基比字节 / mebibyte to kibibyte");
        assert_factor_exact::<Gibibyte, Mebibyte>("1024", "吉比字节到梅比字节 / gibibyte to mebibyte");
        assert_factor_exact::<Tebibyte, Gibibyte>("1024", "特比字节到吉比字节 / tebibyte to gibibyte");
    }

    #[test]
    fn test_binary_information_prefix_byte_values() {
        // 二进制前缀对应的字节绝对值 / Absolute byte values of binary prefixes
        assert_factor_exact::<Kibibyte, Bit>("8192", "基比字节到比特 / kibibyte to bit");
        assert_factor_exact::<Mebibyte, Byte>("1048576", "梅比字节到字节 / mebibyte to byte");
        assert_factor_exact::<Gibibyte, Byte>("1073741824", "吉比字节到字节 / gibibyte to byte");
        assert_factor_exact::<Tebibyte, Byte>("1099511627776", "特比字节到字节 / tebibyte to byte");
    }

    #[test]
    fn test_kilobyte_and_kibibyte_differ() {
        // 十进制千字节与二进制基比字节不同：比例约为 0.9765625
        // Decimal kilobyte and binary kibibyte differ with a ratio of about 0.9765625
        assert_factor_exact::<Kilobyte, Kibibyte>("0.9765625", "千字节到基比字节 / kilobyte to kibibyte");
        assert_ne!(Kilobyte::SCALE.value(), Kibibyte::SCALE.value());
        assert_not_identity::<Kibibyte>("基比字节 / kibibyte");
    }

    #[test]
    fn test_information_domains() {
        // 比特与字节为离散量，带前缀单位为连续量 / Bit and byte are discrete while prefixed units are continuous
        assert_domain::<Bit>(QuantityDomain::Discrete, "比特取值域 / bit domain");
        assert_domain::<Byte>(QuantityDomain::Discrete, "字节取值域 / byte domain");
        assert_domain::<Kibibyte>(QuantityDomain::Continuous, "基比字节取值域 / kibibyte domain");
        assert_domain::<Mebibyte>(QuantityDomain::Continuous, "梅比字节取值域 / mebibyte domain");
        assert_domain::<Gibibyte>(QuantityDomain::Continuous, "吉比字节取值域 / gibibyte domain");
        assert_domain::<Tebibyte>(QuantityDomain::Continuous, "特比字节取值域 / tebibyte domain");
        assert_domain::<Kilobyte>(QuantityDomain::Continuous, "千字节取值域 / kilobyte domain");
        assert_domain::<Megabyte>(QuantityDomain::Continuous, "兆字节取值域 / megabyte domain");
    }

    #[test]
    fn test_information_units_share_dimension() {
        // 所有信息量单位共享信息量量纲 / All information units share the information dimension
        assert_same_dimension::<Bit, Byte>("比特与字节 / bit vs byte");
        assert_same_dimension::<Bit, Tebibyte>("比特与特比字节 / bit vs tebibyte");
        assert_different_dimension::<Bit, crate::unit::derived::Meter>("比特与米 / bit vs meter");
        assert_different_dimension::<Bit, crate::unit::derived::Second>("比特与秒 / bit vs second");
    }

    #[test]
    fn test_information_dimension_symbol() {
        // 信息量量纲符号为 ℐ / Information dimension symbol is ℐ
        assert_dimension_symbol::<Bit>("ℐ", "比特量纲 / bit dimension");
    }

    #[test]
    fn test_information_instance_metadata() {
        // 运行时单位实例的符号、名称与比例尺一致 / Runtime unit instance metadata is consistent
        assert_instance_symbol_and_name::<Megabyte>("MB", "megabyte", "兆字节实例 / megabyte instance");
        assert_instance_symbol_and_name::<Kibibyte>("KiB", "kibibyte", "基比字节实例 / kibibyte instance");
    }
}
