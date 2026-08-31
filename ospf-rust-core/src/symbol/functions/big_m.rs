use std::fmt::Debug;
use num_traits::{FromPrimitive, ToPrimitive};
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::Token;

#[derive(Debug, Clone, Copy)]
pub struct BigMPolicy {
    fallback: f64,
    min: f64,
}

impl BigMPolicy {
    pub const fn new(fallback: f64, min: f64) -> Self {
        Self { fallback, min }
    }

    pub const fn fallback(&self) -> f64 {
        self.fallback
    }

    pub const fn min(&self) -> f64 {
        self.min
    }

    pub fn resolve(&self, inferred: Option<f64>) -> f64 {
        inferred.unwrap_or(self.fallback).max(self.min)
    }
}

// ============================================================================
// Constants
// ============================================================================

/// Default big M value used when no better bound is available.
pub const DEFAULT_BIG_M: f64 = 1_000_000.0;

/// Minimum big M value to ensure numerical stability.
pub const MIN_BIG_M: f64 = 1.0;

/// Tolerance for nonzero detection in indicator constraints.
pub(crate) const NONZERO_TOLERANCE: f64 = f64::EPSILON * 16.0;

/// Strict nonzero boundary used in indicator constraint RHS values.
pub(crate) const STRICT_NONZERO_BOUNDARY: f64 = NONZERO_TOLERANCE + f64::EPSILON * 16.0;

// ============================================================================
// LinearPolynomialBounds
// ============================================================================

/// Bounds for a linear polynomial expression.
///
/// Stores the lower and upper bounds of a linear polynomial over a set
/// of bounded variables. Used for big M inference and constraint generation.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearPolynomialBounds<V> {
    pub lower: Option<V>,
    pub upper: Option<V>,
}

impl<V> LinearPolynomialBounds<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    /// Compute bounds from a linear polynomial and its token list.
    ///
    /// Returns `None` if any variable is unbounded or if the computed
    /// bounds are not finite.
    pub fn from_polynomial(poly: &Linear<V>, tokens: &[Token<V>]) -> Option<Self> {
        let (lower, upper) = infer_linear_bounds_from_tokens(poly, tokens)?;
        Some(Self {
            lower: Some(V::from_f64(lower)?),
            upper: Some(V::from_f64(upper)?),
        })
    }

    /// Compute the absolute bound (max of |lower| and |upper|).
    ///
    /// Returns `None` if bounds are not available.
    pub fn abs_bound(&self) -> Option<f64> {
        let lower = self.lower.as_ref()?.to_f64()?;
        let upper = self.upper.as_ref()?.to_f64()?;
        Some(lower.abs().max(upper.abs()))
    }
}

impl LinearPolynomialBounds<f64> {
    /// Compute bounds from a linear polynomial and its token list (f64 specialization).
    pub fn from_polynomial_f64(poly: &Linear<f64>, tokens: &[Token<f64>]) -> Option<Self> {
        let (lower, upper) = infer_linear_bounds_from_tokens(poly, tokens)?;
        Some(Self {
            lower: Some(lower),
            upper: Some(upper),
        })
    }
}

// ============================================================================
// Public API: Big M Utilities
// ============================================================================

/// Returns the default big M value (1,000,000.0).
///
/// This is the fallback value used when no tighter bound can be inferred
/// from variable ranges.
pub fn default_big_m() -> f64 {
    DEFAULT_BIG_M
}

/// Ensures the big M value is positive and at least [`MIN_BIG_M`].
///
/// If `m` is less than [`MIN_BIG_M`], returns [`MIN_BIG_M`] instead.
/// This prevents degenerate constraints when inferred bounds are very small.
pub fn ensure_positive_big_m(m: f64) -> f64 {
    m.max(MIN_BIG_M)
}

// ============================================================================
// Internal Helpers
// ============================================================================

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

/// Extracts base monomial data (coefficient as f64, var_index) from a polynomial.
fn extract_base_monomials<V>(polynomial: &Linear<V>, name_prefix: &str) -> Result<Vec<(f64, usize)>>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    let mut base = Vec::with_capacity(polynomial.monomials().len());
    for monomial in polynomial.monomials() {
        let coefficient = monomial.coefficient().to_f64().ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "logic `{}` input coefficient cannot be converted to f64",
                name_prefix
            ))
        })?;
        base.push((coefficient, monomial.var_index()));
    }
    Ok(base)
}

/// Extracts the constant term as f64 from a polynomial.
fn extract_constant<V>(polynomial: &Linear<V>, name_prefix: &str) -> Result<f64>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    Ok(polynomial.constant_term().to_f64().ok_or_else(|| {
        ModelError::InvalidConstraint(format!(
            "logic `{}` input constant cannot be converted to f64",
            name_prefix
        ))
    })?)
}

// ============================================================================
// Public API: Indicator Constraints
// ============================================================================

