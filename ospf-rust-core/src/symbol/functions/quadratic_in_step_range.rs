//! Quadratic in-step-range function.

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality, QuadraticConstraint};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{ContinuousVariableItem, new_standalone_id};
use super::super::{Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol, QuadraticFunctionSymbol};
use super::RoundingFunction;
use super::quadratic_linear::*;

#[derive(Debug, Clone)]
pub struct QuadraticInStepRangeFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    pub(super) id: IntermediateSymbolId,
    pub(super) lower: Quadratic<V>,
    pub(super) upper: Quadratic<V>,
    pub(super) step: V,
    pub(super) upper_cap: Option<V>,
    pub(super) lower_bridge: QuadraticLinearFunction<V>,
    pub(super) upper_bridge: QuadraticLinearFunction<V>,
    pub(super) floor_inner: Option<RoundingFunction<V>>,
    pub(super) result_var: ContinuousVariableItem,
    pub(super) declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticInStepRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    /// Compatibility constructor:
    /// treats `input` as the runtime upper bound and keeps a hard upper cap.
    pub fn new(id: u64, name: &str, input: Quadratic<V>, lower: V, upper: V, step: V) -> Self {
        let lower_poly = Quadratic::new(vec![], lower);
        Self::with_quadratic_bounds_and_cap(id, name, lower_poly, input, step, Some(upper))
    }

    pub fn with_quadratic_bounds(
        id: u64,
        name: &str,
        lower: Quadratic<V>,
        upper: Quadratic<V>,
        step: V,
    ) -> Self {
        Self::with_quadratic_bounds_and_cap(id, name, lower, upper, step, None)
    }

    fn with_quadratic_bounds_and_cap(
        id: u64,
        name: &str,
        lower: Quadratic<V>,
        upper: Quadratic<V>,
        step: V,
        upper_cap: Option<V>,
    ) -> Self {
        let lower_bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 1501),
            &format!("{}_lower_bridge", name),
            lower.clone(),
        );
        let upper_bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 1502),
            &format!("{}_upper_bridge", name),
            upper.clone(),
        );
        let step_abs = to_f64(&step).unwrap_or(0.0).abs();
        let floor_inner = if step_abs <= 1e-8 {
            None
        } else {
            let q_input = Linear::new(
                vec![
                    LinearMonomial::new(
                        from_f64(1.0 / step_abs).expect("convert reciprocal step"),
                        upper_bridge.result_variable().index(),
                    ),
                    LinearMonomial::new(
                        from_f64(-1.0 / step_abs).expect("convert reciprocal step"),
                        lower_bridge.result_variable().index(),
                    ),
                ],
                from_f64(0.0).expect("convert 0.0"),
            );
            Some(RoundingFunction::floor(
                auxiliary_id(id, 1503),
                &format!("{}_floor_div", name),
                q_input,
            ))
        };
        let result_var =
            ContinuousVariableItem::create(new_standalone_id(), &format!("{}_in_step_range", name));

        Self {
            id: IntermediateSymbolId::new(id, name),
            lower,
            upper,
            step,
            upper_cap,
            lower_bridge,
            upper_bridge,
            floor_inner,
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

impl<V> Display for QuadraticInStepRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qin_step_range({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticInStepRangeFunction<V>
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

impl<V> Symbol for QuadraticInStepRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticInStepRangeFunction<V>
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
        let mut constraints = self.lower_bridge.mechanism_constraints(symbol_to_index)?;
        constraints.extend(self.upper_bridge.mechanism_constraints(symbol_to_index)?);

        let y_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic in_step_range result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;
        let lower_index = symbol_to_index
            .get(&(self.lower_bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic in_step_range lower bridge variable id {}",
                    self.lower_bridge.result_variable().id().unique_id()
                ))
            })?;

        let step_abs = to_f64(&self.step)
            .ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "quadratic in_step_range `{}` step cannot be converted to f64",
                    self.id.name
                ))
            })?
            .abs();

        if step_abs <= 1e-8 {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(from_f64(1.0).expect("convert 1.0"), y_index),
                            LinearMonomial::new(from_f64(-1.0).expect("convert -1.0"), lower_index),
                        ],
                        from_f64(0.0).expect("convert 0.0"),
                    ),
                    ConstraintRelation::Equal,
                    from_f64(0.0).expect("convert 0.0"),
                ),
                &format!("{}_qstep_equal_lower", self.id.name),
                Arc::new(self.clone()),
            ));
        } else if let Some(floor_inner) = &self.floor_inner {
            let q_index = symbol_to_index
                .get(&(floor_inner.result_variable().id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "quadratic in_step_range floor variable id {}",
                        floor_inner.result_variable().id().unique_id()
                    ))
                })?;
            let q_int_index = symbol_to_index
                .get(&(floor_inner.integer_variable().id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "quadratic in_step_range floor integer variable id {}",
                        floor_inner.integer_variable().id().unique_id()
                    ))
                })?;
            let upper_index = symbol_to_index
                .get(&(self.upper_bridge.result_variable().id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "quadratic in_step_range upper bridge variable id {}",
                        self.upper_bridge.result_variable().id().unique_id()
                    ))
                })?;

            // qstep 内部 floor 使用显式线性化，避免内部表达式索引与 solver 映射错位。
            // Use explicit floor linearization to avoid index mismatch between inner polynomial and solver mapping.
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(from_f64(1.0).expect("convert 1.0"), q_index),
                            LinearMonomial::new(from_f64(-1.0).expect("convert -1.0"), q_int_index),
                        ],
                        from_f64(0.0).expect("convert 0.0"),
                    ),
                    ConstraintRelation::Equal,
                    from_f64(0.0).expect("convert 0.0"),
                ),
                &format!("{}_qstep_floor_result_link", self.id.name),
                Arc::new(self.clone()),
            ));

            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(
                                from_f64(1.0 / step_abs).expect("convert reciprocal step"),
                                upper_index,
                            ),
                            LinearMonomial::new(
                                from_f64(-1.0 / step_abs).expect("convert reciprocal step"),
                                lower_index,
                            ),
                            LinearMonomial::new(from_f64(-1.0).expect("convert -1.0"), q_int_index),
                        ],
                        from_f64(0.0).expect("convert 0.0"),
                    ),
                    ConstraintRelation::GreaterEqual,
                    from_f64(0.0).expect("convert 0.0"),
                ),
                &format!("{}_qstep_floor_lb", self.id.name),
                Arc::new(self.clone()),
            ));

            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(
                                from_f64(1.0 / step_abs).expect("convert reciprocal step"),
                                upper_index,
                            ),
                            LinearMonomial::new(
                                from_f64(-1.0 / step_abs).expect("convert reciprocal step"),
                                lower_index,
                            ),
                            LinearMonomial::new(from_f64(-1.0).expect("convert -1.0"), q_int_index),
                        ],
                        from_f64(0.0).expect("convert 0.0"),
                    ),
                    ConstraintRelation::LessEqual,
                    from_f64(1.0 - 1e-8).expect("convert floor epsilon"),
                ),
                &format!("{}_qstep_floor_ub", self.id.name),
                Arc::new(self.clone()),
            ));

            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(from_f64(1.0).expect("convert 1.0"), y_index),
                            LinearMonomial::new(
                                from_f64(-step_abs).expect("convert step"),
                                q_index,
                            ),
                            LinearMonomial::new(from_f64(-1.0).expect("convert -1.0"), lower_index),
                        ],
                        from_f64(0.0).expect("convert 0.0"),
                    ),
                    ConstraintRelation::Equal,
                    from_f64(0.0).expect("convert 0.0"),
                ),
                &format!("{}_qstep_link", self.id.name),
                Arc::new(self.clone()),
            ));
        }

        if let Some(upper_cap) = &self.upper_cap {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![LinearMonomial::new(
                            from_f64(1.0).expect("convert 1.0"),
                            y_index,
                        )],
                        from_f64(0.0).expect("convert 0.0"),
                    ),
                    ConstraintRelation::LessEqual,
                    upper_cap.clone(),
                ),
                &format!("{}_qstep_cap", self.id.name),
                Arc::new(self.clone()),
            ));
        }
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        let mut constraints = self
            .lower_bridge
            .quadratic_mechanism_constraints(symbol_to_index)?;
        constraints.extend(
            self.upper_bridge
                .quadratic_mechanism_constraints(symbol_to_index)?,
        );
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
        values.get(&self.result_var.index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qin_step_range({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticInStepRangeFunction<V>
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
        self.lower_bridge.register_tokens(tokens)?;
        self.upper_bridge.register_tokens(tokens)?;
        if let Some(floor_inner) = &self.floor_inner {
            floor_inner.register_tokens(tokens)?;
        }
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let lower = to_f64(&evaluate_quadratic(&self.lower, token_table, zero_if_none)?)?;
        let mut upper = to_f64(&evaluate_quadratic(&self.upper, token_table, zero_if_none)?)?;
        if let Some(cap) = &self.upper_cap {
            upper = upper.min(to_f64(cap)?);
        }
        let step = to_f64(&self.step)?.abs();
        if step <= 1e-8 {
            return from_f64(lower);
        }
        let q = ((upper - lower) / step).floor();
        from_f64(lower + q * step)
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticInStepRangeFunction<V>
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

impl<V> QuadraticFunctionSymbol<V> for QuadraticInStepRangeFunction<V>
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
