//! 求解值转换校验辅助 / Solve value conversion validation helpers
//!
//! 提供求解值转换过程中的有限性校验函数。
//! Provides finiteness validation functions during solve value conversion.

use crate::error::{CoreError, Result, SolverError};

/// 确保值有限 / Ensure value is finite
///
/// 检查给定的 f64 值是否为有限值（非 NaN、非无穷大），
/// 若非有限值则返回包含字段路径的错误。
/// Checks whether the given f64 value is finite (not NaN, not infinite),
/// returning an error containing the field path if the value is non-finite.
///
/// # 参数 / Parameters
/// - `value`: 待检查的值 / Value to check
/// - `field_path`: 字段路径，用于错误信息定位 / Field path for error message localization
///
/// # 返回 / Returns
/// 有限值时返回 `Ok(())`，否则返回错误 / `Ok(())` if finite, error otherwise
pub fn ensure_finite(value: f64, field_path: &str) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(CoreError::Solver(SolverError::NonFinite(format!(
            "non-finite value at `{}`: {}",
            field_path, value
        ))))
    }
}