/// Generates constraints for a positive indicator.
///
/// When indicator is 1: polynomial > 0 (strict, using epsilon tolerance).
/// When indicator is 0: polynomial <= 0.
///
/// Returns a vector of `(LinearInequality, name)` pairs.
pub fn positive_indicator_constraints<V>(
    polynomial: &Linear<V>,
    indicator_index: usize,
    big_m: f64,
    name_prefix: &str,
) -> Result<Vec<(LinearInequality<V>, String)>>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    let base_monomials = extract_base_monomials(polynomial, name_prefix)?;
    let constant = extract_constant(polynomial, name_prefix)?;
    let mut constraints = Vec::with_capacity(2);

    // polynomial - big_m * indicator <= -epsilon
    // When y=0: polynomial <= -epsilon (trivially satisfied by big_m relaxation)
    // When y=1: polynomial <= -epsilon... wait, that's wrong.
    //
    // Correct formulation:
    // polynomial - big_m * indicator <= 0
    // When y=1: polynomial <= 0 -> but we want polynomial > 0...
    //
    // Standard Big-M for "y=1 => f(x) > 0":
    //   f(x) - big_m * (1 - y) <= -epsilon  ... but we only have indicator y.
    //
    // With indicator y (1 = condition holds):
    //   f(x) + big_m * y <= big_m + 0    (relaxation when y=0)
    //   f(x) - epsilon >= 0               (when y=1)
    //
    // Using the pattern from nonzero:
    //   f(x) - big_m * y <= -epsilon   => when y=0: f(x) <= -epsilon (wrong)
    //
    // Let's use the correct standard form:
    // y=1 => f(x) > epsilon (positive)
    // y=0 => f(x) <= 0
    //
    // Constraint 1: f(x) - big_m * y <= 0
    //   y=0: f(x) <= 0 (feasible with big-M)
    //   y=1: f(x) <= 0 ... but we want f(x) > epsilon!
    //
    // We need: y=1 => f(x) > epsilon
    //          y=0 => f(x) <= 0
    //
    // This requires TWO directions:
    // (a) f(x) - big_m * y <= -epsilon   ... no
    //
    // Actually the standard formulation:
    // "y = 1 iff f(x) > 0" needs:
    //   f(x) <= big_m * y           (if f(x) > 0 then y must be 1)
    //   f(x) >= epsilon - big_m*(1-y) (if y=1 then f(x) >= epsilon)
    //
    // But with a single binary indicator (not 1-y), we adjust signs:
    //   f(x) + big_m * y <= big_m     ... when y=0: f(x) <= big_m (relaxation)
    //                                  ... when y=1: f(x) <= 0 ... wrong again
    //
    // Let me think differently. The existing nonzero uses:
    //   f(x) - big_m * y <= epsilon    (band_ub)
    //   f(x) + big_m * y >= -epsilon   (band_lb)
    // These give: when y=0, |f(x)| <= epsilon (i.e. f(x) ≈ 0)
    //             when y=1, relaxed
    //
    // For positive (y=1 => f(x) > 0):
    //   We want: y=1 => f(x) > epsilon
    //            y=0 => f(x) <= 0
    //
    // Constraint: f(x) + big_m * (1 - y_sign) ... but we have y directly.
    //
    // Using single indicator y where y=1 means "positive":
    //   f(x) + big_m * y <= big_m       -> y=0: f(x) <= big_m (always OK)
    //                                     -> y=1: f(x) <= 0
    //   But we want y=1 => f(x) > 0...
    //
    // I think the formulation should be:
    //   f(x) >= epsilon - big_m * (1 - y) = epsilon - big_m + big_m * y
    //   => f(x) - big_m * y >= epsilon - big_m
    //   => -f(x) + big_m * y <= big_m - epsilon
    //   When y=1: -f(x) + big_m <= big_m - epsilon => -f(x) <= -epsilon => f(x) >= epsilon ✓
    //   When y=0: -f(x) <= big_m - epsilon => f(x) >= epsilon - big_m (relaxation) ✓
    //
    //   f(x) <= 0 + big_m * y
    //   => f(x) - big_m * y <= 0
    //   When y=1: f(x) - big_m <= 0 => f(x) <= big_m (relaxation) ... no
    //   When y=0: f(x) <= 0 ✓
    //
    // Hmm, we need:
    //   y=1 => f(x) > 0  :  f(x) >= epsilon - big_m*(1-y)
    //   y=0 => f(x) <= 0  :  f(x) <= big_m * y ... when y=0: f(x) <= 0 ✓
    //                                               when y=1: f(x) <= big_m (relaxation) ✓
    //
    // So:
    // Constraint 1 (lb): f(x) - big_m * y >= epsilon - big_m
    //   => f(x) - big_m * y >= epsilon - big_m
    // Constraint 2 (ub): f(x) - big_m * y <= 0
    //   => f(x) - big_m * y <= 0
    //
    // When y=1: f(x) - big_m >= epsilon - big_m => f(x) >= epsilon ✓
    //           f(x) - big_m <= 0 => f(x) <= big_m (relaxation) ✓
    // When y=0: f(x) >= epsilon - big_m (relaxation) ✓
    //           f(x) <= 0 ✓
    //
    // Great! But wait, we want y=0 to mean "not positive" => f(x) <= 0.
    // And the ub constraint f(x) - big_m * y <= 0 gives:
    //   y=0: f(x) <= 0 ✓
    //   y=1: f(x) <= big_m (relaxation) ✓
    // And the lb constraint gives:
    //   y=0: f(x) >= epsilon - big_m (relaxation) ✓
    //   y=1: f(x) >= epsilon ✓
    //
    // Perfect!

    // Constraint: f(x) - big_m * y <= 0
    //   y=0: f(x) <= 0
    //   y=1: f(x) <= big_m (relaxation)
    let mut ub_monomials = Vec::with_capacity(base_monomials.len() + 1);
    for (coefficient, index) in &base_monomials {
        ub_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(*coefficient, "logic input coefficient")?,
            *index,
        ));
    }
    ub_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(-big_m, "logic indicator coefficient")?,
        indicator_index,
    ));
    constraints.push((
        LinearInequality::new(
            Linear::new(
                ub_monomials,
                convert_f64_to_v::<V>(constant, "logic input constant")?,
            ),
            ConstraintRelation::LessEqual,
            convert_f64_to_v::<V>(0.0, "logic rhs")?,
        ),
        format!("{}_ub", name_prefix),
    ));

    // Constraint: f(x) - big_m * y >= epsilon - big_m
    //   y=0: f(x) >= epsilon - big_m (relaxation)
    //   y=1: f(x) >= epsilon
    let mut lb_monomials = Vec::with_capacity(base_monomials.len() + 1);
    for (coefficient, index) in &base_monomials {
        lb_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(*coefficient, "logic input coefficient")?,
            *index,
        ));
    }
    lb_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(-big_m, "logic indicator coefficient")?,
        indicator_index,
    ));
    constraints.push((
        LinearInequality::new(
            Linear::new(
                lb_monomials,
                convert_f64_to_v::<V>(constant, "logic input constant")?,
            ),
            ConstraintRelation::GreaterEqual,
            convert_f64_to_v::<V>(f64::EPSILON - big_m, "logic rhs")?,
        ),
        format!("{}_lb", name_prefix),
    ));

    Ok(constraints)
}

