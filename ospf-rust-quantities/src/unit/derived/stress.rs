use crate::dimension::derived::Pressure;
use crate::scale::{KILO, MEGA, Scale};
use crate::unit::{CTUnit, CTUnitDiv};
use super::area::SquareMeter;
use super::force::Newton;

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
