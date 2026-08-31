use std::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};

use crate::algebra::*;

pub(self) trait RationalConstructor<I: Integer> {
    fn new(num: I, den: I) -> Self;
}

#[derive(Clone, PartialEq, Eq)]
pub struct Rational<I: Integer + NumberField + Pow> {
    num: I,
    den: I,
}

default impl<I: Integer + NumberField + Pow> RationalConstructor<I> for Rational<I> {
    fn new(num: I, den: I) -> Self {
        let divisor = ordinary::gcd(num.clone(), den.clone());
        Self {
            num: num / divisor.clone(),
            den: den / divisor,
        }
    }
}

impl<I: Integer + Signed + NumberField + Pow> RationalConstructor<I> for Rational<I> {
    fn new(num: I, den: I) -> Self {
        let divisor = ordinary::gcd(num.clone(), den.clone());
        let negative = (num < I::ZERO) ^ (den < I::ZERO);
        if negative {
            Self {
                num: num.abs().neg() / divisor.clone(),
                den: den.abs() / divisor,
            }
        } else {
            Self {
                num: num.abs() / divisor.clone(),
                den: den.abs() / divisor,
            }
        }
    }
}

impl<I: Integer + Copy> Copy for Rational<I> {}

impl<I: Integer> Ord for Rational<I>
where
    Rational<I>: PartialOrd,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl<I: Integer> Add<Rational<I>> for Rational<I> {
    type Output = Rational<I>;

    fn add(self, rhs: Self) -> Rational<I> {
        Self::new(
            self.num.clone() * rhs.den.clone() + rhs.num.clone() * self.den.clone(),
            self.den.clone() * rhs.den.clone(),
        )
    }
}

impl<'a, I: Integer> Add<Rational<I>> for &'a Rational<I>
where
    &I: Add<Output = I>,
    &I: Mul<Output = I>,
{
    type Output = Rational<I>;

    fn add(self, rhs: Rational<I>) -> Rational<I> {
        Self::new(
            &self.num * &rhs.den + &rhs.num * &self.den,
            &self.den * &rhs.den,
        )
    }
}

impl<I: Integer> Add<&Rational<I>> for Rational<I> {
    type Output = Rational<I>;

    fn add(self, rhs: &Rational<I>) -> Rational<I> {
        Self::new(self.num * rhs.den + rhs.num * self.den, self.den * rhs.den)
    }
}

impl<I: Integer> Add<&Rational<I>> for &Rational<I> {
    type Output = Rational<I>;

    fn add(self, rhs: &Rational<I>) -> Rational<I> {
        Self::new(self.num * rhs.den + rhs.num * self.den, self.den * rhs.den)
    }
}

impl<I: Integer> AddAssign<Self> for Rational<I> {
    fn add_assign(&mut self, rhs: Self) {
        self.assign(self.add(&rhs))
    }
}

impl<I: Integer> AddAssign<&Self> for Rational<I> {
    fn add_assign(&mut self, rhs: &Self) {
        self.assign(self.add(rhs))
    }
}

impl<I: Integer> Sub<Rational<I>> for Rational<I> {
    type Output = Rational<I>;

    fn sub(self, rhs: Rational<I>) -> Rational<I> {
        Self::new(self.num * rhs.den - rhs.num * self.den, self.den * rhs.den)
    }
}

impl<'a, I: Integer> Sub<Rational<I>> for &'a Rational<I> {
    type Output = Rational<I>;

    fn sub(self, rhs: Rational<I>) -> Rational<I> {
        Self::new(self.num * rhs.den - rhs.num * self.den, self.den * rhs.den)
    }
}

impl<I: Integer> Sub<&Rational<I>> for Rational<I> {
    type Output = Rational<I>;

    fn sub(self, rhs: &Rational<I>) -> Rational<I> {
        Self::new(self.num * rhs.den - rhs.num * self.den, self.den * rhs.den)
    }
}

impl<I: Integer> Sub<&Rational<I>> for &Rational<I> {
    type Output = Rational<I>;

    fn sub(self, rhs: &Rational<I>) -> Rational<I> {
        Self::new(self.num * rhs.den - rhs.num * self.den, self.den * rhs.den)
    }
}