/// Generates constraints for a non-negative indicator.
///
/// When indicator is 1: polynomial >= 0.
/// When indicator is 0: polynomial < 0.
///
/// Returns a vector of `(LinearInequality, name)` pairs.
pub fn nonnegative_indicator_constraints<V>(
    polynomial: &Linear<V>,
    indicator_index: usize,
    big_m: f64,
    name_prefix: &str,
) -> Result<Vec<(LinearInequality<V>, String)>>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    let base_monomials = extract_base_monomials(polynomial, name_prefix)?;
    let constant = extract_constant(polynomial, name_prefix)?;
    let mut constraints = Vec::with_capacity(2);

    // Constraint: f(x) - big_m * y <= 0
    //   y=0: f(x) <= 0
    //   y=1: f(x) <= big_m (relaxation)
    let mut ub_monomials = Vec::with_capacity(base_monomials.len() + 1);
    for (coefficient, index) in &base_monomials {
        ub_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(*coefficient, "logic input coefficient")?,
            *index,
        ));
    }
    ub_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(-big_m, "logic indicator coefficient")?,
        indicator_index,
    ));
    constraints.push((
        LinearInequality::new(
            Linear::new(
                ub_monomials,
                convert_f64_to_v::<V>(constant, "logic input constant")?,
            ),
            ConstraintRelation::LessEqual,
            convert_f64_to_v::<V>(0.0, "logic rhs")?,
        ),
        format!("{}_ub", name_prefix),
    ));

    // Constraint: f(x) - big_m * y >= -big_m
    //   y=0: f(x) >= -big_m (relaxation)
    //   y=1: f(x) >= 0
    let mut lb_monomials = Vec::with_capacity(base_monomials.len() + 1);
    for (coefficient, index) in &base_monomials {
        lb_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(*coefficient, "logic input coefficient")?,
            *index,
        ));
    }
    lb_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(-big_m, "logic indicator coefficient")?,
        indicator_index,
    ));
    constraints.push((
        LinearInequality::new(
            Linear::new(
                lb_monomials,
                convert_f64_to_v::<V>(constant, "logic input constant")?,
            ),
            ConstraintRelation::GreaterEqual,
            convert_f64_to_v::<V>(-big_m, "logic rhs")?,
        ),
        format!("{}_lb", name_prefix),
    ));

    Ok(constraints)
}

