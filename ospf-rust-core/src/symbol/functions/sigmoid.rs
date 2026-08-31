//! Sigmoid function symbol.

use std::any::Any;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;

use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};

use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{BinaryVariableItem, ContinuousVariableItem, VariableId, new_group_id};

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
use super::{Point2, UnivariateLinearPiecewiseFunction};

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

fn to_f64<V>(value: &V) -> Option<f64>
where
    V: ToPrimitive,
{
    value.to_f64()
}

fn from_f64<V>(value: f64) -> Option<V>
where
    V: FromPrimitive,
{
    V::from_f64(value)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigmoidPrecision {
    Full,
    Half,
}

/// Piecewise-linear sigmoid symbol with exact-value evaluator.
#[derive(Debug, Clone)]
pub struct SigmoidFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Linear<V>,
    inner: UnivariateLinearPiecewiseFunction<V>,
    segment_vars: Vec<BinaryVariableItem>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> SigmoidFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    pub fn new(id: u64, name: &str, input: Linear<V>) -> Self {
        Self::with_precision(id, name, input, SigmoidPrecision::Full)
    }

    pub fn with_precision(
        id: u64,
        name: &str,
        input: Linear<V>,
        precision: SigmoidPrecision,
    ) -> Self {
        Self::with_precision_decimal(
            id,
            name,
            input,
            precision,
            from_f64(1e-5).expect("convert default sigmoid decimal precision"),
        )
    }

    pub fn with_precision_decimal(
        id: u64,
        name: &str,
        input: Linear<V>,
        precision: SigmoidPrecision,
        decimal_precision: V,
    ) -> Self {
        let points = Self::sampling_points(precision, decimal_precision);
        let segment_group_id = new_group_id();
        let segment_vars = (0..points.len().saturating_sub(1))
            .map(|i| {
                BinaryVariableItem::create(
                    VariableId::new(segment_group_id, i),
                    &format!("{}_sigmoid_b{}", name, i),
                )
            })
            .collect();
        let inner = UnivariateLinearPiecewiseFunction::new(id, name, input.clone(), points);
        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            inner,
            segment_vars,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_points(id: u64, name: &str, input: Linear<V>, points: Vec<Point2<V>>) -> Self {
        let segment_group_id = new_group_id();
        let segment_vars = (0..points.len().saturating_sub(1))
            .map(|i| {
                BinaryVariableItem::create(
                    VariableId::new(segment_group_id, i),
                    &format!("{}_sigmoid_b{}", name, i),
                )
            })
            .collect();
        let inner = UnivariateLinearPiecewiseFunction::new(id, name, input.clone(), points);
        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            inner,
            segment_vars,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub(crate) fn with_input_polynomial(&self, input: Linear<V>) -> Self {
        let mut cloned = self.clone();
        cloned.input = input.clone();
        cloned.inner = self.inner.with_input_polynomial(input);
        cloned
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }

    pub fn points(&self) -> &[Point2<V>] {
        self.inner.points()
    }

    pub fn segment_variables(&self) -> &[BinaryVariableItem] {
        &self.segment_vars
    }

    pub fn sigmoid(x: f64) -> f64 {
        1.0 / (1.0 + (-x).exp())
    }

    fn x_from_y(y: f64) -> f64 {
        -((1.0 - y) / y).ln()
    }

    pub fn sampling_points(precision: SigmoidPrecision, decimal_precision: V) -> Vec<Point2<V>> {
        let mut decimal = to_f64(&decimal_precision).unwrap_or(1e-5);
        if !decimal.is_finite() || decimal <= 0.0 {
            decimal = 1e-5;
        }
        if decimal > 1e-2 {
            decimal = 1e-2;
        }

        let points_f64 = match precision {
            SigmoidPrecision::Full => vec![
                (-1.0 / decimal, 0.0),
                (Self::x_from_y(decimal), decimal),
                (-4.0, Self::sigmoid(-4.0)),
                (-2.0, Self::sigmoid(-2.0)),
                (Self::x_from_y(0.2), 0.2),
                (0.0, 0.5),
                (Self::x_from_y(0.8), 0.8),
                (2.0, Self::sigmoid(2.0)),
                (4.0, Self::sigmoid(4.0)),
                (Self::x_from_y(1.0 - decimal), 1.0 - decimal),
                (1.0 / decimal, 1.0),
            ],
            SigmoidPrecision::Half => vec![
                (-1.0 / decimal, 0.0),
                (-4.0, Self::sigmoid(-4.0)),
                (-2.0, Self::sigmoid(-2.0)),
                (0.0, 0.5),
                (2.0, Self::sigmoid(2.0)),
                (4.0, Self::sigmoid(4.0)),
                (1.0 / decimal, 1.0),
            ],
        };

        points_f64
            .into_iter()
            .map(|(x, y)| {
                Point2::new(
                    from_f64(x).expect("convert sigmoid sample x"),
                    from_f64(y).expect("convert sigmoid sample y"),
                )
            })
            .collect()
    }
}

impl<V> Display for SigmoidFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "sigmoid({})", self.id.name)
    }
}

impl<V> DynSymbol for SigmoidFunction<V>
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

