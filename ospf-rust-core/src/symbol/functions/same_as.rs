//! Same-as function symbol.

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
use crate::variable::{BinaryVariableItem, VariableId, new_group_id};
use super::super::{

    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
use super::big_m::{BigMPolicy, infer_linear_difference_abs_bound_from_tokens};

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
const ZERO_INDICATOR_TOLERANCE: f64 = 1.0e-10;
const ZERO_INDICATOR_STRICT_BOUNDARY: f64 = ZERO_INDICATOR_TOLERANCE * 16.0 + f64::EPSILON * 16.0;

/// Checks if two polynomials are effectively equal.
#[derive(Debug, Clone)]
pub struct SameAsFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    first: Linear<V>,
    second: Linear<V>,
    result_var: BinaryVariableItem,
    side_var: BinaryVariableItem,
    tolerance: V,
    declared_dependency_ids: Vec<u64>,
}

impl<V> SameAsFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    pub fn new(id: u64, name: &str, first: Linear<V>, second: Linear<V>, tolerance: V) -> Self {
        let group_id = new_group_id();
        let result_var = BinaryVariableItem::create(VariableId::new(group_id, 0), name);
        let side_var =
            BinaryVariableItem::create(VariableId::new(group_id, 1), &format!("{}_side", name));

        Self {
            id: IntermediateSymbolId::new(id, name),
            first,
            second,
            result_var,
            side_var,
            tolerance,
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

    pub fn first_polynomial(&self) -> &Linear<V> {
        &self.first
    }

    pub fn second_polynomial(&self) -> &Linear<V> {
        &self.second
    }

    pub fn tolerance(&self) -> &V {
        &self.tolerance
    }

    pub fn side_variable(&self) -> &BinaryVariableItem {
        &self.side_var
    }
}

impl<V> SameAsFunction<V>
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
    fn infer_big_m_from_tokens(&self, tokens: &[Token<V>]) -> Option<f64> {
        infer_linear_difference_abs_bound_from_tokens(&self.first, &self.second, tokens)
            .map(|big_m| big_m.max(BIG_M_POLICY.min()))
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
                    "same_as result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;
        let side_index = symbol_to_index
            .get(&(self.side_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "same_as side variable id {}",
                    self.side_var.id().unique_id()
                ))
            })?;

        let tolerance = to_f64(&self.tolerance).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "same_as `{}` tolerance cannot be converted to f64",
                self.id.name
            ))
        })?;
        if !tolerance.is_finite() || tolerance < 0.0 {
            return Err(ModelError::InvalidConstraint(format!(
                "same_as `{}` requires finite non-negative tolerance",
                self.id.name
            ))
            .into());
        }
        if !big_m.is_finite() || big_m < 0.0 {
            return Err(ModelError::InvalidConstraint(format!(
                "same_as `{}` requires finite non-negative big-M",
                self.id.name
            ))
            .into());
        }
        let strict_boundary = tolerance + ZERO_INDICATOR_STRICT_BOUNDARY;

        let mut difference_monomials: Vec<LinearMonomial<V>> = Vec::new();
        for monomial in self.first.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "same_as `{}` first polynomial coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            difference_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(coefficient, "same_as first coefficient")?,
                monomial.var_index(),
            ));
        }
        for monomial in self.second.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "same_as `{}` second polynomial coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            difference_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-coefficient, "same_as second coefficient")?,
                monomial.var_index(),
            ));
        }
        let first_constant = to_f64(self.first.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "same_as `{}` first polynomial constant cannot be converted to f64",
                self.id.name
            ))
        })?;
        let second_constant = to_f64(self.second.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "same_as `{}` second polynomial constant cannot be converted to f64",
                self.id.name
            ))
        })?;
        let difference_constant = first_constant - second_constant;

        let build_constraint = |name_suffix: &str,
                                relation: ConstraintRelation,
                                rhs: f64,
                                result_coefficient: f64,
                                side_coefficient: f64|
         -> Result<LinearConstraint<V>> {
            let mut monomials = difference_monomials.clone();
            monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(result_coefficient, "same_as result coefficient")?,
                result_index,
            ));
            monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(side_coefficient, "same_as side coefficient")?,
                side_index,
            ));
            Ok(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        monomials,
                        convert_f64_to_v::<V>(difference_constant, "same_as constant")?,
                    ),
                    relation,
                    convert_f64_to_v::<V>(rhs, "same_as rhs")?,
                ),
                &format!("{}_{}", self.id.name, name_suffix),
                Arc::new(self.clone()),
            ))
        };

        Ok(vec![
            build_constraint(
                "same_band_ub",
                ConstraintRelation::LessEqual,
                tolerance + big_m,
                big_m,
                0.0,
            )?,
            build_constraint(
                "same_band_lb",
                ConstraintRelation::GreaterEqual,
                -tolerance - big_m,
                -big_m,
                0.0,
            )?,
            build_constraint(
                "same_out_lb",
                ConstraintRelation::GreaterEqual,
                strict_boundary - big_m,
                big_m,
                -big_m,
            )?,
            build_constraint(
                "same_out_ub",
                ConstraintRelation::LessEqual,
                -strict_boundary,
                -big_m,
                -big_m,
            )?,
        ])
    }
}

impl<V> Display for SameAsFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "same_as({})", self.id.name)
    }
}

impl<V> DynSymbol for SameAsFunction<V>
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

