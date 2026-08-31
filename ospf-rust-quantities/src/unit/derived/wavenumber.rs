use crate::unit::{CTUnit, CTUnitReciprocal};
use super::length::Meter;

define_unit_by!(
    ReciprocalMeter,
    "reciprocal meter",
    "1/m",
    CTUnitReciprocal<Meter>
);