impl<V> Symbol for SigmoidFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for SigmoidFunction<V>
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
        Category::Nonlinear
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
        let mut constraints = self.inner.mechanism_constraints(symbol_to_index)?;
        let source: Arc<dyn IntermediateSymbol<V>> = Arc::new(self.clone());

        let points = self.points();
        if points.len() >= 2 && !self.segment_vars.is_empty() {
            let lambda_indices = self
                .inner
                .lambda_variables()
                .iter()
                .map(|lambda| {
                    symbol_to_index
                        .get(&(lambda.id().unique_id() as usize))
                        .copied()
                        .ok_or_else(|| {
                            ModelError::SymbolNotRegistered(format!(
                                "sigmoid lambda variable id {}",
                                lambda.id().unique_id()
                            ))
                        })
                })
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let segment_indices = self
                .segment_vars
                .iter()
                .map(|segment| {
                    symbol_to_index
                        .get(&(segment.id().unique_id() as usize))
                        .copied()
                        .ok_or_else(|| {
                            ModelError::SymbolNotRegistered(format!(
                                "sigmoid segment variable id {}",
                                segment.id().unique_id()
                            ))
                        })
                })
                .collect::<std::result::Result<Vec<_>, _>>()?;

            let mut segment_sum_monomials = Vec::with_capacity(segment_indices.len());
            for segment_index in &segment_indices {
                segment_sum_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(1.0, "sigmoid segment sum coefficient")?,
                    *segment_index,
                ));
            }
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        segment_sum_monomials,
                        convert_f64_to_v::<V>(0.0, "sigmoid segment sum constant")?,
                    ),
                    ConstraintRelation::Equal,
                    convert_f64_to_v::<V>(1.0, "sigmoid segment sum rhs")?,
                ),
                &format!("{}_sigmoid_seg_sum", self.id.name),
                source.clone(),
            ));

            for i in 0..lambda_indices.len() {
                let mut rhs_monomials = Vec::with_capacity(2);
                if i > 0 {
                    rhs_monomials.push(LinearMonomial::new(
                        convert_f64_to_v::<V>(1.0, "sigmoid lambda-link lhs segment coefficient")?,
                        segment_indices[i - 1],
                    ));
                }
                if i < segment_indices.len() {
                    rhs_monomials.push(LinearMonomial::new(
                        convert_f64_to_v::<V>(1.0, "sigmoid lambda-link rhs segment coefficient")?,
                        segment_indices[i],
                    ));
                }
                let mut link_monomials = vec![LinearMonomial::new(
                    convert_f64_to_v::<V>(1.0, "sigmoid lambda-link lambda coefficient")?,
                    lambda_indices[i],
                )];
                for rhs in rhs_monomials {
                    link_monomials.push(LinearMonomial::new(
                        convert_f64_to_v::<V>(
                            -1.0,
                            "sigmoid lambda-link negated segment coefficient",
                        )?,
                        rhs.var_index(),
                    ));
                }
                constraints.push(LinearConstraint::from_symbol(
                    LinearInequality::new(
                        Linear::new(
                            link_monomials,
                            convert_f64_to_v::<V>(0.0, "sigmoid lambda-link constant")?,
                        ),
                        ConstraintRelation::LessEqual,
                        convert_f64_to_v::<V>(0.0, "sigmoid lambda-link rhs")?,
                    ),
                    &format!("{}_sigmoid_lambda_link_{}", self.id.name, i),
                    source.clone(),
                ));
            }
        }

        let result_index = symbol_to_index
            .get(&(self.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "sigmoid result variable id {}",
                    self.result_variable().id().unique_id()
                ))
            })?;
        let y_min = points
            .iter()
            .filter_map(|point| to_f64(&point.y))
            .fold(f64::INFINITY, f64::min);
        let y_max = points
            .iter()
            .filter_map(|point| to_f64(&point.y))
            .fold(f64::NEG_INFINITY, f64::max);
        if y_min.is_finite() && y_max.is_finite() {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "sigmoid y upper bound coefficient")?,
                            result_index,
                        )],
                        convert_f64_to_v::<V>(0.0, "sigmoid y upper bound constant")?,
                    ),
                    ConstraintRelation::LessEqual,
                    convert_f64_to_v::<V>(y_max, "sigmoid y upper bound rhs")?,
                ),
                &format!("{}_sigmoid_y_ub", self.id.name),
                source.clone(),
            ));
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "sigmoid y lower bound coefficient")?,
                            result_index,
                        )],
                        convert_f64_to_v::<V>(0.0, "sigmoid y lower bound constant")?,
                    ),
                    ConstraintRelation::GreaterEqual,
                    convert_f64_to_v::<V>(y_min, "sigmoid y lower bound rhs")?,
                ),
                &format!("{}_sigmoid_y_lb", self.id.name),
                source,
            ));
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

    fn prepare(&self, values: &std::collections::HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("sigmoid({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for SigmoidFunction<V>
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
        self.inner.register_tokens(tokens)?;
        for segment in &self.segment_vars {
            tokens.push(Token::from_generic(segment.clone(), segment.index()));
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let x = to_f64(&evaluate_linear(&self.input, token_table, zero_if_none)?)?;
        from_f64(Self::sigmoid(x))
    }
}

impl<V> LinearIntermediateSymbol<V> for SigmoidFunction<V>
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::flatten::LinearMonomial;
    use crate::token::{MutableTokenList, Token, VecTokenList};
    use crate::variable::{ContinuousVariableItem, VariableId};

    #[test]
    fn sigmoid_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(10), "x");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 10);
        tx.set_result(0.0);
        tokens.add_token(tx);

        let sigmoid = SigmoidFunction::new(
            20,
            "sigmoid",
            Linear::new(vec![LinearMonomial::new(1.0, 10)], 0.0),
        );
        assert_eq!(sigmoid.calculate_value(&tokens, false), Some(0.5));
    }

    #[test]
    fn sigmoid_sampling_points_full_has_expected_count() {
        let points = SigmoidFunction::<f64>::sampling_points(SigmoidPrecision::Full, 1e-5);
        assert_eq!(points.len(), 11);
    }
}
