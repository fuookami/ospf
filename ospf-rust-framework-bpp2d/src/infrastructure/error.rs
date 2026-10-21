//! BPP2D 领域错误 / BPP2D domain errors

use ospf_rust_base::error::{Error, ErrorCode, ErrorPosition, WithErrorPosition};
use ospf_rust_base::error_type;
use ospf_rust_base::error;
use std::fmt::{Debug, Display, Formatter};

// BPP2D 校验错误 / BPP2D validation error
error_type!(
    #[derive(Clone, Debug)]
    pub struct Bpp2dValidationError {
        /// 错误详情 / Error detail
        pub detail: Option<String>,
    }
);

impl Display for Bpp2dValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(detail) => write!(f, "BPP2D 校验错误: {}", detail),
            None => write!(f, "BPP2D 校验错误。"),
        }
    }
}

impl Error for Bpp2dValidationError {
    fn code(&self) -> ErrorCode {
        ErrorCode::IllegalArgument
    }

    fn msg(&self) -> String {
        format!("{}", self)
    }
}

// BPP2D 求解错误 / BPP2D solving error
error_type!(
    #[derive(Clone, Debug)]
    pub struct Bpp2dSolvingError {
        /// 错误详情 / Error detail
        pub detail: Option<String>,
    }
);

impl Display for Bpp2dSolvingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(detail) => write!(f, "BPP2D 求解错误: {}", detail),
            None => write!(f, "BPP2D 求解错误。"),
        }
    }
}

impl Error for Bpp2dSolvingError {
    fn code(&self) -> ErrorCode {
        ErrorCode::OREngineSolvingException
    }

    fn msg(&self) -> String {
        format!("{}", self)
    }
}

// BPP2D 能力错误 / BPP2D capability error
error_type!(
    #[derive(Clone, Debug)]
    pub struct Bpp2dCapabilityError {
        /// 错误详情 / Error detail
        pub detail: Option<String>,
    }
);

impl Display for Bpp2dCapabilityError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(detail) => write!(f, "BPP2D 能力错误: {}", detail),
            None => write!(f, "BPP2D 能力错误。"),
        }
    }
}

impl Error for Bpp2dCapabilityError {
    fn code(&self) -> ErrorCode {
        ErrorCode::ApplicationError
    }

    fn msg(&self) -> String {
        format!("{}", self)
    }
}

// BPP2D 内部错误 / BPP2D internal error
error_type!(
    #[derive(Clone, Debug)]
    pub struct Bpp2dInternalError {
        /// 错误详情 / Error detail
        pub detail: Option<String>,
    }
);

impl Display for Bpp2dInternalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(detail) => write!(f, "BPP2D 内部错误: {}", detail),
            None => write!(f, "BPP2D 内部错误。"),
        }
    }
}

impl Error for Bpp2dInternalError {
    fn code(&self) -> ErrorCode {
        ErrorCode::ApplicationError
    }

    fn msg(&self) -> String {
        format!("{}", self)
    }
}

/// 构造校验错误 / Build a validation error
pub fn validation_error(detail: impl Into<String>) -> Box<dyn Error> {
    Box::new(error!(Bpp2dValidationError { detail: Some(detail.into()) }))
}

/// 构造求解错误 / Build a solving error
pub fn solving_error(detail: impl Into<String>) -> Box<dyn Error> {
    Box::new(error!(Bpp2dSolvingError { detail: Some(detail.into()) }))
}

/// 构造能力错误 / Build a capability error
pub fn capability_error(detail: impl Into<String>) -> Box<dyn Error> {
    Box::new(error!(Bpp2dCapabilityError { detail: Some(detail.into()) }))
}

/// 构造内部错误 / Build an internal error
pub fn internal_error(detail: impl Into<String>) -> Box<dyn Error> {
    Box::new(error!(Bpp2dInternalError { detail: Some(detail.into()) }))
}

/// BPP2D 结果类型 / BPP2D result type
pub type Bpp2dResult<T> = ospf_rust_base::error::Ret<T>;
