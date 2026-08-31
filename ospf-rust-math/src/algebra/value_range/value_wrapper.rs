use std::cmp::Ordering;
use std::fmt::{Debug, Display};
use std::ops::{Add, Div, Mul, Neg, Sub};
use std::time::{Duration, Instant};

use chrono::NaiveDateTime;

use crate::algebra::concept::{Infinity, NegativeInfinity, RealNumber, SemiArithmetic};

use super::IllegalArgumentError;

#[derive(Clone, Copy)]
pub enum ValueWrapper<T> {
    Value(T),
    Inf,
    NegInf,
}

impl<T> ValueWrapper<T> {
    fn to<U>(self) -> ValueWrapper<U>
    where
        U: From<T>,
    {
        match self {
            Self::Value(value) => ValueWrapper::from(U::from(value)),
            Self::Inf => ValueWrapper::Inf,
            Self::NegInf => ValueWrapper::NegInf,
        }
    }
}

impl<T: SemiArithmetic> From<Infinity> for ValueWrapper<T> {
    fn from(_: Infinity) -> Self {
        Self::Inf
    }
}

impl<T: SemiArithmetic> From<NegativeInfinity> for ValueWrapper<T> {
    fn from(_: NegativeInfinity) -> Self {
        Self::NegInf
    }
}

impl<T> From<T> for ValueWrapper<T> {
    default fn from(value: T) -> Self {
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

impl<T: SemiArithmetic + for<'a> From<&'a U>, U: SemiArithmetic> From<&ValueWrapper<U>>
    for ValueWrapper<T>
{
    fn from(value: &ValueWrapper<U>) -> Self {
        match value {
            ValueWrapper::Value(value) => ValueWrapper::from(T::from(value)),
            ValueWrapper::Inf => ValueWrapper::Inf,
            ValueWrapper::NegInf => ValueWrapper::NegInf,
        }
    }
}

impl<T: Display> Display for ValueWrapper<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Value(value) => write!(f, "{}", value),
            Self::Inf => write!(f, "inf"),
            Self::NegInf => write!(f, "-inf"),
        }
    }
}

impl<T: Display> Debug for ValueWrapper<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Value(value) => write!(f, "{}", value),
            Self::Inf => write!(f, "inf"),
            Self::NegInf => write!(f, "-inf"),
        }
    }
}

impl<T, U: SemiArithmetic> PartialEq<U> for ValueWrapper<T>
where
    T: PartialEq<U>,
{
    default fn eq(&self, other: &U) -> bool {
        match self {
            ValueWrapper::Value(value) => value.eq(other),
            ValueWrapper::Inf => false,
            ValueWrapper::NegInf => false,
        }
    }
}

impl<T, U: RealNumber> PartialEq<U> for ValueWrapper<T>
where
    T: PartialEq<U>,
{
    fn eq(&self, other: &U) -> bool {
        if other.is_nan() {
            false
        } else if other.is_inf() {
            match self {
                ValueWrapper::Value(_) | ValueWrapper::NegInf => false,
                ValueWrapper::Inf => true,
            }
        } else if other.is_neg_inf() {
            match self {
                ValueWrapper::Value(_) | ValueWrapper::Inf => false,
                ValueWrapper::NegInf => true,
            }
        } else {
            match self {
                ValueWrapper::Value(value) => value.eq(other),
                ValueWrapper::Inf => false,
                ValueWrapper::NegInf => false,
            }
        }
    }
}

impl<T, U> PartialEq<ValueWrapper<U>> for ValueWrapper<T>
where
    T: PartialEq<U>,
{
    default fn eq(&self, other: &ValueWrapper<U>) -> bool {
        match self {
            ValueWrapper::Value(value) => match other {
                ValueWrapper::Value(other_value) => value.eq(other_value),
                _ => false,
            },
            ValueWrapper::Inf => match other {
                ValueWrapper::Inf => true,
                _ => false,
            },
            ValueWrapper::NegInf => match other {
                ValueWrapper::NegInf => true,
                _ => false,
            },
        }
    }
}

impl PartialEq for ValueWrapper<Instant> {
    fn eq(&self, other: &Self) -> bool {
        match self {
            ValueWrapper::Value(value) => match other {
                ValueWrapper::Value(other_value) => value.eq(other_value),
                _ => false,
            },
            ValueWrapper::Inf => match other {
                ValueWrapper::Inf => true,
                _ => false,
            },
            ValueWrapper::NegInf => match other {
                ValueWrapper::NegInf => true,
                _ => false,
            },
        }
    }
}

impl PartialEq<Instant> for ValueWrapper<Instant> {
    fn eq(&self, other: &Instant) -> bool {
        match self {
            ValueWrapper::Value(value) => value.eq(other),
            ValueWrapper::Inf => false,
            ValueWrapper::NegInf => false,
        }
    }
}

impl PartialEq for ValueWrapper<NaiveDateTime> {
    fn eq(&self, other: &Self) -> bool {
        match self {
            ValueWrapper::Value(value) => match other {
                ValueWrapper::Value(other_value) => value.eq(other_value),
                _ => false,
            },
            ValueWrapper::Inf => match other {
                ValueWrapper::Inf => true,
                _ => false,
            },
            ValueWrapper::NegInf => match other {
                ValueWrapper::NegInf => true,
                _ => false,
            },
        }
    }
}

impl PartialEq<NaiveDateTime> for ValueWrapper<NaiveDateTime> {
    fn eq(&self, other: &NaiveDateTime) -> bool {
        match self {
            ValueWrapper::Value(value) => value.eq(other),
            ValueWrapper::Inf => false,
            ValueWrapper::NegInf => false,
        }
    }
}

