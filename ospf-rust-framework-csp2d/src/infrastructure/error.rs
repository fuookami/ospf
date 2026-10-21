//! CSP2D 领域错误 / CSP2D domain errors

use ospf_rust_base::error::{Error, ErrorCode, ErrorPosition, WithErrorPosition};
use ospf_rust_base::error_type;
use ospf_rust_base::error;
use std::fmt::{Debug, Display, Formatter};

// CSP2D 校验错误 / CSP2D validation error
error_type!(
    #[derive(Clone, Debug)]
    pub struct Csp2dValidationError {
        /// 错误详情 / Error detail
        pub detail: Option<String>,
    }
);

impl Display for Csp2dValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(detail) => write!(f, "CSP2D 校验错误: {}", detail),
            None => write!(f, "CSP2D 校验错误。"),
        }
    }
}

impl Error for Csp2dValidationError {
    fn code(&self) -> ErrorCode {
        ErrorCode::IllegalArgument
    }

    fn msg(&self) -> String {
        format!("{}", self)
    }
}

// CSP2D 求解错误 / CSP2D solving error
error_type!(
    #[derive(Clone, Debug)]
    pub struct Csp2dSolvingError {
        /// 错误详情 / Error detail
        pub detail: Option<String>,
    }
);

impl Display for Csp2dSolvingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(detail) => write!(f, "CSP2D 求解错误: {}", detail),
            None => write!(f, "CSP2D 求解错误。"),
        }
    }
}

impl Error for Csp2dSolvingError {
    fn code(&self) -> ErrorCode {
        ErrorCode::OREngineSolvingException
    }

    fn msg(&self) -> String {
        format!("{}", self)
    }
}

// CSP2D 能力错误 / CSP2D capability error
error_type!(
    #[derive(Clone, Debug)]
    pub struct Csp2dCapabilityError {
        /// 错误详情 / Error detail
        pub detail: Option<String>,
    }
);

impl Display for Csp2dCapabilityError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(detail) => write!(f, "CSP2D 能力错误: {}", detail),
            None => write!(f, "CSP2D 能力错误。"),
        }
    }
}

impl Error for Csp2dCapabilityError {
    fn code(&self) -> ErrorCode {
        ErrorCode::ApplicationError
    }

    fn msg(&self) -> String {
        format!("{}", self)
    }
}

// CSP2D 内部错误 / CSP2D internal error
error_type!(
    #[derive(Clone, Debug)]
    pub struct Csp2dInternalError {
        /// 错误详情 / Error detail
        pub detail: Option<String>,
    }
);

impl Display for Csp2dInternalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(detail) => write!(f, "CSP2D 内部错误: {}", detail),
            None => write!(f, "CSP2D 内部错误。"),
        }
    }
}

impl Error for Csp2dInternalError {
    fn code(&self) -> ErrorCode {
        ErrorCode::ApplicationError
    }

    fn msg(&self) -> String {
        format!("{}", self)
    }
}

/// 构造校验错误 / Build a validation error
pub fn validation_error(detail: impl Into<String>) -> Box<dyn Error> {
    Box::new(error!(Csp2dValidationError { detail: Some(detail.into()) }))
}

/// 构造求解错误 / Build a solving error
pub fn solving_error(detail: impl Into<String>) -> Box<dyn Error> {
    Box::new(error!(Csp2dSolvingError { detail: Some(detail.into()) }))
}

/// 构造能力错误 / Build a capability error
pub fn capability_error(detail: impl Into<String>) -> Box<dyn Error> {
    Box::new(error!(Csp2dCapabilityError { detail: Some(detail.into()) }))
}

/// CSP2D 结果类型 / CSP2D result type
pub type Csp2dResult<T> = ospf_rust_base::error::Ret<T>;
