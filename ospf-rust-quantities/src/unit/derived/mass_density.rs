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
