//! In-step-range function symbol.

use std::any::Any;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;

use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};

use crate::error::{ModelError, Result};
use crate::flatten::{Linear, LinearMonomial, Quadratic};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{BinaryVariableItem, VariableId, new_group_id};

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
use super::big_m::{BigMPolicy, infer_linear_bounds_from_tokens};

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

const DEFAULT_BIG_M: f64 = 1_000_000.0;
const BIG_M_POLICY: BigMPolicy = BigMPolicy::new(DEFAULT_BIG_M, 1.0);
const MAX_STEP_POINTS: usize = 4096;
const STEP_EPSILON: f64 = 1e-8;

/// Checks if value is in step range.
#[derive(Debug, Clone)]
pub struct InStepRangeFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Linear<V>,
    lower: V,
    upper: V,
    step: V,
    aux_group_id: usize,
    result_var: BinaryVariableItem,
    declared_dependency_ids: Vec<u64>,
}

impl<V> InStepRangeFunction<V>
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
    pub fn new(id: u64, name: &str, input: Linear<V>, lower: V, upper: V, step: V) -> Self {
        let aux_group_id = new_group_id();
        let result_var = BinaryVariableItem::create(VariableId::new(aux_group_id, 0), name);

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            lower,
            upper,
            step,
            aux_group_id,
            result_var,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &BinaryVariableItem {
        &self.result_var
    }

    pub fn input_polynomial(&self) -> &Linear<V> {
        &self.input
    }

    pub fn lower_bound(&self) -> &V {
        &self.lower
    }

    pub fn upper_bound(&self) -> &V {
        &self.upper
    }

    pub fn step(&self) -> &V {
        &self.step
    }

    fn step_points_f64(&self) -> Result<Vec<f64>>
    where
        V: ToPrimitive,
    {
        let lower = to_f64(&self.lower).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "in_step_range `{}` lower bound cannot be converted to f64",
                self.id.name
            ))
        })?;
        let upper = to_f64(&self.upper).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "in_step_range `{}` upper bound cannot be converted to f64",
                self.id.name
            ))
        })?;
        let step_raw = to_f64(&self.step).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "in_step_range `{}` step cannot be converted to f64",
                self.id.name
            ))
        })?;

        if !lower.is_finite() || !upper.is_finite() || !step_raw.is_finite() {
            return Err(ModelError::InvalidConstraint(format!(
                "in_step_range `{}` requires finite lower/upper/step values",
                self.id.name
            ))
            .into());
        }

        if lower > upper + STEP_EPSILON {
            return Ok(Vec::new());
        }

        let step = step_raw.abs();
        if step <= STEP_EPSILON {
            return Ok(vec![lower]);
        }

        let estimate = ((upper + STEP_EPSILON - lower) / step).floor();
        if !estimate.is_finite() || estimate < 0.0 {
            return Ok(Vec::new());
        }
        let estimated_count = estimate as usize + 1;
        if estimated_count > MAX_STEP_POINTS {
            return Err(ModelError::InvalidConstraint(format!(
                "in_step_range `{}` expands to {} step points (> {}), refusing linear mechanism injection",
                self.id.name, estimated_count, MAX_STEP_POINTS
            ))
            .into());
        }

        let mut points = Vec::with_capacity(estimated_count);
        for k in 0..estimated_count {
            points.push(lower + step * k as f64);
        }
        Ok(points)
    }

    fn point_indicator_variable(&self, point_index: usize) -> BinaryVariableItem {
        BinaryVariableItem::create(
            VariableId::new(self.aux_group_id, point_index + 1),
            &format!("{}_step_pt{}", self.id.name, point_index),
        )
    }

    fn point_side_variable(&self, point_count: usize, point_index: usize) -> BinaryVariableItem {
        BinaryVariableItem::create(
            VariableId::new(self.aux_group_id, point_count + point_index + 1),
            &format!("{}_step_side{}", self.id.name, point_index),
        )
    }

    fn infer_big_m_from_tokens(&self, tokens: &[Token<V>], points: &[f64]) -> Option<f64> {
        let (input_lower, input_upper) = infer_linear_bounds_from_tokens(&self.input, tokens)?;
        let mut inferred = BIG_M_POLICY.min();
        for point in points {
            let lower_diff = (input_lower - *point).abs();
            let upper_diff = (input_upper - *point).abs();
            inferred = inferred.max(lower_diff.max(upper_diff));
        }
        Some(inferred.max(BIG_M_POLICY.min()))
    }

    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &std::collections::HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "in_step_range result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        let points = self.step_points_f64()?;
        if points.is_empty() {
            return Ok(vec![LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "in_step_range result coefficient")?,
                            result_index,
                        )],
                        convert_f64_to_v::<V>(0.0, "in_step_range constant")?,
                    ),
                    ConstraintRelation::Equal,
                    convert_f64_to_v::<V>(0.0, "in_step_range rhs")?,
                ),
                &format!("{}_empty", self.id.name),
                Arc::new(self.clone()),
            )]);
        }
        if !big_m.is_finite() || big_m < 0.0 {
            return Err(ModelError::InvalidConstraint(format!(
                "in_step_range `{}` requires finite non-negative big-M",
                self.id.name
            ))
            .into());
        }

        let step_abs = to_f64(&self.step)
            .ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "in_step_range `{}` step cannot be converted to f64",
                    self.id.name
                ))
            })?
            .abs();
        let point_tolerance = if step_abs <= STEP_EPSILON {
            STEP_EPSILON
        } else {
            step_abs * STEP_EPSILON + STEP_EPSILON
        };
        let strict_boundary = point_tolerance + STEP_EPSILON;
        let source = Arc::new(self.clone());

        let mut base_monomials = Vec::with_capacity(self.input.monomials().len());
        for monomial in self.input.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "in_step_range `{}` input coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            base_monomials.push((coefficient, monomial.var_index()));
        }
        let input_constant = to_f64(self.input.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "in_step_range `{}` input constant cannot be converted to f64",
                self.id.name
            ))
        })?;

        let mut point_indices = Vec::with_capacity(points.len());
        let mut point_side_indices = Vec::with_capacity(points.len());

        for i in 0..points.len() {
            let point_var = self.point_indicator_variable(i);
            let side_var = self.point_side_variable(points.len(), i);

            let point_index = symbol_to_index
                .get(&(point_var.id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "in_step_range point indicator variable id {}",
                        point_var.id().unique_id()
                    ))
                })?;
            let side_index = symbol_to_index
                .get(&(side_var.id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "in_step_range side variable id {}",
                        side_var.id().unique_id()
                    ))
                })?;
            point_indices.push(point_index);
            point_side_indices.push(side_index);
        }

        let mut constraints = Vec::with_capacity(points.len() * 6 + 1);

        for (i, point) in points.iter().copied().enumerate() {
            let point_index = point_indices[i];
            let side_index = point_side_indices[i];
            let shifted_constant = input_constant - point;

            let build_point_constraint = |name_suffix: &str,
                                          relation: ConstraintRelation,
                                          rhs: f64,
                                          point_coeff: f64,
                                          side_coeff: f64|
             -> Result<LinearConstraint<V>> {
                let mut monomials = Vec::with_capacity(base_monomials.len() + 2);
                for (coefficient, var_index) in &base_monomials {
                    monomials.push(LinearMonomial::new(
                        convert_f64_to_v::<V>(*coefficient, "in_step_range input coefficient")?,
                        *var_index,
                    ));
                }
                monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(point_coeff, "in_step_range point coefficient")?,
                    point_index,
                ));
                monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(side_coeff, "in_step_range side coefficient")?,
                    side_index,
                ));

                Ok(LinearConstraint::from_symbol(
                    LinearInequality::new(
                        Linear::new(
                            monomials,
                            convert_f64_to_v::<V>(shifted_constant, "in_step_range constant")?,
                        ),
                        relation,
                        convert_f64_to_v::<V>(rhs, "in_step_range rhs")?,
                    ),
                    &format!("{}_pt{}_{}", self.id.name, i, name_suffix),
                    source.clone(),
                ))
            };

            constraints.push(build_point_constraint(
                "band_ub",
                ConstraintRelation::LessEqual,
                point_tolerance + big_m,
                big_m,
                0.0,
            )?);
            constraints.push(build_point_constraint(
                "band_lb",
                ConstraintRelation::GreaterEqual,
                -point_tolerance - big_m,
                -big_m,
                0.0,
            )?);
            constraints.push(build_point_constraint(
                "out_lb",
                ConstraintRelation::GreaterEqual,
                strict_boundary - big_m,
                big_m,
                -big_m,
            )?);
            constraints.push(build_point_constraint(
                "out_ub",
                ConstraintRelation::LessEqual,
                -strict_boundary,
                -big_m,
                -big_m,
            )?);

            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(1.0, "in_step_range y coefficient")?,
                                result_index,
                            ),
                            LinearMonomial::new(
                                convert_f64_to_v::<V>(-1.0, "in_step_range z coefficient")?,
                                point_index,
                            ),
                        ],
                        convert_f64_to_v::<V>(0.0, "in_step_range link constant")?,
                    ),
                    ConstraintRelation::GreaterEqual,
                    convert_f64_to_v::<V>(0.0, "in_step_range link rhs")?,
                ),
                &format!("{}_or_lb_{}", self.id.name, i),
                source.clone(),
            ));
        }

        let mut sum_monomials = Vec::with_capacity(point_indices.len() + 1);
        sum_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "in_step_range y coefficient")?,
            result_index,
        ));
        for point_index in point_indices {
            sum_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-1.0, "in_step_range z coefficient")?,
                point_index,
            ));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    sum_monomials,
                    convert_f64_to_v::<V>(0.0, "in_step_range sum constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(0.0, "in_step_range sum rhs")?,
            ),
            &format!("{}_or_ub", self.id.name),
            source,
        ));

        Ok(constraints)
    }
}

