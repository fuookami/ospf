use std::ops::*;

pub trait Bits:
    Sized
    + BitAnd<Self, Output=Self>
    + for<'a> BitAnd<&'a Self, Output=Self>
    // + BitAnd<usize, Output=Self>
    + BitAndAssign<Self>
    + for<'a> BitAndAssign<&'a Self>
    // + BitAndAssign<usize>
    + BitOr<Self, Output=Self>
    + for<'a> BitOr<&'a Self, Output=Self>
    // + BitOr<usize, Output=Self>
    + BitOrAssign<Self>
    + for<'a> BitOrAssign<&'a Self>
    // + BitOrAssign<usize>
    + BitXor<Self, Output=Self>
    + for<'a> BitXor<&'a Self, Output=Self>
    // + BitXor<usize, Output=Self>
    + BitXorAssign<Self>
    + for<'a> BitXorAssign<&'a Self>
    // + BitXorAssign<usize>
    + Shl<Self, Output=Self>
    + for<'a> Shl<&'a Self, Output=Self>
    + Shl<usize, Output=Self>
    + ShlAssign<Self>
    + for<'a> ShlAssign<&'a Self>
    + ShlAssign<usize>
    + Shr<Self, Output=Self>
    + for<'a> Shr<&'a Self, Output=Self>
    + Shr<usize, Output=Self>
    + ShrAssign<Self>
    + for<'a> ShrAssign<&'a Self>
    + ShrAssign<usize> {}

impl<T:
    Sized
    + BitAnd<T, Output=T>
    + for<'a> BitAnd<&'a T, Output=T>
    // + BitAnd<usize, Output=T>
    + BitAndAssign<T>
    + for<'a> BitAndAssign<&'a T>
    // + BitAndAssign<usize>
    + BitOr<T, Output=T>
    + for<'a> BitOr<&'a T, Output=T>
    // + BitOr<usize, Output=T>
    + BitOrAssign<T>
    + for<'a> BitOrAssign<&'a T>
    // + BitOrAssign<usize>
    + BitXor<T, Output=T>
    + for<'a> BitXor<&'a T, Output=T>
    // + BitXor<usize, Output=T>
    + BitXorAssign<T>
    + for<'a> BitXorAssign<&'a T>
    // + BitXorAssign<usize>
    + Shl<T, Output=T>
    + for<'a> Shl<&'a T, Output=T>
    + Shl<usize, Output=T>
    + ShlAssign<T>
    + for<'a> ShlAssign<&'a T>
    + ShlAssign<usize>
    + Shr<T, Output=T>
    + for<'a> Shr<&'a T, Output=T>
    + Shr<usize, Output=T>
    + ShrAssign<T>
    + for<'a> ShrAssign<&'a T>
    + ShrAssign<usize>
> Bits for T {}
