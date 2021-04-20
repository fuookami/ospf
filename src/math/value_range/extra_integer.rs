use super::super::{Arithmetic, Integer, Real, Scalar, Signed};
use std::cmp::{Ordering, PartialEq, PartialOrd};
use std::convert::From;
use std::fmt;
use std::fmt::Display;
use std::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
};

#[derive(Debug, Clone, Copy)]
pub enum ExInteger<T: Integer> {
    NegInf,
    PosInf,
    NAN,
    Value(T),
}

impl<T: Integer> ExInteger<T> {
    fn to_real<U: Real>(&self) -> U
    where
        U: From<T>,
    {
        match self {
            ExInteger::NegInf => U::neg_infinity().unwrap(),
            ExInteger::PosInf => U::infinity().unwrap(),
            ExInteger::NAN => U::nan().unwrap(),
            ExInteger::Value(value) => U::from(value.clone()),
        }
    }

    fn to_integer<U: Integer>(&self) -> ExInteger<U>
    where
        U: From<T>,
    {
        match self {
            ExInteger::NegInf => ExInteger::<U>::NegInf,
            ExInteger::PosInf => ExInteger::<U>::PosInf,
            ExInteger::NAN => ExInteger::<U>::NAN,
            ExInteger::Value(value) => ExInteger::Value(U::from(value.clone())),
        }
    }
}

impl<T: Integer> Add for ExInteger<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        match self {
            ExInteger::NegInf => match rhs {
                ExInteger::PosInf => panic!("Undefined limit"),
                ExInteger::NAN => panic!("Nan"),
                _ => ExInteger::NegInf,
            },
            ExInteger::PosInf => match rhs {
                ExInteger::NegInf => panic!("Undefined limit"),
                ExInteger::NAN => panic!("Nan"),
                _ => ExInteger::PosInf,
            },
            ExInteger::NAN => panic!("Nan"),
            ExInteger::Value(value1) => match rhs {
                ExInteger::NegInf => ExInteger::NegInf,
                ExInteger::PosInf => ExInteger::PosInf,
                ExInteger::NAN => panic!("Nan"),
                ExInteger::Value(value2) => ExInteger::Value(value1 + value2),
            },
        }
    }
}

impl<T: Integer> Add<T> for ExInteger<T> {
    type Output = Self;

    fn add(self, rhs: T) -> Self {
        match self {
            ExInteger::NegInf => ExInteger::NegInf,
            ExInteger::PosInf => ExInteger::PosInf,
            ExInteger::NAN => panic!("Nan"),
            ExInteger::Value(value) => ExInteger::Value(value + rhs),
        }
    }
}

impl<T: Integer> AddAssign for ExInteger<T> {
    fn add_assign(&mut self, rhs: Self) {
        *self = match self {
            ExInteger::NegInf => match rhs {
                ExInteger::PosInf => panic!("Undefined limit"),
                ExInteger::NAN => panic!("Nan"),
                _ => ExInteger::<T>::NegInf,
            },
            ExInteger::PosInf => match rhs {
                ExInteger::NegInf => panic!("Undefined limit"),
                ExInteger::NAN => panic!("Nan"),
                _ => ExInteger::<T>::PosInf,
            },
            ExInteger::NAN => panic!("Nan"),
            ExInteger::Value(value1) => match rhs {
                ExInteger::NegInf => ExInteger::<T>::NegInf,
                ExInteger::PosInf => ExInteger::<T>::PosInf,
                ExInteger::NAN => panic!("Nan"),
                ExInteger::Value(value2) => ExInteger::Value(value1.clone() + value2),
            },
        };
    }
}

impl<T: Integer> AddAssign<T> for ExInteger<T> {
    fn add_assign(&mut self, rhs: T) {
        *self = match self {
            ExInteger::NegInf => ExInteger::NegInf,
            ExInteger::PosInf => ExInteger::PosInf,
            ExInteger::NAN => panic!("Nan"),
            ExInteger::Value(value) => ExInteger::Value(value.clone() + rhs),
        }
    }
}

impl<T: Integer> Sub for ExInteger<T> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        match self {
            ExInteger::NegInf => match rhs {
                ExInteger::NegInf => panic!("Undefined limit"),
                ExInteger::NAN => panic!("Nan"),
                _ => ExInteger::NegInf,
            },
            ExInteger::PosInf => match rhs {
                ExInteger::PosInf => panic!("Undefined limit"),
                ExInteger::NAN => panic!("Nan"),
                _ => ExInteger::PosInf,
            },
            ExInteger::NAN => panic!("Nan"),
            ExInteger::Value(value1) => match rhs {
                ExInteger::NegInf => ExInteger::PosInf,
                ExInteger::PosInf => ExInteger::NegInf,
                ExInteger::NAN => panic!("Nan"),
                ExInteger::Value(value2) => ExInteger::Value(value1 + value2),
            },
        }
    }
}

