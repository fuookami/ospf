//! 错误类型定义
//! Error Type Definitions

use crate::variable::VariableId;
use ospf_rust_base::error::{ErrorCode, ErrorPosition, WithErrorPosition};
use ospf_rust_base::error_type;
use std::fmt::{Display, Formatter};
use thiserror::Error;

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

/// 求解器边界错误的结构化分类 / Structured classification for solver-facing failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverErrorClass {
    /// 输入无效或值不受支持 / Invalid user input or unsupported value.
    Input,
    /// 模型构建或展平失败 / Model construction or flattening failure.
    Modeling,
    /// native 运行时、插件或许可证环境失败 / Native runtime, plugin, or license environment failure.
    Environment,
    /// 许可证或授权失败 / License or entitlement failure.
    License,
    /// 回调或上报器失败 / Callback or reporter failure.
    Callback,
    /// backend 启动后的失败 / Backend failure after startup.
    Backend,
    /// 解析或 artifact 物化失败 / Parsing or artifact materialization failure.
    Parsing,
    /// 数值不稳定或转换失败 / Numerical instability or conversion failure.
    Numerical,
    /// 内部合同违反 / Internal contract violation.
    InternalContract,
    /// 应由 SolveReport 表示的旧终态错误 / A legacy terminal error that should be represented by SolveReport.
    TerminalProjection,
    /// feature 或 backend 支持不可用 / Feature or backend support is unavailable.
    Unsupported,
}

impl SolverErrorClass {
    /// 返回稳定的机器可读分类代码 / Return the stable machine-readable class code.
    pub const fn code(self) -> &'static str {
        match self {
            Self::Input => "INPUT",
            Self::Modeling => "MODELING",
            Self::Environment => "ENVIRONMENT",
            Self::License => "LICENSE",
            Self::Callback => "CALLBACK",
            Self::Backend => "BACKEND",
            Self::Parsing => "PARSING",
            Self::Numerical => "NUMERICAL",
            Self::InternalContract => "INTERNAL_CONTRACT",
            Self::TerminalProjection => "TERMINAL_PROJECTION",
            Self::Unsupported => "UNSUPPORTED",
        }
    }

    /// 判断该分类是否表示正常终态兼容投影 / Check whether this class is a normal terminal projection.
    pub const fn is_normal_terminal(self) -> bool {
        matches!(self, Self::TerminalProjection)
    }
}

