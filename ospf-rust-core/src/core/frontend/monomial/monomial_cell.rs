use ospf_rust_math::Arithmetic;

use crate::core::frontend::token::{
    AbstractTokenList, Evaluate, TokenValueType, VariableItemWrapper,
};
use crate::core::frontend::VariableItem;

pub trait MonomialCell<T>: Evaluate<T, ResultType = T> {
    fn is_constant(&self) -> bool;
    fn constant(&self) -> Option<&T>;
}

pub struct IllegalOperation {
    pub(crate) operation: String,
    pub(crate) left_variable: Option<VariableItemWrapper>,
    pub(crate) right_variable: Option<VariableItemWrapper>,
}