impl<V> Display for InStepRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "in_step_range({})", self.id.name)
    }
}

impl<V> DynSymbol for InStepRangeFunction<V>
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

impl<V> Symbol for InStepRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for InStepRangeFunction<V>
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
        self.build_mechanism_constraints(symbol_to_index, BIG_M_POLICY.fallback())
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &std::collections::HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let points = self.step_points_f64()?;
        let big_m = BIG_M_POLICY.resolve(self.infer_big_m_from_tokens(tokens, &points));
        self.build_mechanism_constraints(symbol_to_index, big_m)
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
        format!("in_step_range({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for InStepRangeFunction<V>
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
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        let points = self.step_points_f64()?;
        for i in 0..points.len() {
            let point_var = self.point_indicator_variable(i);
            tokens.push(Token::from_generic(point_var.clone(), point_var.index()));
        }
        for i in 0..points.len() {
            let side_var = self.point_side_variable(points.len(), i);
            tokens.push(Token::from_generic(side_var.clone(), side_var.index()));
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let value = to_f64(&evaluate_linear(&self.input, token_table, zero_if_none)?)?;
        let lower = to_f64(&self.lower)?;
        let upper = to_f64(&self.upper)?;
        let step = to_f64(&self.step)?;
        let eps = 1e-8;

        let in_range = value + eps >= lower && value <= upper + eps;
        if !in_range {
            return from_f64(0.0);
        }

        if step.abs() <= eps {
            return from_f64(if (value - lower).abs() <= eps {
                1.0
            } else {
                0.0
            });
        }

        let step = step.abs();
        let offset = (value - lower) / step;
        let on_step = (offset - offset.round()).abs() <= eps;
        from_f64(if on_step { 1.0 } else { 0.0 })
    }
}