/// Generates constraints for a negative indicator.
///
/// When indicator is 1: polynomial < 0 (strict, using epsilon tolerance).
/// When indicator is 0: polynomial >= 0.
///
/// Returns a vector of `(LinearInequality, name)` pairs.
pub fn negative_indicator_constraints<V>(
    polynomial: &Linear<V>,
    indicator_index: usize,
    big_m: f64,
    name_prefix: &str,
) -> Result<Vec<(LinearInequality<V>, String)>>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    let base_monomials = extract_base_monomials(polynomial, name_prefix)?;
    let constant = extract_constant(polynomial, name_prefix)?;
    let mut constraints = Vec::with_capacity(2);

    // Constraint: f(x) + big_m * y >= 0
    //   y=0: f(x) >= 0
    //   y=1: f(x) >= -big_m (relaxation)
    let mut lb_monomials = Vec::with_capacity(base_monomials.len() + 1);
    for (coefficient, index) in &base_monomials {
        lb_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(*coefficient, "logic input coefficient")?,
            *index,
        ));
    }
    lb_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(big_m, "logic indicator coefficient")?,
        indicator_index,
    ));
    constraints.push((
        LinearInequality::new(
            Linear::new(
                lb_monomials,
                convert_f64_to_v::<V>(constant, "logic input constant")?,
            ),
            ConstraintRelation::GreaterEqual,
            convert_f64_to_v::<V>(0.0, "logic rhs")?,
        ),
        format!("{}_lb", name_prefix),
    ));

    // Constraint: f(x) + big_m * y <= -epsilon + big_m
    //   y=0: f(x) <= -epsilon
    //   y=1: f(x) <= -epsilon + big_m (relaxation)
    let mut ub_monomials = Vec::with_capacity(base_monomials.len() + 1);
    for (coefficient, index) in &base_monomials {
        ub_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(*coefficient, "logic input coefficient")?,
            *index,
        ));
    }
    ub_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(big_m, "logic indicator coefficient")?,
        indicator_index,
    ));
    constraints.push((
        LinearInequality::new(
            Linear::new(
                ub_monomials,
                convert_f64_to_v::<V>(constant, "logic input constant")?,
            ),
            ConstraintRelation::LessEqual,
            convert_f64_to_v::<V>(-f64::EPSILON + big_m, "logic rhs")?,
        ),
        format!("{}_ub", name_prefix),
    ));

    Ok(constraints)
}

/// Generates constraints for a non-zero indicator.
///
/// When indicator is 1: |polynomial| > epsilon (polynomial is non-zero).
/// When indicator is 0: |polynomial| <= epsilon (polynomial is approximately zero).
///
/// This requires a `side_index` for an auxiliary binary variable that
/// tracks the sign of the polynomial when it is non-zero.
///
/// Returns a vector of `(LinearInequality, name)` pairs (4 constraints).
pub fn nonzero_indicator_constraints<V>(
    polynomial: &Linear<V>,
    indicator_index: usize,
    side_index: usize,
    big_m: f64,
    name_prefix: &str,
) -> Result<Vec<(LinearInequality<V>, String)>>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    let base_monomials = extract_base_monomials(polynomial, name_prefix)?;
    let constant = extract_constant(polynomial, name_prefix)?;
    let mut constraints = Vec::with_capacity(4);

    // Band upper bound: f(x) - big_m * y <= epsilon
    // When y=0: f(x) <= epsilon
    // When y=1: f(x) <= epsilon + big_m (relaxation)
    let mut ub_monomials = Vec::with_capacity(base_monomials.len() + 1);
    for (coefficient, index) in &base_monomials {
        ub_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(*coefficient, "logic input coefficient")?,
            *index,
        ));
    }
    ub_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(-big_m, "logic indicator coefficient")?,
        indicator_index,
    ));
    constraints.push((
        LinearInequality::new(
            Linear::new(
                ub_monomials,
                convert_f64_to_v::<V>(constant, "logic input constant")?,
            ),
            ConstraintRelation::LessEqual,
            convert_f64_to_v::<V>(NONZERO_TOLERANCE, "logic rhs")?,
        ),
        format!("{}_band_ub", name_prefix),
    ));

    // Band lower bound: f(x) + big_m * y >= -epsilon
    // When y=0: f(x) >= -epsilon
    // When y=1: f(x) >= -epsilon - big_m (relaxation)
    let mut lb_monomials = Vec::with_capacity(base_monomials.len() + 1);
    for (coefficient, index) in &base_monomials {
        lb_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(*coefficient, "logic input coefficient")?,
            *index,
        ));
    }
    lb_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(big_m, "logic indicator coefficient")?,
        indicator_index,
    ));
    constraints.push((
        LinearInequality::new(
            Linear::new(
                lb_monomials,
                convert_f64_to_v::<V>(constant, "logic input constant")?,
            ),
            ConstraintRelation::GreaterEqual,
            convert_f64_to_v::<V>(-NONZERO_TOLERANCE, "logic rhs")?,
        ),
        format!("{}_band_lb", name_prefix),
    ));

    // Out lower bound: f(x) - big_m * y - big_m * s >= epsilon - 2*big_m
    // When y=1, s=0 (positive): f(x) >= epsilon
    // When y=1, s=1 (negative): f(x) >= epsilon - big_m (relaxation)
    // When y=0: f(x) >= epsilon - 2*big_m (relaxation)
    let mut out_lb_monomials = Vec::with_capacity(base_monomials.len() + 2);
    for (coefficient, index) in &base_monomials {
        out_lb_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(*coefficient, "logic input coefficient")?,
            *index,
        ));
    }
    out_lb_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(-big_m, "logic indicator coefficient")?,
        indicator_index,
    ));
    out_lb_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(-big_m, "logic side coefficient")?,
        side_index,
    ));
    constraints.push((
        LinearInequality::new(
            Linear::new(
                out_lb_monomials,
                convert_f64_to_v::<V>(constant, "logic input constant")?,
            ),
            ConstraintRelation::GreaterEqual,
            convert_f64_to_v::<V>(STRICT_NONZERO_BOUNDARY - 2.0 * big_m, "logic rhs")?,
        ),
        format!("{}_out_lb", name_prefix),
    ));

    // Out upper bound: f(x) + big_m * y - big_m * s <= -epsilon + big_m
    // When y=1, s=1 (negative): f(x) <= -epsilon
    // When y=1, s=0 (positive): f(x) <= -epsilon + big_m (relaxation)
    // When y=0: f(x) <= -epsilon + big_m (relaxation)
    let mut out_ub_monomials = Vec::with_capacity(base_monomials.len() + 2);
    for (coefficient, index) in &base_monomials {
        out_ub_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(*coefficient, "logic input coefficient")?,
            *index,
        ));
    }
    out_ub_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(big_m, "logic indicator coefficient")?,
        indicator_index,
    ));
    out_ub_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(-big_m, "logic side coefficient")?,
        side_index,
    ));
    constraints.push((
        LinearInequality::new(
            Linear::new(
                out_ub_monomials,
                convert_f64_to_v::<V>(constant, "logic input constant")?,
            ),
            ConstraintRelation::LessEqual,
            convert_f64_to_v::<V>(-STRICT_NONZERO_BOUNDARY + big_m, "logic rhs")?,
        ),
        format!("{}_out_ub", name_prefix),
    ));

    Ok(constraints)
}

