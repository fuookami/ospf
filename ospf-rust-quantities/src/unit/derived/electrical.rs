//! 电学单位 / Electrical units

use super::power::Watt;
use super::time::{Hour, Second};
use crate::dimension::derived::{Capacitance, ElectricCharge, ElectricCurrent, ElectricPotential};
use crate::scale::{KILO, MEGA, MICRO, MILLI, NANO, PICO};
use crate::unit::{CTUnit, CTUnitDiv, CTUnitMul};

define_unit!(Ampere, "ampere", "A", ElectricCurrent);
define_unit!(
    Milliampere,
    "milliampere",
    "mA",
    ElectricCurrent,
    &*Ampere::SCALE * &*MILLI
);
define_unit!(
    Microampere,
    "microampere",
    "uA",
    ElectricCurrent,
    &*Ampere::SCALE * &*MICRO
);
define_unit!(
    Kiloampere,
    "kiloampere",
    "kA",
    ElectricCurrent,
    &*Ampere::SCALE * &*KILO
);

define_unit_by!(Coulomb, "coulomb", "C", CTUnitMul<Ampere, Second>);
define_unit!(
    Millicoulomb,
    "millicoulomb",
    "mC",
    ElectricCharge,
    &*Coulomb::SCALE * &*MILLI
);
define_unit!(
    Microcoulomb,
    "microcoulomb",
    "uC",
    ElectricCharge,
    &*Coulomb::SCALE * &*MICRO
);
define_unit!(
    Kilocoulomb,
    "kilocoulomb",
    "kC",
    ElectricCharge,
    &*Coulomb::SCALE * &*KILO
);
define_unit_by!(AmpereSecond, "ampere-second", "As", CTUnitMul<Ampere, Second>);
define_unit_by!(
    MilliampereHour,
    "milliampere-hour",
    "mAh",
    CTUnitMul<Milliampere, Hour>
);
define_unit_by!(
    MicroampereHour,
    "microampere-hour",
    "uAh",
    CTUnitMul<Microampere, Hour>
);
define_unit_by!(
    AmpereHour,
    "ampere-hour",
    "Ah",
    CTUnitMul<Ampere, Hour>
);
define_unit_by!(
    KiloampereHour,
    "kiloampere-hour",
    "kAh",
    CTUnitMul<Kiloampere, Hour>
);

define_unit_by!(Volt, "volt", "V", CTUnitDiv<Watt, Ampere>);
define_unit!(
    Microvolt,
    "microvolt",
    "uV",
    ElectricPotential,
    &*Volt::SCALE * &*MICRO
);
define_unit!(
    Millivolt,
    "millivolt",
    "mV",
    ElectricPotential,
    &*Volt::SCALE * &*MILLI
);
define_unit!(
    Kilovolt,
    "kilovolt",
    "kV",
    ElectricPotential,
    &*Volt::SCALE * &*KILO
);
define_unit!(
    Megavolt,
    "megavolt",
    "MV",
    ElectricPotential,
    &*Volt::SCALE * &*MEGA
);

define_unit_by!(Farad, "farad", "F", CTUnitDiv<Coulomb, Volt>);
define_unit!(
    Millifarad,
    "millifarad",
    "mF",
    Capacitance,
    &*Farad::SCALE * &*MILLI
);
define_unit!(
    Microfarad,
    "microfarad",
    "uF",
    Capacitance,
    &*Farad::SCALE * &*MICRO
);
define_unit!(
    Nanofarad,
    "nanofarad",
    "nF",
    Capacitance,
    &*Farad::SCALE * &*NANO
);
define_unit!(
    Picofarad,
    "picofarad",
    "pF",
    Capacitance,
    &*Farad::SCALE * &*PICO
);