impl Display for SolverErrorClass {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
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
        /// 变量下界 / Variable lower bound
        lower: Option<f64>,
        /// 变量上界 / Variable upper bound
        upper: Option<f64>,
    },

    /// 无效的变量值 / Invalid variable value
    #[error("Invalid variable value: {value} for variable {var_id}")]
    InvalidValue {
        /// 变量标识 / Variable identifier
        var_id: VariableId,
        /// 变量值 / Variable value
        value: f64,
    },

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

    /// 约束规划模型错误 / Constraint programming model error
    #[error("Constraint programming model error: {0}")]
    ConstraintProgramming(String),
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

    /// 求解被取消 / Solve cancelled
    #[error("Solve cancelled: {0}")]
    Cancelled(String),

    /// 求解被中断 / Solve interrupted
    #[error("Solve interrupted: {0}")]
    Interrupted(String),

    /// 求解输入无效 / Invalid solver input
    #[error("Invalid solver input: {0}")]
    InvalidInput(String),

    /// 回调失败 / Callback failure
    #[error("Solver callback failed: {0}")]
    Callback(String),

    /// 结果或 artifact 解析失败 / Result or artifact parsing failure
    #[error("Solver result parsing failed: {0}")]
    Parsing(String),

    /// 求解合同违反 / Solver contract violation
    #[error("Solver contract violation: {0}")]
    ContractViolation(String),

    /// 无解 / No solution
    #[error("No solution found")]
    NoSolution,

    /// 无界解 / Unbounded solution
    #[error("Solution is unbounded")]
    Unbounded,

    /// 不可行 / Infeasible
    #[error("Problem is infeasible")]
    Infeasible,

    /// 不可行或无界 / Infeasible or unbounded
    #[error("Problem is infeasible or unbounded")]
    InfeasibleOrUnbounded,

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
    /// 不依赖自由文本消息分类错误 / Classify an error without relying on free-text messages.
    pub fn solver_error_class(&self) -> SolverErrorClass {
        match self {
            Self::Variable(_) => SolverErrorClass::Input,
            Self::Model(_) => SolverErrorClass::Modeling,
            Self::Solver(error) => match error {
                SolverError::NotAvailable(_) => SolverErrorClass::Environment,
                SolverError::SolveFailed(_) => SolverErrorClass::Backend,
                SolverError::Cancelled(_) | SolverError::Interrupted(_) => {
                    SolverErrorClass::TerminalProjection
                }
                SolverError::InvalidInput(_) => SolverErrorClass::Input,
                SolverError::Callback(_) => SolverErrorClass::Callback,
                SolverError::Parsing(_) => SolverErrorClass::Parsing,
                SolverError::ContractViolation(_) => SolverErrorClass::InternalContract,
                SolverError::NoSolution
                | SolverError::Unbounded
                | SolverError::Infeasible
                | SolverError::InfeasibleOrUnbounded
                | SolverError::Timeout(_) => SolverErrorClass::TerminalProjection,
                SolverError::NumericalError(_)
                | SolverError::PrecisionLoss(_)
                | SolverError::Overflow(_)
                | SolverError::NonFinite(_) => SolverErrorClass::Numerical,
                SolverError::UnsupportedValueType(_) => SolverErrorClass::Unsupported,
                SolverError::LicenseError(_) => SolverErrorClass::License,
            },
            Self::SolverNotFound(_) | Self::SolverEnvironmentLost(_) => {
                SolverErrorClass::Environment
            }
            Self::SolverSolving(_) => SolverErrorClass::Backend,
            Self::SolverModeling(_) => SolverErrorClass::Modeling,
            Self::SolverTerminated(_) => SolverErrorClass::TerminalProjection,
            Self::NotImplemented(_) => SolverErrorClass::Unsupported,
            Self::Internal(_) => SolverErrorClass::InternalContract,
        }
    }

    /// 判断该错误是否为应转换为报告的旧终态投影 / Whether this error is a legacy terminal projection that should become a report.
    pub fn is_terminal_projection(&self) -> bool {
        self.solver_error_class() == SolverErrorClass::TerminalProjection
    }

    /// 判断错误是否只是旧 API 对正常终态的有损投影 / Check whether the error is only a legacy projection of a normal terminal state.
    pub fn is_normal_terminal(&self) -> bool {
        self.solver_error_class().is_normal_terminal()
    }

    /// 返回结构化错误分类代码 / Return the structured error classification code.
    pub fn solver_error_class_code(&self) -> &'static str {
        self.solver_error_class().code()
    }

    /// Create a structured callback failure / 创建结构化回调失败。
    pub fn callback_error(message: impl Into<String>) -> Self {
        Self::Solver(SolverError::Callback(message.into()))
    }

    /// Create a structured result-parsing failure / 创建结构化结果解析失败。
    pub fn parsing_error(message: impl Into<String>) -> Self {
        Self::Solver(SolverError::Parsing(message.into()))
    }

    /// Create a structured solver-contract failure / 创建结构化求解合同失败。
    pub fn contract_error(message: impl Into<String>) -> Self {
        Self::Solver(SolverError::ContractViolation(message.into()))
    }

    /// 创建 native 建模错误 / Create a native modeling error.
    pub fn solver_modeling(message: impl Into<String>) -> Self {
        Self::SolverModeling(SolverModelingError::new(message))
    }

    /// 创建 native 环境错误 / Create a native environment error.
    pub fn solver_environment(message: impl Into<String>) -> Self {
        Self::SolverEnvironmentLost(SolverEnvironmentLostError::new(message))
    }

    /// 创建 native 许可证错误 / Create a native license error.
    pub fn solver_license(message: impl Into<String>) -> Self {
        Self::Solver(SolverError::LicenseError(message.into()))
    }

    /// 创建 native 求解阶段错误 / Create a native solve-stage error.
    pub fn solver_backend(message: impl Into<String>) -> Self {
        Self::SolverSolving(SolverSolvingError::new(message))
    }

    /// 创建带来源的取消终态兼容错误 / Create a cancellation terminal-projection error with origin.
    pub fn cancelled(origin: impl Into<String>) -> Self {
        Self::Solver(SolverError::Cancelled(format!(
            "solver attempt cancelled; origin={}",
            origin.into()
        )))
    }

    /// 创建带来源的中断终态兼容错误 / Create an interruption terminal-projection error with origin.
    pub fn interrupted(origin: impl Into<String>) -> Self {
        Self::Solver(SolverError::Interrupted(format!(
            "solver attempt interrupted; origin={}",
            origin.into()
        )))
    }

    /// 转换为 `Box<dyn Error>` / Convert to `Box<dyn Error>`
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
        pub solver: Option<String>,
    }
);

