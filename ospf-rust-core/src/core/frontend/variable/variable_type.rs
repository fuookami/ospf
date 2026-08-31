use std::{f64, i128, i8, u128, u8};
use std::convert::Into;

use ospf_rust_math::algebra::concept::*;
use ospf_rust_math::algebra::value_range::*;

pub trait VariableType: Copy + Eq {
    type VariableValueType;

    const MINIMUM: &'static Self::VariableValueType;
    const MAXIMUM: &'static Self::VariableValueType;

    const NAME: &'static str;
    const SHORT_NAME: &'static str;

    fn new() -> Self;

    fn is_binary() -> bool {
        false
    }

    fn is_integer() -> bool {
        false
    }

    fn is_unsigned_integer() -> bool {
        false
    }

    fn is_continuous() -> bool {
        !Self::is_integer()
    }

    fn is_not_binary_integer() -> bool {
        !Self::is_binary() && Self::is_integer()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Binary;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Ternary;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BalancedTernary;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Percentage;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Integer;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UInteger;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Continuous;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UContinuous;
