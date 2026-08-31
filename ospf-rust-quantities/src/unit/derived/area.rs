use super::length::{
    Cetimeter, Chain, Decimeter, Foot, Inch, Kilometer, Meter, Mile, Millimeter, Rod, Yard,
};
use crate::dimension::derived::Area;
use crate::scale::Scale;
use crate::unit::CTUnitMul;
use crate::unit::physical_unit::CTUnit;

define_unit_by!(
    SquareMillimeter,
    "square millimeter",
    "mm^2",
    CTUnitMul<Millimeter, Millimeter>
);
define_unit_by!(
    SquareCentimeter,
    "square centimeter",
    "cm^2",
    CTUnitMul<Cetimeter, Cetimeter>
);
define_unit_by!(
    SquareDecimeter,
    "square decimeter",
    "dm^2",
    CTUnitMul<Decimeter, Decimeter>
);
define_unit_by!(SquareMeter, "square meter", "m^2", CTUnitMul<Meter, Meter>);
define_unit_by!(
    SquareKilometer,
    "square kilometer",
    "km^2",
    CTUnitMul<Kilometer, Kilometer>
);

define_unit!(Are, "are", "are", Area, Scale::from_f64(100.0));
define_unit!(Hectare, "hectare", "ha", Area, Scale::from_f64(10000.0));

define_unit_by!(
    SquareInch,
    "square inch",
    "sq.in",
    CTUnitMul<Inch, Inch>
);
define_unit_by!(
    SquareFoot,
    "square foot",
    "sq.ft",
    CTUnitMul<Foot, Foot>
);
define_unit_by!(
    SquareYard,
    "square yard",
    "sq.yd",
    CTUnitMul<Yard, Yard>
);
define_unit_by!(
    SquareChain,
    "square chain",
    "sq.ch",
    CTUnitMul<Chain, Chain>
);
define_unit_by!(
    SquareRod,
    "square rod",
    "sq.rd",
    CTUnitMul<Rod, Rod>
);
define_unit_by!(
    SquareMile,
    "square mile",
    "sq.mi",
    CTUnitMul<Mile, Mile>
);

define_unit!(Acre, "acre", "acre", Area, Scale::from_f64(4046.8564224));
