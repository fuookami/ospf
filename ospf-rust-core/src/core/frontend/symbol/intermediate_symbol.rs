use crate::core::frontend::expression::Expression;
use crate::core::frontend::polynomial::Polynomial;
use crate::core::frontend::token::{Evaluate, TokenValueType};
use ospf_rust_math::{Bounded, Category, CompositeSymbol, RealNumber, SymbolIdentify};
use std::fmt::Display;

pub struct IntermediateSymbolIdentifier {
    name: String,
}

pub trait IntermediateSymbol<const category: Category, T: RealNumber + TokenValueType>:
    SymbolIdentify<Identifier = IntermediateSymbolIdentifier> + CompositeSymbol + Expression<T>
{
    fn set_name(&self, name: &str);
}

pub trait LinearIntermediateSymbol<T: RealNumber + TokenValueType = f64>:
    IntermediateSymbol<{ Category::Linear }, T>
{
}
pub trait QuadraticIntermediateSymbol<T: RealNumber + TokenValueType = f64>:
    IntermediateSymbol<{ Category::Quadratic }, T>
{
}
