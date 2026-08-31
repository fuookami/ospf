//! 能量单位 / Energy units

use crate::dimension::derived::Energy;
use crate::scale::{GIGA, KILO, MEGA, Scale};
use crate::unit::{CTUnit, CTUnitMul};
use super::force::Newton;
use super::length::Meter;
use super::power::Kilowatt;
use super::time::Hour;

define_unit_by!(Joule, "joule", "J", CTUnitMul<Newton, Meter>);
define_unit!(
    Kilojoule,
    "kilojoule",
    "kJ",
    Energy,
    &*Joule::SCALE * &*KILO
);
define_unit!(
    Megajoule,
    "megajoule",
    "MJ",
    Energy,
    &*Joule::SCALE * &*MEGA
);
define_unit!(
    Gigajoule,
    "gigajoule",
    "GJ",
    Energy,
    &*Joule::SCALE * &*GIGA
);

define_unit!(
    ElectronVolt,
    "electronvolt",
    "eV",
    Energy,
    Scale::from_f64(1.602176634e-19)
);
define_unit!(
    Kiloelectronvolt,
    "kiloelectronvolt",
    "keV",
    Energy,
    &*ElectronVolt::SCALE * &*KILO
);
define_unit!(
    Megaelectronvolt,
    "megaelectronvolt",
    "MeV",
    Energy,
    &*ElectronVolt::SCALE * &*MEGA
);
define_unit!(
    Gigaelectronvolt,
    "gigaelectronvolt",
    "GeV",
    Energy,
    &*ElectronVolt::SCALE * &*GIGA
);

define_unit_by!(KilowattHour, "kilowatt-hour", "kWh", CTUnitMul<Kilowatt, Hour>);
define_unit!(
    MegawattHour,
    "megawatt-hour",
    "MWh",
    Energy,
    &*KilowattHour::SCALE * &*KILO
);
define_unit!(
    GigawattHour,
    "gigawatt-hour",
    "GWh",
    Energy,
    &*KilowattHour::SCALE * &*MEGA
);
define_unit!(
    WattHour,
    "watt-hour",
    "Wh",
    Energy,
    &*KilowattHour::SCALE / &*KILO
);

define_unit!(Calorie, "calorie", "cal", Energy, Scale::from_f64(4.184));
define_unit!(
    Kilocalorie,
    "kilocalorie",
    "kcal",
    Energy,
    &*Calorie::SCALE * &*KILO
);
define_unit!(
    BritishThermalUnit,
    "british thermal unit",
    "BTU",
    Energy,
    Scale::from_f64(1055.06)
);
define_unit!(Erg, "erg", "erg", Energy, Scale::from_f64(1e-7));
