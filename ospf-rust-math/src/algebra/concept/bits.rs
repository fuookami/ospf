use std::ops::*;

pub trait Bits:
    Sized
    + BitAnd<Self, Output = Self>
    + for<'a> BitAnd<&'a Self, Output = Self>
    + BitAndAssign<Self>
    + BitOr<Self, Output = Self>
    + for<'a> BitOr<&'a Self, Output = Self>
    + BitOrAssign<Self>
    + BitXor<Self, Output = Self>
    + for<'a> BitXor<&'a Self, Output = Self>
    + BitXorAssign<Self>
    + Shl<Self, Output = Self>
    + for<'a> Shl<&'a Self, Output = Self>
    + Shl<usize, Output = Self>
    + ShlAssign<Self>
    + ShlAssign<usize>
    + Shr<Self, Output = Self>
    + for<'a> Shr<&'a Self, Output = Self>
    + ShrAssign<Self>
    + Shr<usize, Output = Self>
    + ShrAssign<usize>
{
}

impl<
    T: Sized
        + BitAnd<T, Output = T>
        + for<'a> BitAnd<&'a T, Output = T>
        + BitAndAssign<T>
        + BitOr<T, Output = T>
        + for<'a> BitOr<&'a T, Output = T>
        + BitOrAssign<T>
        + BitXor<T, Output = T>
        + for<'a> BitXor<&'a T, Output = T>
        + BitXorAssign<T>
        + Shl<T, Output = T>
        + for<'a> Shl<&'a T, Output = T>
        + Shl<usize, Output = T>
        + ShlAssign<T>
        + ShlAssign<usize>
        + Shr<T, Output = T>
        + for<'a> Shr<&'a T, Output = T>
        + Shr<usize, Output = T>
        + ShrAssign<T>
        + ShrAssign<usize>
> Bits for T
{
}
