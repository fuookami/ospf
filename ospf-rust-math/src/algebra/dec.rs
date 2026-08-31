use std::ops::*;

use bigdecimal::*;

use crate::algebra::concept::*;
use crate::algebra::numeric_integer::*;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dec {
    value: BigDecimal,
}

impl<T> From<T> for Dec
where
    BigDecimal: From<T>,
{
    fn from(value: T) -> Self {
        Dec {
            value: BigDecimal::from(value),
        }
    }
}

impl From<f32> for Dec {
    fn from(value: f32) -> Self {
        Dec {
            value: BigDecimal::from_f32(value).unwrap(),
        }
    }
}

impl From<f64> for Dec {
    fn from(value: f64) -> Self {
        Dec {
            value: BigDecimal::from_f64(value).unwrap(),
        }
    }
}

impl From<ix> for Dec {
    fn from(value: ix) -> Self {
        Dec {
            value: BigDecimal::from(value.value).unwrap(),
        }
    }
}

impl From<uix> for Dec {
    fn from(value: uix) -> Self {
        Dec {
            value: BigDecimal::from(value.value).unwrap(),
        }
    }
}

impl Add for Dec {
    type Output = Dec;

    fn add(self, other: Self) -> Dec {
        Dec {
            value: self.value + other.value,
        }
    }
}

impl Sub for Dec {
    type Output = Dec;

    fn sub(self, other: Self) -> Dec {
        Dec {
            value: self.value - other.value,
        }
    }
}

impl Mul for Dec {
    type Output = Dec;

    fn mul(self, other: Self) -> Dec {
        Dec {
            value: self.value * other.value,
        }
    }
}

impl Div for Dec {
    type Output = Dec;

    fn div(self, other: Self) -> Dec {
        Dec {
            value: self.value / other.value,
        }
    }
}

impl Reciprocal for Dec {
    type Output = Dec;

    fn reciprocal(&self) -> Dec {
        Dec::ONE / self
    }
}

impl Pow for Dec {
    type Output = Dec;

    fn pow(self, index: i64) -> Dec {
        ordinary::pow_times_group(self, index)
    }
}

impl PowF for Dec {
    type Output = Dec;

    fn powf(self, index: Dec) -> Option<Dec> {
        ordinary::powf(self, index)
    }

    fn sqr(self) -> Option<Dec> {
        self.powf(Self::ONE / Self::TWO)
    }

    fn cbr(self) -> Option<Dec> {
        self.powf(Self::ONE / Self::THREE)
    }
}

impl Exp for Dec {
    type Output = Dec;

    fn exp(self) -> Dec {
        ordinary::exp(self)
    }
}

impl Bounded for Dec {
    const MINIMUM: Option<Self> = None;
    const MAXIMUM: Option<Self> = None;
    const POSITIVE_MINIMUM: Self = Dec::from(1e-28);
}

impl Arithmetic for Dec {
    const ZERO: Self = Dec::ZERO;
    const ONE: Self = Dec::ONE;
}

impl Precision for Dec {
    const EPSILON: Self = Dec::from(1e-28);
    const DECIMAL_DIGITS: Option<usize> = Some(28);
    const DECIMAL_PRECISION: Self = Dec::from(1e-28);
}

impl Scalar for Dec {}

impl RealNumber for Dec {
    const TWO: Self = Dec::from(2);
    const THREE: Self = Dec::from(3);
    const TEN: Self = Dec::from(10);

    const NAN: Option<Self> = None;
    const INF: Option<Self> = None;
    const NEG_INF: Option<Self> = None;
}

impl FloatingNumber for Dec {
    const PI: Self = Dec::PI;
    const E: Self = Dec::E;

    fn floor(&self) -> Self {
        self.floor()
    }

    fn ceil(&self) -> Self {
        self.ceil()
    }

    fn round(&self) -> Self {
        self.round()
    }

    fn trunc(&self) -> Self {
        self.trunc()
    }

    fn fract(&self) -> Self {
        self.fract()
    }
}