impl<I: Integer> SubAssign<Rational<I>> for Rational<I> {
    fn sub_assign(&mut self, rhs: Rational<I>) {
        self.assign(self.sub(&rhs))
    }
}

impl<I: Integer> SubAssign<&Rational<I>> for Rational<I> {
    fn sub_assign(&mut self, rhs: &Rational<I>) {
        self.assign(self.sub(&rhs))
    }
}

impl<I: Integer> Mul<Rational<I>> for Rational<I> {
    type Output = Rational<I>;

    fn mul(self, rhs: Rational<I>) -> Rational<I> {
        Self::new(self.num * rhs.num, self.den * rhs.den)
    }
}

impl<'a, I: Integer> Mul<Rational<I>> for &'a Rational<I> {
    type Output = Rational<I>;

    fn mul(self, rhs: Rational<I>) -> Rational<I> {
        Self::new(self.num * rhs.num, self.den * rhs.den)
    }
}

impl<I: Integer> Mul<&Rational<I>> for Rational<I> {
    type Output = Rational<I>;

    fn mul(self, rhs: &Rational<I>) -> Rational<I> {
        Self::new(self.num * rhs.num, self.den * rhs.den)
    }
}

impl<I: Integer> Div for Rational<I> {
    type Output = Rational<I>;

    fn div(self, rhs: Self) -> Rational<I> {
        Self::new(self.num * rhs.den, rhs.num * rhs.den)
    }
}

impl<I: Integer> Abs for Rational<I> {
    type Output = Rational<I>;

    fn abs(&self) -> Rational<I> {
        Self::new(self.num.abs(), self.den)
    }
}

impl<I: Integer> Cross for Rational<I> {
    type Output = Rational<I>;

    fn cross(self, rhs: Self) -> Rational<I> {
        self.mul(rhs)
    }
}

impl<I: Integer> IntDiv for Rational<I> {
    type Output = Rational<I>;

    fn int_div(self, rhs: Self) -> Rational<I> {
        self.div(rhs)
    }
}

impl<I: Integer> Log<f64> for Rational<I> {}

impl<I: Integer> Neg for Rational<I> {
    type Output = Rational<I>;

    fn neg(&self) -> Rational<I> {
        Self::new(self.num.neg(), self.den)
    }
}

impl<I: Integer> Reciprocal for Rational<I> {
    type Output = Rational<I>;

    fn reciprocal(&self) -> Rational<I> {
        Self {
            num: self.den,
            den: self.num,
        }
    }
}

impl<I: Integer + Signed> Signed for Rational<I> {}

impl<I: Integer + Unsigned> Unsigned for Rational<I> {}

impl<I: Integer> Invariant for Rational<I>
where
    Rational<I>: PartialOrd,
{
    type ValueType = Self;

    fn value(&self) -> &Self::ValueType {
        self
    }
}

impl<I: Integer> Bounded for Rational<I> {
    const MINIMUM: Option<Self> = Some(Self::new(I::MINIMUM, I::one));
    const MAXIMUM: Option<Self> = Some(Self::new(I::MAXIMUM, I::ONE));
    const POSITIVE_MINIMUM: Self = Self::new(I::ONE, I::MAXIMUM);
}

impl<I: Integer> Arithmetic for Rational<I>
where
    Rational<I>: PartialOrd,
{
    const ZERO: Self = Self::new(I::ZERO, I::ONE);
    const ONE: Self = Self::new(I::ONE, I::ONE);
}

impl<I: Integer> Scalar for Rational<I> where Rational<I>: PartialOrd {}

impl<I: Integer> RealNumber for Rational<I> where Rational<I>: PartialOrd + Precision {}

impl<I: Integer> RationalNumber<I> for Rational<I> where Rational<I>: PartialOrd + Precision {}

pub type Rtn8 = Rational<i8>;
pub type Rtn16 = Rational<i16>;
pub type Rtn32 = Rational<i32>;
pub type Rtn64 = Rational<i64>;
pub type Rtn128 = Rational<i128>;
pub type RtnX = Rational<ix>;

pub type URtn8 = Rational<u8>;
pub type URtn16 = Rational<u16>;
pub type URtn32 = Rational<u32>;
pub type URtn64 = Rational<u64>;
pub type URtn128 = Rational<u128>;
pub type URtnX = Rational<uix>;

