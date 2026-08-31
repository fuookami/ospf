use ospf_rust_math::Category;
use ospf_rust_math::polynomial::Polynomial;
use super::intermediate_symbol::IntermediateSymbol;

pub trait FunctionSymbol<const category: Category, T> : IntermediateSymbol<category> {
    type PolynomialType: Polynomial<T>;
}

pub trait LogicFunctionSymbol<const category: Category, T> : FunctionSymbol<category, T> {

}

pub trait LinearFunctionSymbol<T> : FunctionSymbol<{ Category::Linear }, T> {}
pub trait LinearLogicFunctionSymbol<T> : LogicFunctionSymbol<{ Category::Linear }, T> {}

pub trait QuadraticFunctionSymbol<T> : FunctionSymbol<{ Category::Quadratic }, T> {}
pub trait QuadraticLogicFunctionSymbol<T> : LogicFunctionSymbol<{ Category::Quadratic }, T> {}
