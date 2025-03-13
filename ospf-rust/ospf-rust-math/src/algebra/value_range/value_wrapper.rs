use super::*;
use crate::algebra::concept::*;
use std::ops::{Add, Div, Mul, Sub};

pub enum ValueWrapper<T: Arithmetic> {
    Value(T),
    Inf,
    NegInf,
}

impl<T: Arithmetic> From<Infinity> for ValueWrapper<T> {
    fn from(_: Infinity) -> Self {
        Self::Inf
    }
}

impl<T: Arithmetic> From<NegativeInfinity> for ValueWrapper<T> {
    fn from(_: NegativeInfinity) -> Self {
        Self::NegInf
    }
}

default impl<T: Arithmetic> From<T> for ValueWrapper<T> {
    fn from(value: T) -> Self {
        Self::Value(value)
    }
}

impl<T: RealNumber> From<T> for ValueWrapper<T> {
    fn from(value: T) -> Self {
        if value.is_inf() {
            Self::Inf
        } else if value.is_neg_inf() {
            Self::NegInf
        } else if value.is_nan() {
            panic!("Illegal argument NaN for value range!!!")
        } else {
            Self::Value(value)
        }
    }
}

impl<T: Arithmetic + Clone> Clone for ValueWrapper<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Value(value) => Ok(ValueWrapper::from(value.clone())),
            Self::Inf => Ok(ValueWrapper::Inf),
            Self::NegInf => Ok(ValueWrapper::NegInf),
        }
    }
}

impl<T: Arithmetic + Copy> Copy for ValueWrapper<T> {}

impl<T: Arithmetic + std::fmt::Display> std::fmt::Display for ValueWrapper<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Value(value) => write!(f, "{}", value),
            Self::Inf => write!(f, "inf"),
            Self::NegInf => write!(f, "-inf"),
        }
    }
}

impl<T: Arithmetic + std::fmt::Debug> std::fmt::Debug for ValueWrapper<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Value(value) => write!(f, "{}", value),
            Self::Inf => write!(f, "inf"),
            Self::NegInf => write!(f, "-inf"),
        }
    }
}

impl<T: Arithmetic> PartialEq for ValueWrapper<T> {
    fn eq(&self, rhs: &Self) -> bool {
        match self {
            ValueWrapper::Value(lhs_value) => match rhs {
                ValueWrapper::Value(rhs_value) => lhs_value == rhs_value,
                _ => false,
            },
            ValueWrapper::Inf => match rhs {
                ValueWrapper::Inf => true,
                _ => false,
            },
            ValueWrapper::NegInf => match rhs {
                ValueWrapper::NegInf => true,
                _ => false,
            },
        }
    }
}

impl<T: Arithmetic + Eq> Eq for ValueWrapper<T> {}

impl<T: Arithmetic> PartialEq<T> for ValueWrapper<T> {
    fn eq(&self, rhs: &T) -> bool {
        match self {
            ValueWrapper::Value(lhs_value) => lhs_value == rhs,
            _ => false,
        }
    }
}

impl<T: RealNumber> PartialEq<T> for ValueWrapper<T> {
    fn eq(&self, rhs: &T) -> bool {
        if rhs.is_nan() {
            false
        } else if rhs.is_inf() {
            self == ValueWrapper::Inf
        } else if rhs.is_neg_inf() {
            self == ValueWrapper::NegInf
        } else {
            if let ValueWrapper::Value(lhs_value) = self {
                lhs_value == rhs
            } else {
                false
            }
        }
    }
}

impl<T: Arithmetic> PartialOrd for ValueWrapper<T> {
    fn partial_cmp(&self, rhs: &Self) -> Option<std::cmp::Ordering> {
        match self {
            ValueWrapper::Value(lhs_value) => match rhs {
                ValueWrapper::Value(rhs_value) => lhs_value.partial_cmp(rhs_value),
                ValueWrapper::Inf => Some(std::cmp::Ordering::Less),
                ValueWrapper::NegInf => Some(std::cmp::Ordering::Greater),
            },
            ValueWrapper::Inf => match rhs {
                ValueWrapper::Value(_) | ValueWrapper::NegInf => Some(std::cmp::Ordering::Greater),
                ValueWrapper::Inf => Some(std::cmp::Ordering::Equal),
            },
            ValueWrapper::NegInf => match rhs {
                ValueWrapper::Value(_) | ValueWrapper::Inf => Some(std::cmp::Ordering::Less),
                ValueWrapper::NegInf => Some(std::cmp::Ordering::Equal),
            },
        }
    }
}

