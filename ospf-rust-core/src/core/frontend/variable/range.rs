use super::variable_type::VariableType;
use crate::math::value_range::*;
use std::convert::Into;
use std::fmt;
use std::fmt::Display;

#[derive(Clone)]
pub struct VariableRange<Type: VariableType> {
    _range: Type::ValueRangeType,
}

impl<Type: VariableType> VariableRange<Type> {
    pub fn new() -> Self {
        Self {
            _range: Type::ValueRangeType::new_range(
                &Type::default_minimum().into(),
                &Type::default_maximum().into(),
            ),
        }
    }
}

impl<Type: VariableType> Display for VariableRange<Type> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self._range)
    }
}
