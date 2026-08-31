//! Quadratic min function.

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use crate::error::{ModelError, Result};
use crate::model::{LinearConstraint, QuadraticConstraint};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::ContinuousVariableItem;
use super::super::{Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol, QuadraticFunctionSymbol};
use super::big_m::infer_big_m_for_quadratic_polynomials;
use super::MinFunction;
use super::quadratic_linear::*;

#[derive(Debug, Clone)]
pub struct QuadraticMinFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    inputs: Vec<Quadratic<V>>,
    pub(super) bridges: Vec<QuadraticLinearFunction<V>>,
    inner: MinFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticMinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    pub fn new(id: u64, name: &str, inputs: Vec<Quadratic<V>>, exact: bool) -> Self {
        let bridges: Vec<QuadraticLinearFunction<V>> = inputs
            .iter()
            .enumerate()
            .map(|(i, input)| {
                QuadraticLinearFunction::new(
                    auxiliary_id(id, 501 + i as u64),
                    &format!("{}_bridge{}", name, i),
                    input.clone(),
                )
            })
            .collect();
        let linear_inputs: Vec<Linear<V>> = bridges
            .iter()
            .map(|bridge| {
                Linear::new(
                    vec![LinearMonomial::new(
                        from_f64(1.0).expect("convert 1.0"),
                        bridge.result_variable().index(),
                    )],
                    from_f64(0.0).expect("convert 0.0"),
                )
            })
            .collect();
        let inner = MinFunction::new(id, name, linear_inputs, exact);

        Self {
            id: IntermediateSymbolId::new(id, name),
            inputs,
            bridges,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for QuadraticMinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qmin({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticMinFunction<V>
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

impl<V> Symbol for QuadraticMinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticMinFunction<V>
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
        let mut constraints = Vec::new();
        for bridge in &self.bridges {
            constraints.extend(bridge.mechanism_constraints(symbol_to_index)?);
        }
        let mut mapped_inputs = Vec::with_capacity(self.bridges.len());
        for bridge in &self.bridges {
            let bridge_index = symbol_to_index
                .get(&(bridge.result_variable().id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "quadratic min bridge variable id {}",
                        bridge.result_variable().id().unique_id()
                    ))
                })?;
            mapped_inputs.push(Linear::new(
                vec![LinearMonomial::new(
                    from_f64(1.0).expect("convert 1.0"),
                    bridge_index,
                )],
                from_f64(0.0).expect("convert 0.0"),
            ));
        }
        let mapped_inner = self.inner.with_polynomials(mapped_inputs);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mut constraints = Vec::new();
        for bridge in &self.bridges {
            constraints.extend(bridge.mechanism_constraints(symbol_to_index)?);
        }
        let mut mapped_inputs = Vec::with_capacity(self.bridges.len());
        for bridge in &self.bridges {
            let bridge_index = symbol_to_index
                .get(&(bridge.result_variable().id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "quadratic min bridge variable id {}",
                        bridge.result_variable().id().unique_id()
                    ))
                })?;
            mapped_inputs.push(Linear::new(
                vec![LinearMonomial::new(
                    from_f64(1.0).expect("convert 1.0"),
                    bridge_index,
                )],
                from_f64(0.0).expect("convert 0.0"),
            ));
        }
        let mapped_inner = self.inner.with_polynomials(mapped_inputs);
        let big_m = infer_big_m_for_quadratic_polynomials(&self.inputs, tokens, MIN_BIG_M);
        match big_m {
            Some(big_m) => constraints
                .extend(mapped_inner.mechanism_constraints_with_big_m(symbol_to_index, big_m)?),
            None => constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?),
        }
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        let mut constraints = Vec::new();
        for bridge in &self.bridges {
            constraints.extend(bridge.quadratic_mechanism_constraints(symbol_to_index)?);
        }
        Ok(constraints)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qmin({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticMinFunction<V>
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
    fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        for bridge in &self.bridges {
            bridge.register_tokens(tokens)?;
        }
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let mut min_value: Option<f64> = None;
        for input in &self.inputs {
            let value = to_f64(&evaluate_quadratic(input, token_table, zero_if_none)?)?;
            min_value = Some(match min_value {
                Some(current) => current.min(value),
                None => value,
            });
        }
        match min_value {
            Some(v) => from_f64(v),
            None if zero_if_none => from_f64(0.0),
            None => None,
        }
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticMinFunction<V>
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
    fn to_linear_polynomial(&self) -> Linear<V> {
        self.inner.to_linear_polynomial()
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

impl<V> QuadraticFunctionSymbol<V> for QuadraticMinFunction<V>
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
