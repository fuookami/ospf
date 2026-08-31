//! Information units - 信息量单位
//! Information units - Information units (bit, byte, etc.)
//!
//! 提供信息量量纲的单位定义，包括比特、字节、千比特、兆比特等。
//! Provides unit definitions for information dimension, including bit, byte, kilobit, megabit, etc.

use crate::dimension::derived::Information;
use crate::scale::{Scale, GIGA, KILO, MEGA, OCTAL, TERA};
use crate::unit::CTUnit;
use bigdecimal::BigDecimal;
use once_cell::sync::Lazy;

// ============================================================================
// 信息量单位 / Information units
// ============================================================================

static INFO_RADIX: Lazy<Scale> = Lazy::new(|| Scale::from_int(2).pow(&BigDecimal::from(10)));

define_unit!(Bit, "bit", "bit", Information);
define_unit!(Kilobit, "kilobit", "kilobit", Information, KILO.clone());
define_unit!(Megabit, "megabit", "megabit", Information, MEGA.clone());
define_unit!(Gigabit, "gigabit", "gigabit", Information, GIGA.clone());
define_unit!(Terabit, "terabit", "terabit", Information, TERA.clone());

define_unit!(Byte, "byte", "B", Information, OCTAL.clone());
define_unit!(
    Kibibyte,
    "kibibyte",
    "KiB",
    Information,
    &*Byte::SCALE * &*INFO_RADIX
);
define_unit!(
    Kilobyte,
    "kilobyte",
    "kB",
    Information,
    &*Byte::SCALE * &*KILO
);
define_unit!(
    Mebibyte,
    "mebibyte",
    "MiB",
    Information,
    &*Kibibyte::SCALE * &*INFO_RADIX
);
define_unit!(
    Megabyte,
    "megabyte",
    "MB",
    Information,
    &*Byte::SCALE * &*MEGA
);
define_unit!(
    Gibibyte,
    "gibibyte",
    "GiB",
    Information,
    &*Mebibyte::SCALE * &*INFO_RADIX
);
define_unit!(
    Gigabyte,
    "gigabyte",
    "GB",
    Information,
    &*Byte::SCALE * &*GIGA
);
define_unit!(
    Tebibyte,
    "tebibyte",
    "TiB",
    Information,
    &*Gibibyte::SCALE * &*INFO_RADIX
);