impl<T, U: SemiArithmetic> PartialOrd<U> for ValueWrapper<T>
where
    T: PartialOrd<U>,
{
    default fn partial_cmp(&self, rhs: &U) -> Option<Ordering> {
        match self {
            ValueWrapper::Value(lhs_value) => lhs_value.partial_cmp(rhs),
            ValueWrapper::Inf => Some(Ordering::Greater),
            ValueWrapper::NegInf => Some(Ordering::Less),
        }
    }
}

impl<T, U: RealNumber> PartialOrd<U> for ValueWrapper<T>
where
    T: PartialOrd<U>,
{
    fn partial_cmp(&self, rhs: &U) -> Option<Ordering> {
        if rhs.is_nan() {
            None
        } else if rhs.is_inf() {
            match self {
                ValueWrapper::Value(_) | ValueWrapper::NegInf => Some(Ordering::Less),
                ValueWrapper::Inf => Some(Ordering::Equal),
            }
        } else if rhs.is_neg_inf() {
            match self {
                ValueWrapper::Value(_) | ValueWrapper::Inf => Some(Ordering::Greater),
                ValueWrapper::NegInf => Some(Ordering::Equal),
            }
        } else {
            match self {
                ValueWrapper::Value(lhs_value) => lhs_value.partial_cmp(rhs),
                ValueWrapper::Inf => Some(Ordering::Greater),
                ValueWrapper::NegInf => Some(Ordering::Less),
            }
        }
    }
}

impl<T, U> PartialOrd<ValueWrapper<U>> for ValueWrapper<T>
where
    T: PartialOrd<U>,
{
    default fn partial_cmp(&self, rhs: &ValueWrapper<U>) -> Option<Ordering> {
        match self {
            ValueWrapper::Value(lhs_value) => match rhs {
                ValueWrapper::Value(rhs_value) => lhs_value.partial_cmp(rhs_value),
                ValueWrapper::Inf => Some(Ordering::Less),
                ValueWrapper::NegInf => Some(Ordering::Greater),
            },
            ValueWrapper::Inf => match rhs {
                ValueWrapper::Value(_) | ValueWrapper::NegInf => Some(Ordering::Greater),
                ValueWrapper::Inf => Some(Ordering::Equal),
            },
            ValueWrapper::NegInf => match rhs {
                ValueWrapper::Value(_) | ValueWrapper::Inf => Some(Ordering::Less),
                ValueWrapper::NegInf => Some(Ordering::Equal),
            },
        }
    }
}

impl PartialOrd<Instant> for ValueWrapper<Instant> {
    fn partial_cmp(&self, other: &Instant) -> Option<Ordering> {
        match self {
            ValueWrapper::Value(value) => value.partial_cmp(other),
            ValueWrapper::Inf => Some(Ordering::Greater),
            ValueWrapper::NegInf => Some(Ordering::Less),
        }
    }
}

impl PartialOrd<NaiveDateTime> for ValueWrapper<NaiveDateTime> {
    fn partial_cmp(&self, other: &NaiveDateTime) -> Option<Ordering> {
        match self {
            ValueWrapper::Value(value) => value.partial_cmp(other),
            ValueWrapper::Inf => Some(Ordering::Greater),
            ValueWrapper::NegInf => Some(Ordering::Less),
        }
    }
}

macro_rules! value_wrapper_template {
    ($type:ident, $rhs:ident) => {
        impl Add<$rhs> for ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Add<$rhs>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: $rhs) -> Self::Output {
                match self {
                    ValueWrapper::Value(lhs_value) => Ok(ValueWrapper::from(lhs_value + rhs)),
                    ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                    ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                }
            }
        }

        impl<'a> Add<&'a $rhs> for ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Add<$rhs>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'a $rhs) -> Self::Output {
                match self {
                    ValueWrapper::Value(lhs_value) => {
                        Ok(ValueWrapper::from(lhs_value + rhs.clone()))
                    }
                    ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                    ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                }
            }
        }

        impl<'a> Add<$rhs> for &'a ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Add<$rhs>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: $rhs) -> Self::Output {
                match self {
                    ValueWrapper::Value(lhs_value) => {
                        Ok(ValueWrapper::from(lhs_value.clone() + rhs))
                    }
                    ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                    ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                }
            }
        }

        impl<'a, 'b> Add<&'b $rhs> for &'a ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Add<$rhs>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'b $rhs) -> Self::Output {
                match self {
                    ValueWrapper::Value(lhs_value) => {
                        Ok(ValueWrapper::from(lhs_value.clone() + rhs.clone()))
                    }
                    ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                    ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                }
            }
        }

        impl Sub<$rhs> for ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Sub<$rhs>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: $rhs) -> Self::Output {
                match self {
                    ValueWrapper::Value(lhs_value) => Ok(ValueWrapper::from(lhs_value - rhs)),
                    ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                    ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                }
            }
        }

        impl<'a> Sub<&'a $rhs> for ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Sub<$rhs>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'a $rhs) -> Self::Output {
                match self {
                    ValueWrapper::Value(lhs_value) => {
                        Ok(ValueWrapper::from(lhs_value - rhs.clone()))
                    }
                    ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                    ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                }
            }
        }

        impl<'a> Sub<$rhs> for &'a ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Sub<$rhs>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: $rhs) -> Self::Output {
                match self {
                    ValueWrapper::Value(lhs_value) => {
                        Ok(ValueWrapper::from(lhs_value.clone() - rhs))
                    }
                    ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                    ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                }
            }
        }

        impl<'a, 'b> Sub<&'b $rhs> for &'a ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Sub<$rhs>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'b $rhs) -> Self::Output {
                match self {
                    ValueWrapper::Value(lhs_value) => {
                        Ok(ValueWrapper::from(lhs_value.clone() - rhs.clone()))
                    }
                    ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                    ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                }
            }
        }
    };
}
value_wrapper_template!(Instant, Duration);
value_wrapper_template!(NaiveDateTime, Duration);

