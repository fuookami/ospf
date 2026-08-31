//! 求解边界转换入口 / Solver boundary conversion entry point

use crate::error::Result;
use super::conversion_context::SolveValueConversionContext;
use super::validation::ensure_finite;
use super::{SolveValue, SolveValueConversionPolicy};

/// 将求解值转换为后端 f64 / Convert solve value to backend f64
///
/// 根据转换策略将求解值转换为后端求解器所需的 f64 值，
/// 并确保结果为有限值。
/// Converts a solve value to the f64 value required by the backend solver
/// according to the conversion policy, ensuring the result is finite.
///
/// # 参数 / Parameters
/// - `value`: 待转换的求解值 / Solve value to convert
/// - `policy`: 转换策略 / Conversion policy
/// - `context`: 转换上下文 / Conversion context
///
/// # 返回 / Returns
/// 转换后的 f64 值 / Converted f64 value
pub fn value_to_backend_f64<V: SolveValue>(
    value: &V,
    policy: SolveValueConversionPolicy,
    context: &SolveValueConversionContext,
) -> Result<f64> {
    let backend = value.to_f64_with_policy(policy)?;
    ensure_finite(backend, &context.field_path)?;
    Ok(backend)
}

/// 将后端 f64 转换为求解值 / Convert backend f64 to solve value
///
/// 根据转换策略将后端求解器返回的 f64 值转换为求解值类型，
/// 并确保输入为有限值。
/// Converts an f64 value returned by the backend solver to a solve value type
/// according to the conversion policy, ensuring the input is finite.
///
/// # 参数 / Parameters
/// - `value`: 后端 f64 值 / Backend f64 value
/// - `policy`: 转换策略 / Conversion policy
/// - `context`: 转换上下文 / Conversion context
///
/// # 返回 / Returns
/// 转换后的求解值 / Converted solve value
pub fn value_from_backend_f64<V: SolveValue>(
    value: f64,
    policy: SolveValueConversionPolicy,
    context: &SolveValueConversionContext,
) -> Result<V> {
    ensure_finite(value, &context.field_path)?;
    V::from_f64_with_policy(value, policy)
}
