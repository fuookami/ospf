use crate::math::concept::*;
use crate::math::value_range::*;
use std::convert::Into;
use std::{f64, i128, i8, u128, u8};

pub struct Binary;
pub struct Ternary;
pub struct BalancedTernary;
pub struct Percentage;
pub struct Integer;
pub struct UInteger;
pub struct Continuous;
pub struct UContinuous;

pub trait VariableType {
    type ValueRangeType: ValueRange;
    type ValueType: Arithmetic + Constant + Into<<Self::ValueRangeType as ValueRange>::ValueType>;

    fn new() -> Self;

    fn default_minimum() -> Self::ValueType;
    fn default_maximum() -> Self::ValueType;

    fn name() -> &'static str;
    fn short_name() -> &'static str;
}

impl VariableType for Binary {
    type ValueType = u8;
    type ValueRangeType = ValueRangeType<u8>;

    fn new() -> Self {
        Self {}
    }

    fn default_minimum() -> u8 {
        0
    }
    fn default_maximum() -> u8 {
        1
    }

    fn name() -> &'static str {
        "Binary"
    }
    fn short_name() -> &'static str {
        "Bin"
    }
}

impl VariableType for Ternary {
    type ValueType = u8;
    type ValueRangeType = ValueRangeType<u8>;

    fn new() -> Self {
        Self {}
    }

    fn default_minimum() -> u8 {
        0
    }
    fn default_maximum() -> u8 {
        2
    }

    fn name() -> &'static str {
        "Ternary"
    }
    fn short_name() -> &'static str {
        "Ter"
    }
}

impl VariableType for BalancedTernary {
    type ValueType = i8;
    type ValueRangeType = ValueRangeType<i8>;

    fn new() -> Self {
        Self {}
    }

    fn default_minimum() -> i8 {
        -1
    }
    fn default_maximum() -> i8 {
        1
    }

    fn name() -> &'static str {
        "BalancedTernary"
    }
    fn short_name() -> &'static str {
        "BTer"
    }
}

impl VariableType for Percentage {
    type ValueType = f64;
    type ValueRangeType = ValueRangeType<f64>;

    fn new() -> Self {
        Self {}
    }

    fn default_minimum() -> f64 {
        0.
    }
    fn default_maximum() -> f64 {
        1.
    }

    fn name() -> &'static str {
        "Percentage"
    }
    fn short_name() -> &'static str {
        "Pct"
    }
}

impl VariableType for Integer {
    type ValueType = i128;
    type ValueRangeType = ValueRangeType<i128>;

    fn new() -> Self {
        Self {}
    }

    fn default_minimum() -> i128 {
        i128::MIN
    }
    fn default_maximum() -> i128 {
        i128::MAX
    }

    fn name() -> &'static str {
        "Integer"
    }
    fn short_name() -> &'static str {
        "Int"
    }
}

impl VariableType for UInteger {
    type ValueType = u128;
    type ValueRangeType = ValueRangeType<u128>;

    fn new() -> Self {
        Self {}
    }

    fn default_minimum() -> u128 {
        u128::MIN
    }
    fn default_maximum() -> u128 {
        u128::MAX
    }

    fn name() -> &'static str {
        "UInteger"
    }
    fn short_name() -> &'static str {
        "UInt"
    }
}

impl VariableType for Continuous {
    type ValueType = f64;
    type ValueRangeType = ValueRangeType<f64>;

    fn new() -> Self {
        Self {}
    }

    fn default_minimum() -> f64 {
        f64::NEG_INFINITY
    }
    fn default_maximum() -> f64 {
        f64::INFINITY
    }

    fn name() -> &'static str {
        "Continuous"
    }
    fn short_name() -> &'static str {
        "Real"
    }
}

impl VariableType for UContinuous {
    type ValueType = f64;
    type ValueRangeType = ValueRangeType<f64>;

    fn new() -> Self {
        Self {}
    }

    fn default_minimum() -> f64 {
        0.
    }
    fn default_maximum() -> f64 {
        f64::INFINITY
    }

    fn name() -> &'static str {
        "UContinuous"
    }
    fn short_name() -> &'static str {
        "UReal"
    }
}
