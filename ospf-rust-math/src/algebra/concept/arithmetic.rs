use crate::algebra::operator::{Neg, Reciprocal};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, Sub, SubAssign};

pub trait Arithmetic: Sized + Clone + PartialEq + PartialOrd {
    const ZERO: Self;
    const ONE: Self;
}

pub trait PlusSemiGroup: Arithmetic + Add<Output = Self> + AddAssign {}
impl<T: Arithmetic + Add<Output = Self> + AddAssign> PlusSemiGroup for T {}

pub trait PlusGroup: PlusSemiGroup + Neg<Output = Self> + Sub<Output = Self> + SubAssign {}
impl<T: PlusSemiGroup + Neg<Output = Self> + Sub<Output = Self> + SubAssign> PlusGroup for T {}

pub trait TimesSemiGroup: Arithmetic + Mul<Output = Self> + MulAssign {}
impl<T: Arithmetic + Mul<Output = Self> + MulAssign> TimesSemiGroup for T {}

pub trait TimesGroup:
    Arithmetic + Reciprocal<Output = Self> + Div<Output = Self> + DivAssign + Rem<Output = Self>
{
}
impl<
        T: Arithmetic
            + Reciprocal<Output = Self>
            + Div<Output = Self>
            + DivAssign
            + Rem<Output = Self>,
    > TimesGroup for T
{
}

pub trait NumberRing: PlusGroup + TimesSemiGroup {}
impl<T: PlusGroup + TimesSemiGroup> NumberRing for T {}

pub trait NumberField: NumberRing + TimesGroup {}
impl<T: NumberRing + TimesGroup> NumberField for T {}

pub struct Infinity {}
pub const INF: Infinity = Infinity {};

impl std::fmt::Display for Infinity {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "inf")
    }
}

impl std::fmt::Debug for Infinity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "inf")
    }
}

pub struct NegativeInfinity {}
pub const NEG_INF: NegativeInfinity = NegativeInfinity {};

impl std::fmt::Display for NegativeInfinity {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "-inf")
    }
}

impl std::fmt::Debug for NegativeInfinity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "-inf")
    }
}

pub struct NaN {}
pub const NAN: NaN = NaN {};

impl std::fmt::Display for NaN {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "nan")
    }
}

impl std::fmt::Debug for NaN {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "nan")
    }
}