impl<T: Integer> Sub<T> for ExInteger<T> {
    type Output = Self;

    fn sub(self, rhs: T) -> Self {
        match self {
            ExInteger::NegInf => ExInteger::NegInf,
            ExInteger::PosInf => ExInteger::PosInf,
            ExInteger::NAN => panic!("Nan"),
            ExInteger::Value(value) => ExInteger::Value(value - rhs),
        }
    }
}

impl<T: Integer> SubAssign for ExInteger<T> {
    fn sub_assign(&mut self, rhs: Self) {
        *self = match self {
            ExInteger::NegInf => match rhs {
                ExInteger::NegInf => panic!("Undefined limit"),
                ExInteger::NAN => panic!("Nan"),
                _ => ExInteger::NegInf,
            },
            ExInteger::PosInf => match rhs {
                ExInteger::PosInf => panic!("Undefined limit"),
                ExInteger::NAN => panic!("Nan"),
                _ => ExInteger::PosInf,
            },
            ExInteger::NAN => panic!("Nan"),
            ExInteger::Value(value1) => match rhs {
                ExInteger::NegInf => ExInteger::PosInf,
                ExInteger::PosInf => ExInteger::NegInf,
                ExInteger::NAN => panic!("Nan"),
                ExInteger::Value(value2) => ExInteger::Value(value1.clone() + value2),
            },
        }
    }
}

impl<T: Integer> SubAssign<T> for ExInteger<T> {
    fn sub_assign(&mut self, rhs: T) {
        *self = match self {
            ExInteger::NegInf => ExInteger::NegInf,
            ExInteger::PosInf => ExInteger::PosInf,
            ExInteger::NAN => panic!("Nan"),
            ExInteger::Value(value) => ExInteger::Value(value.clone() - rhs),
        }
    }
}

impl<T: Integer> Mul for ExInteger<T> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        match self {
            ExInteger::NegInf => match rhs {
                ExInteger::NegInf => ExInteger::PosInf,
                ExInteger::PosInf => ExInteger::NegInf,
                ExInteger::NAN => panic!("Nan"),
                ExInteger::Value(value) => match value {
                    _ if value > T::zero() => ExInteger::NegInf,
                    _ if value < T::zero() => ExInteger::PosInf,
                    _ => ExInteger::Value(T::zero()),
                },
            },
            ExInteger::PosInf => match rhs {
                ExInteger::NegInf => ExInteger::PosInf,
                ExInteger::PosInf => ExInteger::NegInf,
                ExInteger::NAN => panic!("Nan"),
                ExInteger::Value(value) => match value {
                    _ if value > T::zero() => ExInteger::PosInf,
                    _ if value < T::zero() => ExInteger::NegInf,
                    _ => ExInteger::Value(T::zero()),
                },
            },
            ExInteger::NAN => panic!("Nan"),
            ExInteger::Value(value1) => match rhs {
                ExInteger::NegInf => match value1 {
                    _ if value1 > T::zero() => ExInteger::NegInf,
                    _ if value1 < T::zero() => ExInteger::PosInf,
                    _ => ExInteger::Value(T::zero()),
                },
                ExInteger::PosInf => match value1 {
                    _ if value1 > T::zero() => ExInteger::PosInf,
                    _ if value1 < T::zero() => ExInteger::NegInf,
                    _ => ExInteger::Value(T::zero()),
                },
                ExInteger::NAN => panic!("Nan"),
                ExInteger::Value(value2) => ExInteger::Value(value1 * value2),
            },
        }
    }
}

impl<T: Integer> Mul<T> for ExInteger<T> {
    type Output = Self;

    fn mul(self, rhs: T) -> Self {
        match self {
            ExInteger::NegInf => match rhs {
                _ if rhs > T::zero() => ExInteger::NegInf,
                _ if rhs < T::zero() => ExInteger::PosInf,
                _ => ExInteger::Value(T::zero()),
            },
            ExInteger::PosInf => match rhs {
                _ if rhs > T::zero() => ExInteger::PosInf,
                _ if rhs < T::zero() => ExInteger::NegInf,
                _ => ExInteger::Value(T::zero()),
            },
            ExInteger::NAN => panic!("Nan"),
            ExInteger::Value(value) => ExInteger::Value(value * rhs),
        }
    }
}