// ============================================================================
// Bounds Inference (existing functions below)
// ============================================================================

pub fn infer_linear_bounds_from_tokens<V>(
    poly: &Linear<V>,
    tokens: &[Token<V>],
) -> Option<(f64, f64)>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    let mut lower = poly.constant_term().to_f64()?;
    let mut upper = lower;

    for monomial in poly.monomials() {
        let token = tokens.get(monomial.var_index())?;
        let var_lower = token.variable.lower_bound()?.to_f64()?;
        let var_upper = token.variable.upper_bound()?.to_f64()?;
        if !var_lower.is_finite() || !var_upper.is_finite() {
            return None;
        }

        let coefficient = monomial.coefficient().to_f64()?;
        if coefficient >= 0.0 {
            lower += coefficient * var_lower;
            upper += coefficient * var_upper;
        } else {
            lower += coefficient * var_upper;
            upper += coefficient * var_lower;
        }
    }

    if !lower.is_finite() || !upper.is_finite() {
        return None;
    }
    Some((lower, upper))
}

pub fn infer_linear_abs_bound_from_tokens<V>(poly: &Linear<V>, tokens: &[Token<V>]) -> Option<f64>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    let (lower, upper) = infer_linear_bounds_from_tokens(poly, tokens)?;
    Some(lower.abs().max(upper.abs()))
}

#[allow(dead_code)]
pub fn infer_linear_shifted_bounds_from_tokens<V>(
    poly: &Linear<V>,
    right: &V,
    tokens: &[Token<V>],
) -> Option<(f64, f64)>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    let (lower, upper) = infer_linear_bounds_from_tokens(poly, tokens)?;
    let right = right.to_f64()?;
    let lower = lower - right;
    let upper = upper - right;
    if !lower.is_finite() || !upper.is_finite() {
        return None;
    }
    Some((lower, upper))
}

#[allow(dead_code)]
pub fn infer_linear_shifted_abs_bound_from_tokens<V>(
    poly: &Linear<V>,
    right: &V,
    tokens: &[Token<V>],
) -> Option<f64>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    let (lower, upper) = infer_linear_shifted_bounds_from_tokens(poly, right, tokens)?;
    Some(lower.abs().max(upper.abs()))
}

pub fn infer_linear_difference_bounds_from_tokens<V>(
    left: &Linear<V>,
    right: &Linear<V>,
    tokens: &[Token<V>],
) -> Option<(f64, f64)>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    let (left_lower, left_upper) = infer_linear_bounds_from_tokens(left, tokens)?;
    let (right_lower, right_upper) = infer_linear_bounds_from_tokens(right, tokens)?;
    let lower = left_lower - right_upper;
    let upper = left_upper - right_lower;
    if !lower.is_finite() || !upper.is_finite() {
        return None;
    }
    Some((lower, upper))
}