macro_rules! signed_value_wrapper_template {
    ($($type:ident)*) => ($(
        impl Neg for ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Neg>::Output>, IllegalArgumentError>;

            fn neg(self) -> Self::Output {
                match self {
                    ValueWrapper::Value(value) => Ok(ValueWrapper::from(-value)),
                    ValueWrapper::Inf => Ok(ValueWrapper::NegInf),
                    ValueWrapper::NegInf => Ok(ValueWrapper::Inf),
                }
            }
        }

        impl<'a> Neg for &'a ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<&'a $type as Neg>::Output>, IllegalArgumentError>;

            fn neg(self) -> Self::Output {
                match self {
                    ValueWrapper::Value(value) => Ok(ValueWrapper::from(-value)),
                    ValueWrapper::Inf => Ok(ValueWrapper::NegInf),
                    ValueWrapper::NegInf => Ok(ValueWrapper::Inf),
                }
            }
        }
    )*)
}
signed_value_wrapper_template! { i8 i16 i32 i64 i128 isize f32 f64 }

macro_rules! real_number_value_wrapper_template {
    ($($type:ident)*) => ($(
        impl Add<$type> for ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: $type) -> Self::Output {
                if rhs.is_nan() {
                    Err(IllegalArgumentError {
                        msg: "Illegal argument NaN for value range!!!".to_string(),
                    })
                } else if rhs.is_inf() {
                    match self {
                        ValueWrapper::Value(_) | ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "invalid addition between inf and -inf!!!".to_string(),
                        }),
                    }
                } else if rhs.is_neg_inf() {
                    match self {
                        ValueWrapper::Value(_) | ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "invalid addition between inf and -inf!!!".to_string(),
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

        impl<'a> Add<&'a $type> for ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Add<&'a $type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'a $type) -> Self::Output {
                if rhs.is_nan() {
                    Err(IllegalArgumentError {
                        msg: "Illegal argument NaN for value range!!!".to_string(),
                    })
                } else if rhs.is_inf() {
                    match self {
                        ValueWrapper::Value(_) | ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "invalid addition between inf and -inf!!!".to_string(),
                        }),
                    }
                } else if rhs.is_neg_inf() {
                    match self {
                        ValueWrapper::Value(_) | ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "invalid addition between inf and -inf!!!".to_string(),
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

        impl<'a> Add<$type> for &'a ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<&'a $type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: $type) -> Self::Output {
                if rhs.is_nan() {
                    Err(IllegalArgumentError {
                        msg: "Illegal argument NaN for value range!!!".to_string(),
                    })
                } else if rhs.is_inf() {
                    match self {
                        ValueWrapper::Value(_) | ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "invalid addition between inf and -inf!!!".to_string(),
                        }),
                    }
                } else if rhs.is_neg_inf() {
                    match self {
                        ValueWrapper::Value(_) | ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "invalid addition between inf and -inf!!!".to_string(),
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

        impl<'a, 'b> Add<&'b $type> for &'a ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<&'a $type as Add<&'b $type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'b $type) -> Self::Output {
                if rhs.is_nan() {
                    Err(IllegalArgumentError {
                        msg: "Illegal argument NaN for value range!!!".to_string(),
                    })
                } else if rhs.is_inf() {
                    match self {
                        ValueWrapper::Value(_) | ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "invalid addition between inf and -inf!!!".to_string(),
                        }),
                    }
                } else if rhs.is_neg_inf() {
                    match self {
                        ValueWrapper::Value(_) | ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "invalid addition between inf and -inf!!!".to_string(),
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

        impl Add<ValueWrapper<$type>> for ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: ValueWrapper<$type>) -> Self::Output {
                match self {
                    ValueWrapper::Value(lhs_value) => match rhs {
                        ValueWrapper::Value(rhs_value) => Ok(ValueWrapper::from(lhs_value + rhs_value)),
                        ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                        ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                    },
                    ValueWrapper::Inf => match rhs {
                        ValueWrapper::Value(_) | ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "invalid addition between inf and -inf!!!".to_string(),
                        }),
                    },
                    ValueWrapper::NegInf => match rhs {
                        ValueWrapper::Value(_) | ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "invalid addition between inf and -inf!!!".to_string(),
                        }),
                    },
                }
            }
        }

        impl<'a> Add<&'a ValueWrapper<$type>> for ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Add<&'a $type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'a ValueWrapper<$type>) -> Self::Output {
                match self {
                    ValueWrapper::Value(lhs_value) => match rhs {
                        ValueWrapper::Value(rhs_value) => Ok(ValueWrapper::from(lhs_value + rhs_value)),
                        ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                        ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                    },
                    ValueWrapper::Inf => match rhs {
                        ValueWrapper::Value(_) | ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "invalid addition between inf and -inf!!!".to_string(),
                        }),
                    },
                    ValueWrapper::NegInf => match rhs {
                        ValueWrapper::Value(_) | ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "invalid addition between inf and -inf!!!".to_string(),
                        }),
                    },
                }
            }
        }

        impl<'a> Add<ValueWrapper<$type>> for &'a ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<&'a $type as Add<$type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: ValueWrapper<$type>) -> Self::Output {
                match self {
                    ValueWrapper::Value(lhs_value) => match rhs {
                        ValueWrapper::Value(rhs_value) => Ok(ValueWrapper::from(lhs_value + rhs_value)),
                        ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                        ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                    },
                    ValueWrapper::Inf => match rhs {
                        ValueWrapper::Value(_) | ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "invalid addition between inf and -inf!!!".to_string(),
                        }),
                    },
                    ValueWrapper::NegInf => match rhs {
                        ValueWrapper::Value(_) | ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "invalid addition between inf and -inf!!!".to_string(),
                        }),
                    },
                }
            }
        }

        impl<'a, 'b> Add<&'b ValueWrapper<$type>> for &'a ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<&'a $type as Add<&'b $type>>::Output>, IllegalArgumentError>;

            fn add(self, rhs: &'b ValueWrapper<$type>) -> Self::Output {
                match self {
                    ValueWrapper::Value(lhs_value) => match rhs {
                        ValueWrapper::Value(rhs_value) => Ok(ValueWrapper::from(lhs_value + rhs_value)),
                        ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                        ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                    },
                    ValueWrapper::Inf => match rhs {
                        ValueWrapper::Value(_) | ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "invalid addition between inf and -inf!!!".to_string(),
                        }),
                    },
                    ValueWrapper::NegInf => match rhs {
                        ValueWrapper::Value(_) | ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "invalid addition between inf and -inf!!!".to_string(),
                        }),
                    },
                }
            }
        }

        impl Sub<$type> for ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: $type) -> Self::Output {
                if rhs.is_nan() {
                    Err(IllegalArgumentError {
                        msg: "Illegal argument NaN for value range!!!".to_string(),
                    })
                } else if rhs.is_inf() {
                    match self {
                        ValueWrapper::Value(_) | ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "invalid subtraction between inf and inf!!!".to_string(),
                        }),
                    }
                } else if rhs.is_neg_inf() {
                    match self {
                        ValueWrapper::Value(_) | ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "invalid subtraction between -inf and -inf!!!".to_string(),
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

        impl<'a> Sub<&'a $type> for ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Sub<&'a $type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'a $type) -> Self::Output {
                if rhs.is_nan() {
                    Err(IllegalArgumentError {
                        msg: "Illegal argument NaN for value range!!!".to_string(),
                    })
                } else if rhs.is_inf() {
                    match self {
                        ValueWrapper::Value(_) | ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "invalid subtraction between inf and inf!!!".to_string(),
                        }),
                    }
                } else if rhs.is_neg_inf() {
                    match self {
                        ValueWrapper::Value(_) | ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "invalid subtraction between -inf and -inf!!!".to_string(),
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

        impl<'a> Sub<$type> for &'a ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<&'a $type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: $type) -> Self::Output {
                if rhs.is_nan() {
                    Err(IllegalArgumentError {
                        msg: "Illegal argument NaN for value range!!!".to_string(),
                    })
                } else if rhs.is_inf() {
                    match self {
                        ValueWrapper::Value(_) | ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "invalid subtraction between inf and inf!!!".to_string(),
                        }),
                    }
                } else if rhs.is_neg_inf() {
                    match self {
                        ValueWrapper::Value(_) | ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "invalid subtraction between -inf and -inf!!!".to_string(),
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

        impl<'a, 'b> Sub<&'b $type> for &'a ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<&'a $type as Sub<&'b $type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'b $type) -> Self::Output {
                if rhs.is_nan() {
                    Err(IllegalArgumentError {
                        msg: "Illegal argument NaN for value range!!!".to_string(),
                    })
                } else if rhs.is_inf() {
                    match self {
                        ValueWrapper::Value(_) | ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "invalid subtraction between inf and inf!!!".to_string(),
                        }),
                    }
                } else if rhs.is_neg_inf() {
                    match self {
                        ValueWrapper::Value(_) | ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "invalid subtraction between -inf and -inf!!!".to_string(),
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

        impl Sub<ValueWrapper<$type>> for ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Sub<$type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: ValueWrapper<$type>) -> Self::Output {
                match self {
                    ValueWrapper::Value(lhs_value) => match rhs {
                        ValueWrapper::Value(rhs_value) => Ok(ValueWrapper::from(lhs_value - rhs_value)),
                        ValueWrapper::NegInf => Ok(ValueWrapper::Inf),
                        ValueWrapper::Inf => Ok(ValueWrapper::NegInf),
                    },
                    ValueWrapper::Inf => match rhs {
                        ValueWrapper::Value(_) | ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "invalid subtraction between inf and inf!!!".to_string(),
                        }),
                    },
                    ValueWrapper::NegInf => match rhs {
                        ValueWrapper::Value(_) | ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "invalid subtraction between -inf and -inf!!!".to_string(),
                        }),
                    },
                }
            }
        }

        impl<'a> Sub<&'a ValueWrapper<$type>> for ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Sub<&'a $type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'a ValueWrapper<$type>) -> Self::Output {
                match self {
                    ValueWrapper::Value(lhs_value) => match rhs {
                        ValueWrapper::Value(rhs_value) => Ok(ValueWrapper::from(lhs_value - rhs_value)),
                        ValueWrapper::NegInf => Ok(ValueWrapper::Inf),
                        ValueWrapper::Inf => Ok(ValueWrapper::NegInf),
                    },
                    ValueWrapper::Inf => match rhs {
                        ValueWrapper::Value(_) | ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "invalid subtraction between inf and inf!!!".to_string(),
                        }),
                    },
                    ValueWrapper::NegInf => match rhs {
                        ValueWrapper::Value(_) | ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "invalid subtraction between -inf and -inf!!!".to_string(),
                        }),
                    },
                }
            }
        }

        impl<'a> Sub<ValueWrapper<$type>> for &'a ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Sub<&'a $type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: ValueWrapper<$type>) -> Self::Output {
                match self {
                    ValueWrapper::Value(lhs_value) => match rhs {
                        ValueWrapper::Value(rhs_value) => Ok(ValueWrapper::from(lhs_value - rhs_value)),
                        ValueWrapper::NegInf => Ok(ValueWrapper::Inf),
                        ValueWrapper::Inf => Ok(ValueWrapper::NegInf),
                    },
                    ValueWrapper::Inf => match rhs {
                        ValueWrapper::Value(_) | ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "invalid subtraction between inf and inf!!!".to_string(),
                        }),
                    },
                    ValueWrapper::NegInf => match rhs {
                        ValueWrapper::Value(_) | ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "invalid subtraction between -inf and -inf!!!".to_string(),
                        }),
                    },
                }
            }
        }

        impl<'a, 'b> Sub<&'b ValueWrapper<$type>> for &'a ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<&'a $type as Sub<&'b $type>>::Output>, IllegalArgumentError>;

            fn sub(self, rhs: &'b ValueWrapper<$type>) -> Self::Output {
                match self {
                    ValueWrapper::Value(lhs_value) => match rhs {
                        ValueWrapper::Value(rhs_value) => Ok(ValueWrapper::from(lhs_value - rhs_value)),
                        ValueWrapper::NegInf => Ok(ValueWrapper::Inf),
                        ValueWrapper::Inf => Ok(ValueWrapper::NegInf),
                    },
                    ValueWrapper::Inf => match rhs {
                        ValueWrapper::Value(_) | ValueWrapper::NegInf => Ok(ValueWrapper::NegInf),
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "invalid subtraction between inf and inf!!!".to_string(),
                        }),
                    },
                    ValueWrapper::NegInf => match rhs {
                        ValueWrapper::Value(_) | ValueWrapper::Inf => Ok(ValueWrapper::Inf),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "invalid subtraction between -inf and -inf!!!".to_string(),
                        }),
                    },
                }
            }
        }

        impl Mul<$type> for ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: $type) -> Self::Output {
                if rhs.is_nan() {
                    Err(IllegalArgumentError {
                        msg: "Illegal argument NaN for value range!!!".to_string(),
                    })
                } else if rhs.is_inf() {
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
                            if &rhs >= $type::ZERO {
                                Ok(ValueWrapper::Inf)
                            } else {
                                Ok(ValueWrapper::NegInf)
                            }
                        }
                        ValueWrapper::NegInf => {
                            if &rhs >= $type::ZERO {
                                Ok(ValueWrapper::NegInf)
                            } else {
                                Ok(ValueWrapper::Inf)
                            }
                        }
                    }
                }
            }
        }

        impl<'a> Mul<&'a $type> for ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Mul<&'a $type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: &'a $type) -> Self::Output {
                if rhs.is_nan() {
                    Err(IllegalArgumentError {
                        msg: "Illegal argument NaN for value range!!!".to_string(),
                    })
                } else if rhs.is_inf() {
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
                            if rhs >= $type::ZERO {
                                Ok(ValueWrapper::Inf)
                            } else {
                                Ok(ValueWrapper::NegInf)
                            }
                        }
                        ValueWrapper::NegInf => {
                            if rhs >= $type::ZERO {
                                Ok(ValueWrapper::NegInf)
                            } else {
                                Ok(ValueWrapper::Inf)
                            }
                        }
                    }
                }
            }
        }

        impl<'a> Mul<$type> for ValueWrapper<&'a $type> {
            type Output = Result<ValueWrapper<<&'a $type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: $type) -> Self::Output {
                if rhs.is_nan() {
                    Err(IllegalArgumentError {
                        msg: "Illegal argument NaN for value range!!!".to_string(),
                    })
                } else if rhs.is_inf() {
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
                            if &rhs >= $type::ZERO {
                                Ok(ValueWrapper::Inf)
                            } else {
                                Ok(ValueWrapper::NegInf)
                            }
                        }
                        ValueWrapper::NegInf => {
                            if &rhs >= $type::ZERO {
                                Ok(ValueWrapper::NegInf)
                            } else {
                                Ok(ValueWrapper::Inf)
                            }
                        }
                    }
                }
            }
        }

        impl<'a, 'b> Mul<&'b $type> for &'a ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<&'a $type as Mul<&'b $type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: &'b $type) -> Self::Output {
                if rhs.is_nan() {
                    Err(IllegalArgumentError {
                        msg: "Illegal argument NaN for value range!!!".to_string(),
                    })
                } else if rhs.is_inf() {
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
                            if rhs >= $type::ZERO {
                                Ok(ValueWrapper::Inf)
                            } else {
                                Ok(ValueWrapper::NegInf)
                            }
                        }
                        ValueWrapper::NegInf => {
                            if rhs >= $type::ZERO {
                                Ok(ValueWrapper::NegInf)
                            } else {
                                Ok(ValueWrapper::Inf)
                            }
                        }
                    }
                }
            }
        }

        impl Mul<ValueWrapper<$type>> for ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Mul<$type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: ValueWrapper<$type>) -> Self::Output {
                match self {
                    ValueWrapper::Value(lhs_value) => match rhs {
                        ValueWrapper::Value(rhs_value) => Ok(ValueWrapper::from(lhs_value * rhs_value)),
                        ValueWrapper::Inf => {
                            if &lhs_value >= $type::ZERO {
                                Ok(ValueWrapper::Inf)
                            } else {
                                Ok(ValueWrapper::NegInf)
                            }
                        }
                        ValueWrapper::NegInf => {
                            if &lhs_value >= $type::ZERO {
                                Ok(ValueWrapper::NegInf)
                            } else {
                                Ok(ValueWrapper::Inf)
                            }
                        }
                    },
                    ValueWrapper::Inf => match rhs {
                        ValueWrapper::Value(rhs_value) => {
                            if &rhs_value >= $type::ZERO {
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
                            if &rhs_value >= $type::ZERO {
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

        impl<'a> Mul<&'a ValueWrapper<$type>> for ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Mul<&'a $type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: &'a ValueWrapper<$type>) -> Self::Output {
                match self {
                    ValueWrapper::Value(lhs_value) => match rhs {
                        ValueWrapper::Value(rhs_value) => Ok(ValueWrapper::from(lhs_value * rhs_value)),
                        ValueWrapper::Inf => {
                            if lhs_value >= *$type::ZERO {
                                Ok(ValueWrapper::Inf)
                            } else {
                                Ok(ValueWrapper::NegInf)
                            }
                        }
                        ValueWrapper::NegInf => {
                            if lhs_value >= *$type::ZERO {
                                Ok(ValueWrapper::NegInf)
                            } else {
                                Ok(ValueWrapper::Inf)
                            }
                        }
                    },
                    ValueWrapper::Inf => match rhs {
                        ValueWrapper::Value(rhs_value) => {
                            if rhs_value >= $type::ZERO {
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
                            if rhs_value >= $type::ZERO {
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

        impl<'a> Mul<ValueWrapper<$type>> for &'a ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Mul<&'a $type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: ValueWrapper<$type>) -> Self::Output {
                match self {
                    ValueWrapper::Value(lhs_value) => match rhs {
                        ValueWrapper::Value(rhs_value) => Ok(ValueWrapper::from(lhs_value * rhs_value)),
                        ValueWrapper::Inf => {
                            if lhs_value >= $type::ZERO {
                                Ok(ValueWrapper::Inf)
                            } else {
                                Ok(ValueWrapper::NegInf)
                            }
                        }
                        ValueWrapper::NegInf => {
                            if lhs_value >= $type::ZERO {
                                Ok(ValueWrapper::NegInf)
                            } else {
                                Ok(ValueWrapper::Inf)
                            }
                        }
                    },
                    ValueWrapper::Inf => match rhs {
                        ValueWrapper::Value(rhs_value) => {
                            if rhs_value >= *$type::ZERO {
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
                            if rhs_value >= *$type::ZERO {
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

        impl<'a, 'b> Mul<&'b ValueWrapper<$type>> for &'a ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<&'a $type as Mul<&'b $type>>::Output>, IllegalArgumentError>;

            fn mul(self, rhs: &'b ValueWrapper<$type>) -> Self::Output {
                match self {
                    ValueWrapper::Value(lhs_value) => match rhs {
                        ValueWrapper::Value(rhs_value) => Ok(ValueWrapper::from(lhs_value * rhs_value)),
                        ValueWrapper::Inf => {
                            if lhs_value >= $type::ZERO {
                                Ok(ValueWrapper::Inf)
                            } else {
                                Ok(ValueWrapper::NegInf)
                            }
                        }
                        ValueWrapper::NegInf => {
                            if lhs_value >= $type::ZERO {
                                Ok(ValueWrapper::NegInf)
                            } else {
                                Ok(ValueWrapper::Inf)
                            }
                        }
                    },
                    ValueWrapper::Inf => match rhs {
                        ValueWrapper::Value(rhs_value) => {
                            if rhs_value >= $type::ZERO {
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
                            if rhs_value >= $type::ZERO {
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

        impl Div<$type> for ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: $type) -> Self::Output {
                if rhs.is_nan() {
                    Err(IllegalArgumentError {
                        msg: "Illegal argument NaN for value range!!!".to_string(),
                    })
                } else if rhs.is_inf() {
                    match self {
                        ValueWrapper::Value(_) => {
                            Ok(ValueWrapper::from(<$type as Div<$type>>::Output::ZERO.clone()))
                        }
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "Invalid div between inf and inf!!!".to_string(),
                        }),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "Invalid div between -inf and inf!!!".to_string(),
                        }),
                    }
                } else if rhs.is_neg_inf() {
                    match self {
                        ValueWrapper::Value(_) => {
                            Ok(ValueWrapper::from(<$type as Div<$type>>::Output::ZERO.clone()))
                        }
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "Invalid div between inf and -inf!!!".to_string(),
                        }),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "Invalid div between -inf and -inf!!!".to_string(),
                        }),
                    }
                } else {
                    match self {
                        ValueWrapper::Value(lhs_value) => Ok(ValueWrapper::from(lhs_value / rhs)),
                        ValueWrapper::Inf => {
                            if &rhs >= $type::ZERO {
                                Ok(ValueWrapper::Inf)
                            } else {
                                Ok(ValueWrapper::NegInf)
                            }
                        }
                        ValueWrapper::NegInf => {
                            if &rhs >= $type::ZERO {
                                Ok(ValueWrapper::NegInf)
                            } else {
                                Ok(ValueWrapper::Inf)
                            }
                        }
                    }
                }
            }
        }

        impl<'a> Div<&'a $type> for ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Div<&'a $type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: &'a $type) -> Self::Output {
                if rhs.is_nan() {
                    Err(IllegalArgumentError {
                        msg: "Illegal argument NaN for value range!!!".to_string(),
                    })
                } else if rhs.is_inf() {
                    match self {
                        ValueWrapper::Value(_) => {
                            Ok(ValueWrapper::from(<$type as Div<$type>>::Output::ZERO.clone()))
                        }
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "Invalid div between inf and inf!!!".to_string(),
                        }),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "Invalid div between -inf and inf!!!".to_string(),
                        }),
                    }
                } else if rhs.is_neg_inf() {
                    match self {
                        ValueWrapper::Value(_) => {
                            Ok(ValueWrapper::from(<$type as Div<$type>>::Output::ZERO.clone()))
                        }
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "Invalid div between inf and -inf!!!".to_string(),
                        }),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "Invalid div between -inf and -inf!!!".to_string(),
                        }),
                    }
                } else {
                    match self {
                        ValueWrapper::Value(lhs_value) => Ok(ValueWrapper::from(lhs_value / rhs)),
                        ValueWrapper::Inf => {
                            if rhs >= $type::ZERO {
                                Ok(ValueWrapper::Inf)
                            } else {
                                Ok(ValueWrapper::NegInf)
                            }
                        }
                        ValueWrapper::NegInf => {
                            if rhs >= $type::ZERO {
                                Ok(ValueWrapper::NegInf)
                            } else {
                                Ok(ValueWrapper::Inf)
                            }
                        }
                    }
                }
            }
        }

        impl<'a> Div<$type> for &'a ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<&'a $type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: $type) -> Self::Output {
                if rhs.is_nan() {
                    Err(IllegalArgumentError {
                        msg: "Illegal argument NaN for value range!!!".to_string(),
                    })
                } else if rhs.is_inf() {
                    match self {
                        ValueWrapper::Value(_) => {
                            Ok(ValueWrapper::from(<$type as Div<$type>>::Output::ZERO.clone()))
                        }
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "Invalid div between inf and inf!!!".to_string(),
                        }),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "Invalid div between -inf and inf!!!".to_string(),
                        }),
                    }
                } else if rhs.is_neg_inf() {
                    match self {
                        ValueWrapper::Value(_) => {
                            Ok(ValueWrapper::from(<$type as Div<$type>>::Output::ZERO.clone()))
                        }
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "Invalid div between inf and -inf!!!".to_string(),
                        }),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "Invalid div between -inf and -inf!!!".to_string(),
                        }),
                    }
                } else {
                    match self {
                        ValueWrapper::Value(lhs_value) => Ok(ValueWrapper::from(lhs_value / rhs)),
                        ValueWrapper::Inf => {
                            if rhs >= *$type::ZERO {
                                Ok(ValueWrapper::Inf)
                            } else {
                                Ok(ValueWrapper::NegInf)
                            }
                        }
                        ValueWrapper::NegInf => {
                            if rhs >= *$type::ZERO {
                                Ok(ValueWrapper::NegInf)
                            } else {
                                Ok(ValueWrapper::Inf)
                            }
                        }
                    }
                }
            }
        }

        impl<'a, 'b> Div<&'b $type> for &'a ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<&'a $type as Div<&'b $type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: &'b $type) -> Self::Output {
                if rhs.is_nan() {
                    Err(IllegalArgumentError {
                        msg: "Illegal argument NaN for value range!!!".to_string(),
                    })
                } else if rhs.is_inf() {
                    match self {
                        ValueWrapper::Value(_) => {
                            Ok(ValueWrapper::from(<$type as Div<$type>>::Output::ZERO.clone()))
                            }
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "Invalid div between inf and inf!!!".to_string(),
                            }),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "Invalid div between -inf and inf!!!".to_string(),
                            }),
                    }
                } else if rhs.is_neg_inf() {
                    match self {
                        ValueWrapper::Value(_) => {
                            Ok(ValueWrapper::from(<$type as Div<$type>>::Output::ZERO.clone()))
                            }
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "Invalid div between inf and -inf!!!".to_string(),
                            }),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "Invalid div between -inf and -inf!!!".to_string(),
                            }),
                    }
                } else {
                    match self {
                        ValueWrapper::Value(lhs_value) => Ok(ValueWrapper::from(lhs_value / rhs)),
                        ValueWrapper::Inf => {
                            if rhs >= $type::ZERO {
                                Ok(ValueWrapper::Inf)
                            } else {
                                Ok(ValueWrapper::NegInf)
                            }
                        }
                        ValueWrapper::NegInf => {
                            if rhs >= $type::ZERO {
                                Ok(ValueWrapper::NegInf)
                            } else {
                                Ok(ValueWrapper::Inf)
                            }
                        }
                    }
                }
            }
        }

        impl Div<ValueWrapper<$type>> for ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: ValueWrapper<$type>) -> Self::Output {
                match self {
                    ValueWrapper::Value(lhs_value) => match rhs {
                        ValueWrapper::Value(rhs_value) => Ok(ValueWrapper::from(lhs_value / rhs_value)),
                        ValueWrapper::Inf | ValueWrapper::NegInf => {
                            Ok(ValueWrapper::from(<$type as Div<$type>>::Output::ZERO.clone()))
                        }
                    },
                    ValueWrapper::Inf => match rhs {
                        ValueWrapper::Value(rhs_value) => {
                            if &rhs_value >= $type::ZERO {
                                Ok(ValueWrapper::Inf)
                            } else {
                                Ok(ValueWrapper::NegInf)
                            }
                        }
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "Invalid div between inf and inf!!!".to_string(),
                        }),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "Invalid div between inf and -inf!!!".to_string(),
                        }),
                    },
                    ValueWrapper::NegInf => match rhs {
                        ValueWrapper::Value(rhs_value) => {
                            if &rhs_value >= $type::ZERO {
                                Ok(ValueWrapper::NegInf)
                            } else {
                                Ok(ValueWrapper::Inf)
                            }
                        }
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "Invalid div between -inf and inf!!!".to_string(),
                        }),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "Invalid div between -inf and -inf!!!".to_string(),
                        }),
                    },
                }
            }
        }

        impl<'a> Div<ValueWrapper<$type>> for &'a ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<&'a $type as Div<$type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: ValueWrapper<$type>) -> Self::Output {
                match self {
                    ValueWrapper::Value(lhs_value) => match rhs {
                        ValueWrapper::Value(rhs_value) => Ok(ValueWrapper::from(lhs_value / rhs_value)),
                        ValueWrapper::Inf | ValueWrapper::NegInf => {
                            Ok(ValueWrapper::from(<$type as Div<$type>>::Output::ZERO.clone()))
                        }
                    },
                    ValueWrapper::Inf => match rhs {
                        ValueWrapper::Value(rhs_value) => {
                            if rhs_value >= *$type::ZERO {
                                Ok(ValueWrapper::Inf)
                            } else {
                                Ok(ValueWrapper::NegInf)
                            }
                        }
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "Invalid div between inf and inf!!!".to_string(),
                        }),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "Invalid div between inf and -inf!!!".to_string(),
                        }),
                    },
                    ValueWrapper::NegInf => match rhs {
                        ValueWrapper::Value(rhs_value) => {
                            if rhs_value >= *$type::ZERO {
                                Ok(ValueWrapper::NegInf)
                            } else {
                                Ok(ValueWrapper::Inf)
                            }
                        }
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "Invalid div between -inf and inf!!!".to_string(),
                        }),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "Invalid div between -inf and -inf!!!".to_string(),
                        }),
                    },
                }
            }
        }

        impl<'a> Div<&'a ValueWrapper<$type>> for ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<$type as Div<&'a $type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: &'a ValueWrapper<$type>) -> Self::Output {
                match self {
                    ValueWrapper::Value(lhs_value) => match rhs {
                        ValueWrapper::Value(rhs_value) => Ok(ValueWrapper::from(lhs_value / rhs_value)),
                        ValueWrapper::Inf | ValueWrapper::NegInf => {
                            Ok(ValueWrapper::from(<$type as Div<$type>>::Output::ZERO.clone()))
                        }
                    },
                    ValueWrapper::Inf => match rhs {
                        ValueWrapper::Value(rhs_value) => {
                            if rhs_value >= $type::ZERO {
                                Ok(ValueWrapper::Inf)
                            } else {
                                Ok(ValueWrapper::NegInf)
                            }
                        }
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "Invalid div between inf and inf!!!".to_string(),
                        }),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "Invalid div between inf and -inf!!!".to_string(),
                        }),
                    },
                    ValueWrapper::NegInf => match rhs {
                        ValueWrapper::Value(rhs_value) => {
                            if rhs_value >= $type::ZERO {
                                Ok(ValueWrapper::NegInf)
                            } else {
                                Ok(ValueWrapper::Inf)
                            }
                        }
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "Invalid div between -inf and inf!!!".to_string(),
                        }),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "Invalid div between -inf and -inf!!!".to_string(),
                        }),
                    },
                }
            }
        }

        impl<'a, 'b> Div<&'b ValueWrapper<$type>> for &'a ValueWrapper<$type> {
            type Output = Result<ValueWrapper<<&'a $type as Div<&'b $type>>::Output>, IllegalArgumentError>;

            fn div(self, rhs: &'b ValueWrapper<$type>) -> Self::Output {
                match self {
                    ValueWrapper::Value(lhs_value) => match rhs {
                        ValueWrapper::Value(rhs_value) => Ok(ValueWrapper::from(lhs_value / rhs_value)),
                        ValueWrapper::Inf | ValueWrapper::NegInf => {
                            Ok(ValueWrapper::from(<$type as Div<$type>>::Output::ZERO.clone()))
                        }
                    },
                    ValueWrapper::Inf => match rhs {
                        ValueWrapper::Value(rhs_value) => {
                            if rhs_value >= $type::ZERO {
                                Ok(ValueWrapper::Inf)
                            } else {
                                Ok(ValueWrapper::NegInf)
                            }
                        }
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "Invalid div between inf and inf!!!".to_string(),
                        }),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "Invalid div between inf and -inf!!!".to_string(),
                        }),
                    },
                    ValueWrapper::NegInf => match rhs {
                        ValueWrapper::Value(rhs_value) => {
                            if rhs_value >= $type::ZERO {
                                Ok(ValueWrapper::NegInf)
                            } else {
                                Ok(ValueWrapper::Inf)
                            }
                        }
                        ValueWrapper::Inf => Err(IllegalArgumentError {
                            msg: "Invalid div between -inf and inf!!!".to_string(),
                        }),
                        ValueWrapper::NegInf => Err(IllegalArgumentError {
                            msg: "Invalid div between -inf and -inf!!!".to_string(),
                        }),
                    },
                }
            }
        }
    )*)
}
real_number_value_wrapper_template! { u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize f32 f64 }