impl<T: Arithmetic + Ord> Ord for ValueWrapper<T> {
    fn cmp(&self, rhs: &Self) -> std::cmp::Ordering {
        self.partial_cmp(rhs).unwrap()
    }
}

impl<T: Arithmetic> PartialOrd<T> for ValueWrapper<T> {
    fn partial_cmp(&self, rhs: &T) -> Option<std::cmp::Ordering> {
        match self {
            ValueWrapper::Value(lhs_value) => lhs_value.partial_cmp(rhs),
            ValueWrapper::Inf => Some(std::cmp::Ordering::Greater),
            ValueWrapper::NegInf => Some(std::cmp::Ordering::Less),
        }
    }
}

impl<T: RealNumber> PartialOrd<T> for ValueWrapper<T> {
    fn partial_cmp(&self, rhs: &T) -> Option<std::cmp::Ordering> {
        if rhs.is_nan() {
            None
        } else {
            if rhs.is_inf() {
                match self {
                    ValueWrapper::Value(_) | ValueWrapper::NegInf => Some(std::cmp::Ordering::Less),
                    ValueWrapper::Inf => Some(std::cmp::Ordering::Equal),
                }
            } else if rhs.is_neg_inf() {
                match self {
                    ValueWrapper::Value(_) | ValueWrapper::Inf => Some(std::cmp::Ordering::Greater),
                    ValueWrapper::NegInf => Some(std::cmp::Ordering::Equal),
                }
            } else {
                match self {
                    ValueWrapper::Value(lhs_value) => lhs_value.partial_cmp(rhs),
                    ValueWrapper::Inf => Some(std::cmp::Ordering::Greater),
                    ValueWrapper::NegInf => Some(std::cmp::Ordering::Less),
                }
            }
        }
    }
}

impl<'a, T: Arithmetic> Add<&'a T> for &'a ValueWrapper<T>
where
    &'a T: Add<&'a T, Output = T>,
{
    type Output = Result<ValueWrapper<T>, IllegalArgumentError>;

    fn add(self, rhs: &'a T) -> Self::Output {
        match self {
            ValueWrapper::Value(lhs_value) => Ok(ValueWrapper::from(lhs_value + rhs)),
            ValueWrapper::Inf => Ok(ValueWrapper::Inf),
            ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
        }
    }
}

impl<'a, T: RealNumber> Add<&'a T> for ValueWrapper<T>
where
    &'a T: Add<&'a T, Output = T>,
{
    type Output = Result<ValueWrapper<T>, IllegalArgumentError>;

    fn add(self, rhs: &'a T) -> Self::Output {
        if rhs.is_nan() {
            return Err(IllegalArgumentError {
                msg: String::from("Illegal argument NaN for value range!!!"),
            });
        }

        if rhs.is_inf() {
            match self {
                ValueWrapper::Value(_) | ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                ValueWrapper::NegInf => Err(IllegalArgumentError {
                    msg: String::from("Invalid plus between inf and -inf!!!"),
                }),
            }
        } else if rhs.is_neg_inf() {
            match self {
                ValueWrapper::Value(_) | ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                ValueWrapper::Inf => Err(IllegalArgumentError {
                    msg: String::from("Invalid plus between inf and -inf!!!"),
                }),
            }
        } else {
            match self {
                ValueWrapper::Value(lhs_value) => Ok(ValueWrapper::from(lhs_value + rhs)),
                ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
            }
        }
    }
}