impl SolverNotFoundError {
    /// 创建无具体求解器名称的未找到错误 / Create not-found error without solver name
    #[track_caller]
    pub fn none() -> Self {
        let caller = std::panic::Location::caller();
        Self {
            solver: None,
            position: ErrorPosition {
                file: caller.file(),
                line: caller.line(),
            },
        }
    }

    /// 创建带求解器名称的未找到错误 / Create not-found error with solver name
    #[track_caller]
    pub fn new(solver: impl Into<String>) -> Self {
        let caller = std::panic::Location::caller();
        Self {
            solver: Some(solver.into()),
            position: ErrorPosition {
                file: caller.file(),
                line: caller.line(),
            },
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
    fn code(&self) -> ErrorCode {
        ErrorCode::SolverNotFound
    }
    fn msg(&self) -> String {
        format!("{}", self)
    }
}

impl std::error::Error for SolverNotFoundError {}

// 求解器环境丢失错误 / Solver environment lost error
error_type!(
    #[derive(Clone, Debug)]
    pub struct SolverEnvironmentLostError {
        /// 错误详情 / Error detail
        pub detail: Option<String>,
    }
);

impl SolverEnvironmentLostError {
    /// 创建求解器环境丢失错误 / Create solver environment lost error
    #[track_caller]
    pub fn new(detail: impl Into<String>) -> Self {
        let caller = std::panic::Location::caller();
        Self {
            detail: Some(detail.into()),
            position: ErrorPosition {
                file: caller.file(),
                line: caller.line(),
            },
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
    fn code(&self) -> ErrorCode {
        ErrorCode::OREngineEnvironmentLost
    }
    fn msg(&self) -> String {
        format!("{}", self)
    }
}

impl std::error::Error for SolverEnvironmentLostError {}

// 求解器求解异常错误 / Solver solving exception error
error_type!(
    #[derive(Clone, Debug)]
    pub struct SolverSolvingError {
        /// 错误详情 / Error detail
        pub detail: Option<String>,
    }
);

impl SolverSolvingError {
    /// 创建求解阶段异常 / Create a solver-stage failure
    #[track_caller]
    pub fn new(detail: impl Into<String>) -> Self {
        let caller = std::panic::Location::caller();
        Self {
            detail: Some(detail.into()),
            position: ErrorPosition {
                file: caller.file(),
                line: caller.line(),
            },
        }
    }
}

impl Display for SolverSolvingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(d) => write!(f, "Solver solving exception: {}", d),
            None => write!(f, "Solver solving exception."),
        }
    }
}

impl ospf_rust_base::error::Error for SolverSolvingError {
    fn code(&self) -> ErrorCode {
        ErrorCode::OREngineSolvingException
    }
    fn msg(&self) -> String {
        format!("{}", self)
    }
}

impl std::error::Error for SolverSolvingError {}

// 求解器建模异常错误 / Solver modeling exception error
error_type!(
    #[derive(Clone, Debug)]
    pub struct SolverModelingError {
        /// 错误详情 / Error detail
        pub detail: Option<String>,
    }
);

impl SolverModelingError {
    /// 创建求解器建模异常错误 / Create solver modeling error
    #[track_caller]
    pub fn new(detail: impl Into<String>) -> Self {
        let caller = std::panic::Location::caller();
        Self {
            detail: Some(detail.into()),
            position: ErrorPosition {
                file: caller.file(),
                line: caller.line(),
            },
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
    fn code(&self) -> ErrorCode {
        ErrorCode::OREngineModelingException
    }
    fn msg(&self) -> String {
        format!("{}", self)
    }
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
    fn code(&self) -> ErrorCode {
        ErrorCode::OREngineTerminated
    }
    fn msg(&self) -> String {
        "Solver terminated.".to_string()
    }
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
            position: ErrorPosition {
                file: file!(),
                line: line!(),
            },
        };
        assert_eq!(err.code(), ErrorCode::SolverNotFound);
        assert!(err.msg().contains("SCIP"));

        let err = SolverTerminatedError {
            position: ErrorPosition {
                file: file!(),
                line: line!(),
            },
        };
        assert_eq!(err.code(), ErrorCode::OREngineTerminated);
        assert!(err.msg().contains("terminated"));
    }

