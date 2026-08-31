//! Function symbol trait.

use std::fmt::Debug;
use crate::error::Result;
use crate::token::{Token, TokenList};
use super::{IntermediateSymbol, LinearIntermediateSymbol};

/// Function symbol abstraction (min/max/abs/etc.).
pub trait FunctionSymbol<V = f64>: IntermediateSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// Register auxiliary tokens required by this function symbol.
    fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()>;

    /// Calculate function value from token values.
    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V>;
}

pub trait LinearFunctionSymbol<V = f64>: FunctionSymbol<V> + LinearIntermediateSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
}

impl<T, V> LinearFunctionSymbol<V> for T
where
    T: FunctionSymbol<V> + LinearIntermediateSymbol<V>,
    V: Clone + Debug + Send + Sync + 'static,
{
}

pub trait QuadraticFunctionSymbol<V = f64>: LinearFunctionSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
}

pub trait LogicFunctionSymbol<V = f64>: LinearFunctionSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic, QuadraticMonomial};
    use crate::symbol::function::{AndFunction, QuadraticSigmoidFunction, SigmoidFunction};

    fn assert_linear<T: LinearFunctionSymbol>() {}
    fn assert_logic<T: LogicFunctionSymbol>() {}
    fn assert_quadratic<T: QuadraticFunctionSymbol>() {}

    #[test]
    fn linear_and_logic_function_symbol_traits_compile() {
        assert_linear::<SigmoidFunction>();
        assert_logic::<AndFunction>();
    }

    #[test]
    fn quadratic_function_symbol_trait_compile() {
        assert_quadratic::<QuadraticSigmoidFunction>();

        let _ = QuadraticSigmoidFunction::new(
            9990,
            "qs",
            Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0),
        );
        let _ = SigmoidFunction::new(
            9991,
            "s",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );
    }
}
