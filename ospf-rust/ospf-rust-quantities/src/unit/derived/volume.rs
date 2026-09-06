//! 体积单位 / Volume units
//!
//! 提供体积量纲的 SI 单位定义，包括立方米、升、毫升等 / Provides SI unit definitions for volume dimension, including cubic meter, liter, milliliter, etc

use super::length::{Cetimeter, Decimeter, Foot, Inch, Meter, Millimeter, Yard};
use crate::dimension::derived::Volume;
use crate::scale::Scale;
use crate::unit::{CTUnit, CTUnitMul};

// ============================================================================
// SI 体积单位 / SI volume units
// ============================================================================

define_unit_by!(
    CubicMeter,
    "cubic meter",
    "m³",
    CTUnitMul<Meter, CTUnitMul<Meter, Meter>>
);

define_unit_by!(
    CubicKilometer,
    "cubic kilometer",
    "km³",
    CTUnitMul<super::length::Kilometer, CTUnitMul<super::length::Kilometer, super::length::Kilometer>>
);

define_unit_by!(
    CubicHectometer,
    "cubic hectometer",
    "hm³",
    CTUnitMul<super::length::Hectometer, CTUnitMul<super::length::Hectometer, super::length::Hectometer>>
);

define_unit_by!(
    CubicDecameter,
    "cubic decameter",
    "dam³",
    CTUnitMul<super::length::Decameter, CTUnitMul<super::length::Decameter, super::length::Decameter>>
);

define_unit_by!(
    CubicDecimeter,
    "cubic decimeter",
    "dm³",
    CTUnitMul<Decimeter, CTUnitMul<Decimeter, Decimeter>>
);

define_unit_by!(
    CubicCentimeter,
    "cubic centimeter",
    "cm³",
    CTUnitMul<Cetimeter, CTUnitMul<Cetimeter, Cetimeter>>
);

define_unit_by!(
    CubicMillimeter,
    "cubic millimeter",
    "mm³",
    CTUnitMul<Millimeter, CTUnitMul<Millimeter, Millimeter>>
);

// ============================================================================
// 公制液体单位 / Metric liquid units
// ============================================================================

// 升 / Liter (1 dm³)
define_unit!(
    Liter,
    "liter",
    "L",
    Volume,
    Scale::from_int(1) / Scale::from_int(1000)
);

//  hectoliter / Hectoliter
define_unit!(Hectoliter, "hectoliter", "hL", Volume, Scale::from_f64(0.1));

//  deciliter / Deciliter
define_unit!(
    Deciliter,
    "deciliter",
    "dL",
    Volume,
    Scale::from_f64(0.0001)
);

//  centiliter / Centiliter
define_unit!(
    Centiliter,
    "centiliter",
    "cL",
    Volume,
    Scale::from_f64(0.00001)
);

//  milliliter / Milliliter
define_unit!(
    Milliliter,
    "milliliter",
    "mL",
    Volume,
    Scale::from_f64(0.000001)
);

//  microliter / Microliter
define_unit!(
    Microliter,
    "microliter",
    "μL",
    Volume,
    Scale::from_f64(0.000000001)
);

// ============================================================================
// 英制体积单位 / Imperial volume units
// ============================================================================

// 立方英寸 / Cubic inch
define_unit_by!(
    CubicInch,
    "cubic inch",
    "cu.in",
    CTUnitMul<Inch, CTUnitMul<Inch, Inch>>
);

// 立方英尺 / Cubic foot
define_unit_by!(
    CubicFoot,
    "cubic foot",
    "cu.ft",
    CTUnitMul<Foot, CTUnitMul<Foot, Foot>>
);

// 立方码 / Cubic yard
define_unit_by!(
    CubicYard,
    "cubic yard",
    "cu.yd",
    CTUnitMul<Yard, CTUnitMul<Yard, Yard>>
);

// ============================================================================
// 英制液体单位 / Imperial liquid units
// ============================================================================

// 英制液量盎司 / UK fluid ounce (28.4130625 mL)
define_unit!(
    UKFluidOunce,
    "uk fluid ounce",
    "uk.fl.oz",
    Volume,
    Scale::from_f64(0.0000284130625)
);

// 美制液量盎司 / US fluid ounce (29.5735295625 mL)
define_unit!(
    USFluidOunce,
    "us fluid ounce",
    "us.fl.oz",
    Volume,
    Scale::from_f64(0.0000295735295625)
);

// 英制加仑 / UK gallon (160 UK fluid ounces)
define_unit!(
    UKGallon,
    "uk gallon",
    "uk.gal",
    Volume,
    Scale::from_f64(0.00454609)
);

// 美制加仑 / US gallon (128 US fluid ounces)
define_unit!(
    USGallon,
    "us gallon",
    "us.gal",
    Volume,
    Scale::from_f64(0.00378541178)
);
