use std::mem::swap;
use std::ops::{BitOr, SubAssign};

use paste::paste;

use crate::algebra::concept::{Arithmetic, Bits, SemiArithmetic};
use crate::algebra::operator::TrailingZeros;

pub fn gcd_stein<T: SemiArithmetic + Bits + TrailingZeros + for<'a> SubAssign<&'a T>>(
    mut x: T,
    mut y: T,
) -> T
where
    for<'a> &'a T: PartialEq + BitOr<Output = T>,
{
    debug_assert!(&x >= T::ZERO);
    debug_assert!(&y >= T::ZERO);

    if &x == T::ZERO {
        return y;
    }
    if &y == T::ZERO {
        return x;
    }

    let shift = T::trailing_zeros(&x | &y);
    x >>= shift;
    y >>= shift;
    x >>= T::trailing_zeros(x.clone());

    loop {
        y >>= T::trailing_zeros(y.clone());
        if x > y {
            swap(&mut x, &mut y);
        }
        y -= &x;
        if &y == T::ZERO {
            break;
        }
    }
    x << shift
}

pub fn gcd_euclid<T: SemiArithmetic + for<'a> SubAssign<&'a T>>(mut x: T, mut y: T) -> T
where
    for<'a> &'a T: PartialEq,
{
    debug_assert!(&x >= T::ZERO);
    debug_assert!(&y >= T::ZERO);

    if &x == T::ZERO {
        return y;
    }
    if &y == T::ZERO {
        return x;
    }

    loop {
        if x > y {
            swap(&mut x, &mut y);
        }
        y -= &x;
        if &y == T::ZERO {
            break;
        }
    }
    x
}

macro_rules! gcd_stein_template {
    ($($type:ident)*) => ($(
        paste! {
            pub fn [<gcd_stein_ $type>](mut x: $type, mut y: $type) -> $type {
                debug_assert!(&x >= $type::ZERO);
                debug_assert!(&y >= $type::ZERO);

                if &x == $type::ZERO {
                    return y;
                }
                if &y == $type::ZERO {
                    return x;
                }

                let shift = $type::trailing_zeros(&x | &y);
                x >>= shift;
                y >>= shift;
                x >>= $type::trailing_zeros(x.clone());

                loop {
                    y >>= $type::trailing_zeros(y.clone());
                    if x > y {
                        swap(&mut x, &mut y);
                    }
                    y -= &x;
                    if &y == $type::ZERO {
                        break;
                    }
                }
                x << shift
            }
        }
    )*)
}
gcd_stein_template! { u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize }

macro_rules! gcd_euclid_template {
    ($($type:ident)*) => ($(
        paste! {
            pub fn [<gcd_euclid_ $type>](mut x: $type, mut y: $type) -> $type {
                debug_assert!(&x >= $type::ZERO);
                debug_assert!(&y >= $type::ZERO);

                if &x == $type::ZERO {
                    return y;
                }
                if &y == $type::ZERO {
                    return x;
                }

                loop {
                    if x > y {
                        swap(&mut x, &mut y);
                    }
                    y -= &x;
                    if &y == $type::ZERO {
                        break;
                    }
                }
                x
            }
        }
    )*)
}
gcd_euclid_template! { u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize f32 f64 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stein() {
        assert_eq!(gcd_stein_i32(4, 6), 2);
        assert_eq!(gcd_stein_i32(6, 9), 3);
        assert_eq!(gcd_stein_i32(24, 30), 6);
    }

    #[test]
    fn test_euclid() {
        assert_eq!(gcd_euclid_i32(4, 6), 2);
        assert_eq!(gcd_euclid_i32(6, 9), 3);
        assert_eq!(gcd_euclid_i32(24, 30), 6);
    }
}
