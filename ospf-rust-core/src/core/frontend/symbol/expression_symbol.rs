use super::intermediate_symbol::IntermediateSymbol;
use crate::core::frontend::polynomial::MutablePolynomial;
use crate::core::frontend::token::{Evaluate, TokenValueType};
use ospf_rust_math::polynomial::Polynomial;
use ospf_rust_math::{Bounded, Category, RealNumber};
use std::cell::RefCell;
use std::fmt::Display;

pub trait ExpressionSymbol<const category: Category, T: RealNumber + TokenValueType>:
    IntermediateSymbol<category, T> + Evaluate<T, ResultType = T>
{
    type PolynomialType: MutablePolynomial<category, T>;

    fn polynomial(&self) -> impl Polynomial;
    fn as_mutable(&self) -> &mut Self::PolynomialType;
}

pub trait LinearExpressionSymbol<T: RealNumber + TokenValueType = f64>:
    ExpressionSymbol<{ Category::Linear }, T>
{
}

pub trait QuadraticExpressionSymbol<T: RealNumber + TokenValueType = f64>:
    ExpressionSymbol<{ Category::Quadratic }, T>
{
}
