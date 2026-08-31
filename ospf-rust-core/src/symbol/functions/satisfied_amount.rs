//! Satisfied amount function symbol.

use std::any::Any;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;

use num_traits::{FromPrimitive, ToPrimitive};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};

use crate::error::{ModelError, Result};
use crate::flatten::{Linear, LinearMonomial, Quadratic};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{BinaryVariableItem, ContinuousVariableItem, new_standalone_id};

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};

fn from_f64<V>(value: f64) -> Option<V>
where
    V: FromPrimitive,
{
    V::from_f64(value)
}

fn to_f64<V>(value: &V) -> Option<f64>
where
    V: ToPrimitive,
{
    value.to_f64()
}

fn convert_f64_to_v<V>(value: f64, context: &str) -> Result<V>
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

/// Counts the number of satisfied indicator constraints.
#[derive(Debug, Clone)]
pub struct SatisfiedAmountFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    indicators: Vec<BinaryVariableItem>,
    result_var: ContinuousVariableItem,
    declared_dependency_ids: Vec<u64>,
    _marker: std::marker::PhantomData<V>,
}

impl<V> SatisfiedAmountFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    pub fn new(id: u64, name: &str, indicators: Vec<BinaryVariableItem>) -> Self {
        let result_var = ContinuousVariableItem::create(new_standalone_id(), name);

        Self {
            id: IntermediateSymbolId::new(id, name),
            indicators,
            result_var,
            declared_dependency_ids: Vec::new(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    pub fn indicator_variables(&self) -> &[BinaryVariableItem] {
        &self.indicators
    }
}

impl<V> Display for SatisfiedAmountFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "satisfied_amount({})", self.id.name)
    }
}

impl<V> DynSymbol for SatisfiedAmountFunction<V>
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

impl<V> Symbol for SatisfiedAmountFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for SatisfiedAmountFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
    f64: IntoValue<V>,
{
    fn category(&self) -> Category {
        Category::Linear
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
        symbol_to_index: &std::collections::HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "satisfied amount result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        let mut monomials = Vec::with_capacity(self.indicators.len() + 1);
        monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "satisfied amount result coefficient")?,
            result_index,
        ));
        for indicator in &self.indicators {
            let indicator_index = symbol_to_index
                .get(&(indicator.id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "satisfied amount indicator variable id {}",
                        indicator.id().unique_id()
                    ))
                })?;
            monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-1.0, "satisfied amount indicator coefficient")?,
                indicator_index,
            ));
        }

        let equality = LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    monomials,
                    convert_f64_to_v::<V>(0.0, "satisfied amount constant")?,
                ),
                ConstraintRelation::Equal,
                convert_f64_to_v::<V>(0.0, "satisfied amount rhs")?,
            ),
            &format!("{}_sat_amount", self.id.name),
            Arc::new(self.clone()),
        );

        Ok(vec![equality])
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &std::collections::HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_var.index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("satisfied_amount({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for SatisfiedAmountFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
    f64: IntoValue<V>,
{
    fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        for var in &self.indicators {
            tokens.push(Token::from_generic(var.clone(), var.index()));
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let mut count = 0.0_f64;
        for indicator in &self.indicators {
            let value = match token_table
                .find_by_id(indicator.id())
                .and_then(|token| token.get_result())
            {
                Some(v) => v,
                None if zero_if_none => from_f64(0.0)?,
                None => return None,
            };
            if to_f64(&value)?.abs() > f64::EPSILON {
                count += 1.0;
            }
        }
        from_f64(count)
    }
}

impl<V> LinearIntermediateSymbol<V> for SatisfiedAmountFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
    f64: IntoValue<V>,
{
    fn to_linear_polynomial(&self) -> Linear<V> {
        let monomials: Vec<_> = self
            .indicators
            .iter()
            .map(|var| LinearMonomial::new(from_f64(1.0).expect("convert 1.0"), var.index()))
            .collect();
        Linear::new(monomials, from_f64(0.0).expect("convert 0.0"))
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}
