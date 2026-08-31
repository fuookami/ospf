use std::fmt::Debug;

use num_traits::ToPrimitive;

use crate::flatten::Linear;
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

pub fn infer_linear_bounds_from_tokens<V>(poly: &Linear<V>, tokens: &[Token<V>]) -> Option<(f64, f64)>
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
