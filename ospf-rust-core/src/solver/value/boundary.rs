//! 求解边界转换入口
//! Solver boundary conversion entry

use crate::error::Result;
use super::conversion_context::SolveValueConversionContext;
use super::validation::ensure_finite;
use super::{SolveValue, SolveValueConversionPolicy};

pub fn value_to_backend_f64<V: SolveValue>(
    value: &V,
    policy: SolveValueConversionPolicy,
    context: &SolveValueConversionContext,
) -> Result<f64> {
    let backend = value.to_f64_with_policy(policy)?;
    ensure_finite(backend, &context.field_path)?;
    Ok(backend)
}

pub fn value_from_backend_f64<V: SolveValue>(
    value: f64,
    policy: SolveValueConversionPolicy,
    context: &SolveValueConversionContext,
) -> Result<V> {
    ensure_finite(value, &context.field_path)?;
    V::from_f64_with_policy(value, policy)
}