impl<V> LinearIntermediateSymbol<V> for InStepRangeFunction<V>
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::token::Token;
    use crate::variable::{ContinuousVariableItem, VariableRange};

    #[test]
    fn in_step_range_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(80_000),
            "x",
            VariableRange::bounded(0.0, 2.0),
        );
        let f: InStepRangeFunction<f64> = InStepRangeFunction::new(
            8000,
            "in_step_bound",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            0.0,
            2.0,
            1.0,
        );

        let points = f.step_points_f64().expect("step points should be generated");
        let result_id = f.result_variable().id().unique_id() as usize;
        let mut symbol_to_index = HashMap::new();
        symbol_to_index.insert(result_id, 1usize);
        for i in 0..points.len() {
            let point = f.point_indicator_variable(i);
            let side = f.point_side_variable(points.len(), i);
            symbol_to_index.insert(point.id().unique_id() as usize, 2 + i);
            symbol_to_index.insert(side.id().unique_id() as usize, 2 + points.len() + i);
        }

        let tokens = vec![Token::from_generic(x, 0)];
        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("in_step_range constraints should be generated");
        let band_ub = constraints
            .iter()
            .find(|constraint| constraint.name == "in_step_bound_pt0_band_ub")
            .expect("pt0 upper-band constraint should exist");
        let point_term = band_ub
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 2)
            .expect("point selector term should exist");

        // x in [0, 2] and point=0 => |x-0| bound is 2.
        assert!((band_ub.inequality.rhs - (2.0 + 2.0e-8)).abs() <= 1e-9);
        assert!((*point_term.coefficient() - 2.0).abs() <= 1e-9);
    }

    #[test]
    fn in_step_range_function_falls_back_to_default_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(80_010), "x");
        let f: InStepRangeFunction<f64> = InStepRangeFunction::new(
            8001,
            "in_step_default",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            0.0,
            2.0,
            1.0,
        );

        let points = f.step_points_f64().expect("step points should be generated");
        let result_id = f.result_variable().id().unique_id() as usize;
        let mut symbol_to_index = HashMap::new();
        symbol_to_index.insert(result_id, 1usize);
        for i in 0..points.len() {
            let point = f.point_indicator_variable(i);
            let side = f.point_side_variable(points.len(), i);
            symbol_to_index.insert(point.id().unique_id() as usize, 2 + i);
            symbol_to_index.insert(side.id().unique_id() as usize, 2 + points.len() + i);
        }

        let tokens = vec![Token::from_generic(x, 0)];
        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("in_step_range constraints should be generated");
        let band_ub = constraints
            .iter()
            .find(|constraint| constraint.name == "in_step_default_pt0_band_ub")
            .expect("pt0 upper-band constraint should exist");
        let point_term = band_ub
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 2)
            .expect("point selector term should exist");

        assert!((band_ub.inequality.rhs - (DEFAULT_BIG_M + 2.0e-8)).abs() <= 1e-6);
        assert!((*point_term.coefficient() - DEFAULT_BIG_M).abs() <= 1e-9);
    }
}