    #[test]
    fn solver_error_class_keeps_err_and_terminal_boundaries_explicit() {
        assert_eq!(
            CoreError::Solver(SolverError::Timeout(std::time::Duration::from_secs(1)))
                .solver_error_class(),
            SolverErrorClass::TerminalProjection
        );
        assert_eq!(
            CoreError::Solver(SolverError::LicenseError("missing".to_owned())).solver_error_class(),
            SolverErrorClass::License
        );
        assert_eq!(
            CoreError::Solver(SolverError::SolveFailed("backend".to_owned())).solver_error_class(),
            SolverErrorClass::Backend
        );
        assert_eq!(
            CoreError::Solver(SolverError::Cancelled("USER".to_owned())).solver_error_class(),
            SolverErrorClass::TerminalProjection
        );
        assert_eq!(
            CoreError::Solver(SolverError::Interrupted("EXTERNAL".to_owned())).solver_error_class(),
            SolverErrorClass::TerminalProjection
        );
        assert_eq!(
            CoreError::Solver(SolverError::InvalidInput("payload".to_owned())).solver_error_class(),
            SolverErrorClass::Input
        );
        assert_eq!(
            CoreError::callback_error("callback").solver_error_class(),
            SolverErrorClass::Callback
        );
        assert_eq!(
            CoreError::parsing_error("artifact").solver_error_class(),
            SolverErrorClass::Parsing
        );
        assert_eq!(
            CoreError::contract_error("identity").solver_error_class(),
            SolverErrorClass::InternalContract
        );
        assert!(CoreError::Solver(SolverError::Infeasible).is_terminal_projection());
        assert!(!CoreError::Internal("contract".to_owned()).is_terminal_projection());
        assert_eq!(
            CoreError::Model(ModelError::MissingObjective).solver_error_class(),
            SolverErrorClass::Modeling
        );
        assert_eq!(
            CoreError::Solver(SolverError::Timeout(std::time::Duration::from_secs(1)))
                .solver_error_class_code(),
            "TERMINAL_PROJECTION"
        );
        assert!(
            CoreError::Solver(SolverError::Timeout(std::time::Duration::from_secs(1)))
                .is_normal_terminal()
        );
        assert_eq!(SolverErrorClass::Callback.to_string(), "CALLBACK");
    }

    #[test]
    fn native_error_constructors_preserve_boundary_classification() {
        assert_eq!(
            CoreError::solver_modeling("invalid row").solver_error_class(),
            SolverErrorClass::Modeling
        );
        assert_eq!(
            CoreError::solver_environment("missing license").solver_error_class(),
            SolverErrorClass::Environment
        );
        assert_eq!(
            CoreError::solver_license("missing license").solver_error_class(),
            SolverErrorClass::License
        );
        assert_eq!(
            CoreError::solver_backend("native solve failed").solver_error_class(),
            SolverErrorClass::Backend
        );
        assert!(!CoreError::solver_backend("native solve failed").is_terminal_projection());
    }

