//! 离散 CVaR 函数 / Discrete CVaR functions

use std::any::Any;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Div, Mul, Sub};
use std::sync::Arc;

use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};

use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::symbol::function::big_m::infer_linear_bounds_from_tokens;
use crate::symbol::function::max::{MaxFunction, MinFunction};
use crate::symbol::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{ContinuousVariableItem, VariableId, VariableRange, new_group_id};

#[derive(Debug, Clone)]
enum CvarKind<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    Exact {
        positive_parts: Vec<Vec<MaxFunction<V>>>,
        minimum: MinFunction<V>,
    },
    Epigraph {
        threshold: ContinuousVariableItem,
        excesses: Vec<ContinuousVariableItem>,
    },
}

/// 离散 CVaR 的共享数据与线性化实现 / Shared data and linearization for discrete CVaR.
#[derive(Debug, Clone)]
struct CvarCore<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    losses: Vec<Linear<V>>,
    probabilities: Vec<V>,
    alpha: V,
    denominator: V,
    weights: Vec<V>,
    kind: CvarKind<V>,
}

impl<V> CvarCore<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Div<Output = V>
        + Mul<Output = V>
        + Sub<Output = V>
        + PartialOrd
        + FromPrimitive
        + ToPrimitive
        + One
        + Zero,
    f64: IntoValue<V>,
{
    fn validate_inputs(losses: &[Linear<V>], probabilities: &[V], alpha: &V) -> Result<V> {
        if losses.is_empty() || losses.len() != probabilities.len() {
            return Err(ModelError::InvalidConstraint(
                "CVaR losses and probabilities must be non-empty and have equal sizes".into(),
            )
            .into());
        }
        if !Self::is_finite(alpha)
            || alpha.partial_cmp(&V::zero()) == Some(Ordering::Less)
            || alpha.partial_cmp(&V::one()) != Some(Ordering::Less)
        {
            return Err(ModelError::InvalidConstraint(
                "CVaR alpha must be finite and in [0, 1)".into(),
            )
            .into());
        }
        if probabilities
            .iter()
            .any(|probability| !Self::is_finite(probability) || probability < &V::zero())
        {
            return Err(ModelError::InvalidConstraint(
                "CVaR probabilities must be finite and non-negative".into(),
            )
            .into());
        }
        let probability_sum = probabilities
            .iter()
            .cloned()
            .fold(V::zero(), |sum, probability| sum + probability);
        if probability_sum.partial_cmp(&V::one()) != Some(Ordering::Equal) {
            return Err(
                ModelError::InvalidConstraint("CVaR probabilities must sum to one".into()).into(),
            );
        }
        let denominator = V::one() - alpha.clone();
        if !Self::is_finite(&denominator) || denominator <= V::zero() {
            return Err(ModelError::InvalidConstraint(
                "CVaR denominator 1 - alpha must be finite and positive".into(),
            )
            .into());
        }
        Ok(denominator)
    }

    fn is_finite(value: &V) -> bool
    where
        V: ToPrimitive,
    {
        value.to_f64().is_some_and(f64::is_finite)
    }

    fn new_exact(
        id: u64,
        name: &str,
        losses: Vec<Linear<V>>,
        probabilities: Vec<V>,
        alpha: V,
    ) -> Result<Self> {
        let denominator = Self::validate_inputs(&losses, &probabilities, &alpha)?;
        let weights = probabilities
            .iter()
            .map(|probability| probability.clone() / denominator.clone())
            .collect::<Vec<_>>();
        if weights.iter().any(|weight| !Self::is_finite(weight)) {
            return Err(ModelError::InvalidConstraint(
                "CVaR linearization weights must be finite".into(),
            )
            .into());
        }

        let mut positive_parts = Vec::with_capacity(losses.len());
        for (threshold_index, threshold) in losses.iter().enumerate() {
            let mut parts = Vec::with_capacity(losses.len());
            for (loss_index, loss) in losses.iter().enumerate() {
                let difference = Self::difference(loss, threshold);
                if !difference
                    .monomials()
                    .iter()
                    .all(|monomial| Self::is_finite(monomial.coefficient()))
                    || !Self::is_finite(difference.constant_term())
                {
                    return Err(ModelError::InvalidConstraint(
                        "CVaR loss differences must be finite".into(),
                    )
                    .into());
                }
                let zero = Linear::new(Vec::new(), V::zero());
                let part_name = format!("{}_excess_{}_{}", name, threshold_index, loss_index);
                parts.push(MaxFunction::new(
                    crate::symbol::next_auto_intermediate_symbol_id(),
                    &part_name,
                    vec![difference, zero],
                    true,
                ));
            }
            positive_parts.push(parts);
        }

        let placeholders =
            Self::candidate_polynomials_for_construction(&losses, &weights, &positive_parts)?;
        let minimum = MinFunction::new(
            crate::symbol::next_auto_intermediate_symbol_id(),
            &format!("{}_minimum", name),
            placeholders,
            true,
        );

        Ok(Self {
            id: IntermediateSymbolId::new(id, name),
            losses,
            probabilities,
            alpha,
            denominator,
            weights,
            kind: CvarKind::Exact {
                positive_parts,
                minimum,
            },
        })
    }

    fn new_epigraph(
        id: u64,
        name: &str,
        losses: Vec<Linear<V>>,
        probabilities: Vec<V>,
        alpha: V,
    ) -> Result<Self> {
        let denominator = Self::validate_inputs(&losses, &probabilities, &alpha)?;
        let weights = probabilities
            .iter()
            .map(|probability| probability.clone() / denominator.clone())
            .collect::<Vec<_>>();
        if weights.iter().any(|weight| !Self::is_finite(weight)) {
            return Err(ModelError::InvalidConstraint(
                "CVaR linearization weights must be finite".into(),
            )
            .into());
        }
        let group_id = new_group_id();
        let threshold = ContinuousVariableItem::with_range(
            VariableId::new(group_id, 0),
            &format!("{}_threshold", name),
            VariableRange::<f64>::unbounded(),
        );
        let excesses = losses
            .iter()
            .enumerate()
            .map(|(index, _)| {
                ContinuousVariableItem::with_range(
                    VariableId::new(group_id, index + 1),
                    &format!("{}_excess_{}", name, index),
                    VariableRange::with_lower(0.0),
                )
            })
            .collect();

        Ok(Self {
            id: IntermediateSymbolId::new(id, name),
            losses,
            probabilities,
            alpha,
            denominator,
            weights,
            kind: CvarKind::Epigraph {
                threshold,
                excesses,
            },
        })
    }

    fn difference(left: &Linear<V>, right: &Linear<V>) -> Linear<V> {
        let mut monomials = left.monomials().to_vec();
        monomials.extend(right.monomials().iter().map(|monomial| {
            LinearMonomial::new(
                V::zero() - monomial.coefficient().clone(),
                monomial.var_index(),
            )
        }));
        Linear::new(
            monomials,
            left.constant_term().clone() - right.constant_term().clone(),
        )
    }

    fn candidate_polynomials_for_construction(
        losses: &[Linear<V>],
        weights: &[V],
        positive_parts: &[Vec<MaxFunction<V>>],
    ) -> Result<Vec<Linear<V>>> {
        let mut candidates = Vec::with_capacity(losses.len());
        for (threshold_index, loss) in losses.iter().enumerate() {
            let mut monomials = loss.monomials().to_vec();
            for (loss_index, weight) in weights.iter().enumerate() {
                let result = positive_parts[threshold_index][loss_index].result_variable();
                monomials.push(LinearMonomial::new(weight.clone(), result.index()));
            }
            candidates.push(Linear::new(monomials, loss.constant_term().clone()));
        }
        Ok(candidates)
    }

    fn mapped_candidates(&self, symbol_to_index: &HashMap<usize, usize>) -> Result<Vec<Linear<V>>> {
        let CvarKind::Exact { positive_parts, .. } = &self.kind else {
            return Err(ModelError::InvalidConstraint(
                "CVaR candidates are only available in exact mode".into(),
            )
            .into());
        };
        let mut candidates = Vec::with_capacity(self.losses.len());
        for (threshold_index, loss) in self.losses.iter().enumerate() {
            let mut monomials = loss.monomials().to_vec();
            for (loss_index, weight) in self.weights.iter().enumerate() {
                let result = positive_parts[threshold_index][loss_index].result_variable();
                let index = Self::index_for(symbol_to_index, result, "CVaR positive-part")?;
                monomials.push(LinearMonomial::new(weight.clone(), index));
            }
            candidates.push(Linear::new(monomials, loss.constant_term().clone()));
        }
        Ok(candidates)
    }

    fn index_for(
        symbol_to_index: &HashMap<usize, usize>,
        variable: &ContinuousVariableItem,
        role: &str,
    ) -> Result<usize> {
        symbol_to_index
            .get(&(variable.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "{} variable id {}",
                    role,
                    variable.id().unique_id()
                ))
                .into()
            })
    }

    fn result_variable(&self) -> Option<&ContinuousVariableItem> {
        match &self.kind {
            CvarKind::Exact { minimum, .. } => Some(minimum.result_variable()),
            CvarKind::Epigraph { .. } => None,
        }
    }

    fn threshold_variable(&self) -> Option<&ContinuousVariableItem> {
        match &self.kind {
            CvarKind::Epigraph { threshold, .. } => Some(threshold),
            CvarKind::Exact { .. } => None,
        }
    }

    fn excess_variables(&self) -> &[ContinuousVariableItem] {
        match &self.kind {
            CvarKind::Epigraph { excesses, .. } => excesses,
            CvarKind::Exact { .. } => &[],
        }
    }

    fn result_polynomial(&self) -> Linear<V>
    where
        V: One + Zero,
    {
        match &self.kind {
            CvarKind::Exact { minimum, .. } => Linear::new(
                vec![LinearMonomial::new(
                    V::one(),
                    minimum.result_variable().index(),
                )],
                V::zero(),
            ),
            CvarKind::Epigraph {
                threshold,
                excesses,
            } => {
                let mut monomials = vec![LinearMonomial::new(V::one(), threshold.index())];
                monomials.extend(
                    excesses
                        .iter()
                        .zip(&self.weights)
                        .map(|(variable, weight)| {
                            LinearMonomial::new(weight.clone(), variable.index())
                        }),
                );
                Linear::new(monomials, V::zero())
            }
        }
    }

    fn result_expression(&self, symbol_to_index: &HashMap<usize, usize>) -> Result<Linear<V>>
    where
        V: One + Zero,
    {
        match &self.kind {
            CvarKind::Exact { minimum, .. } => Ok(Linear::new(
                vec![LinearMonomial::new(
                    V::one(),
                    Self::index_for(symbol_to_index, minimum.result_variable(), "CVaR result")?,
                )],
                V::zero(),
            )),
            CvarKind::Epigraph {
                threshold,
                excesses,
            } => {
                let threshold_index =
                    Self::index_for(symbol_to_index, threshold, "CVaR threshold")?;
                let mut monomials = vec![LinearMonomial::new(V::one(), threshold_index)];
                for (variable, weight) in excesses.iter().zip(&self.weights) {
                    monomials.push(LinearMonomial::new(
                        weight.clone(),
                        Self::index_for(symbol_to_index, variable, "CVaR excess")?,
                    ));
                }
                Ok(Linear::new(monomials, V::zero()))
            }
        }
    }

    fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()>
    where
        V: Add<Output = V>
            + Div<Output = V>
            + Mul<Output = V>
            + Sub<Output = V>
            + PartialOrd
            + FromPrimitive
            + ToPrimitive
            + One
            + Zero,
        f64: IntoValue<V>,
    {
        match &self.kind {
            CvarKind::Exact {
                positive_parts,
                minimum,
            } => {
                minimum.register_tokens(tokens)?;
                for part in positive_parts.iter().flatten() {
                    part.register_tokens(tokens)?;
                }
            }
            CvarKind::Epigraph {
                threshold,
                excesses,
            } => {
                tokens.push(Token::from_generic(threshold.clone(), threshold.index()));
                for excess in excesses {
                    tokens.push(Token::from_generic(excess.clone(), excess.index()));
                }
            }
        }
        Ok(())
    }

    fn finite_loss_bounds(&self, tokens: &[Token<V>]) -> Result<Vec<(f64, f64)>>
    where
        V: ToPrimitive,
    {
        self.losses
            .iter()
            .map(|loss| {
                let (lower, upper) =
                    infer_linear_bounds_from_tokens(loss, tokens).ok_or_else(|| {
                        ModelError::InvalidConstraint(format!(
                            "{} requires finite bounds for every scenario loss",
                            self.id.name
                        ))
                    })?;
                if !lower.is_finite() || !upper.is_finite() || !(upper - lower).is_finite() {
                    return Err(ModelError::InvalidConstraint(format!(
                        "{} scenario loss bounds are not representable",
                        self.id.name
                    ))
                    .into());
                }
                Ok((lower, upper))
            })
            .collect()
    }

    fn refine(&self, auxiliary: &mut [Token<V>], registered: &[Token<V>]) -> Result<()>
    where
        V: Add<Output = V>
            + Div<Output = V>
            + Mul<Output = V>
            + Sub<Output = V>
            + PartialOrd
            + FromPrimitive
            + ToPrimitive
            + One
            + Zero,
        f64: IntoValue<V>,
    {
        let loss_bounds = self.finite_loss_bounds(registered)?;
        match &self.kind {
            CvarKind::Exact {
                positive_parts,
                minimum,
            } => {
                for part in positive_parts.iter().flatten() {
                    part.refine_auxiliary_tokens(auxiliary, registered)?;
                }
                let (minimum_loss, maximum_loss) = loss_bounds.iter().fold(
                    (f64::INFINITY, f64::NEG_INFINITY),
                    |(lower, upper), (loss_lower, loss_upper)| {
                        (lower.min(*loss_lower), upper.max(*loss_upper))
                    },
                );
                if !(maximum_loss - minimum_loss).is_finite() {
                    return Err(ModelError::InvalidConstraint(format!(
                        "{} total loss range is not representable",
                        self.id.name
                    ))
                    .into());
                }
                for threshold_bounds in &loss_bounds {
                    let mut candidate_lower = threshold_bounds.0;
                    let mut candidate_upper = threshold_bounds.1;
                    for (loss_index, loss_bounds) in loss_bounds.iter().enumerate() {
                        let difference_lower = loss_bounds.0 - threshold_bounds.1;
                        let difference_upper = loss_bounds.1 - threshold_bounds.0;
                        if !difference_lower.is_finite() || !difference_upper.is_finite() {
                            return Err(ModelError::InvalidConstraint(format!(
                                "{} loss-difference bounds are not representable",
                                self.id.name
                            ))
                            .into());
                        }
                        let positive_lower = difference_lower.max(0.0);
                        let positive_upper = difference_upper.max(0.0);
                        let weight = self.weights[loss_index].to_f64().ok_or_else(|| {
                            ModelError::InvalidConstraint(
                                "CVaR weight cannot be represented as f64".into(),
                            )
                        })?;
                        candidate_lower += weight * positive_lower;
                        candidate_upper += weight * positive_upper;
                        if !candidate_lower.is_finite() || !candidate_upper.is_finite() {
                            return Err(ModelError::InvalidConstraint(format!(
                                "{} candidate bounds are not representable",
                                self.id.name
                            ))
                            .into());
                        }
                    }
                    if !(candidate_upper - candidate_lower).is_finite() {
                        return Err(ModelError::InvalidConstraint(format!(
                            "{} candidate range width is not representable",
                            self.id.name
                        ))
                        .into());
                    }
                }

                let (candidate_map, bound_tokens) =
                    self.temporary_helper_indexes(positive_parts, auxiliary, registered)?;
                let candidates = self.mapped_candidates(&candidate_map)?;
                let mapped_minimum = minimum.with_polynomials(candidates.clone());
                mapped_minimum.refine_auxiliary_tokens(auxiliary, &bound_tokens)?;
                for candidate in &candidates {
                    if infer_linear_bounds_from_tokens(candidate, &bound_tokens).is_none() {
                        return Err(ModelError::InvalidConstraint(format!(
                            "{} requires finite bounds for every exact CVaR candidate",
                            self.id.name
                        ))
                        .into());
                    }
                }
                Ok(())
            }
            CvarKind::Epigraph {
                threshold,
                excesses,
            } => {
                let minimum = loss_bounds
                    .iter()
                    .map(|bounds| bounds.0)
                    .fold(f64::INFINITY, f64::min);
                let maximum = loss_bounds
                    .iter()
                    .map(|bounds| bounds.1)
                    .fold(f64::NEG_INFINITY, f64::max);
                if !(maximum - minimum).is_finite() {
                    return Err(ModelError::InvalidConstraint(format!(
                        "{} total loss range is not representable",
                        self.id.name
                    ))
                    .into());
                }
                Self::tighten_range(auxiliary, threshold, minimum, maximum)?;
                for (variable, (_, loss_upper)) in excesses.iter().zip(&loss_bounds) {
                    let upper = (loss_upper - minimum).max(0.0);
                    if !upper.is_finite() {
                        return Err(ModelError::InvalidConstraint(format!(
                            "{} excess-variable upper bound is not representable",
                            self.id.name
                        ))
                        .into());
                    }
                    Self::tighten_range(auxiliary, variable, 0.0, upper)?;
                }
                Ok(())
            }
        }
    }

    fn temporary_helper_indexes(
        &self,
        positive_parts: &[Vec<MaxFunction<V>>],
        auxiliary: &[Token<V>],
        registered: &[Token<V>],
    ) -> Result<(HashMap<usize, usize>, Vec<Token<V>>)> {
        let mut next_index = registered
            .iter()
            .filter_map(|token| (token.solver_index != usize::MAX).then_some(token.solver_index))
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .filter(|index| *index < usize::MAX)
            .ok_or_else(|| {
                ModelError::InvalidConstraint(
                    "cannot allocate temporary CVaR bounds indexes".into(),
                )
            })?;
        let mut indexes = HashMap::new();
        let mut tokens = registered.to_vec();
        for part in positive_parts.iter().flatten() {
            let variable = part.result_variable();
            let mut token = auxiliary
                .iter()
                .find(|token| token.id() == variable.id())
                .cloned()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "CVaR positive-part variable id {}",
                        variable.id().unique_id()
                    ))
                })?;
            token.solver_index = next_index;
            indexes.insert(variable.id().unique_id() as usize, next_index);
            tokens.push(token);
            next_index = next_index.checked_add(1).ok_or_else(|| {
                ModelError::InvalidConstraint(
                    "cannot allocate temporary CVaR bounds indexes".into(),
                )
            })?;
        }
        Ok((indexes, tokens))
    }

    fn tighten_range(
        auxiliary: &mut [Token<V>],
        variable: &ContinuousVariableItem,
        lower: f64,
        upper: f64,
    ) -> Result<()>
    where
        V: FromPrimitive + ToPrimitive,
    {
        let token = auxiliary
            .iter_mut()
            .find(|token| token.id() == variable.id())
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "CVaR helper variable id {}",
                    variable.id().unique_id()
                ))
            })?;
        let range = token.range();
        let lower = range
            .lower_bound
            .as_ref()
            .and_then(ToPrimitive::to_f64)
            .map_or(lower, |current| current.max(lower));
        let upper = range
            .upper_bound
            .as_ref()
            .and_then(ToPrimitive::to_f64)
            .map_or(upper, |current| current.min(upper));
        if lower > upper || !lower.is_finite() || !upper.is_finite() {
            return Err(ModelError::InvalidConstraint(format!(
                "CVaR helper `{}` has inconsistent finite bounds [{}, {}]",
                variable.name(),
                lower,
                upper
            ))
            .into());
        }
        let (Some(lower), Some(upper)) = (V::from_f64(lower), V::from_f64(upper)) else {
            return Err(ModelError::InvalidConstraint(
                "CVaR helper bounds cannot be represented by the model value type".into(),
            )
            .into());
        };
        token.set_range(VariableRange::bounded(lower, upper));
        Ok(())
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: Option<&[Token<V>]>,
        source: Arc<dyn IntermediateSymbol<V>>,
    ) -> Result<Vec<LinearConstraint<V>>>
    where
        V: Add<Output = V>
            + Div<Output = V>
            + Mul<Output = V>
            + Sub<Output = V>
            + PartialOrd
            + FromPrimitive
            + ToPrimitive
            + One
            + Zero,
        f64: IntoValue<V>,
    {
        match &self.kind {
            CvarKind::Exact {
                positive_parts,
                minimum,
            } => {
                let tokens = tokens.ok_or_else(|| {
                    ModelError::InvalidConstraint(format!(
                        "{} requires token bounds to generate exact CVaR constraints",
                        self.id.name
                    ))
                })?;
                self.finite_loss_bounds(tokens)?;
                let mut constraints = Vec::new();
                for part in positive_parts.iter().flatten() {
                    constraints
                        .extend(part.mechanism_constraints_with_tokens(symbol_to_index, tokens)?);
                }
                let candidates = self.mapped_candidates(symbol_to_index)?;
                let mapped_minimum = minimum.with_polynomials(candidates);
                constraints.extend(
                    mapped_minimum.mechanism_constraints_with_tokens(symbol_to_index, tokens)?,
                );
                Ok(constraints)
            }
            CvarKind::Epigraph {
                threshold,
                excesses,
            } => {
                if let Some(tokens) = tokens {
                    self.finite_loss_bounds(tokens)?;
                }
                let threshold_index =
                    Self::index_for(symbol_to_index, threshold, "CVaR threshold")?;
                let mut constraints = Vec::with_capacity(self.losses.len());
                for (index, (loss, excess)) in self.losses.iter().zip(excesses.iter()).enumerate() {
                    let excess_index = Self::index_for(symbol_to_index, excess, "CVaR excess")?;
                    let mut monomials = loss.monomials().to_vec();
                    monomials.push(LinearMonomial::new(
                        V::from_i64(-1).ok_or_else(|| {
                            ModelError::InvalidConstraint(
                                "cannot represent CVaR coefficient -1".into(),
                            )
                        })?,
                        threshold_index,
                    ));
                    monomials.push(LinearMonomial::new(
                        V::from_i64(-1).ok_or_else(|| {
                            ModelError::InvalidConstraint(
                                "cannot represent CVaR coefficient -1".into(),
                            )
                        })?,
                        excess_index,
                    ));
                    constraints.push(LinearConstraint::from_symbol(
                        LinearInequality::new(
                            Linear::new(monomials, loss.constant_term().clone()),
                            ConstraintRelation::LessEqual,
                            V::zero(),
                        ),
                        &format!("{}_excess_lb_{}", self.id.name, index),
                        source.clone(),
                    ));
                }
                Ok(constraints)
            }
        }
    }

    fn evaluate(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V>
    where
        V: Add<Output = V>
            + Div<Output = V>
            + Mul<Output = V>
            + Sub<Output = V>
            + PartialOrd
            + Zero,
    {
        let evaluated = self
            .losses
            .iter()
            .map(|loss| Self::evaluate_linear(loss, token_table, zero_if_none))
            .collect::<Option<Vec<_>>>()?;
        let mut minimum = None;
        for threshold in &evaluated {
            let mut candidate = threshold.clone();
            for (loss, probability) in evaluated.iter().zip(&self.probabilities) {
                if loss.partial_cmp(threshold)? == Ordering::Greater {
                    candidate = candidate
                        + probability.clone() * (loss.clone() - threshold.clone())
                            / self.denominator.clone();
                }
            }
            if minimum
                .as_ref()
                .is_none_or(|current: &V| candidate.partial_cmp(current) == Some(Ordering::Less))
            {
                minimum = Some(candidate);
            }
        }
        minimum
    }

    fn evaluate_linear(
        polynomial: &Linear<V>,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V>
    where
        V: Add<Output = V> + Mul<Output = V> + Zero,
    {
        let mut value = polynomial.constant_term().clone();
        for monomial in polynomial.monomials() {
            let token_value = match token_table
                .find_by_index(monomial.var_index())
                .and_then(Token::get_result)
            {
                Some(value) => value,
                None if zero_if_none => V::zero(),
                None => return None,
            };
            value = value + monomial.coefficient().clone() * token_value;
        }
        Some(value)
    }
}

/// 离散概率加权损失的精确 CVaR / Exact CVaR of a discrete probability-weighted loss.
///
/// 对每个观测损失值枚举阈值，并精确线性化正部与最小值；损失必须具有有限边界。
/// Enumerates every observed loss as a threshold and exactly linearizes positive parts and the minimum;
/// all losses must have finite bounds.
#[derive(Debug, Clone)]
pub struct CvarFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    core: CvarCore<V>,
}

