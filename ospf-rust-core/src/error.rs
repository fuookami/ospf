//! 错误类型定义
//! Error Type Definitions

use thiserror::Error;
use crate::variable::VariableId;

/// 核心模块错误类型 / Core module error type
#[derive(Debug, Error)]
pub enum CoreError {
    /// 变量错误 / Variable error
    #[error("Variable error: {0}")]
    Variable(#[from] VariableError),

    /// 模型错误 / Model error
    #[error("Model error: {0}")]
    Model(#[from] ModelError),

    /// 求解器错误 / Solver error
    #[error("Solver error: {0}")]
    Solver(#[from] SolverError),

    /// 未实现错误 / Not implemented error
    #[error("Not implemented: {0}")]
    NotImplemented(String),

    /// 内部错误 / Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

/// 变量相关错误 / Variable-related errors
#[derive(Debug, Error)]
pub enum VariableError {
    /// 变量未找到 / Variable not found
    #[error("Variable not found: {0}")]
    NotFound(VariableId),

    /// 变量已存在 / Variable already exists
    #[error("Variable already exists: {0}")]
    AlreadyExists(VariableId),

    /// 无效的变量范围 / Invalid variable range
    #[error("Invalid variable range: lower bound {lower:?} > upper bound {upper:?}")]
    InvalidRange {
        lower: Option<f64>,
        upper: Option<f64>,
    },

    /// 无效的变量值 / Invalid variable value
    #[error("Invalid variable value: {value} for variable {var_id}")]
    InvalidValue { var_id: VariableId, value: f64 },

    /// 变量名称冲突 / Variable name conflict
    #[error("Variable name conflict: {0}")]
    NameConflict(String),
}

/// 模型相关错误 / Model-related errors
#[derive(Debug, Error)]
pub enum ModelError {
    /// 模型未初始化 / Model not initialized
    #[error("Model not initialized")]
    NotInitialized,

    /// 模型已求解 / Model already solved
    #[error("Model already solved")]
    AlreadySolved,

    /// 约束冲突 / Constraint conflict
    #[error("Constraint conflict: {0}")]
    ConstraintConflict(String),

    /// 目标函数缺失 / Missing objective
    #[error("Missing objective function")]
    MissingObjective,

    /// 无效的约束 / Invalid constraint
    #[error("Invalid constraint: {0}")]
    InvalidConstraint(String),

    /// 符号未注册 / Symbol not registered
    #[error("Symbol not registered: {0}")]
    SymbolNotRegistered(String),
}

/// 求解器相关错误 / Solver-related errors
#[derive(Debug, Error)]
pub enum SolverError {
    /// 求解器不可用 / Solver not available
    #[error("Solver not available: {0}")]
    NotAvailable(String),

    /// 求解失败 / Solve failed
    #[error("Solve failed: {0}")]
    SolveFailed(String),

    /// 无解 / No solution
    #[error("No solution found")]
    NoSolution,

    /// 无界解 / Unbounded solution
    #[error("Solution is unbounded")]
    Unbounded,

    /// 不可行 / Infeasible
    #[error("Problem is infeasible")]
    Infeasible,

    /// 数值错误 / Numerical error
    #[error("Numerical error: {0}")]
    NumericalError(String),

    /// 精度损失 / Precision loss
    #[error("Precision loss: {0}")]
    PrecisionLoss(String),

    /// 数值溢出 / Numeric overflow
    #[error("Overflow: {0}")]
    Overflow(String),

    /// 非有限值 / Non-finite value
    #[error("Non-finite value: {0}")]
    NonFinite(String),

    /// 不支持的值类型 / Unsupported value type
    #[error("Unsupported value type: {0}")]
    UnsupportedValueType(String),

    /// 超时 / Timeout
    #[error("Solver timeout after {0:?}")]
    Timeout(std::time::Duration),

    /// 许可证错误 / License error
    #[error("License error: {0}")]
    LicenseError(String),
}

/// 结果类型别名 / Result type alias
pub type Result<T> = std::result::Result<T, CoreError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = VariableError::NotFound(VariableId::standalone(0));
        assert!(err.to_string().contains("Variable not found"));

        let err = CoreError::Variable(err);
        assert!(err.to_string().contains("Variable error"));
    }
}