impl<'a, T: Arithmetic> Add for &'a ValueWrapper<T>
where
    &'a T: Add<&'a T, Output = T>,
{
    type Output = Result<ValueWrapper<T>, IllegalArgumentError>;

    fn add(self, rhs: &'a ValueWrapper<T>) -> Self::Output {
        match self {
            ValueWrapper::Value(lhs_value) => match rhs {
                ValueWrapper::Value(rhs_value) => Ok(ValueWrapper::from(lhs_value + rhs_value)),
                ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                ValueWrapper::Inf => Ok(ValueWrapper::Inf),
            },
            ValueWrapper::Inf => match rhs {
                ValueWrapper::Value(_) | ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                ValueWrapper::NegInf => Err(IllegalArgumentError {
                    msg: String::from("Invalid plus between inf and -inf!!!"),
                }),
            },
            ValueWrapper::NegInf => match rhs {
                ValueWrapper::Value(_) | ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                ValueWrapper::Inf => Err(IllegalArgumentError {
                    msg: String::from("Invalid plus between inf and -inf!!!"),
                }),
            },
        }
    }
}

impl<'a, T: Arithmetic> Sub<&'a T> for &'a ValueWrapper<T>
where
    &'a T: Sub<&'a T, Output = T>,
{
    type Output = Result<ValueWrapper<T>, IllegalArgumentError>;

    fn sub(self, rhs: &'a T) -> Self::Output {
        match self {
            ValueWrapper::Value(lhs_value) => Ok(ValueWrapper::from(lhs_value - rhs)),
            ValueWrapper::Inf => Ok(ValueWrapper::Inf),
            ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
        }
    }
}

impl<'a, T: RealNumber> Sub<&'a T> for &'a ValueWrapper<T>
where
    &'a T: Sub<&'a T, Output = T>,
{
    type Output = Result<ValueWrapper<T>, IllegalArgumentError>;

    fn sub(self, rhs: &'a T) -> Self::Output {
        if rhs.is_nan() {
            return Err(IllegalArgumentError {
                msg: String::from("Illegal argument NaN for value range!!!"),
            });
        }

        if rhs.is_inf() {
            match self {
                ValueWrapper::Value(_) | ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                ValueWrapper::Inf => Err(IllegalArgumentError {
                    msg: String::from("Invalid sub between inf and inf!!!"),
                }),
            }
        } else if rhs.is_neg_inf() {
            match self {
                ValueWrapper::Value(_) | ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                ValueWrapper::NegInf => Err(IllegalArgumentError {
                    msg: String::from("Invalid sub between -inf and -inf!!!"),
                }),
            }
        } else {
            match self {
                ValueWrapper::Value(lhs_value) => Ok(ValueWrapper::from(lhs_value - rhs)),
                ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
            }
        }
    }
}

impl<'a, T: Arithmetic> Sub for &'a ValueWrapper<T>
where
    &'a T: Sub<&'a T, Output = T>,
{
    type Output = Result<ValueWrapper<T>, IllegalArgumentError>;

    fn sub(self, rhs: &'a ValueWrapper<T>) -> Self::Output {
        match self {
            ValueWrapper::Value(lhs_value) => match rhs {
                ValueWrapper::Value(rhs_value) => Ok(ValueWrapper::from(lhs_value - rhs_value)),
                ValueWrapper::Inf => Ok(ValueWrapper::NegInf),
                ValueWrapper::NegInf => Ok(ValueWrapper::Inf),
            },
            ValueWrapper::Inf => match rhs {
                ValueWrapper::Value(_) | ValueWrapper::NegInf => Ok(ValueWrapper::Inf),
                ValueWrapper::Inf => Err(IllegalArgumentError {
                    msg: String::from("Invalid sub between inf and inf!!!"),
                }),
            },
            ValueWrapper::NegInf => match rhs {
                ValueWrapper::Value(_) | ValueWrapper::Inf => Ok(ValueWrapper::NegInf),
                ValueWrapper::NegInf => Err(IllegalArgumentError {
                    msg: String::from("Invalid sub between -inf and -inf!!!"),
                }),
            },
        }
    }
}