impl PartialOrd for Rtn8 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let lhs = (self.num as i16) * (other.den as i16);
        let rhs = (other.num as i16) * (self.den as i16);
        Some(lhs.cmp(&rhs))
    }
}

impl From<&Rtn8> for f64 {
    fn from(value: &Rtn8) -> Self {
        (value.num.into()) / (value.den.into())
    }
}

impl PartialOrd for Rtn16 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let lhs = (self.num as i32) * (other.den as i32);
        let rhs = (other.num as i32) * (self.den as i32);
        Some(lhs.cmp(&rhs))
    }
}

impl From<&Rtn16> for f64 {
    fn from(value: &Rtn16) -> Self {
        (value.num.into()) / (value.den.into())
    }
}

impl PartialOrd for Rtn32 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let lhs = (self.num as i64) * (other.den as i64);
        let rhs = (other.num as i64) * (self.den as i64);
        Some(lhs.cmp(&rhs))
    }
}

impl From<&Rtn32> for f64 {
    fn from(value: &Rtn32) -> Self {
        (value.num.into()) / (value.den.into())
    }
}

impl PartialOrd for Rtn64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let lhs = (self.num as i128) * (other.den as i128);
        let rhs = (other.num as i128) * (self.den as i128);
        Some(lhs.cmp(&rhs))
    }
}

impl From<&Rtn64> for f64 {
    fn from(value: &Rtn64) -> Self {
        (value.num.into()) / (value.den.into())
    }
}

impl PartialOrd for Rtn128 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let lhs = ix::from(self.num) * ix::from(other.den);
        let rhs = ix::from(other.num) * ix::from(self.den);
        Some(lhs.cmp(&rhs))
    }
}

impl From<&Rtn128> for f64 {
    fn from(value: &Rtn128) -> Self {
        (value.num.into()) / (value.den.into())
    }
}

impl PartialOrd for RtnX {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let lhs = self.num * other.den;
        let rhs = other.num * self.den;
        Some(lhs.cmp(&rhs))
    }
}

impl From<&RtnX> for dec {
    fn from(value: &RtnX) -> Self {
        (value.num.into()) / (value.den.into())
    }
}

impl PartialOrd for URtn8 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let lhs = (self.num as u16) * (other.den as u16);
        let rhs = (other.num as u16) * (self.den as u16);
        Some(lhs.cmp(&rhs))
    }
}

impl From<&URtn8> for f64 {
    fn from(value: &URtn8) -> Self {
        (value.num.into()) / (value.den.into())
    }
}

impl PartialOrd for URtn16 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let lhs = (self.num as u32) * (other.den as u32);
        let rhs = (other.num as u32) * (self.den as u32);
        Some(lhs.cmp(&rhs))
    }
}

impl From<&URtn16> for f64 {
    fn from(value: &URtn16) -> Self {
        (value.num.into()) / (value.den.into())
    }
}

impl PartialOrd for URtn32 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let lhs = (self.num as u64) * (other.den as u64);
        let rhs = (other.num as u64) * (self.den as u64);
        Some(lhs.cmp(&rhs))
    }
}

impl From<&URtn32> for f64 {
    fn from(value: &URtn32) -> Self {
        (value.num.into()) / (value.den.into())
    }
}

impl PartialOrd for URtn64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let lhs = (self.num as u128) * (other.den as u128);
        let rhs = (other.num as u128) * (self.den as u128);
        Some(lhs.cmp(&rhs))
    }
}

impl From<&URtn64> for f64 {
    fn from(value: &URtn64) -> Self {
        (value.num.into()) / (value.den.into())
    }
}

impl PartialOrd for URtn128 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let lhs = uix::from(self.num) * uix::from(other.den);
        let rhs = uix::from(other.num) * uix::from(self.den);
        Some(lhs.cmp(&rhs))
    }
}

impl From<&URtn128> for f64 {
    fn from(value: &URtn128) -> Self {
        (value.num.into()) / (value.den.into())
    }
}

impl PartialOrd for URtnX {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let lhs = self.num * other.den;
        let rhs = other.num * self.den;
        Some(lhs.cmp(&rhs))
    }
}

impl From<&URtnX> for dec {
    fn from(value: &URtnX) -> Self {
        (value.num.into()) / (value.den.into())
    }
}
