use std::{f64, i8, u8};

use ospf_rust_math::algebra::concept::*;
use ospf_rust_math::algebra::value_range::*;

// for static

pub trait VariableTypeBound {
    type ValueType: RealNumber;

    const MINIMUM: &'static Self::ValueType;
    const MAXIMUM: &'static Self::ValueType;
}

pub trait AbstractVariableType: VariableTypeBound + Copy + Eq {
    const NAME: &'static str;
    const SHORT_NAME: &'static str;

    fn instance() -> Self;

    fn is_binary() -> bool {
        false
    }

    fn is_unsigned() -> bool {
        false
    }

    fn is_integer() -> bool {
        false
    }

    fn is_unsigned_integer() -> bool {
        Self::is_integer() && Self::is_unsigned()
    }

    fn is_continuous() -> bool {
        !Self::is_integer()
    }

    fn is_unsigned_continuous() -> bool {
        Self::is_continuous() && Self::is_unsigned()
    }

    fn is_not_binary_integer() -> bool {
        !Self::is_binary() && Self::is_integer()
    }

    fn is_discrete() -> bool {
        Self::is_binary() || Self::is_integer()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binary;

impl VariableTypeBound for Binary {
    type ValueType = u8;

    const MINIMUM: &'static Self::ValueType = &0;
    const MAXIMUM: &'static Self::ValueType = &1;
}

impl AbstractVariableType for Binary {
    const NAME: &'static str = "binary";
    const SHORT_NAME: &'static str = "bin";

    fn instance() -> Self {
        Binary
    }

    fn is_binary() -> bool {
        true
    }

    fn is_unsigned() -> bool {
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ternary;

impl VariableTypeBound for Ternary {
    type ValueType = u8;

    const MINIMUM: &'static Self::ValueType = &0;
    const MAXIMUM: &'static Self::ValueType = &2;
}

impl AbstractVariableType for Ternary {
    const NAME: &'static str = "ternary";
    const SHORT_NAME: &'static str = "ter";

    fn instance() -> Self {
        Ternary
    }

    fn is_unsigned() -> bool {
        true
    }

    fn is_integer() -> bool {
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BalancedTernary;

impl VariableTypeBound for BalancedTernary {
    type ValueType = i8;

    const MINIMUM: &'static Self::ValueType = &-1;
    const MAXIMUM: &'static Self::ValueType = &1;
}

impl AbstractVariableType for BalancedTernary {
    const NAME: &'static str = "balanced_ternary";
    const SHORT_NAME: &'static str = "bter";

    fn instance() -> Self {
        BalancedTernary
    }

    fn is_integer() -> bool {
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Percentage;

impl VariableTypeBound for Percentage {
    type ValueType = f64;

    const MINIMUM: &'static Self::ValueType = &0.0;
    const MAXIMUM: &'static Self::ValueType = &1.0;
}

impl AbstractVariableType for Percentage {
    const NAME: &'static str = "percentage";
    const SHORT_NAME: &'static str = "pct";

    fn instance() -> Self {
        Percentage
    }

    fn is_unsigned() -> bool {
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Integer;

impl VariableTypeBound for Integer {
    type ValueType = i64;

    const MINIMUM: &'static Self::ValueType = i64::MINIMUM.as_ref().unwrap();
    const MAXIMUM: &'static Self::ValueType = i64::MAXIMUM.as_ref().unwrap();
}

impl AbstractVariableType for Integer {
    const NAME: &'static str = "integer";
    const SHORT_NAME: &'static str = "int";

    fn instance() -> Self {
        Integer
    }

    fn is_integer() -> bool {
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UInteger;

impl VariableTypeBound for UInteger {
    type ValueType = u64;

    const MINIMUM: &'static Self::ValueType = u64::MINIMUM.as_ref().unwrap();
    const MAXIMUM: &'static Self::ValueType = u64::MAXIMUM.as_ref().unwrap();
}

impl AbstractVariableType for UInteger {
    const NAME: &'static str = "uinteger";
    const SHORT_NAME: &'static str = "uint";

    fn instance() -> Self {
        UInteger
    }

    fn is_unsigned() -> bool {
        true
    }

    fn is_integer() -> bool {
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Continuous;

impl VariableTypeBound for Continuous {
    type ValueType = f64;

    const MINIMUM: &'static Self::ValueType = f64::MINIMUM.as_ref().unwrap();
    const MAXIMUM: &'static Self::ValueType = f64::MAXIMUM.as_ref().unwrap();
}

impl AbstractVariableType for Continuous {
    const NAME: &'static str = "continuous";
    const SHORT_NAME: &'static str = "real";

    fn instance() -> Self {
        Continuous
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UContinuous;

impl VariableTypeBound for UContinuous {
    type ValueType = f64;

    const MINIMUM: &'static Self::ValueType = &0.0;
    const MAXIMUM: &'static Self::ValueType = f64::MAXIMUM.as_ref().unwrap();
}

impl AbstractVariableType for UContinuous {
    const NAME: &'static str = "continuous";
    const SHORT_NAME: &'static str = "real";

    fn instance() -> Self {
        UContinuous
    }

    fn is_unsigned() -> bool {
        true
    }
}

// for dynamic

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VariableType {
    Binary,
    Ternary,
    BalancedTernary,
    Percentage,
    Integer,
    UInteger,
    Continuous,
    UContinuous,
}

impl From<Binary> for VariableType {
    fn from(_: Binary) -> Self {
        VariableType::Binary
    }
}

impl From<Ternary> for VariableType {
    fn from(_: Ternary) -> Self {
        VariableType::Ternary
    }
}

impl From<BalancedTernary> for VariableType {
    fn from(_: BalancedTernary) -> Self {
        VariableType::BalancedTernary
    }
}

impl From<Percentage> for VariableType {
    fn from(_: Percentage) -> Self {
        VariableType::Percentage
    }
}

impl From<Integer> for VariableType {
    fn from(_: Integer) -> Self {
        VariableType::Integer
    }
}

impl From<UInteger> for VariableType {
    fn from(_: UInteger) -> Self {
        VariableType::UInteger
    }
}

impl From<Continuous> for VariableType {
    fn from(_: Continuous) -> Self {
        VariableType::Continuous
    }
}

impl From<UContinuous> for VariableType {
    fn from(_: UContinuous) -> Self {
        VariableType::UContinuous
    }
}

impl VariableType {
    pub fn name(&self) -> &'static str {
        match self {
            VariableType::Binary => Binary::NAME,
            VariableType::Ternary => Ternary::NAME,
            VariableType::BalancedTernary => BalancedTernary::NAME,
            VariableType::Percentage => Percentage::NAME,
            VariableType::Integer => Integer::NAME,
            VariableType::UInteger => UInteger::NAME,
            VariableType::Continuous => Continuous::NAME,
            VariableType::UContinuous => UContinuous::NAME,
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            VariableType::Binary => Binary::SHORT_NAME,
            VariableType::Ternary => Ternary::SHORT_NAME,
            VariableType::BalancedTernary => BalancedTernary::SHORT_NAME,
            VariableType::Percentage => Percentage::SHORT_NAME,
            VariableType::Integer => Integer::SHORT_NAME,
            VariableType::UInteger => UInteger::SHORT_NAME,
            VariableType::Continuous => Continuous::SHORT_NAME,
            VariableType::UContinuous => UContinuous::SHORT_NAME,
        }
    }

    pub fn is_binary(&self) -> bool {
        match self {
            VariableType::Binary => Binary::is_binary(),
            VariableType::Ternary => Ternary::is_binary(),
            VariableType::BalancedTernary => BalancedTernary::is_binary(),
            VariableType::Percentage => Percentage::is_binary(),
            VariableType::Integer => Integer::is_binary(),
            VariableType::UInteger => UInteger::is_binary(),
            VariableType::Continuous => Continuous::is_binary(),
            VariableType::UContinuous => UContinuous::is_binary(),
        }
    }

    pub fn is_unsigned(&self) -> bool {
        match self {
            VariableType::Binary => Binary::is_unsigned(),
            VariableType::Ternary => Ternary::is_unsigned(),
            VariableType::BalancedTernary => BalancedTernary::is_unsigned(),
            VariableType::Percentage => Percentage::is_unsigned(),
            VariableType::Integer => Integer::is_unsigned(),
            VariableType::UInteger => UInteger::is_unsigned(),
            VariableType::Continuous => Continuous::is_unsigned(),
            VariableType::UContinuous => UContinuous::is_unsigned(),
        }
    }

    pub fn is_integer(&self) -> bool {
        match self {
            VariableType::Binary => Binary::is_integer(),
            VariableType::Ternary => Ternary::is_integer(),
            VariableType::BalancedTernary => BalancedTernary::is_integer(),
            VariableType::Percentage => Percentage::is_integer(),
            VariableType::Integer => Integer::is_integer(),
            VariableType::UInteger => UInteger::is_integer(),
            VariableType::Continuous => Continuous::is_integer(),
            VariableType::UContinuous => UContinuous::is_integer(),
        }
    }

    pub fn is_unsigned_integer(&self) -> bool {
        self.is_integer() && self.is_unsigned()
    }

    pub fn is_continuous(&self) -> bool {
        !self.is_integer()
    }

    pub fn is_unsigned_continuous(&self) -> bool {
        self.is_continuous() && self.is_unsigned()
    }

    pub fn is_not_binary_integer(&self) -> bool {
        !self.is_binary() && self.is_integer()
    }

    fn is_discrete(&self) -> bool {
        self.is_binary() || self.is_integer()
    }
}