// impl<T: SemiArithmetic, U: SemiArithmetic> Mul<U> for ValueWrapper<T>
// where
//     T: Mul<U>,
//     <T as Mul<U>>::Output: SemiArithmetic,
// {
//     type Output = Result<ValueWrapper<<T as Mul<U>>::Output>, IllegalArgumentError>;
//
//     default fn mul(self, rhs: U) -> Self::Output {
//         match self {
//             ValueWrapper::Value(lhs_value) => Ok(ValueWrapper::from(lhs_value * rhs)),
//             ValueWrapper::Inf => {
//                 if &rhs >= U::ZERO {
//                     Ok(ValueWrapper::Inf)
//                 } else {
//                     Ok(ValueWrapper::NegInf)
//                 }
//             }
//             ValueWrapper::NegInf => {
//                 if &rhs >= U::ZERO {
//                     Ok(ValueWrapper::NegInf)
//                 } else {
//                     Ok(ValueWrapper::Inf)
//                 }
//             }
//         }
//     }
// }
//
// impl<T: SemiArithmetic, U: SemiArithmetic> Div<U> for ValueWrapper<T>
// where
//     T: Div<U>,
//     <T as Div<U>>::Output: SemiArithmetic,
// {
//     type Output = Result<ValueWrapper<<T as Div<U>>::Output>, IllegalArgumentError>;
//
//     default fn div(self, rhs: U) -> Self::Output {
//         match self {
//             ValueWrapper::Value(lhs_value) => Ok(ValueWrapper::from(lhs_value / rhs)),
//             ValueWrapper::Inf => {
//                 if &rhs >= U::ZERO {
//                     Ok(ValueWrapper::Inf)
//                 } else {
//                     Ok(ValueWrapper::NegInf)
//                 }
//             }
//             ValueWrapper::NegInf => {
//                 if &rhs >= U::ZERO {
//                     Ok(ValueWrapper::NegInf)
//                 } else {
//                     Ok(ValueWrapper::Inf)
//                 }
//             }
//         }
//     }
// }