impl<'a, T: Arithmetic> Mul<&'a T> for &'a ValueWrapper<T>
where
    &'a T: Mul<&'a T, Output = T>,
{
    type Output = Result<ValueWrapper<T>, IllegalArgumentError>;

    fn mul(self, rhs: &'a T) -> Self::Output {
        match self {
            ValueWrapper::Value(lhs_value) => Ok(ValueWrapper::from(lhs_value * rhs)),
            ValueWrapper::Inf => {
                if rhs >= &T::ZERO {
                    Ok(ValueWrapper::Inf)
                } else {
                    Ok(ValueWrapper::NegInf)
                }
            }
            ValueWrapper::NegInf => {
                if rhs >= &T::ZERO {
                    Ok(ValueWrapper::NegInf)
                } else {
                    Ok(ValueWrapper::Inf)
                }
            }
        }
    }
}

impl<'a, T: RealNumber> Mul<&'a T> for &'a ValueWrapper<T>
where
    &'a T: Mul<&'a T, Output = T>,
{
    type Output = Result<ValueWrapper<T>, IllegalArgumentError>;

    fn mul(self, rhs: &'a T) -> Self::Output {
        if rhs.is_nan() {
            return Err(IllegalArgumentError {
                msg: String::from("Illegal argument NaN for value range!!!"),
            });
        }

        if rhs.is_inf() {
            match self {
                ValueWrapper::Value(_) | ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
            }
        } else if rhs.is_neg_inf() {
            match self {
                ValueWrapper::Value(_) | ValueWrapper::Inf => Ok(ValueWrapper::NegInf),
                ValueWrapper::NegInf => Ok(ValueWrapper::Inf),
            }
        } else {
            match self {
                ValueWrapper::Value(lhs_value) => Ok(ValueWrapper::from(lhs_value * rhs)),
                ValueWrapper::Inf => {
                    if rhs >= &T::ZERO {
                        Ok(ValueWrapper::Inf)
                    } else {
                        Ok(ValueWrapper::NegInf)
                    }
                }
                ValueWrapper::NegInf => {
                    if rhs >= &T::ZERO {
                        Ok(ValueWrapper::NegInf)
                    } else {
                        Ok(ValueWrapper::Inf)
                    }
                }
            }
        }
    }
}

impl<'a, T: RealNumber> Mul for &'a ValueWrapper<T>
where
    &'a T: Mul<&'a T, Output = T>,
{
    type Output = Result<ValueWrapper<T>, IllegalArgumentError>;

    fn mul(self, rhs: &'a ValueWrapper<T>) -> Self::Output {
        match self {
            ValueWrapper::Value(lhs_value) => match rhs {
                ValueWrapper::Value(rhs_value) => Ok(ValueWrapper::from(lhs_value * rhs_value)),
                ValueWrapper::Inf => {
                    if lhs_value >= &T::ZERO {
                        Ok(ValueWrapper::Inf)
                    } else {
                        Ok(ValueWrapper::NegInf)
                    }
                }
                ValueWrapper::NegInf => {
                    if lhs_value >= &T::ZERO {
                        Ok(ValueWrapper::NegInf)
                    } else {
                        Ok(ValueWrapper::Inf)
                    }
                }
            },
            ValueWrapper::Inf => match rhs {
                ValueWrapper::Value(rhs_value) => {
                    if rhs_value >= &T::ZERO {
                        Ok(ValueWrapper::Inf)
                    } else {
                        Ok(ValueWrapper::NegInf)
                    }
                }
                ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
            },
            ValueWrapper::NegInf => match rhs {
                ValueWrapper::Value(rhs_value) => {
                    if rhs >= &T::ZERO {
                        Ok(ValueWrapper::NegInf)
                    } else {
                        Ok(ValueWrapper::Inf)
                    }
                }
                ValueWrapper::Inf => Ok(ValueWrapper::NegInf),
                ValueWrapper::NegInf => Ok(ValueWrapper::Inf),
            },
        }
    }
}

impl<'a, T: Arithmetic> Div<&'a T> for &'a ValueWrapper<T>
where
    &'a T: Div<&'a T, Output = T>,
{
    type Output = Result<ValueWrapper<T>, IllegalArgumentError>;

    fn div(self, rhs: &'a ValueWrapper<T>) -> Self::Output {
        match self {
            ValueWrapper::Value(lhs_value) => Ok(ValueWrapper::from(lhs_value / rhs)),
            ValueWrapper::Inf => {
                if rhs >= &T::ZERO {
                    Ok(ValueWrapper::Inf)
                } else {
                    Ok(ValueWrapper::NegInf)
                }
            }
            ValueWrapper::NegInf => {
                if rhs >= &T::ZERO {
                    Ok(ValueWrapper::NegInf)
                } else {
                    Ok(ValueWrapper::Inf)
                }
            }
        }
    }
}

