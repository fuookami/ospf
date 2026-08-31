//! Area units - 面积单位
//! Area units - SI area units
//!
//! 提供面积量纲的 SI 单位定义，包括平方米、平方千米、公顷等。
//! Provides SI unit definitions for area dimension, including square meter, square kilometer, hectare, etc.

use super::length::{
    Chain, Foot, Inch, Kilometer, Meter, Mile, Rod, Yard,
};
use crate::dimension::derived::Area;
use crate::scale::Scale;
use crate::unit::physical_unit::CTUnit;
use crate::unit::CTUnitMul;

// ============================================================================
// SI 面积单位 / SI area units
// ============================================================================

define_unit_by!(SquareMeter, "square meter", "m²", CTUnitMul<Meter, Meter>);
define_unit_by!(
    SquareKilometer,
    "square kilometer",
    "km²",
    CTUnitMul<Kilometer, Kilometer>
);

// ============================================================================
// 公制面积单位 / Metric area units
// ============================================================================

// 公亩 / Are (100 square meters)
define_unit!(Are, "are", "are", Area, Scale::from_f64(100.0));

// 公顷 / Hectare (10,000 square meters)
define_unit!(Hectare, "hectare", "ha", Area, Scale::from_f64(10000.0));

// ============================================================================
// 英制面积单位 / Imperial area units
// ============================================================================

// 平方英寸 / Square inch
define_unit_by!(
    SquareInch,
    "square inch",
    "sq.in",
    CTUnitMul<Inch, Inch>
);

// 平方英尺 / Square foot
define_unit_by!(
    SquareFoot,
    "square foot",
    "sq.ft",
    CTUnitMul<Foot, Foot>
);

// 平方码 / Square yard
define_unit_by!(
    SquareYard,
    "square yard",
    "sq.yd",
    CTUnitMul<Yard, Yard>
);

// 平方链 / Square chain
define_unit_by!(
    SquareChain,
    "square chain",
    "sq.ch",
    CTUnitMul<Chain, Chain>
);

// 平方杆 / Square rod
define_unit_by!(
    SquareRod,
    "square rod",
    "sq.rd",
    CTUnitMul<Rod, Rod>
);

// 平方英里 / Square mile
define_unit_by!(
    SquareMile,
    "square mile",
    "sq.mi",
    CTUnitMul<Mile, Mile>
);

// 英亩 / Acre (43,560 square feet)
define_unit!(Acre, "acre", "acre", Area, Scale::from_f64(4046.8564224));
