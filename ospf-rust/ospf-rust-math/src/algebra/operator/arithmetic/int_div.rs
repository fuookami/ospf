use crate::algebra::*;

pub trait IntDiv<Rhs = Self> {
    type Output;

    fn int_div(self, rhs: Rhs) -> Self::Output;
}

fn int_div<T: IntDiv>(lhs: T, rhs: T) -> T::Output {
    lhs.int_div(rhs)
}

macro_rules! int_int_div_template {
    ($($type:ty)*) => ($(
        impl IntDiv for $type {
            type Output = $type;

            fn int_div(self, rhs: Self) -> Self::Output {
                return self / rhs
            }
        }
    )*)
}
int_int_div_template! { i8 i16 i32 i64 i128 IntX u8 u16 u32 u64 u128 UIntX }

macro_rules! floating_int_div_template {
    ($($type:ty)*) => ($(
        impl IntDiv for $type {
            type Output = $type;

            fn int_div(self, rhs: Self) -> Self::Output {
                return self - self % rhs
            }
        }
    )*)
}
floating_int_div_template! { f32 f64 Decimal }
