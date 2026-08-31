//! 求解值转换校验辅助
//! Solve value conversion validation helpers

use crate::error::{CoreError, Result, SolverError};

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