impl<V> Symbol for SameAsFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for SameAsFunction<V>
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
        let big_m = BIG_M_POLICY.resolve(self.infer_big_m_from_tokens(tokens));
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
        format!("same_as({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for SameAsFunction<V>
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
        tokens.push(Token::from_generic(
            self.side_var.clone(),
            self.side_var.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let left = to_f64(&evaluate_linear(&self.first, token_table, zero_if_none)?)?;
        let right = to_f64(&evaluate_linear(&self.second, token_table, zero_if_none)?)?;
        let tolerance = to_f64(&self.tolerance)?;
        let eps = f64::EPSILON * 16.0;
        let matched = (left - right).abs() <= tolerance + eps;
        from_f64(if matched { 1.0 } else { 0.0 })
    }
}

impl<V> LinearIntermediateSymbol<V> for SameAsFunction<V>
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
    use crate::model::{ConstraintRelation, LinearConstraint};
    use crate::token::Token;
    use crate::variable::{ContinuousVariableItem, VariableRange};

    fn constraint_lhs(constraint: &LinearConstraint<f64>, values: &HashMap<usize, f64>) -> f64 {
        let mut lhs = *constraint.inequality.polynomial.constant_term();
        for monomial in constraint.inequality.polynomial.monomials() {
            lhs += *monomial.coefficient()
                * values
                    .get(&monomial.var_index())
                    .copied()
                    .unwrap_or_default();
        }
        lhs
    }

    fn satisfies(constraint: &LinearConstraint<f64>, values: &HashMap<usize, f64>) -> bool {
        let lhs = constraint_lhs(constraint, values);
        match constraint.inequality.relation {
            ConstraintRelation::LessEqual => lhs <= constraint.inequality.rhs + 1e-9,
            ConstraintRelation::Equal => (lhs - constraint.inequality.rhs).abs() <= 1e-9,
            ConstraintRelation::GreaterEqual => lhs + 1e-9 >= constraint.inequality.rhs,
        }
    }

    #[test]
    fn same_as_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(70_000),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let f: SameAsFunction<f64> = SameAsFunction::new(
            7000,
            "same_as_bound",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            Linear::new(vec![], 0.0),
            0.0,
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let side_id = f.side_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (side_id, 2usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(f.result_variable().clone(), 1),
            Token::from_generic(f.side_variable().clone(), 2),
        ];

        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("same_as constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "same_as_bound_same_band_ub")
            .expect("same-band upper constraint should exist");
        let result_term = upper
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 1)
            .expect("result term should exist");

        // x in [-2, 3] and 0 => |x - 0| bound is 3.
        assert!((upper.inequality.rhs - 3.0).abs() <= 1e-9);
        assert!((*result_term.coefficient() - 3.0).abs() <= 1e-9);
    }

    #[test]
    fn same_as_function_falls_back_to_default_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(70_010), "x");
        let f: SameAsFunction<f64> = SameAsFunction::new(
            7001,
            "same_as_default",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            Linear::new(vec![], 0.0),
            0.0,
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let side_id = f.side_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (side_id, 2usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(f.result_variable().clone(), 1),
            Token::from_generic(f.side_variable().clone(), 2),
        ];

        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("same_as constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "same_as_default_same_band_ub")
            .expect("same-band upper constraint should exist");
        let result_term = upper
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 1)
            .expect("result term should exist");

        assert!((upper.inequality.rhs - DEFAULT_BIG_M).abs() <= 1e-9);
        assert!((*result_term.coefficient() - DEFAULT_BIG_M).abs() <= 1e-9);
    }

    #[test]
    fn same_as_indicator_treats_zero_difference_as_satisfied() {
        let f: SameAsFunction<f64> = SameAsFunction::new(
            7002,
            "same_as_zero",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            Linear::new(vec![], 0.0),
            0.0,
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let side_id = f.side_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (side_id, 2usize)]);
        let constraints = f
            .build_mechanism_constraints(&symbol_to_index, 10.0)
            .expect("same_as constraints should be generated");

        let satisfied = HashMap::from([(0usize, 0.0), (1usize, 1.0), (2usize, 0.0)]);
        assert!(
            constraints
                .iter()
                .all(|constraint| satisfies(constraint, &satisfied))
        );

        let zero_result_side0 = HashMap::from([(0usize, 0.0), (1usize, 0.0), (2usize, 0.0)]);
        let zero_result_side1 = HashMap::from([(0usize, 0.0), (1usize, 0.0), (2usize, 1.0)]);
        assert!(
            !constraints
                .iter()
                .all(|constraint| satisfies(constraint, &zero_result_side0))
        );
        assert!(
            !constraints
                .iter()
                .all(|constraint| satisfies(constraint, &zero_result_side1))
        );
    }

    #[test]
    fn same_as_indicator_treats_nonzero_difference_as_violated() {
        let f: SameAsFunction<f64> = SameAsFunction::new(
            7003,
            "same_as_nonzero",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            Linear::new(vec![], 0.0),
            0.0,
        );

        let result_id = f.result_variable().id().unique_id() as usize;
        let side_id = f.side_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (side_id, 2usize)]);
        let constraints = f
            .build_mechanism_constraints(&symbol_to_index, 10.0)
            .expect("same_as constraints should be generated");

        let violated = HashMap::from([(0usize, 1.0e-6), (1usize, 0.0), (2usize, 1.0)]);
        assert!(
            constraints
                .iter()
                .all(|constraint| satisfies(constraint, &violated))
        );

        let wrong_result = HashMap::from([(0usize, 1.0e-6), (1usize, 1.0), (2usize, 0.0)]);
        assert!(
            !constraints
                .iter()
                .all(|constraint| satisfies(constraint, &wrong_result))
        );
    }
}