impl<V> CvarFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Div<Output = V>
        + Mul<Output = V>
        + Sub<Output = V>
        + PartialOrd
        + FromPrimitive
        + ToPrimitive
        + One
        + Zero,
    f64: IntoValue<V>,
{
    /// 创建精确离散 CVaR 符号 / Create an exact discrete CVaR symbol.
    pub fn new(
        id: u64,
        name: &str,
        losses: Vec<Linear<V>>,
        probabilities: Vec<V>,
        alpha: V,
    ) -> Result<Self> {
        Ok(Self {
            core: CvarCore::new_exact(id, name, losses, probabilities, alpha)?,
        })
    }

    /// 获取精确结果变量 / Get the exact result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.core
            .result_variable()
            .expect("exact CVaR has a result variable")
    }

    /// 按已注册变量 ID 映射生成 CVaR 结果表达式 / Build the CVaR result expression from registered variable IDs.
    pub fn result_expression(&self, symbol_to_index: &HashMap<usize, usize>) -> Result<Linear<V>>
    where
        V: One + Zero,
    {
        self.core.result_expression(symbol_to_index)
    }
}

/// CVaR 的 LP epigraph 表示 / LP epigraph representation of CVaR.
///
/// 该形式仅在最小化或用于上界约束时收紧；损失必须具有有限边界。
/// This form is tight only when minimized or used in an upper-bound constraint; all losses must have finite bounds.
#[derive(Debug, Clone)]
pub struct CvarEpigraphFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    core: CvarCore<V>,
}