impl<'a, T: RealNumber> Div<&'a T> for &'a ValueWrapper<T>
where
    &'a T: Div<&'a T, Output = T>,
{
    type Output = Result<ValueWrapper<T>, IllegalArgumentError>;

    fn div(self, rhs: &'a ValueWrapper<T>) -> Self::Output {
        if rhs.is_nan() {
            return Err(IllegalArgumentError {
                msg: String::from("Illegal argument NaN for value range!!!"),
            });
        }

        if rhs.is_inf() {
            match self {
                ValueWrapper::Value(_) => Ok(ValueWrapper::from(T::ZERO)),
                ValueWrapper::Inf => Err(IllegalArgumentError {
                    msg: String::from("Invalid div between inf and inf!!!"),
                }),
                ValueWrapper::NegInf => Err(IllegalArgumentError {
                    msg: String::from("Invalid div between -inf and inf!!!"),
                }),
            }
        } else if rhs.is_neg_inf() {
            match self {
                ValueWrapper::Value(_) => Ok(ValueWrapper::from(T::ZERO)),
                ValueWrapper::Inf => Err(IllegalArgumentError {
                    msg: String::from("Invalid div between inf and -inf!!!"),
                }),
                ValueWrapper::NegInf => Err(IllegalArgumentError {
                    msg: String::from("Invalid div between -inf and -inf!!!"),
                }),
            }
        } else {
            match self {
                ValueWrapper::Value(lhs_value) => Ok(ValueWrapper::from(lhs_value / rhs)),
                ValueWrapper::Inf => {
                    if rhs >= &T::ZERO {
                        Ok(ValueWrapper::Inf)
                    } else {
                        Ok(ValueWrapper::NegInf)
                    }
                }
                ValueWrapper::NegInf => {
                    if rhs >= &T::ZERO {
                        Ok(ValueWrapper::NegInf)
                    } else {
                        Ok(ValueWrapper::Inf)
                    }
                }
            }
        }
    }
}

impl<'a, T: Arithmetic> Div for &'a ValueWrapper<T>
where
    &'a T: Div<&'a T, Output = T>,
{
    type Output = Result<ValueWrapper<T>, IllegalArgumentError>;

    fn div(self, rhs: &'a ValueWrapper<T>) -> Self::Output {
        match self {
            ValueWrapper::Value(lhs_value) => match rhs {
                ValueWrapper::Value(rhs_value) => Ok(ValueWrapper::from(lhs_value / rhs_value)),
                ValueWrapper::Inf | ValueWrapper::NegInf => Ok(ValueWrapper::from(T::ZERO)),
            },
            ValueWrapper::Inf => match rhs {
                ValueWrapper::Value(rhs_value) => {
                    if rhs >= &T::ZERO {
                        Ok(ValueWrapper::Inf)
                    } else {
                        Ok(ValueWrapper::NegInf)
                    }
                }
                ValueWrapper::Inf => Err(IllegalArgumentError {
                    msg: String::from("Invalid div between inf and inf!!!"),
                }),
                ValueWrapper::NegInf => Err(IllegalArgumentError {
                    msg: String::from("Invalid div between inf and -inf!!!"),
                }),
            },
            ValueWrapper::NegInf => match rhs {
                ValueWrapper::Value(rhs_value) => {
                    if rhs >= &T::ZERO {
                        Ok(ValueWrapper::NegInf)
                    } else {
                        Ok(ValueWrapper::Inf)
                    }
                }
                ValueWrapper::Inf => Err(IllegalArgumentError {
                    msg: String::from("Invalid div between -inf and inf!!!"),
                }),
                ValueWrapper::NegInf => Err(IllegalArgumentError {
                    msg: String::from("Invalid div between -inf and -inf!!!"),
                }),
            },
        }
    }
}
