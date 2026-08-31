use super::intermediate_symbol::IntermediateSymbol;
use crate::core::frontend::token::{Evaluate, TokenValueType};
use ospf_rust_math::{Bounded, Category, RealNumber};
use std::fmt::Display;

pub trait FunctionSymbol<const category: Category, T: RealNumber + TokenValueType>:
    IntermediateSymbol<category, T> + Evaluate<T, ResultType = T>
{
}

pub trait LogicFunctionSymbol<const category: Category, T: RealNumber + TokenValueType>:
    FunctionSymbol<category, T>
{
}

pub trait LinearFunctionSymbol<T: RealNumber + TokenValueType = f64>:
    FunctionSymbol<{ Category::Linear }, T>
{
}
pub trait LinearLogicFunctionSymbol<T: RealNumber + TokenValueType = f64>:
    LogicFunctionSymbol<{ Category::Linear }, T>
{
}

pub trait QuadraticFunctionSymbol<T: RealNumber + TokenValueType = f64>:
    FunctionSymbol<{ Category::Quadratic }, T>
{
}
pub trait QuadraticLogicFunctionSymbol<T: RealNumber + TokenValueType = f64>:
    LogicFunctionSymbol<{ Category::Quadratic }, T>
{
}
