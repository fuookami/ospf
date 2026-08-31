//! Rounding function symbol.

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;

use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};

use crate::error::{ModelError, Result};
use crate::flatten::{Linear, LinearMonomial, Quadratic};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{
    BinaryVariableItem, ContinuousVariableItem, IntegerVariableItem, VariableId, new_group_id,
};

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
use super::big_m::{BigMPolicy, infer_linear_abs_bound_from_tokens};

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
const ROUNDING_EPSILON: f64 = 1e-8;

/// Rounding type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundingKind {
    Floor,
    Ceil,
    Round,
    Trunc,
}

/// Rounding function symbol.
#[derive(Debug, Clone)]
pub struct RoundingFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Linear<V>,
    kind: RoundingKind,
    result_var: ContinuousVariableItem,
    integer_var: IntegerVariableItem,
    sign_var: Option<BinaryVariableItem>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> RoundingFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    pub fn new(id: u64, name: &str, input: Linear<V>, kind: RoundingKind) -> Self {
        let group_id = new_group_id();
        let result_var = ContinuousVariableItem::create(VariableId::new(group_id, 0), name);
        let integer_var =
            IntegerVariableItem::create(VariableId::new(group_id, 1), &format!("{}_int", name));
        let sign_var = if matches!(kind, RoundingKind::Round | RoundingKind::Trunc) {
            Some(BinaryVariableItem::create(
                VariableId::new(group_id, 2),
                &format!("{}_sign", name),
            ))
        } else {
            None
        };

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            kind,
            result_var,
            integer_var,
            sign_var,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn floor(id: u64, name: &str, input: Linear<V>) -> Self {
        Self::new(id, name, input, RoundingKind::Floor)
    }

    pub fn ceil(id: u64, name: &str, input: Linear<V>) -> Self {
        Self::new(id, name, input, RoundingKind::Ceil)
    }

    pub fn round(id: u64, name: &str, input: Linear<V>) -> Self {
        Self::new(id, name, input, RoundingKind::Round)
    }

    pub fn trunc(id: u64, name: &str, input: Linear<V>) -> Self {
        Self::new(id, name, input, RoundingKind::Trunc)
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub(crate) fn with_input_polynomial(&self, input: Linear<V>) -> Self {
        let mut cloned = self.clone();
        cloned.input = input;
        cloned
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    pub fn integer_variable(&self) -> &IntegerVariableItem {
        &self.integer_var
    }

    pub fn sign_variable(&self) -> Option<&BinaryVariableItem> {
        self.sign_var.as_ref()
    }

    pub fn input_polynomial(&self) -> &Linear<V> {
        &self.input
    }

    pub fn rounding_kind(&self) -> RoundingKind {
        self.kind
    }
}

impl<V> RoundingFunction<V>
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
        infer_linear_abs_bound_from_tokens(&self.input, tokens)
            .map(|bound| bound.max(BIG_M_POLICY.min()))
    }

    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "rounding result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;
        let integer_index = symbol_to_index
            .get(&(self.integer_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "rounding integer variable id {}",
                    self.integer_var.id().unique_id()
                ))
            })?;
        let sign_index = self
            .sign_var
            .as_ref()
            .map(|var| {
                symbol_to_index
                    .get(&(var.id().unique_id() as usize))
                    .copied()
                    .ok_or_else(|| {
                        ModelError::SymbolNotRegistered(format!(
                            "rounding sign variable id {}",
                            var.id().unique_id()
                        ))
                    })
            })
            .transpose()?;

        let mut input_monomials = Vec::with_capacity(self.input.monomials().len());
        for monomial in self.input.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "rounding `{}` input coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            let input_index = monomial.var_index();
            input_monomials.push((coefficient, input_index));
        }
        let input_constant = to_f64(self.input.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "rounding `{}` input constant cannot be converted to f64",
                self.id.name
            ))
        })?;

        let source = Arc::new(self.clone());

        let build_constraint = |name_suffix: &str,
                                relation: ConstraintRelation,
                                rhs: f64,
                                integer_coeff: f64,
                                sign_coeff: f64,
                                constant_offset: f64|
         -> Result<LinearConstraint<V>> {
            let mut monomials = Vec::with_capacity(input_monomials.len() + 2);
            for (coefficient, index) in &input_monomials {
                monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(*coefficient, "rounding input coefficient")?,
                    *index,
                ));
            }
            monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(integer_coeff, "rounding integer coefficient")?,
                integer_index,
            ));
            if sign_coeff != 0.0 {
                let sign_index = sign_index.ok_or_else(|| {
                    ModelError::InvalidConstraint(format!(
                        "rounding `{}` kind {:?} requires sign variable for mechanism injection",
                        self.id.name, self.kind
                    ))
                })?;
                monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(sign_coeff, "rounding sign coefficient")?,
                    sign_index,
                ));
            }
            Ok(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        monomials,
                        convert_f64_to_v::<V>(
                            input_constant + constant_offset,
                            "rounding constant",
                        )?,
                    ),
                    relation,
                    convert_f64_to_v::<V>(rhs, "rounding rhs")?,
                ),
                &format!("{}_{}", self.id.name, name_suffix),
                source.clone(),
            ))
        };

        let mut constraints = Vec::new();

        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    vec![
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "rounding result coefficient")?,
                            result_index,
                        ),
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(-1.0, "rounding integer coefficient")?,
                            integer_index,
                        ),
                    ],
                    convert_f64_to_v::<V>(0.0, "rounding equality constant")?,
                ),
                ConstraintRelation::Equal,
                convert_f64_to_v::<V>(0.0, "rounding equality rhs")?,
            ),
            &format!("{}_result_link", self.id.name),
            source.clone(),
        ));

        match self.kind {
            RoundingKind::Floor => {
                constraints.push(build_constraint(
                    "floor_lb",
                    ConstraintRelation::GreaterEqual,
                    0.0,
                    -1.0,
                    0.0,
                    0.0,
                )?);
                constraints.push(build_constraint(
                    "floor_ub",
                    ConstraintRelation::LessEqual,
                    1.0 - ROUNDING_EPSILON,
                    -1.0,
                    0.0,
                    0.0,
                )?);
            }
            RoundingKind::Ceil => {
                constraints.push(build_constraint(
                    "ceil_ub",
                    ConstraintRelation::LessEqual,
                    0.0,
                    -1.0,
                    0.0,
                    0.0,
                )?);
                constraints.push(build_constraint(
                    "ceil_lb",
                    ConstraintRelation::GreaterEqual,
                    -1.0 + ROUNDING_EPSILON,
                    -1.0,
                    0.0,
                    0.0,
                )?);
            }
            RoundingKind::Trunc => {
                constraints.push(build_constraint(
                    "sign_lb",
                    ConstraintRelation::GreaterEqual,
                    -big_m,
                    0.0,
                    -big_m,
                    0.0,
                )?);
                constraints.push(build_constraint(
                    "sign_ub",
                    ConstraintRelation::LessEqual,
                    0.0,
                    0.0,
                    -big_m,
                    0.0,
                )?);

                constraints.push(build_constraint(
                    "trunc_pos_lb",
                    ConstraintRelation::GreaterEqual,
                    0.0,
                    -1.0,
                    -big_m,
                    big_m,
                )?);
                constraints.push(build_constraint(
                    "trunc_pos_ub",
                    ConstraintRelation::LessEqual,
                    1.0 - ROUNDING_EPSILON + big_m,
                    -1.0,
                    big_m,
                    0.0,
                )?);
                constraints.push(build_constraint(
                    "trunc_neg_ub",
                    ConstraintRelation::LessEqual,
                    0.0,
                    -1.0,
                    -big_m,
                    0.0,
                )?);
                constraints.push(build_constraint(
                    "trunc_neg_lb",
                    ConstraintRelation::GreaterEqual,
                    -1.0 + ROUNDING_EPSILON,
                    -1.0,
                    big_m,
                    0.0,
                )?);
            }
            RoundingKind::Round => {
                constraints.push(build_constraint(
                    "sign_lb",
                    ConstraintRelation::GreaterEqual,
                    -big_m,
                    0.0,
                    -big_m,
                    0.0,
                )?);
                constraints.push(build_constraint(
                    "sign_ub",
                    ConstraintRelation::LessEqual,
                    0.0,
                    0.0,
                    -big_m,
                    0.0,
                )?);

                constraints.push(build_constraint(
                    "round_pos_lb",
                    ConstraintRelation::GreaterEqual,
                    -0.5,
                    -1.0,
                    -big_m,
                    big_m,
                )?);
                constraints.push(build_constraint(
                    "round_pos_ub",
                    ConstraintRelation::LessEqual,
                    0.5 - ROUNDING_EPSILON + big_m,
                    -1.0,
                    big_m,
                    0.0,
                )?);
                constraints.push(build_constraint(
                    "round_neg_ub",
                    ConstraintRelation::LessEqual,
                    0.5,
                    -1.0,
                    -big_m,
                    0.0,
                )?);
                constraints.push(build_constraint(
                    "round_neg_lb",
                    ConstraintRelation::GreaterEqual,
                    -0.5 + ROUNDING_EPSILON,
                    -1.0,
                    big_m,
                    0.0,
                )?);
            }
        }

        Ok(constraints)
    }
}