impl<V> CvarEpigraphFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Div<Output = V>
        + Mul<Output = V>
        + Sub<Output = V>
        + PartialOrd
        + FromPrimitive
        + ToPrimitive
        + One
        + Zero,
    f64: IntoValue<V>,
{
    /// 创建 CVaR LP epigraph 符号 / Create a CVaR LP epigraph symbol.
    pub fn new(
        id: u64,
        name: &str,
        losses: Vec<Linear<V>>,
        probabilities: Vec<V>,
        alpha: V,
    ) -> Result<Self> {
        Ok(Self {
            core: CvarCore::new_epigraph(id, name, losses, probabilities, alpha)?,
        })
    }

    /// 获取 CVaR 阈值变量 / Get the CVaR threshold variable.
    pub fn threshold_variable(&self) -> &ContinuousVariableItem {
        self.core
            .threshold_variable()
            .expect("CVaR epigraph has a threshold variable")
    }

    /// 获取各情景过量变量 / Get scenario excess variables.
    pub fn excess_variables(&self) -> &[ContinuousVariableItem] {
        self.core.excess_variables()
    }

    /// 按已注册变量 ID 映射生成 CVaR 结果表达式 / Build the CVaR result expression from registered variable IDs.
    pub fn result_expression(&self, symbol_to_index: &HashMap<usize, usize>) -> Result<Linear<V>>
    where
        V: One + Zero,
    {
        self.core.result_expression(symbol_to_index)
    }
}

