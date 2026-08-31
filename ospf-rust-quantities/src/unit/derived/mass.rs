//! Mass units - 质量单位
//! Mass units - SI mass units (kilogram, gram, etc.)
//!
//! 提供质量量纲的 SI 单位定义，包括千克、克、毫克、吨等。
//! Provides SI unit definitions for mass dimension, including kilogram, gram, milligram, tonne, etc.

use crate::dimension::derived::Mass;
use crate::scale::{Scale, KILO, MILLI};
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
define_unit!(Microgram, "microgram", "μg", Mass, Scale::from_f64(0.000000001));
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
define_unit!(LongTon, "long ton", "lt", Mass, Scale::from_f64(1016.0469088));

// 美吨 / Short ton (2000 pounds)
define_unit!(ShortTon, "short ton", "st", Mass, Scale::from_f64(907.18474));

// 英石 / Stone (14 pounds)
define_unit!(Stone, "stone", "st", Mass, Scale::from_f64(6.35029318));

// 盎司 / Ounce
define_unit!(Ounce, "ounce", "oz", Mass, Scale::from_f64(0.028349523125));

// 金衡盎司 / Troy ounce
define_unit!(TroyOunce, "troy ounce", "oz.tr", Mass, Scale::from_f64(0.0311034768));

// 打兰 / Dram
define_unit!(Dram, "dram", "dr", Mass, Scale::from_f64(0.0017718451953125));
