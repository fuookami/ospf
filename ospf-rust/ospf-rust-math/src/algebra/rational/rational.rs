use crate::algebra::*;
use std::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};

pub(self) trait RationalConstructor<I: Integer> {
    fn new(num: I, den: I) -> Self;
    fn assign(&mut self, rhs: Self);
}

#[derive(Clone, PartialEq, Eq)]
pub struct Rational<I: Integer> {
    num: I,
    den: I,
}

default impl<I: Integer> RationalConstructor<I> for Rational<I> {
    fn new(num: I, den: I) -> Self {
        let divisor = ordinary::gcd(num, den);
        Self {
            num: num / divisor,
            den: den / divisor,
        }
    }

    fn assign(&mut self, rhs: Self) {
        self.num = rhs.num;
        self.den = rhs.den;
    }
}

impl<I: Integer + Signed> RationalConstructor<I> for Rational<I> {
    fn new(num: I, den: I) -> Self {
        let divisor = ordinary::gcd(num, den);
        let negative = (num < I::ZERO) ^ (den < I::ZERO);
        if negative {
            Self {
                num: num.abs().neg() / divisor,
                den: den.abs() / divisor,
            }
        } else {
            Self {
                num: num.abs() / divisor,
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

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.num * rhs.den + rhs.num * self.den, self.den * rhs.den)
    }
}

impl<'a, I: Integer> Add<Rational<I>> for &'a Rational<I> {
    type Output = Rational<I>;

    fn add(self, rhs: Rational<I>) -> Self::Output {
        Self::new(self.num * rhs.den + rhs.num * self.den, self.den * rhs.den)
    }
}

impl<I: Integer> Add<&Rational<I>> for Rational<I> {
    type Output = Rational<I>;

    fn add(self, rhs: &Rational<I>) -> Self::Output {
        Self::new(self.num * rhs.den + rhs.num * self.den, self.den * rhs.den)
    }
}

impl<I: Integer> Add<&Rational<I>> for &Rational<I> {
    type Output = Rational<I>;

    fn add(self, rhs: &Rational<I>) -> Self::Output {
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

    fn sub(self, rhs: Rational<I>) -> Self::Output {
        Self::new(self.num * rhs.den - rhs.num * self.den, self.den * rhs.den)
    }
}

impl<'a, I: Integer> Sub<Rational<I>> for &'a Rational<I> {
    type Output = Rational<I>;

    fn sub(self, rhs: Rational<I>) -> Self::Output {
        Self::new(self.num * rhs.den - rhs.num * self.den, self.den * rhs.den)
    }
}

impl<I: Integer> Sub<&Rational<I>> for Rational<I> {
    type Output = Rational<I>;

    fn sub(self, rhs: &Rational<I>) -> Self::Output {
        Self::new(self.num * rhs.den - rhs.num * self.den, self.den * rhs.den)
    }
}

impl<I: Integer> Sub<&Rational<I>> for &Rational<I> {
    type Output = Rational<I>;

    fn sub(self, rhs: &Rational<I>) -> Self::Output {
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

    fn mul(self, rhs: Rational<I>) -> Self::Output {
        Self::new(self.num * rhs.num, self.den * rhs.den)
    }
}

impl<'a, I: Integer> Mul<Rational<I>> for &'a Rational<I> {
    type Output = Rational<I>;

    fn mul(self, rhs: Rational<I>) -> Self::Output {
        Self::new(self.num * rhs.num, self.den * rhs.den)
    }
}

impl<I: Integer> Mul<&Rational<I>> for Rational<I> {
    type Output = Rational<I>;

    fn mul(self, rhs: &Rational<I>) -> Self::Output {
        Self::new(self.num * rhs.num, self.den * rhs.den)
    }
}

impl<I: Integer> Div for Rational<I> {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self::new(self.num * rhs.den, rhs.num * rhs.den)
    }
}

impl<I: Integer> Abs for Rational<I> {
    type Output = Self;

    fn abs(&self) -> Self::Output {
        Self::new(self.num.abs(), self.den)
    }
}

impl<I: Integer> Cross for Rational<I> {
    type Output = Self;

    fn cross(self, rhs: Self) -> Self::Output {
        self.mul(rhs)
    }
}

impl<I: Integer> IntDiv for Rational<I> {
    type Output = Self;

    fn int_div(self, rhs: Self) -> Self::Output {
        self.div(rhs)
    }
}

impl<I: Integer> Log<f64> for Rational<I> {}

impl<I: Integer> Neg for Rational<I> {
    type Output = Self;

    fn neg(&self) -> Self::Output {
        Self::new(self.num.neg(), self.den)
    }
}

impl<I: Integer> Reciprocal for Rational<I> {
    type Output = Self;

    fn reciprocal(&self) -> Self::Output {
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
pub type RtnX = Rational<IntX>;

pub type URtn8 = Rational<u8>;
pub type URtn16 = Rational<u16>;
pub type URtn32 = Rational<u32>;
pub type URtn64 = Rational<u64>;
pub type URtn128 = Rational<u128>;
pub type URtnX = Rational<UIntX>;

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
        let lhs = IntX::from(self.num) * IntX::from(other.den);
        let rhs = IntX::from(other.num) * IntX::from(self.den);
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

impl From<&RtnX> for Decimal {
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
        let lhs = UIntX::from(self.num) * UIntX::from(other.den);
        let rhs = UIntX::from(other.num) * UIntX::from(self.den);
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

impl From<&URtnX> for Decimal {
    fn from(value: &URtnX) -> Self {
        (value.num.into()) / (value.den.into())
    }
}
