use std::ops::*;

use num::*;

use crate::algebra::concept::*;
use crate::algebra::numeric_integer::*;
use crate::algebra::operator::*;
use crate::algebra::ordinary::*;

use super::dec;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ix {
    pub(crate) value: BigInt,
}

impl<T> From<T> for ix
where
    BigInt: From<T>,
{
    fn from(value: T) -> Self {
        ix {
            value: BigInt::from(value),
        }
    }
}

impl Bounded for ix {
    const MINIMUM: Option<Self> = None;
    const MAXIMUM: Option<Self> = None;
    const POSITIVE_MINIMUM: Self = Self::ONE;
}

impl Precision for ix {
    const EPSILON: Self = Self::ZERO;
    const DECIMAL_DIGITS: Option<usize> = None;
    const DECIMAL_PRECISION: Self = Self::EPSILON;
}

impl Invariant for ix {
    type ValueType = Self;

    fn value(&self) -> &Self::ValueType {
        &self
    }
}

impl Signed for ix {}

impl Arithmetic for ix {
    const ZERO: Self = ix::from(0);
    const ONE: Self = ix::from(1);
}

impl Add for ix {
    type Output = ix;

    fn add(self, other: ix) -> ix {
        ix {
            value: self.value + other.value,
        }
    }
}

impl AddAssign for ix {
    fn add_assign(&mut self, rhs: ix) {
        self.value += rhs.value;
    }
}

impl Sub for ix {
    type Output = ix;

    fn sub(self, other: ix) -> ix {
        ix {
            value: self.value - other.value,
        }
    }
}

impl SubAssign for ix {
    fn sub_assign(&mut self, rhs: ix) {
        self.value -= rhs.value;
    }
}

impl Neg for ix {
    type Output = ix;

    fn sub(self, other: ix) -> ix {
        ix { value: -self.value }
    }
}

impl Mul for ix {
    type Output = ix;

    fn mul(self, other: ix) -> ix {
        ix {
            value: self.value * other.value,
        }
    }
}

impl MulAssign for ix {
    fn mul_assign(&mut self, rhs: ix) {
        self.value *= rhs.value;
    }
}

impl Div for ix {
    type Output = ix;

    fn div(self, other: ix) -> ix {
        ix {
            value: self.value / other.value,
        }
    }
}

impl DivAssign for ix {
    fn div_assign(&mut self, rhs: ix) {
        self.value /= rhs.value;
    }
}

impl Pow for ix {
    type Output = ix;

    fn pow(self, index: i64) -> ix {
        pow_times_semi_group(self, index).unwrap()
    }
}

impl PowF<dec> for ix {
    type Output = dec;

    fn powf(self, index: dec) -> Option<dec> {
        dec::from(self).powf(index)
    }

    fn sqr(self) -> Option<dec> {
        dec::from(self).sqr()
    }

    fn cbr(self) -> Option<dec> {
        dec::from(self).cbr()
    }
}

impl Exp for ix {
    type Output = dec;

    fn exp(self) -> dec {
        dec::from(self).exp()
    }
}

impl PlusSemiGroup for ix {}

impl TimesSemiGroup for ix {}

impl Scalar for ix {}

impl RealNumber for ix {
    const TWO: Self = ix::from(2);
    const THREE: Self = ix::from(3);
    const TEN: Self = ix::from(10);
}

impl Integer for ix {}

impl PlusGroup for ix {}

impl TimesGroup for ix {}

impl NumberRing for ix {}

impl NumberField for ix {}

impl IntegerNumber for ix {}
