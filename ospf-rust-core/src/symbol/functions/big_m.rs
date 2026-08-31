use std::fmt::Debug;
use num_traits::ToPrimitive;
use crate::symbol::flatten::{Linear, Quadratic};
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
}