    #[test]
    fn every_solver_error_variant_has_a_stable_classification() {
        let cases = [
            (
                SolverError::NotAvailable("plugin".to_owned()),
                SolverErrorClass::Environment,
            ),
            (
                SolverError::SolveFailed("backend".to_owned()),
                SolverErrorClass::Backend,
            ),
            (
                SolverError::Cancelled("USER".to_owned()),
                SolverErrorClass::TerminalProjection,
            ),
            (
                SolverError::Interrupted("EXTERNAL".to_owned()),
                SolverErrorClass::TerminalProjection,
            ),
            (
                SolverError::InvalidInput("payload".to_owned()),
                SolverErrorClass::Input,
            ),
            (
                SolverError::Callback("observer".to_owned()),
                SolverErrorClass::Callback,
            ),
            (
                SolverError::Parsing("report".to_owned()),
                SolverErrorClass::Parsing,
            ),
            (
                SolverError::ContractViolation("report".to_owned()),
                SolverErrorClass::InternalContract,
            ),
            (
                SolverError::NoSolution,
                SolverErrorClass::TerminalProjection,
            ),
            (SolverError::Unbounded, SolverErrorClass::TerminalProjection),
            (
                SolverError::Infeasible,
                SolverErrorClass::TerminalProjection,
            ),
            (
                SolverError::InfeasibleOrUnbounded,
                SolverErrorClass::TerminalProjection,
            ),
            (
                SolverError::NumericalError("numerical".to_owned()),
                SolverErrorClass::Numerical,
            ),
            (
                SolverError::PrecisionLoss("precision".to_owned()),
                SolverErrorClass::Numerical,
            ),
            (
                SolverError::Overflow("overflow".to_owned()),
                SolverErrorClass::Numerical,
            ),
            (
                SolverError::NonFinite("nan".to_owned()),
                SolverErrorClass::Numerical,
            ),
            (
                SolverError::UnsupportedValueType("i128".to_owned()),
                SolverErrorClass::Unsupported,
            ),
            (
                SolverError::Timeout(std::time::Duration::from_secs(1)),
                SolverErrorClass::TerminalProjection,
            ),
            (
                SolverError::LicenseError("missing".to_owned()),
                SolverErrorClass::License,
            ),
        ];

        for (solver_error, expected) in cases {
            let error = CoreError::Solver(solver_error);
            assert_eq!(error.solver_error_class(), expected);
            assert_eq!(error.solver_error_class_code(), expected.code());
            assert_eq!(
                error.is_terminal_projection(),
                expected.is_normal_terminal()
            );
        }
    }

    #[test]
    fn every_core_error_wrapper_has_a_stable_classification() {
        let cases = [
            (
                CoreError::Variable(VariableError::NotFound(VariableId::standalone(0))),
                SolverErrorClass::Input,
            ),
            (
                CoreError::Model(ModelError::MissingObjective),
                SolverErrorClass::Modeling,
            ),
            (
                CoreError::SolverNotFound(SolverNotFoundError {
                    solver: Some("SCIP".to_owned()),
                    position: ErrorPosition {
                        file: file!(),
                        line: line!(),
                    },
                }),
                SolverErrorClass::Environment,
            ),
            (
                CoreError::SolverEnvironmentLost(SolverEnvironmentLostError::new("lost")),
                SolverErrorClass::Environment,
            ),
            (
                CoreError::SolverSolving(SolverSolvingError::new("solve")),
                SolverErrorClass::Backend,
            ),
            (
                CoreError::SolverModeling(SolverModelingError::new("model")),
                SolverErrorClass::Modeling,
            ),
            (
                CoreError::SolverTerminated(SolverTerminatedError {
                    position: ErrorPosition {
                        file: file!(),
                        line: line!(),
                    },
                }),
                SolverErrorClass::TerminalProjection,
            ),
            (
                CoreError::NotImplemented("feature".to_owned()),
                SolverErrorClass::Unsupported,
            ),
            (
                CoreError::Internal("contract".to_owned()),
                SolverErrorClass::InternalContract,
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(error.solver_error_class(), expected);
            assert_eq!(error.solver_error_class_code(), expected.code());
            assert_eq!(
                error.is_terminal_projection(),
                expected.is_normal_terminal()
            );
        }

        assert!(
            CoreError::Solver(SolverError::Timeout(std::time::Duration::from_secs(1)))
                .is_normal_terminal()
        );
        assert!(
            !CoreError::Solver(SolverError::SolveFailed("backend".to_owned())).is_normal_terminal()
        );
    }
}