impl<V> Display for RoundingFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            RoundingKind::Floor => write!(f, "floor({})", self.id.name),
            RoundingKind::Ceil => write!(f, "ceil({})", self.id.name),
            RoundingKind::Round => write!(f, "round({})", self.id.name),
            RoundingKind::Trunc => write!(f, "trunc({})", self.id.name),
        }
    }
}

impl<V> DynSymbol for RoundingFunction<V>
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

impl<V> Symbol for RoundingFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for RoundingFunction<V>
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
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.build_mechanism_constraints(symbol_to_index, BIG_M_POLICY.fallback())
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
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
        match self.kind {
            RoundingKind::Floor => format!("floor({})", self.id.name),
            RoundingKind::Ceil => format!("ceil({})", self.id.name),
            RoundingKind::Round => format!("round({})", self.id.name),
            RoundingKind::Trunc => format!("trunc({})", self.id.name),
        }
    }
}

impl<V> FunctionSymbol<V> for RoundingFunction<V>
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
            self.integer_var.clone(),
            self.integer_var.index(),
        ));
        if let Some(sign_var) = &self.sign_var {
            tokens.push(Token::from_generic(sign_var.clone(), sign_var.index()));
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let value = evaluate_linear(&self.input, token_table, zero_if_none)?;
        let value = to_f64(&value)?;
        let rounded = match self.kind {
            RoundingKind::Floor => value.floor(),
            RoundingKind::Ceil => value.ceil(),
            RoundingKind::Round => value.round(),
            RoundingKind::Trunc => value.trunc(),
        };
        from_f64(rounded)
    }
}