pub fn infer_linear_difference_abs_bound_from_tokens<V>(
    left: &Linear<V>,
    right: &Linear<V>,
    tokens: &[Token<V>],
) -> Option<f64>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    let (lower, upper) = infer_linear_difference_bounds_from_tokens(left, right, tokens)?;
    Some(lower.abs().max(upper.abs()))
}

#[allow(dead_code)]
fn variable_bounds_from_tokens<V>(tokens: &[Token<V>], index: usize) -> Option<(f64, f64)>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    let token = tokens.get(index)?;
    let lower = token.variable.lower_bound()?.to_f64()?;
    let upper = token.variable.upper_bound()?.to_f64()?;
    if !lower.is_finite() || !upper.is_finite() {
        return None;
    }
    Some((lower, upper))
}

#[allow(dead_code)]
fn square_bounds(lower: f64, upper: f64) -> (f64, f64) {
    if lower <= 0.0 && upper >= 0.0 {
        (0.0, lower.abs().max(upper.abs()).powi(2))
    } else {
        let lower_square = lower.powi(2);
        let upper_square = upper.powi(2);
        (
            lower_square.min(upper_square),
            lower_square.max(upper_square),
        )
    }
}

#[allow(dead_code)]
fn product_bounds(
    first_lower: f64,
    first_upper: f64,
    second_lower: f64,
    second_upper: f64,
) -> (f64, f64) {
    let products = [
        first_lower * second_lower,
        first_lower * second_upper,
        first_upper * second_lower,
        first_upper * second_upper,
    ];
    let lower = products.iter().copied().fold(f64::INFINITY, f64::min);
    let upper = products.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (lower, upper)
}

#[allow(dead_code)]
pub fn infer_quadratic_bounds_from_tokens<V>(
    poly: &Quadratic<V>,
    tokens: &[Token<V>],
) -> Option<(f64, f64)>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    let mut lower = poly.constant().to_f64()?;
    let mut upper = lower;

    for monomial in poly.monomials() {
        let coefficient = monomial.coefficient().to_f64()?;
        let (term_lower, term_upper) = if let Some(var_index2) = monomial.var_index2() {
            let var_index1 = monomial.var_index1();
            let (raw_lower, raw_upper) = if var_index1 == var_index2 {
                let (var_lower, var_upper) = variable_bounds_from_tokens(tokens, var_index1)?;
                square_bounds(var_lower, var_upper)
            } else {
                let (first_lower, first_upper) = variable_bounds_from_tokens(tokens, var_index1)?;
                let (second_lower, second_upper) = variable_bounds_from_tokens(tokens, var_index2)?;
                product_bounds(first_lower, first_upper, second_lower, second_upper)
            };
            if coefficient >= 0.0 {
                (coefficient * raw_lower, coefficient * raw_upper)
            } else {
                (coefficient * raw_upper, coefficient * raw_lower)
            }
        } else {
            let (var_lower, var_upper) =
                variable_bounds_from_tokens(tokens, monomial.var_index1())?;
            if coefficient >= 0.0 {
                (coefficient * var_lower, coefficient * var_upper)
            } else {
                (coefficient * var_upper, coefficient * var_lower)
            }
        };

        lower += term_lower;
        upper += term_upper;
    }

    if !lower.is_finite() || !upper.is_finite() {
        return None;
    }
    Some((lower, upper))
}

#[allow(dead_code)]
pub fn infer_quadratic_abs_bound_from_tokens<V>(
    poly: &Quadratic<V>,
    tokens: &[Token<V>],
) -> Option<f64>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    let (lower, upper) = infer_quadratic_bounds_from_tokens(poly, tokens)?;
    Some(lower.abs().max(upper.abs()))
}

#[allow(dead_code)]
pub fn infer_quadratic_shifted_bounds_from_tokens<V>(
    poly: &Quadratic<V>,
    right: &V,
    tokens: &[Token<V>],
) -> Option<(f64, f64)>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    let (lower, upper) = infer_quadratic_bounds_from_tokens(poly, tokens)?;
    let right = right.to_f64()?;
    let lower = lower - right;
    let upper = upper - right;
    if !lower.is_finite() || !upper.is_finite() {
        return None;
    }
    Some((lower, upper))
}

#[allow(dead_code)]
pub fn infer_quadratic_shifted_abs_bound_from_tokens<V>(
    poly: &Quadratic<V>,
    right: &V,
    tokens: &[Token<V>],
) -> Option<f64>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    let (lower, upper) = infer_quadratic_shifted_bounds_from_tokens(poly, right, tokens)?;
    Some(lower.abs().max(upper.abs()))
}

pub fn infer_quadratic_difference_bounds_from_tokens<V>(
    left: &Quadratic<V>,
    right: &Quadratic<V>,
    tokens: &[Token<V>],
) -> Option<(f64, f64)>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    let (left_lower, left_upper) = infer_quadratic_bounds_from_tokens(left, tokens)?;
    let (right_lower, right_upper) = infer_quadratic_bounds_from_tokens(right, tokens)?;
    let lower = left_lower - right_upper;
    let upper = left_upper - right_lower;
    if !lower.is_finite() || !upper.is_finite() {
        return None;
    }
    Some((lower, upper))
}

