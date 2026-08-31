use std::convert::Into;
use std::fmt;
use std::fmt::Display;

use ospf_rust_math::value_range::*;

use super::variable_type::VariableType;

#[derive(Clone)]
pub struct VariableRange<Type: VariableType> {
    _range: ValueRange<Type::VariableValueType>,
}

impl<Type: VariableType> VariableRange<Type> {
    pub fn new() -> Result<Self, IllegalArgumentError> {
        Ok(Self {
            _range: ValueRange::<Type::VariableValueType>::new_with(
                Type::VariableValueType::MINIMUM,
                Type::VariableValueType::MAXIMUM,
                Interval::Closed,
                Interval::Closed
            )?
        })
    }
}

impl<Type: VariableType> Display for VariableRange<Type> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self._range)
    }
}