macro_rules! impl_cvar_symbol {
    ($wrapper:ident) => {
        impl<V> Display for $wrapper<V>
        where
            V: Clone + Debug + Send + Sync + 'static,
        {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "cvar({})", self.core.id.name)
            }
        }

        impl<V> DynSymbol for $wrapper<V>
        where
            V: Clone + Debug + Send + Sync + 'static,
        {
            fn name(&self) -> &str {
                &self.core.id.name
            }
            fn display_name(&self) -> &str {
                &self.core.id.name
            }
            fn dyn_id(&self) -> SymbolDynId<'_> {
                SymbolDynId::standalone(self.core.id.id as usize)
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
        }

        impl<V> Symbol for $wrapper<V>
        where
            V: Clone + Debug + Send + Sync + 'static,
        {
            type Id = IntermediateSymbolId;
            fn id(&self) -> Self::Id {
                self.core.id.clone()
            }
        }

        impl<V> IntermediateSymbol<V> for $wrapper<V>
        where
            V: Clone
                + Debug
                + Send
                + Sync
                + 'static
                + Add<Output = V>
                + Div<Output = V>
                + Mul<Output = V>
                + Sub<Output = V>
                + PartialOrd
                + FromPrimitive
                + ToPrimitive
                + One
                + Zero,
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
            fn flush(&self, _force: bool) {}
            fn register_auxiliary_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
                self.core.register_tokens(tokens)
            }
            fn register_auxiliary_tokens_with_context(
                &self,
                auxiliary: &mut Vec<Token<V>>,
                registered: &[Token<V>],
            ) -> Result<()> {
                self.core.register_tokens(auxiliary)?;
                self.core.refine(auxiliary, registered)
            }
            fn refine_auxiliary_tokens(
                &self,
                auxiliary: &mut [Token<V>],
                registered: &[Token<V>],
            ) -> Result<()> {
                self.core.refine(auxiliary, registered)
            }
            fn mechanism_constraints(
                &self,
                symbol_to_index: &HashMap<usize, usize>,
            ) -> Result<Vec<LinearConstraint<V>>> {
                self.core
                    .mechanism_constraints(symbol_to_index, None, Arc::new(self.clone()))
            }
            fn mechanism_constraints_with_tokens(
                &self,
                symbol_to_index: &HashMap<usize, usize>,
                tokens: &[Token<V>],
            ) -> Result<Vec<LinearConstraint<V>>> {
                self.core.mechanism_constraints(
                    symbol_to_index,
                    Some(tokens),
                    Arc::new(self.clone()),
                )
            }
            fn evaluate_from_tokens(
                &self,
                token_table: &dyn TokenList<V>,
                zero_if_none: bool,
            ) -> Option<V> {
                self.core.evaluate(token_table, zero_if_none)
            }
            fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
                match self.core.result_variable() {
                    Some(variable) => values.get(&variable.index()).cloned(),
                    None => None,
                }
            }
            fn to_raw_string(&self, _unfold: u64) -> String {
                format!("cvar({})", self.core.id.name)
            }
        }

        impl<V> FunctionSymbol<V> for $wrapper<V>
        where
            V: Clone
                + Debug
                + Send
                + Sync
                + 'static
                + Add<Output = V>
                + Div<Output = V>
                + Mul<Output = V>
                + Sub<Output = V>
                + PartialOrd
                + FromPrimitive
                + ToPrimitive
                + One
                + Zero,
            f64: IntoValue<V>,
        {
            fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
                self.core.register_tokens(tokens)
            }
            fn calculate_value(
                &self,
                token_table: &dyn TokenList<V>,
                zero_if_none: bool,
            ) -> Option<V> {
                self.core.evaluate(token_table, zero_if_none)
            }
        }

        impl<V> LinearIntermediateSymbol<V> for $wrapper<V>
        where
            V: Clone
                + Debug
                + Send
                + Sync
                + 'static
                + Add<Output = V>
                + Div<Output = V>
                + Mul<Output = V>
                + Sub<Output = V>
                + PartialOrd
                + FromPrimitive
                + ToPrimitive
                + One
                + Zero,
            f64: IntoValue<V>,
        {
            fn to_linear_polynomial(&self) -> Linear<V> {
                self.core.result_polynomial()
            }
            fn to_quadratic_polynomial(&self) -> Quadratic<V> {
                Quadratic::from_linear(&self.to_linear_polynomial())
            }
        }
    };
}