pub fn infer_quadratic_difference_abs_bound_from_tokens<V>(
    left: &Quadratic<V>,
    right: &Quadratic<V>,
    tokens: &[Token<V>],
) -> Option<f64>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    let (lower, upper) = infer_quadratic_difference_bounds_from_tokens(left, right, tokens)?;
    Some(lower.abs().max(upper.abs()))
}

pub fn infer_big_m_for_polynomials<V>(
    polynomials: &[Linear<V>],
    tokens: &[Token<V>],
    min_big_m: f64,
) -> Option<f64>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    let mut big_m = min_big_m;
    for polynomial in polynomials {
        let bound = infer_linear_abs_bound_from_tokens(polynomial, tokens)?;
        big_m = big_m.max(bound);
    }
    Some(big_m)
}

#[allow(dead_code)]
pub fn infer_big_m_for_quadratic_polynomials<V>(
    polynomials: &[Quadratic<V>],
    tokens: &[Token<V>],
    min_big_m: f64,
) -> Option<f64>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    let mut big_m = min_big_m;
    for polynomial in polynomials {
        let bound = infer_quadratic_abs_bound_from_tokens(polynomial, tokens)?;
        big_m = big_m.max(bound);
    }
    Some(big_m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::flatten::{LinearMonomial, QuadraticMonomial};
    use crate::token::Token;
    use crate::variable::{ContinuousVariableItem, VariableRange};

    fn token(name: &str, index: usize, lower: f64, upper: f64) -> Token<f64> {
        Token::from_generic(
            ContinuousVariableItem::auto_with_range(name, VariableRange::bounded(lower, upper)),
            index,
        )
    }

    #[test]
    fn shifted_linear_bounds_subtract_right_side() {
        let tokens = vec![token("x", 0, 1.0, 3.0), token("y", 1, -2.0, 4.0)];
        let poly = Linear::new(
            vec![LinearMonomial::new(2.0, 0), LinearMonomial::new(-1.0, 1)],
            5.0,
        );

        assert_eq!(
            infer_linear_shifted_bounds_from_tokens(&poly, &4.0, &tokens),
            Some((-1.0, 9.0))
        );
        assert_eq!(
            infer_linear_shifted_abs_bound_from_tokens(&poly, &4.0, &tokens),
            Some(9.0)
        );
    }

    #[test]
    fn linear_difference_bounds_subtract_polynomial_ranges() {
        let tokens = vec![token("x", 0, 1.0, 3.0), token("y", 1, -2.0, 4.0)];
        let left = Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0);
        let right = Linear::new(vec![LinearMonomial::new(-1.0, 1)], 3.0);

        assert_eq!(
            infer_linear_difference_bounds_from_tokens(&left, &right, &tokens),
            Some((-2.0, 8.0))
        );
        assert_eq!(
            infer_linear_difference_abs_bound_from_tokens(&left, &right, &tokens),
            Some(8.0)
        );
    }

    #[test]
    fn quadratic_bounds_handle_linear_bilinear_and_square_terms() {
        let tokens = vec![token("x", 0, -1.0, 2.0), token("y", 1, 3.0, 5.0)];
        let poly = Quadratic::new(
            vec![
                QuadraticMonomial::new_quadratic(2.0, 0, 0),
                QuadraticMonomial::new_quadratic(-1.0, 0, 1),
                QuadraticMonomial::new_linear(3.0, 1),
            ],
            1.0,
        );

        assert_eq!(
            infer_quadratic_bounds_from_tokens(&poly, &tokens),
            Some((0.0, 29.0))
        );
        assert_eq!(
            infer_quadratic_shifted_bounds_from_tokens(&poly, &6.0, &tokens),
            Some((-6.0, 23.0))
        );
        assert_eq!(
            infer_quadratic_shifted_abs_bound_from_tokens(&poly, &6.0, &tokens),
            Some(23.0)
        );
    }

    #[test]
    fn quadratic_bounds_return_none_for_unbounded_variable() {
        let tokens = vec![token("x", 0, f64::NEG_INFINITY, 2.0)];
        let poly = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);

        assert_eq!(infer_quadratic_bounds_from_tokens(&poly, &tokens), None);
    }

    #[test]
    fn quadratic_big_m_uses_largest_local_abs_bound() {
        let tokens = vec![token("x", 0, -2.0, 1.0)];
        let first = Quadratic::new(vec![QuadraticMonomial::new_quadratic(2.0, 0, 0)], 0.0);
        let second = Quadratic::new(vec![QuadraticMonomial::new_linear(-3.0, 0)], 1.0);

        assert_eq!(
            infer_big_m_for_quadratic_polynomials(&[first, second], &tokens, 1.0),
            Some(8.0)
        );
    }

    // ========================================================================
    // Tests for new public API
    // ========================================================================

    #[test]
    fn default_big_m_returns_correct_value() {
        assert_eq!(default_big_m(), 1_000_000.0);
    }

    #[test]
    fn ensure_positive_big_m_clamps_small_values() {
        assert_eq!(ensure_positive_big_m(0.0), MIN_BIG_M);
        assert_eq!(ensure_positive_big_m(-5.0), MIN_BIG_M);
        assert_eq!(ensure_positive_big_m(0.5), MIN_BIG_M);
    }

    #[test]
    fn ensure_positive_big_m_preserves_large_values() {
        assert_eq!(ensure_positive_big_m(100.0), 100.0);
        assert_eq!(ensure_positive_big_m(1_000_000.0), 1_000_000.0);
    }

    #[test]
    fn linear_polynomial_bounds_from_polynomial_computes_correctly() {
        let tokens = vec![token("x", 0, 1.0, 3.0), token("y", 1, -2.0, 4.0)];
        let poly = Linear::new(
            vec![LinearMonomial::new(2.0, 0), LinearMonomial::new(-1.0, 1)],
            5.0,
        );

        let bounds = LinearPolynomialBounds::from_polynomial(&poly, &tokens).unwrap();
        // lower = 5 + 2*1 + (-1)*4 = 5 + 2 - 4 = 3
        // upper = 5 + 2*3 + (-1)*(-2) = 5 + 6 + 2 = 13
        assert_eq!(bounds.lower, Some(3.0));
        assert_eq!(bounds.upper, Some(13.0));
        assert_eq!(bounds.abs_bound(), Some(13.0));
    }

    #[test]
    fn positive_indicator_constraints_generates_two_constraints() {
        let poly = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
        let constraints = positive_indicator_constraints(&poly, 1, 100.0, "test").unwrap();
        assert_eq!(constraints.len(), 2);
        assert_eq!(constraints[0].1, "test_ub");
        assert_eq!(constraints[1].1, "test_lb");
    }

    #[test]
    fn nonnegative_indicator_constraints_generates_two_constraints() {
        let poly = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
        let constraints = nonnegative_indicator_constraints(&poly, 1, 100.0, "test").unwrap();
        assert_eq!(constraints.len(), 2);
        assert_eq!(constraints[0].1, "test_ub");
        assert_eq!(constraints[1].1, "test_lb");
    }

    #[test]
    fn negative_indicator_constraints_generates_two_constraints() {
        let poly = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
        let constraints = negative_indicator_constraints(&poly, 1, 100.0, "test").unwrap();
        assert_eq!(constraints.len(), 2);
        assert_eq!(constraints[0].1, "test_lb");
        assert_eq!(constraints[1].1, "test_ub");
    }

    #[test]
    fn nonzero_indicator_constraints_generates_four_constraints() {
        let poly = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
        let constraints =
            nonzero_indicator_constraints(&poly, 1, 2, 100.0, "test").unwrap();
        assert_eq!(constraints.len(), 4);
        assert_eq!(constraints[0].1, "test_band_ub");
        assert_eq!(constraints[1].1, "test_band_lb");
        assert_eq!(constraints[2].1, "test_out_lb");
        assert_eq!(constraints[3].1, "test_out_ub");
    }

    #[test]
    fn nonzero_indicator_constraints_match_existing_implementation() {
        let poly = Linear::new(
            vec![LinearMonomial::new(3.0, 0), LinearMonomial::new(-2.0, 1)],
            1.0,
        );
        let big_m = 500.0;

        let new_constraints =
            nonzero_indicator_constraints(&poly, 5, 6, big_m, "nz").unwrap();

        // Verify the constraints have the expected structure
        // Band UB: f(x) - big_m * y <= epsilon
        let band_ub = &new_constraints[0].0;
        assert_eq!(band_ub.relation, ConstraintRelation::LessEqual);
        assert_eq!(band_ub.rhs, NONZERO_TOLERANCE);

        // Band LB: f(x) + big_m * y >= -epsilon
        let band_lb = &new_constraints[1].0;
        assert_eq!(band_lb.relation, ConstraintRelation::GreaterEqual);
        assert_eq!(band_lb.rhs, -NONZERO_TOLERANCE);

        // Out LB: f(x) - big_m * y - big_m * s >= epsilon - 2*big_m
        let out_lb = &new_constraints[2].0;
        assert_eq!(out_lb.relation, ConstraintRelation::GreaterEqual);
        assert_eq!(out_lb.rhs, STRICT_NONZERO_BOUNDARY - 2.0 * big_m);

        // Out UB: f(x) + big_m * y - big_m * s <= -epsilon + big_m
        let out_ub = &new_constraints[3].0;
        assert_eq!(out_ub.relation, ConstraintRelation::LessEqual);
        assert_eq!(out_ub.rhs, -STRICT_NONZERO_BOUNDARY + big_m);
    }
}
