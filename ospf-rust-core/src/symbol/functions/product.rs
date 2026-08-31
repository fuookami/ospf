//! Quadratic product function symbol.
//!
//! `ProductFunction` models the product of two linear polynomials:
//! `y = left * right`.
//! The symbol itself is quadratic and can be used in expression/evaluation
//! pipelines even when no linearized mechanism constraints are required.

use std::any::Any;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;

use num_traits::Zero;
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};

use crate::error::Result;
use crate::model::LinearConstraint;
use crate::symbol::flatten::{Linear, Quadratic, QuadraticMonomial};
use crate::token::{Token, TokenList};

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, QuadraticIntermediateSymbol,
};

fn evaluate_linear<V>(
    poly: &Linear<V>,
    token_table: &dyn TokenList<V>,
    zero_if_none: bool,
) -> Option<V>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V> + Zero,
{
    let mut value = poly.constant_term().clone();
    for monomial in poly.monomials() {
        let term_value = match token_table
            .find_by_index(monomial.var_index())
            .and_then(|token| token.get_result())
        {
            Some(v) => v,
            None if zero_if_none => V::zero(),
            None => return None,
        };
        value = value + monomial.coefficient().clone() * term_value;
    }
    Some(value)
}

fn evaluate_linear_from_values<V>(
    poly: &Linear<V>,
    values: &std::collections::HashMap<usize, V>,
) -> Option<V>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V>,
{
    let mut value = poly.constant_term().clone();
    for monomial in poly.monomials() {
        let term_value = values.get(&monomial.var_index())?.clone();
        value = value + monomial.coefficient().clone() * term_value;
    }
    Some(value)
}

/// Product of two linear polynomials.
#[derive(Debug, Clone)]
pub struct ProductFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    left: Linear<V>,
    right: Linear<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> ProductFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    pub fn new(id: u64, name: &str, left: Linear<V>, right: Linear<V>) -> Self {
        Self {
            id: IntermediateSymbolId::new(id, name),
            left,
            right,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn left_polynomial(&self) -> &Linear<V> {
        &self.left
    }

    pub fn right_polynomial(&self) -> &Linear<V> {
        &self.right
    }
}

impl<V> Display for ProductFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "product({})", self.id.name)
    }
}

impl<V> DynSymbol for ProductFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn name(&self) -> &str {
        &self.id.name
    }

    fn display_name(&self) -> &str {
        &self.id.name
    }

    fn dyn_id(&self) -> SymbolDynId<'_> {
        SymbolDynId::standalone(self.id.id as usize)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<V> Symbol for ProductFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for ProductFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V> + Zero,
{
    fn category(&self) -> Category {
        Category::Quadratic
    }

    fn cached(&self) -> bool {
        false
    }

    fn dependencies(&self) -> HashSet<Arc<dyn IntermediateSymbol<V>>> {
        HashSet::new()
    }

    fn declared_dependency_ids(&self) -> Vec<u64> {
        self.declared_dependency_ids.clone()
    }

    fn flush(&self, _force: bool) {}

    fn register_auxiliary_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        <Self as FunctionSymbol<V>>::register_tokens(self, tokens)
    }

    fn mechanism_constraints(
        &self,
        _symbol_to_index: &std::collections::HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        Ok(Vec::new())
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &std::collections::HashMap<usize, V>) -> Option<V> {
        let left = evaluate_linear_from_values(&self.left, values)?;
        let right = evaluate_linear_from_values(&self.right, values)?;
        Some(left * right)
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("product({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for ProductFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V> + Zero,
{
    fn register_tokens(&self, _tokens: &mut Vec<Token<V>>) -> Result<()> {
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let left = evaluate_linear(&self.left, token_table, zero_if_none)?;
        let right = evaluate_linear(&self.right, token_table, zero_if_none)?;
        Some(left * right)
    }
}

impl<V> QuadraticIntermediateSymbol<V> for ProductFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V> + Zero,
{
    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        let left_const = self.left.constant_term().clone();
        let right_const = self.right.constant_term().clone();

        let mut monomials = Vec::with_capacity(
            self.left.monomials().len() * self.right.monomials().len()
                + self.left.monomials().len()
                + self.right.monomials().len(),
        );

        // quadratic terms: (a_i x_i) * (b_j x_j)
        for left in self.left.monomials() {
            for right in self.right.monomials() {
                monomials.push(QuadraticMonomial::new_quadratic(
                    left.coefficient().clone() * right.coefficient().clone(),
                    left.var_index(),
                    right.var_index(),
                ));
            }
        }

        // linear terms from constant cross terms
        for left in self.left.monomials() {
            monomials.push(QuadraticMonomial::new_linear(
                left.coefficient().clone() * right_const.clone(),
                left.var_index(),
            ));
        }
        for right in self.right.monomials() {
            monomials.push(QuadraticMonomial::new_linear(
                right.coefficient().clone() * left_const.clone(),
                right.var_index(),
            ));
        }

        Quadratic::new(monomials, left_const * right_const)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::flatten::{LinearMonomial, QuadraticMonomialKind};
    use crate::token::{MutableTokenList, VecTokenList};
    use crate::variable::{ContinuousVariableItem, VariableId};

    #[test]
    fn product_function_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let y = ContinuousVariableItem::create(VariableId::standalone(1), "y");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(3.0);
        tokens.add_token(tx);
        let ty = Token::from_generic(y, 1);
        ty.set_result(5.0);
        tokens.add_token(ty);

        let left = Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0); // 2x + 1 = 7
        let right = Linear::new(vec![LinearMonomial::new(1.0, 1)], -4.0); // y - 4 = 1
        let product = ProductFunction::new(9000, "prod", left, right);

        assert_eq!(product.calculate_value(&tokens, false), Some(7.0));
    }

    #[test]
    fn product_function_to_quadratic_polynomial() {
        let left = Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0); // 2x + 1
        let right = Linear::new(vec![LinearMonomial::new(3.0, 1)], 4.0); // 3y + 4
        let product = ProductFunction::new(9001, "prod_poly", left, right);
        let quadratic = product.to_quadratic_polynomial();

        assert_eq!(*quadratic.constant(), 4.0);

        let mut has_xy = false;
        let mut has_x = false;
        let mut has_y = false;

        for monomial in quadratic.monomials() {
            match monomial.kind() {
                QuadraticMonomialKind::Quadratic => {
                    if monomial.var_index1() == 0
                        && monomial.var_index2() == Some(1)
                        && (*monomial.coefficient() - 6.0_f64).abs() <= 1e-9
                    {
                        has_xy = true;
                    }
                }
                QuadraticMonomialKind::Linear => {
                    if monomial.var_index1() == 0
                        && (*monomial.coefficient() - 8.0_f64).abs() <= 1e-9
                    {
                        has_x = true;
                    }
                    if monomial.var_index1() == 1
                        && (*monomial.coefficient() - 3.0_f64).abs() <= 1e-9
                    {
                        has_y = true;
                    }
                }
            }
        }

        assert!(has_xy);
        assert!(has_x);
        assert!(has_y);
    }
}
