use crate::algebra::*;

pub trait Log<Base: FloatingNumber = Self>: Sized {
    type Output;

    fn log(self, base: &Base) -> Option<Self::Output>;

    fn lg2(self) -> Option<Self::Output> {
        self.log(Base::TWO)
    }

    fn lg(self) -> Option<Self::Output> {
        self.log(Base::TEN)
    }

    fn ln(self) -> Option<Self::Output> {
        self.log(Base::E)
    }
}

pub fn log<Lhs: Log<Rhs>, Rhs: FloatingNumber>(lhs: Lhs, rhs: &Rhs) -> Option<Lhs::Output> {
    lhs.log(&rhs)
}

pub fn lg2<Lhs: Log<Rhs>, Rhs: FloatingNumber>(lhs: Lhs) -> Option<Lhs::Output> {
    lhs.lg2()
}

pub fn lg<Lhs: Log<Rhs>, Rhs: FloatingNumber>(lhs: Lhs) -> Option<Lhs::Output> {
    lhs.lg()
}

pub fn ln<Lhs: Log<Rhs>, Rhs: FloatingNumber>(lhs: Lhs) -> Option<Lhs::Output> {
    lhs.ln()
}

macro_rules! int_log_template {
    ($($type:ident)*) => ($(
        impl Log<f64> for $type {
            type Output = f64;

            fn log(self, base: &f64) -> Option<f64> {
                Some((self as f64).log(*base))
            }
        }


        impl Log<f64> for &$type {
            type Output = f64;

            fn log(self, base: &f64) -> Option<f64> {
                Some((*self as f64).log(*base))
            }
        }
    )*)
}
int_log_template! { i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }

macro_rules! floating_log_template {
    ($($type:ident)*) => ($(
        impl Log for $type {
            type Output = $type;

            fn log(self, base: &Self) -> Option<$type> {
                Some(self.log(*base))
            }
        }

        impl Log<$type> for &$type {
            type Output = $type;

            fn log(self, base: &$type) -> Option<$type> {
                Some((*self).log(*base))
            }
        }
    )*);
}
floating_log_template! { f32 f64 }

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use crate::algebra::concept::{Integer, FloatingNumber};

    use super::*;

    fn test_int<T: Integer + Log<f64, Output = f64> + Debug>()
    where
            for<'a> &'a T: Log<f64, Output = f64>
    {

    }

    fn test_flt<T: FloatingNumber + Log<T, Output = T> + Debug>()
    where
            for<'a> &'a T: Log<T, Output = T>
    {
        assert_eq!(&T::TWO.lg2().unwrap(), T::ONE);
    }

    #[test]
    fn test() {
        test_int::<u8>();
        test_int::<u16>();
        test_int::<u32>();
        test_int::<u64>();
        test_int::<u128>();
        test_int::<i8>();
        test_int::<i16>();
        test_int::<i32>();
        test_int::<i64>();
        test_int::<i128>();
        test_flt::<f32>();
        test_flt::<f64>();
    }
}