impl<T: Integer> MulAssign for ExInteger<T> {
    fn mul_assign(&mut self, rhs: Self) {
        *self = match self {
            ExInteger::NegInf => match rhs {
                ExInteger::NegInf => ExInteger::PosInf,
                ExInteger::PosInf => ExInteger::NegInf,
                ExInteger::NAN => panic!("Nan"),
                ExInteger::Value(value) => match value {
                    _ if value > T::zero() => ExInteger::NegInf,
                    _ if value < T::zero() => ExInteger::PosInf,
                    _ => ExInteger::NAN,
                },
            },
            ExInteger::PosInf => match rhs {
                ExInteger::NegInf => ExInteger::PosInf,
                ExInteger::PosInf => ExInteger::NegInf,
                ExInteger::NAN => panic!("Nan"),
                ExInteger::Value(value) => match value {
                    _ if value > T::zero() => ExInteger::PosInf,
                    _ if value < T::zero() => ExInteger::NegInf,
                    _ => ExInteger::NAN,
                },
            },
            ExInteger::NAN => panic!("Nan"),
            ExInteger::Value(value1) => match rhs {
                ExInteger::NegInf => match value1 {
                    _ if value1.clone() > T::zero() => ExInteger::NegInf,
                    _ if value1.clone() < T::zero() => ExInteger::PosInf,
                    _ => ExInteger::Value(T::zero()),
                },
                ExInteger::PosInf => match value1 {
                    _ if value1.clone() > T::zero() => ExInteger::PosInf,
                    _ if value1.clone() < T::zero() => ExInteger::NegInf,
                    _ => ExInteger::Value(T::zero()),
                },
                ExInteger::NAN => panic!("Nan"),
                ExInteger::Value(value2) => ExInteger::Value(value1.clone() * value2),
            },
        }
    }
}

impl<T: Integer> MulAssign<T> for ExInteger<T> {
    fn mul_assign(&mut self, rhs: T) {
        *self = match self {
            ExInteger::NegInf => match rhs {
                _ if rhs > T::zero() => ExInteger::NegInf,
                _ if rhs < T::zero() => ExInteger::PosInf,
                _ => ExInteger::NAN,
            },
            ExInteger::PosInf => match rhs {
                _ if rhs > T::zero() => ExInteger::PosInf,
                _ if rhs < T::zero() => ExInteger::NegInf,
                _ => ExInteger::NAN,
            },
            ExInteger::NAN => panic!("Nan"),
            ExInteger::Value(value) => ExInteger::Value(value.clone() * rhs),
        }
    }
}

impl<T: Integer> Div for ExInteger<T> {
    type Output = Self;

    fn div(self, rhs: Self) -> Self {
        match self {
            ExInteger::NegInf => match rhs {
                ExInteger::NegInf | ExInteger::PosInf => panic!("Undefined limit"),
                ExInteger::NAN => panic!("Nan"),
                ExInteger::Value(value) => match value {
                    _ if value > T::zero() => ExInteger::NegInf,
                    _ if value < T::zero() => ExInteger::PosInf,
                    _ => ExInteger::NAN,
                },
            },
            ExInteger::PosInf => match rhs {
                ExInteger::NegInf | ExInteger::PosInf => panic!("Undefined limit"),
                ExInteger::NAN => panic!("Nan"),
                ExInteger::Value(value) => match value {
                    _ if value > T::zero() => ExInteger::PosInf,
                    _ if value < T::zero() => ExInteger::NegInf,
                    _ => ExInteger::NAN,
                },
            },
            ExInteger::NAN => panic!("Nan"),
            ExInteger::Value(value1) => match rhs {
                ExInteger::NegInf | ExInteger::PosInf => ExInteger::Value(T::zero()),
                ExInteger::NAN => panic!("Nan"),
                ExInteger::Value(value2) => match value2 {
                    _ if value2 == T::zero() => ExInteger::NAN,
                    value2 => ExInteger::Value(value1 / value2),
                },
            },
        }
    }
}

impl<T: Integer> Div<T> for ExInteger<T> {
    type Output = Self;

