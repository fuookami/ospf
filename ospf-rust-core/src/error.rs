//! 错误类型定义
//! Error Type Definitions

use std::fmt::{Display, Formatter};
use thiserror::Error;
use ospf_rust_base::error::{ErrorCode, ErrorPosition, WithErrorPosition};
use ospf_rust_base::error_type;
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

    /// 求解器未找到错误 / Solver not found error
    #[error("{0}")]
    SolverNotFound(#[from] SolverNotFoundError),

    /// 求解器环境丢失错误 / Solver environment lost error
    #[error("{0}")]
    SolverEnvironmentLost(#[from] SolverEnvironmentLostError),

    /// 求解器求解异常错误 / Solver solving exception error
    #[error("{0}")]
    SolverSolving(#[from] SolverSolvingError),

    /// 求解器建模异常错误 / Solver modeling exception error
    #[error("{0}")]
    SolverModeling(#[from] SolverModelingError),

    /// 求解器终止错误 / Solver terminated error
    #[error("{0}")]
    SolverTerminated(#[from] SolverTerminatedError),

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

// ============================================================================
// StructuredError 转换 / StructuredError conversion
// ============================================================================

impl CoreError {
    /// 转换为 Box<dyn Error> / Convert to Box<dyn Error>
    ///
    /// 对应 Kotlin `toError()`。/ Corresponds to Kotlin `toError()`.
    pub fn to_boxed_error(self) -> Box<dyn std::error::Error> {
        Box::new(self)
    }

    /// 转换为失败的 Result / Convert to failed Result
    ///
    /// 对应 Kotlin `toFailed()`。/ Corresponds to Kotlin `toFailed()`.
    pub fn to_failed<T>(self) -> std::result::Result<T, Box<dyn std::error::Error>> {
        Err(self.to_boxed_error())
    }
}

// ============================================================================
// 命名错误子类型：用于重复构造点去重
// Named error subtypes: deduplicate repeated construction points
// ============================================================================

// 求解器未找到错误 / Solver not found error
// 替代重复的 `CoreError::Solver(SolverError::NotAvailable(...))` 构造。
error_type!(
    #[derive(Clone, Debug)]
    pub struct SolverNotFoundError {
        /// 求解器名称 / Solver name
        pub solver: Option<String>
    }
);

impl SolverNotFoundError {
    /// 创建无具体求解器名称的未找到错误 / Create not-found error without solver name
    #[track_caller]
    pub fn none() -> Self {
        let caller = std::panic::Location::caller();
        Self {
            solver: None,
            position: ErrorPosition { file: caller.file(), line: caller.line() },
        }
    }

    /// 创建带求解器名称的未找到错误 / Create not-found error with solver name
    #[track_caller]
    pub fn new(solver: impl Into<String>) -> Self {
        let caller = std::panic::Location::caller();
        Self {
            solver: Some(solver.into()),
            position: ErrorPosition { file: caller.file(), line: caller.line() },
        }
    }
}

impl Display for SolverNotFoundError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.solver {
            Some(s) => write!(f, "No solver valid: {}", s),
            None => write!(f, "No solver valid."),
        }
    }
}

impl ospf_rust_base::error::Error for SolverNotFoundError {
    fn code(&self) -> ErrorCode { ErrorCode::SolverNotFound }
    fn msg(&self) -> String { format!("{}", self) }
}

impl std::error::Error for SolverNotFoundError {}

// 求解器环境丢失错误 / Solver environment lost error
error_type!(
    #[derive(Clone, Debug)]
    pub struct SolverEnvironmentLostError {
        /// 错误详情 / Error detail
        pub detail: Option<String>
    }
);

impl SolverEnvironmentLostError {
    /// 创建求解器环境丢失错误 / Create solver environment lost error
    #[track_caller]
    pub fn new(detail: impl Into<String>) -> Self {
        let caller = std::panic::Location::caller();
        Self {
            detail: Some(detail.into()),
            position: ErrorPosition { file: caller.file(), line: caller.line() },
        }
    }
}

impl Display for SolverEnvironmentLostError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(d) => write!(f, "Solver environment lost: {}", d),
            None => write!(f, "Solver environment lost."),
        }
    }
}

impl ospf_rust_base::error::Error for SolverEnvironmentLostError {
    fn code(&self) -> ErrorCode { ErrorCode::OREngineEnvironmentLost }
    fn msg(&self) -> String { format!("{}", self) }
}

impl std::error::Error for SolverEnvironmentLostError {}

// 求解器求解异常错误 / Solver solving exception error
error_type!(
    #[derive(Clone, Debug)]
    pub struct SolverSolvingError {
        /// 错误详情 / Error detail
        pub detail: Option<String>
    }
);

impl Display for SolverSolvingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(d) => write!(f, "Solver solving exception: {}", d),
            None => write!(f, "Solver solving exception."),
        }
    }
}

impl ospf_rust_base::error::Error for SolverSolvingError {
    fn code(&self) -> ErrorCode { ErrorCode::OREngineSolvingException }
    fn msg(&self) -> String { format!("{}", self) }
}

impl std::error::Error for SolverSolvingError {}

// 求解器建模异常错误 / Solver modeling exception error
error_type!(
    #[derive(Clone, Debug)]
    pub struct SolverModelingError {
        /// 错误详情 / Error detail
        pub detail: Option<String>
    }
);

impl SolverModelingError {
    /// 创建求解器建模异常错误 / Create solver modeling error
    #[track_caller]
    pub fn new(detail: impl Into<String>) -> Self {
        let caller = std::panic::Location::caller();
        Self {
            detail: Some(detail.into()),
            position: ErrorPosition { file: caller.file(), line: caller.line() },
        }
    }
}

impl Display for SolverModelingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(d) => write!(f, "Solver modeling exception: {}", d),
            None => write!(f, "Solver modeling exception."),
        }
    }
}

impl ospf_rust_base::error::Error for SolverModelingError {
    fn code(&self) -> ErrorCode { ErrorCode::OREngineModelingException }
    fn msg(&self) -> String { format!("{}", self) }
}

impl std::error::Error for SolverModelingError {}

// 求解器终止错误 / Solver terminated error
error_type!(
    #[derive(Clone, Copy, Debug)]
    pub struct SolverTerminatedError {}
);

impl Display for SolverTerminatedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Solver terminated.")
    }
}

impl ospf_rust_base::error::Error for SolverTerminatedError {
    fn code(&self) -> ErrorCode { ErrorCode::OREngineTerminated }
    fn msg(&self) -> String { "Solver terminated.".to_string() }
}

impl std::error::Error for SolverTerminatedError {}

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

    #[test]
    fn test_named_error_subtypes() {
        use ospf_rust_base::error::Error as BaseError;

        let err = SolverNotFoundError {
            solver: Some("SCIP".to_string()),
            position: ErrorPosition { file: file!(), line: line!() },
        };
        assert_eq!(err.code(), ErrorCode::SolverNotFound);
        assert!(err.msg().contains("SCIP"));

        let err = SolverTerminatedError {
            position: ErrorPosition { file: file!(), line: line!() },
        };
        assert_eq!(err.code(), ErrorCode::OREngineTerminated);
        assert!(err.msg().contains("terminated"));
    }
}
