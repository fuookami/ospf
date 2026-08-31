use super::length::Meter;
use crate::unit::{CTUnit, CTUnitReciprocal};

define_unit_by!(
    ReciprocalMeter,
    "reciprocal meter",
    "1/m",
    CTUnitReciprocal<Meter>
);