    fn div(self, rhs: T) -> Self {
        match self {
            ExInteger::NegInf => match rhs {
                _ if rhs > T::zero() => ExInteger::NegInf,
                _ if rhs < T::zero() => ExInteger::PosInf,
                _ => ExInteger::NAN,
            },
            ExInteger::PosInf => match rhs {
                _ if rhs > T::zero() => ExInteger::PosInf,
                _ if rhs < T::zero() => ExInteger::NegInf,
                _ => ExInteger::NAN,
            },
            ExInteger::NAN => panic!("Nan"),
            ExInteger::Value(value) => match rhs {
                _ if rhs == T::zero() => ExInteger::NAN,
                rhs => ExInteger::Value(value / rhs),
            },
        }
    }
}

impl<T: Integer> DivAssign for ExInteger<T> {
    fn div_assign(&mut self, rhs: Self) {
        *self = match self {
            ExInteger::NegInf => match rhs {
                ExInteger::NegInf | ExInteger::PosInf => panic!("Undefined limit"),
                ExInteger::NAN => panic!("Nan"),
                ExInteger::Value(value) => match value {
                    _ if value > T::zero() => ExInteger::NegInf,
                    _ if value < T::zero() => ExInteger::PosInf,
                    _ => ExInteger::NAN,
                },
            },
            ExInteger::PosInf => match rhs {
                ExInteger::NegInf | ExInteger::PosInf => panic!("Undefined limit"),
                ExInteger::NAN => panic!("Nan"),
                ExInteger::Value(value) => match value {
                    _ if value > T::zero() => ExInteger::PosInf,
                    _ if value < T::zero() => ExInteger::NegInf,
                    _ => ExInteger::NAN,
                },
            },
            ExInteger::NAN => panic!("Nan"),
            ExInteger::Value(value1) => match rhs {
                ExInteger::NegInf | ExInteger::PosInf => ExInteger::Value(T::zero()),
                ExInteger::NAN => panic!("Nan"),
                ExInteger::Value(value2) => match value2 {
                    _ if value2 == T::zero() => ExInteger::NAN,
                    value2 => ExInteger::Value(value1.clone() / value2),
                },
            },
        }
    }
}

impl<T: Integer> DivAssign<T> for ExInteger<T> {
    fn div_assign(&mut self, rhs: T) {
        *self = match self {
            ExInteger::NegInf => match rhs {
                _ if rhs > T::zero() => ExInteger::NegInf,
                _ if rhs < T::zero() => ExInteger::PosInf,
                _ => ExInteger::NAN,
            },
            ExInteger::PosInf => match rhs {
                _ if rhs > T::zero() => ExInteger::PosInf,
                _ if rhs < T::zero() => ExInteger::NegInf,
                _ => ExInteger::NAN,
            },
            ExInteger::NAN => panic!("Nan"),
            ExInteger::Value(value) => match rhs {
                _ if rhs == T::zero() => ExInteger::NAN,
                rhs => ExInteger::Value(value.clone() / rhs),
            },
        }
    }
}

impl<T: Integer> Rem for ExInteger<T> {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self {
        match self {
            ExInteger::NegInf => match rhs {
                ExInteger::NegInf | ExInteger::PosInf => panic!("Undefined limit"),
                ExInteger::NAN => panic!("Nan"),
                ExInteger::Value(value) => match value {
                    _ if value > T::zero() => ExInteger::NegInf,
                    _ if value < T::zero() => ExInteger::PosInf,
                    _ => ExInteger::NAN,
                },
            },
            ExInteger::PosInf => match rhs {
                ExInteger::NegInf | ExInteger::PosInf => panic!("Undefined limit"),
                ExInteger::NAN => panic!("Nan"),
                ExInteger::Value(value) => match value {
                    _ if value > T::zero() => ExInteger::PosInf,
                    _ if value < T::zero() => ExInteger::NegInf,
                    _ => ExInteger::NAN,
                },
            },
            ExInteger::NAN => panic!("Nan"),
            ExInteger::Value(value1) => match rhs {
                ExInteger::NegInf | ExInteger::PosInf => ExInteger::Value(value1),
                ExInteger::NAN => panic!("Nan"),
                ExInteger::Value(value2) => match value2 {
                    _ if value2 == T::zero() => ExInteger::NAN,
                    value2 => ExInteger::Value(value1 % value2),
                },
            },
        }
    }
}

impl<T: Integer> Rem<T> for ExInteger<T> {
    type Output = Self;

