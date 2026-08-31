//! 二次线性桥接函数 / Quadratic linear bridge function

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use crate::error::{ModelError, Result};
use crate::model::{
    ConstraintRelation, LinearConstraint, LinearInequality, QuadraticConstraint,
    QuadraticInequality,
};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic, QuadraticMonomial};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{ContinuousVariableItem, new_standalone_id};
use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    QuadraticFunctionSymbol,
};

pub(super) const MIN_BIG_M: f64 = 1.0;

pub(super) fn evaluate_quadratic<V>(
    poly: &Quadratic<V>,
    token_table: &dyn TokenList<V>,
    zero_if_none: bool,
) -> Option<V>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V> + Zero,
{
    let mut value = poly.constant().clone();
    for monomial in poly.monomials() {
        let value1 = match token_table
            .find_by_index(monomial.var_index1())
            .and_then(|token| token.get_result())
        {
            Some(v) => v,
            None if zero_if_none => V::zero(),
            None => return None,
        };

        let term = if let Some(var2) = monomial.var_index2() {
            let value2 = match token_table
                .find_by_index(var2)
                .and_then(|token| token.get_result())
            {
                Some(v) => v,
                None if zero_if_none => V::zero(),
                None => return None,
            };
            monomial.coefficient().clone() * value1 * value2
        } else {
            monomial.coefficient().clone() * value1
        };
        value = value + term;
    }
    Some(value)
}

pub(super) fn evaluate_quadratic_from_values<V>(poly: &Quadratic<V>, values: &HashMap<usize, V>) -> Option<V>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V>,
{
    let mut value = poly.constant().clone();
    for monomial in poly.monomials() {
        let value1 = values.get(&monomial.var_index1())?.clone();
        let term = if let Some(var2) = monomial.var_index2() {
            let value2 = values.get(&var2)?.clone();
            monomial.coefficient().clone() * value1 * value2
        } else {
            monomial.coefficient().clone() * value1
        };
        value = value + term;
    }
    Some(value)
}

pub(super) fn quadratic_has_square_terms<V>(poly: &Quadratic<V>) -> bool {
    poly.monomials().iter().any(|m| m.var_index2().is_some())
}

pub(super) fn try_quadratic_to_linear<V>(poly: &Quadratic<V>) -> Option<Linear<V>>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    let mut monomials = Vec::with_capacity(poly.monomials().len());
    for monomial in poly.monomials() {
        if monomial.var_index2().is_some() {
            return None;
        }
        monomials.push(LinearMonomial::new(
            monomial.coefficient().clone(),
            monomial.var_index1(),
        ));
    }
    Some(Linear::new(monomials, poly.constant().clone()))
}

pub(super) fn to_f64<V>(value: &V) -> Option<f64>
where
    V: ToPrimitive,
{
    value.to_f64()
}

pub(super) fn from_f64<V>(value: f64) -> Option<V>
where
    V: FromPrimitive,
{
    V::from_f64(value)
}

pub(super) fn convert_f64_to_v<V>(value: f64, context: &str) -> Result<V>
where
    V: FromPrimitive,
{
    from_f64(value).ok_or_else(|| {
        ModelError::InvalidConstraint(format!(
            "failed to convert `{}` value {} from f64 into model value type",
            context, value
        ))
        .into()
    })
}

pub(super) fn auxiliary_id(base: u64, salt: u64) -> u64 {
    base.wrapping_mul(0x9e37_79b9_7f4a_7c15)
        .wrapping_add(salt.wrapping_mul(0x517c_c1b7_2722_0a95))
}

#[derive(Debug, Clone)]
pub struct QuadraticLinearFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    result_var: ContinuousVariableItem,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticLinearFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    pub fn new(id: u64, name: &str, input: Quadratic<V>) -> Self {
        let result_var =
            ContinuousVariableItem::create(new_standalone_id(), &format!("{}_lin_y", name));
        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            result_var,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }
}

impl<V> Display for QuadraticLinearFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qlinear({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticLinearFunction<V>
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

impl<V> Symbol for QuadraticLinearFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticLinearFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn category(&self) -> Category {
        Category::Linear
    }

    fn operation_category(&self) -> Category {
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
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let result_symbol_id = self.result_var.id().unique_id() as usize;
        let result_index = symbol_to_index
            .get(&result_symbol_id)
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic linear bridge result variable id {}",
                    result_symbol_id
                ))
            })?;
        let Some(input_linear) = try_quadratic_to_linear(&self.input) else {
            return Ok(Vec::new());
        };

        let mut monomials = input_linear.monomials().to_vec();
        monomials.push(LinearMonomial::new(
            from_f64(-1.0).expect("convert -1.0"),
            result_index,
        ));
        let eq = LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(monomials, input_linear.constant_term().clone()),
                ConstraintRelation::Equal,
                from_f64(0.0).expect("convert 0.0"),
            ),
            &format!("{}_lin_eq", self.id.name),
            Arc::new(self.clone()),
        );
        Ok(vec![eq])
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        if !quadratic_has_square_terms(&self.input) {
            return Ok(Vec::new());
        }

        let result_symbol_id = self.result_var.id().unique_id() as usize;
        let result_index = symbol_to_index
            .get(&result_symbol_id)
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic linear bridge result variable id {}",
                    result_symbol_id
                ))
            })?;

        let mut monomials = self.input.monomials().to_vec();
        monomials.push(QuadraticMonomial::new_linear(
            from_f64(-1.0).expect("convert -1.0"),
            result_index,
        ));
        let eq = QuadraticConstraint::from_symbol(
            QuadraticInequality::new(
                Quadratic::new(monomials, self.input.constant().clone()),
                ConstraintRelation::Equal,
                from_f64(0.0).expect("convert 0.0"),
            ),
            &format!("{}_quad_eq", self.id.name),
            Arc::new(self.clone()),
        );
        Ok(vec![eq])
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values
            .get(&self.result_var.index())
            .cloned()
            .or_else(|| evaluate_quadratic_from_values(&self.input, values))
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qlinear({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticLinearFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        evaluate_quadratic(&self.input, token_table, zero_if_none)
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticLinearFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn to_linear_polynomial(&self) -> Linear<V> {
        Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                self.result_var.index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        )
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

impl<V> QuadraticFunctionSymbol<V> for QuadraticLinearFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + ToPrimitive
        + FromPrimitive,
    f64: IntoValue<V>,
{
}
