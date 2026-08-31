use std::ops::*;

pub trait PlusSemiGroup:
    Sized
    + Add<Output = Self>
    + for<'a> Add<&'a Self, Output = Self>
    + AddAssign
    + for<'a> AddAssign<&'a Self>
{
}

impl<
        T: Sized
            + Add<Output = Self>
            + for<'a> Add<&'a Self, Output = Self>
            + AddAssign
            + for<'a> AddAssign<&'a Self>,
    > PlusSemiGroup for T
{
}

pub trait PlusGroup:
    PlusSemiGroup
    + Neg<Output = Self>
    + Sub<Output = Self>
    + for<'a> Sub<&'a Self, Output = Self>
    + SubAssign
    + for<'a> SubAssign<&'a Self>
{
}

impl<
        T: PlusSemiGroup
            + Neg<Output = T>
            + Sub<Output = T>
            + for<'a> Sub<&'a T, Output = T>
            + SubAssign
            + for<'a> SubAssign<&'a Self>,
    > PlusGroup for T
{
}

pub trait TimesSemiGroup:
    Sized
    + Mul<Output = Self>
    + for<'a> Mul<&'a Self, Output = Self>
    + MulAssign
    + for<'a> MulAssign<&'a Self>
{
}

impl<
        T: Sized
            + Mul<Output = T>
            + for<'a> Mul<&'a T, Output = T>
            + MulAssign
            + for<'a> MulAssign<&'a T>,
    > TimesSemiGroup for T
{
}

pub trait TimesGroup:
    TimesSemiGroup
    + Div<Output = Self>
    + for<'a> Div<&'a Self, Output = Self>
    + DivAssign
    + for<'a> DivAssign<&'a Self>
    + Rem<Output = Self>
    + for<'a> Rem<&'a Self, Output = Self>
    + RemAssign
    + for<'a> RemAssign<&'a Self>
{
}

impl<
        T: TimesSemiGroup
            + Div<Output = T>
            + for<'a> Div<&'a T, Output = T>
            + DivAssign
            + for<'a> DivAssign<&'a T>
            + Rem<Output = T>
            + for<'a> Rem<&'a T, Output = T>
            + RemAssign
            + for<'a> RemAssign<&'a T>,
    > TimesGroup for T
{
}

pub trait NumberRing: PlusGroup + TimesSemiGroup {}

impl<T: PlusGroup + TimesSemiGroup> NumberRing for T {}

pub trait NumberField: NumberRing + TimesGroup {}

impl<T: NumberRing + TimesGroup> NumberField for T {}