impl_cvar_symbol!(CvarFunction);
impl_cvar_symbol!(CvarEpigraphFunction);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::BasicModel;
    use crate::symbol::flatten::LinearMonomial;
    use crate::token::{MutableTokenList, Token, VecTokenList};
    use crate::variable::{ContinuousVariableItem, VariableId, VariableRange, new_group_id};
    use std::sync::Arc;

    fn bounded_input(name: &str, lower: f64, upper: f64) -> ContinuousVariableItem {
        ContinuousVariableItem::with_range(
            VariableId::new(new_group_id(), 0),
            name,
            VariableRange::bounded(lower, upper),
        )
    }

    fn model_with_losses() -> (BasicModel<f64>, Vec<Linear<f64>>, usize, usize) {
        let mut model = BasicModel::new("cvar_test");
        let x_index = model
            .register_variable(bounded_input("x", 0.0, 10.0))
            .expect("register x");
        let y_index = model
            .register_variable(bounded_input("y", 0.0, 5.0))
            .expect("register y");
        let losses = vec![
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 1.0),
            Linear::new(vec![LinearMonomial::new(-1.0, y_index)], 10.0),
        ];
        (model, losses, x_index, y_index)
    }

    fn symbol_indexes(model: &BasicModel<f64>) -> HashMap<usize, usize> {
        model
            .tokens()
            .iter()
            .map(|token| (token.id().unique_id() as usize, token.solver_index))
            .collect::<HashMap<_, _>>()
    }

    fn value_of(expression: &Linear<f64>, values: &HashMap<usize, f64>) -> f64 {
        expression.constant_term()
            + expression
                .monomials()
                .iter()
                .map(|monomial| monomial.coefficient() * values[&monomial.var_index()])
                .sum::<f64>()
    }

    fn rows_are_feasible(rows: &[LinearConstraint<f64>], values: &HashMap<usize, f64>) -> bool {
        rows.iter().all(|constraint| {
            let lhs = value_of(&constraint.inequality.polynomial, values);
            let rhs = constraint.inequality.rhs;
            match constraint.inequality.relation {
                ConstraintRelation::LessEqual => lhs <= rhs + 1e-8,
                ConstraintRelation::Equal => (lhs - rhs).abs() <= 1e-8,
                ConstraintRelation::GreaterEqual => lhs + 1e-8 >= rhs,
            }
        })
    }

    fn solver_index(map: &HashMap<usize, usize>, id: VariableId) -> usize {
        map[&(id.unique_id() as usize)]
    }

    fn exact_assignment(
        symbol: &CvarFunction<f64>,
        indexes: &HashMap<usize, usize>,
        input_values: &HashMap<usize, f64>,
    ) -> HashMap<usize, f64> {
        let mut values = input_values.clone();
        let losses = symbol
            .core
            .losses
            .iter()
            .map(|loss| value_of(loss, &values))
            .collect::<Vec<_>>();
        let CvarKind::Exact {
            positive_parts,
            minimum,
        } = &symbol.core.kind
        else {
            unreachable!();
        };
        let mut candidates = Vec::with_capacity(losses.len());
        for (threshold_index, parts) in positive_parts.iter().enumerate() {
            let mut candidate = losses[threshold_index];
            for (loss_index, part) in parts.iter().enumerate() {
                let difference = losses[loss_index] - losses[threshold_index];
                let result = difference.max(0.0);
                values.insert(solver_index(indexes, part.result_variable().id()), result);
                for (selector_index, selector) in part
                    .selector_variables()
                    .expect("exact positive part has selectors")
                    .iter()
                    .enumerate()
                {
                    let active_selector = if difference >= 0.0 { 0 } else { 1 };
                    values.insert(
                        solver_index(indexes, selector.id()),
                        if selector_index == active_selector {
                            1.0
                        } else {
                            0.0
                        },
                    );
                }
                candidate += symbol.core.weights[loss_index] * result;
            }
            candidates.push(candidate);
        }
        let minimum_index = candidates
            .iter()
            .enumerate()
            .min_by(|(_, left), (_, right)| left.total_cmp(right))
            .map(|(index, _)| index)
            .expect("at least one CVaR candidate");
        let minimum_value = candidates[minimum_index];
        values.insert(
            solver_index(indexes, minimum.result_variable().id()),
            minimum_value,
        );
        for (index, selector) in minimum
            .selector_variables()
            .expect("exact minimum has selectors")
            .iter()
            .enumerate()
        {
            values.insert(
                solver_index(indexes, selector.id()),
                if index == minimum_index { 1.0 } else { 0.0 },
            );
        }
        values
    }

    #[test]
    fn evaluates_non_uniform_weighted_cvar_and_alpha_zero() {
        let (mut model, losses, x_index, y_index) = model_with_losses();
        let mut tokens = model.tokens().to_vec();
        tokens[x_index].set_result(10.0);
        tokens[y_index].set_result(0.0);
        let mut table = VecTokenList::new();
        table.try_add_tokens(tokens).expect("valid token indexes");
        let cvar = CvarFunction::new(901, "risk", losses.clone(), vec![0.25, 0.75], 0.5)
            .expect("valid CVaR inputs");
        assert_eq!(cvar.calculate_value(&table, false), Some(10.5));

        let mean = CvarFunction::new(902, "mean_risk", losses, vec![0.25, 0.75], 0.0)
            .expect("alpha zero is valid");
        assert_eq!(mean.calculate_value(&table, false), Some(10.25));
    }

    #[test]
    fn rejects_invalid_probability_and_confidence_inputs() {
        let (_, losses, _, _) = model_with_losses();
        assert!(CvarFunction::new(903, "empty", Vec::new(), Vec::new(), 0.5).is_err());
        assert!(CvarFunction::new(904, "size", losses.clone(), vec![1.0], 0.5).is_err());
        assert!(CvarFunction::new(905, "sum", losses.clone(), vec![0.2, 0.7], 0.5).is_err());
        assert!(CvarFunction::new(906, "negative", losses.clone(), vec![-0.1, 1.1], 0.5).is_err());
        assert!(CvarFunction::new(907, "alpha", losses, vec![0.25, 0.75], 1.0).is_err());
    }

    #[test]
    fn exact_rows_are_feasible_only_at_the_cvar_value_and_use_registered_indexes() {
        let (mut model, losses, x_index, y_index) = model_with_losses();
        let exact = CvarFunction::new(908, "exact", losses.clone(), vec![0.25, 0.75], 0.5)
            .expect("valid exact CVaR");
        model
            .add_symbol(Arc::new(exact.clone()))
            .expect("register exact CVaR in the model");
        let indexes = symbol_indexes(&model);
        let tokens = model.tokens();
        let constraints = exact
            .mechanism_constraints_with_tokens(&indexes, tokens)
            .expect("exact constraints should use final token indexes");
        let known_indexes = indexes.values().copied().collect::<HashSet<_>>();
        assert!(!constraints.is_empty());
        assert!(constraints.iter().all(|constraint| {
            constraint
                .inequality
                .polynomial
                .monomials()
                .iter()
                .all(|monomial| known_indexes.contains(&monomial.var_index()))
        }));

        let mut input_values = HashMap::new();
        input_values.insert(x_index, 10.0);
        input_values.insert(y_index, 0.0);
        let values = exact_assignment(&exact, &indexes, &input_values);
        assert!(rows_are_feasible(&constraints, &values));
        let expression = exact
            .result_expression(&indexes)
            .expect("mapped exact result expression");
        assert_eq!(value_of(&expression, &values), 10.5);
        let mut perturbed = values;
        let result_index = solver_index(&indexes, exact.result_variable().id());
        *perturbed.get_mut(&result_index).expect("mapped result") += 1.0;
        assert!(!rows_are_feasible(&constraints, &perturbed));
        assert!(exact.result_expression(&HashMap::new()).is_err());
    }

    #[test]
    fn epigraph_constraints_allow_excess_slack_and_map_the_result_expression() {
        let (mut model, losses, x_index, y_index) = model_with_losses();
        let epigraph = CvarEpigraphFunction::new(909, "epigraph", losses, vec![0.25, 0.75], 0.5)
            .expect("valid CVaR epigraph");
        model
            .add_symbol(Arc::new(epigraph.clone()))
            .expect("register epigraph in the model");
        let indexes = symbol_indexes(&model);
        let tokens = model.tokens();
        let constraints = epigraph
            .mechanism_constraints_with_tokens(&indexes, tokens)
            .expect("epigraph constraints should use final token indexes");
        assert_eq!(constraints.len(), 2);
        assert!(constraints.iter().all(|constraint| {
            constraint
                .inequality
                .polynomial
                .monomials()
                .iter()
                .all(|monomial| indexes.values().any(|index| *index == monomial.var_index()))
        }));

        let mut values = HashMap::new();
        values.insert(x_index, 10.0);
        values.insert(y_index, 0.0);
        values.insert(
            solver_index(&indexes, epigraph.threshold_variable().id()),
            10.0,
        );
        values.insert(
            solver_index(&indexes, epigraph.excess_variables()[0].id()),
            1.0,
        );
        values.insert(
            solver_index(&indexes, epigraph.excess_variables()[1].id()),
            0.0,
        );
        assert!(rows_are_feasible(&constraints, &values));
        let expression = epigraph
            .result_expression(&indexes)
            .expect("mapped epigraph result expression");
        let tight_value = value_of(&expression, &values);
        assert_eq!(tight_value, 10.5);

        let first_excess = solver_index(&indexes, epigraph.excess_variables()[0].id());
        values.insert(first_excess, 0.0);
        assert!(!rows_are_feasible(&constraints, &values));
        values.insert(first_excess, 2.0);
        assert!(rows_are_feasible(&constraints, &values));
        assert!(value_of(&expression, &values) > tight_value);
    }

    #[test]
    fn exact_cvar_requires_finite_loss_bounds() {
        let unbounded =
            ContinuousVariableItem::create(VariableId::new(new_group_id(), 0), "unbounded");
        let losses = vec![
            Linear::new(vec![LinearMonomial::new(1.0, 17)], 0.0),
            Linear::constant(3.0),
        ];
        let tokens = vec![Token::from_generic(unbounded, 17)];
        let exact = CvarFunction::new(910, "unbounded_cvar", losses, vec![0.5, 0.5], 0.5)
            .expect("input syntax is valid");
        let mut auxiliary = Vec::new();
        assert!(
            exact
                .register_auxiliary_tokens_with_context(&mut auxiliary, &tokens)
                .is_err()
        );
    }
}
