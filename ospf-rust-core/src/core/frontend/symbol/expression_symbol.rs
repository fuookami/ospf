use ospf_rust_math::Category;
use ospf_rust_math::polynomial::Polynomial;
use super::intermediate_symbol::IntermediateSymbol;

pub trait ExpressionSymbol<const category: Category, T = f64> : IntermediateSymbol<category> {
    type PolynomialType: Polynomial<T>;
}

pub trait LinearExpressionSymbol<T = f64> : ExpressionSymbol<{ Category::Linear }, T> {}

pub trait QuadraticExpressionSymbol<T = f64> : ExpressionSymbol<{ Category::Quadratic }, T> {}