impl<V> LinearIntermediateSymbol<V> for RoundingFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + One
        + ToPrimitive
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn to_linear_polynomial(&self) -> Linear<V> {
        Linear::new(
            vec![LinearMonomial::new(V::one(), self.result_var.index())],
            V::zero(),
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
    use crate::variable::{ContinuousVariableItem, VariableRange};

    #[test]
    fn rounding_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(90_000),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let f: RoundingFunction<f64> = RoundingFunction::trunc(
            3000,
            "trunc_bound",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
        );

        let mut aux_tokens = Vec::new();
        <RoundingFunction<f64> as FunctionSymbol<f64>>::register_tokens(&f, &mut aux_tokens)
            .expect("rounding tokens should be registered");
        let mut symbol_to_index = HashMap::new();
        for token in &aux_tokens {
            symbol_to_index.insert(token.id().unique_id() as usize, token.solver_index);
        }
        let tokens = vec![Token::from_generic(x, 0)];

        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("rounding constraints should be generated");
        let sign_lb = constraints
            .iter()
            .find(|constraint| constraint.name == "trunc_bound_sign_lb")
            .expect("trunc sign_lb constraint should exist");
        let sign_index = symbol_to_index
            .get(&(f.sign_variable().expect("trunc should have sign var").id().unique_id() as usize))
            .copied()
            .expect("sign variable index should exist");
        let sign_term = sign_lb
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == sign_index)
            .expect("sign term should exist");

        assert!((*sign_term.coefficient() + 7.0).abs() <= 1e-9);
    }

    #[test]
    fn rounding_function_falls_back_to_default_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(90_010), "x");
        let f: RoundingFunction<f64> = RoundingFunction::trunc(
            3001,
            "trunc_default",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );

        let mut aux_tokens = Vec::new();
        <RoundingFunction<f64> as FunctionSymbol<f64>>::register_tokens(&f, &mut aux_tokens)
            .expect("rounding tokens should be registered");
        let mut symbol_to_index = HashMap::new();
        for token in &aux_tokens {
            symbol_to_index.insert(token.id().unique_id() as usize, token.solver_index);
        }
        let tokens = vec![Token::from_generic(x, 0)];

        let constraints = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("rounding constraints should be generated");
        let sign_lb = constraints
            .iter()
            .find(|constraint| constraint.name == "trunc_default_sign_lb")
            .expect("trunc sign_lb constraint should exist");
        let sign_index = symbol_to_index
            .get(&(f.sign_variable().expect("trunc should have sign var").id().unique_id() as usize))
            .copied()
            .expect("sign variable index should exist");
        let sign_term = sign_lb
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == sign_index)
            .expect("sign term should exist");

        assert!((*sign_term.coefficient() + DEFAULT_BIG_M).abs() <= 1e-9);
    }
}
