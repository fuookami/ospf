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
