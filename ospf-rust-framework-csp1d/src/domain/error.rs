//! CSP1D 领域错误类型
//! CSP1D domain error types

use ospf_rust_base::error::{Error, ErrorCode, ErrorPosition, WithErrorPosition};
use ospf_rust_base::error_enum;
use ospf_rust_base::error_type;
use std::fmt::{Debug, Display, Formatter};

// ============================================================================
// CSP1D 领域错误
// ============================================================================

// CSP1D 生命周期错误 / CSP1D lifecycle error
error_type!(
    #[derive(Clone, Debug)]
    pub struct Csp1dLifecycleError {
        /// 错误详情 / Error detail
        pub detail: Option<String>,
    }
);

impl Display for Csp1dLifecycleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(d) => write!(f, "CSP1D 生命周期错误: {}", d),
            None => write!(f, "CSP1D 生命周期错误。"),
        }
    }
}

impl Error for Csp1dLifecycleError {
    fn code(&self) -> ErrorCode {
        ErrorCode::ApplicationError
    }
    fn msg(&self) -> String {
        format!("{}", self)
    }
}

// CSP1D 类型错误 / CSP1D type error
error_type!(
    #[derive(Clone, Debug)]
    pub struct Csp1dTypeError {
        /// 错误详情 / Error detail
        pub detail: Option<String>,
    }
);

impl Display for Csp1dTypeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(d) => write!(f, "CSP1D 类型错误: {}", d),
            None => write!(f, "CSP1D 类型错误。"),
        }
    }
}

impl Error for Csp1dTypeError {
    fn code(&self) -> ErrorCode {
        ErrorCode::IllegalArgument
    }
    fn msg(&self) -> String {
        format!("{}", self)
    }
}

// CSP1D 求解错误 / CSP1D solving error
error_type!(
    #[derive(Clone, Debug)]
    pub struct Csp1dSolvingError {
        /// 错误详情 / Error detail
        pub detail: Option<String>,
    }
);

impl Display for Csp1dSolvingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(d) => write!(f, "CSP1D 求解错误: {}", d),
            None => write!(f, "CSP1D 求解错误。"),
        }
    }
}

impl Error for Csp1dSolvingError {
    fn code(&self) -> ErrorCode {
        ErrorCode::ApplicationFailed
    }
    fn msg(&self) -> String {
        format!("{}", self)
    }
}

// CSP1D 能力错误 / CSP1D capability error
error_type!(
    #[derive(Clone, Debug)]
    pub struct Csp1dCapabilityError {
        /// 错误详情 / Error detail
        pub detail: Option<String>,
    }
);

impl Display for Csp1dCapabilityError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(d) => write!(f, "CSP1D 能力错误: {}", d),
            None => write!(f, "CSP1D 能力错误。"),
        }
    }
}

impl Error for Csp1dCapabilityError {
    fn code(&self) -> ErrorCode {
        ErrorCode::IllegalArgument
    }
    fn msg(&self) -> String {
        format!("{}", self)
    }
}

// CSP1D 统一错误枚举 / CSP1D unified error enum
error_enum!(
    #[derive(Clone)]
    pub enum Csp1dError {
        Lifecycle(Csp1dLifecycleError),
        TypeError(Csp1dTypeError),
        Solving(Csp1dSolvingError),
        Capability(Csp1dCapabilityError),
    }
);

// 域错误到 crate 级错误的转换
// Domain error to crate-level error conversion
impl From<Csp1dError> for crate::Csp1dError {
    fn from(err: Csp1dError) -> Self {
        match err {
            Csp1dError::Lifecycle(e) => crate::Csp1dError::Unsupported { message: e.msg() },
            Csp1dError::TypeError(e) => crate::Csp1dError::InvalidInput { message: e.msg() },
            Csp1dError::Solving(e) => crate::Csp1dError::Calculation { message: e.msg() },
            Csp1dError::Capability(e) => crate::Csp1dError::Unsupported { message: e.msg() },
        }
    }
}