    fn rem(self, rhs: T) -> Self {
        match self {
            ExInteger::NegInf => match rhs {
                _ if rhs > T::zero() => ExInteger::NegInf,
                _ if rhs < T::zero() => ExInteger::PosInf,
                _ => ExInteger::NAN,
            },
            ExInteger::PosInf => match rhs {
                _ if rhs > T::zero() => ExInteger::PosInf,
                _ if rhs < T::zero() => ExInteger::NegInf,
                _ => ExInteger::NAN,
            },
            ExInteger::NAN => panic!("Nan"),
            ExInteger::Value(value) => match rhs {
                _ if rhs == T::zero() => ExInteger::NAN,
                rhs => ExInteger::Value(value % rhs),
            },
        }
    }
}

impl<T: Integer> RemAssign for ExInteger<T> {
    fn rem_assign(&mut self, rhs: Self) {
        *self = match self {
            ExInteger::NegInf => match rhs {
                ExInteger::NegInf | ExInteger::PosInf => panic!("Undefined limit"),
                ExInteger::NAN => panic!("Nan"),
                ExInteger::Value(value) => match value {
                    _ if value > T::zero() => ExInteger::NegInf,
                    _ if value < T::zero() => ExInteger::PosInf,
                    _ => ExInteger::NAN,
                },
            },
            ExInteger::PosInf => match rhs {
                ExInteger::NegInf | ExInteger::PosInf => panic!("Undefined limit"),
                ExInteger::NAN => panic!("Nan"),
                ExInteger::Value(value) => match value {
                    _ if value > T::zero() => ExInteger::PosInf,
                    _ if value < T::zero() => ExInteger::NegInf,
                    _ => ExInteger::NAN,
                },
            },
            ExInteger::NAN => panic!("Nan"),
            ExInteger::Value(value1) => match rhs {
                ExInteger::NegInf | ExInteger::PosInf => ExInteger::Value(value1.clone()),
                ExInteger::NAN => panic!("Nan"),
                ExInteger::Value(value2) => match value2 {
                    _ if value2 == T::zero() => ExInteger::NAN,
                    value2 => ExInteger::Value(value1.clone() % value2),
                },
            },
        }
    }
}

impl<T: Integer> RemAssign<T> for ExInteger<T> {
    fn rem_assign(&mut self, rhs: T) {
        *self = match self {
            ExInteger::NegInf => match rhs {
                _ if rhs > T::zero() => ExInteger::NegInf,
                _ if rhs < T::zero() => ExInteger::PosInf,
                _ => ExInteger::NAN,
            },
            ExInteger::PosInf => match rhs {
                _ if rhs > T::zero() => ExInteger::PosInf,
                _ if rhs < T::zero() => ExInteger::NegInf,
                _ => ExInteger::NAN,
            },
            ExInteger::NAN => panic!("Nan"),
            ExInteger::Value(value) => match rhs {
                _ if rhs == T::zero() => ExInteger::NAN,
                rhs => ExInteger::Value(value.clone() % rhs),
            },
        }
    }
}

impl<T: Integer> PartialEq for ExInteger<T> {
    fn eq(&self, rhs: &Self) -> bool {
        match self {
            ExInteger::NegInf => match rhs {
                ExInteger::NegInf => true,
                _ => false,
            },
            ExInteger::PosInf => match rhs {
                ExInteger::PosInf => true,
                _ => false,
            },
            ExInteger::NAN => match rhs {
                ExInteger::NAN => true,
                _ => false,
            },
            ExInteger::Value(value1) => match rhs {
                ExInteger::Value(value2) => value1 == value2,
                _ => false,
            },
        }
    }
}

impl<T: Integer> PartialEq<T> for ExInteger<T> {
    fn eq(&self, rhs: &T) -> bool {
        match self {
            ExInteger::Value(value) => value == rhs,
            _ => false,
        }
    }
}

impl<T: Integer> PartialOrd for ExInteger<T> {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        match self {
            ExInteger::NegInf => match rhs {
                ExInteger::NegInf | ExInteger::NAN => Option::None,
                _ => Option::Some(Ordering::Less),
            },
            ExInteger::PosInf => match rhs {
                ExInteger::PosInf | ExInteger::NAN => Option::None,
                _ => Option::Some(Ordering::Greater),
            },
            ExInteger::NAN => Option::None,
            ExInteger::Value(value1) => match rhs {
                ExInteger::NegInf => Option::Some(Ordering::Greater),
                ExInteger::PosInf => Option::Some(Ordering::Less),
                ExInteger::NAN => Option::None,
                ExInteger::Value(value2) => value1.partial_cmp(value2),
            },
        }
    }
}

impl<T: Integer> PartialOrd<T> for ExInteger<T> {
    fn partial_cmp(&self, rhs: &T) -> Option<Ordering> {
        match self {
            ExInteger::NegInf => Option::Some(Ordering::Less),
            ExInteger::PosInf => Option::Some(Ordering::Greater),
            ExInteger::NAN => Option::None,
            ExInteger::Value(value) => value.partial_cmp(rhs),
        }
    }
}

impl<T: Integer + Signed> Neg for ExInteger<T> {
    type Output = Self;

    fn neg(self) -> Self {
        match self {
            ExInteger::NegInf => ExInteger::PosInf,
            ExInteger::PosInf => ExInteger::NegInf,
            ExInteger::NAN => panic!("Nan"),
            ExInteger::Value(value) => ExInteger::Value(-value),
        }
    }
}

impl<T: Integer> Arithmetic for ExInteger<T> {
    fn zero() -> Self {
        ExInteger::Value(T::zero())
    }

    fn minimum() -> Self {
        ExInteger::<T>::NegInf
    }

    fn maximum() -> Self {
        ExInteger::<T>::PosInf
    }

    fn nan() -> Option<Self> {
        Option::Some(Self::NAN)
    }

    fn is_nan(&self) -> bool {
        match self {
            Self::NAN => true,
            _ => false,
        }
    }

    fn infinity() -> Option<Self> {
        Option::Some(Self::PosInf)
    }

    fn neg_infinity() -> Option<Self> {
        Option::Some(Self::NegInf)
    }

    fn is_infinity(&self) -> bool {
        match self {
            Self::NegInf | Self::PosInf => true,
            _ => false,
        }
    }

    fn is_normal(&self) -> bool {
        match self {
            Self::NegInf | Self::PosInf => false,
            Self::NAN => false,
            _ => true,
        }
    }
}

impl<T: Integer> Display for ExInteger<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExInteger::NegInf => write!(f, "-inf"),
            ExInteger::PosInf => write!(f, "inf"),
            ExInteger::NAN => panic!("Nan"),
            ExInteger::Value(value) => write!(f, "{}", value),
        }
    }
}

impl<T: Integer> Scalar for ExInteger<T> {}

impl<T: Integer> Integer for ExInteger<T> {}

impl<T: Integer + Signed> Signed for ExInteger<T> {}

impl<T: Integer> From<T> for ExInteger<T> {
    fn from(value: T) -> Self {
        ExInteger::Value(value.clone())
    }
}

impl<T: Integer> From<&T> for ExInteger<T> {
    fn from(value: &T) -> Self {
        ExInteger::Value(value.clone())
    }
}

#[test]
fn test_ex_integer_add() {
    let mut v1 = ExInteger::<i128>::Value(3);
    let v2 = v1 + ExInteger::NegInf;
    assert!(v2 == ExInteger::NegInf);
    let v3 = v1 + ExInteger::PosInf;
    assert!(v3 == ExInteger::PosInf);
    let v4 = v1 + 3;
    assert!(v4 == 6);
    v1 += 3;
    assert!(v1 == 6);
}

#[test]
fn test_ex_integer_sub() {
    let mut v1 = ExInteger::<i128>::Value(3);
    let v2 = v1 - ExInteger::NegInf;
    assert!(v2 == ExInteger::PosInf);
    let v3 = v1 - ExInteger::PosInf;
    assert!(v3 == ExInteger::NegInf);
    let v4 = v1 - 3;
    assert!(v4 == 0);
    v1 -= 3;
    assert!(v1 == 0);
}

#[test]
fn test_ex_integer_mul() {
    let mut v1 = ExInteger::<i128>::Value(3);
    let v2 = v1 * ExInteger::NegInf;
    assert!(v2 == ExInteger::NegInf);
    let v3 = v1 * ExInteger::PosInf;
    assert!(v3 == ExInteger::PosInf);
    let v4 = v1 * 3;
    assert!(v4 == 9);
    v1 *= 3;
    assert!(v1 == 9);
}

#[test]
fn test_ex_integer_div() {
    let mut v1 = ExInteger::<i128>::Value(3);
    let v2 = v1 / ExInteger::NegInf;
    assert!(v2 == 0);
    let v3 = v1 / ExInteger::PosInf;
    assert!(v3 == 0);
    let v4 = v1 / 3;
    assert!(v4 == 1);
    v1 /= 3;
    assert!(v1 == 1);
}
