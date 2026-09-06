//! BPP3D 领域错误类型
//! BPP3D domain error types

use ospf_rust_base::error::{Error, ErrorCode, ErrorPosition, WithErrorPosition};
use ospf_rust_base::error_enum;
use ospf_rust_base::error_type;
use std::fmt::{Debug, Display, Formatter};

// ============================================================================
// BPP3D 领域错误
// ============================================================================

error_type!(
    /// BPP3D 能力不支持错误 / BPP3D capability not supported error
    #[derive(Clone, Debug)]
    pub struct Bpp3dCapabilityError {
        /// 不支持的能力名称 / Name of the unsupported capability
        pub capability: Option<String>,
    }
);

impl Display for Bpp3dCapabilityError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.capability {
            Some(c) => write!(f, "BPP3D 能力不支持: {}", c),
            None => write!(f, "BPP3D 能力不支持。"),
        }
    }
}

impl ospf_rust_base::error::Error for Bpp3dCapabilityError {
    fn code(&self) -> ErrorCode {
        ErrorCode::IllegalArgument
    }
    fn msg(&self) -> String {
        format!("{}", self)
    }
}

error_type!(
    /// BPP3D 求解错误 / BPP3D solving error
    #[derive(Clone, Debug)]
    pub struct Bpp3dSolvingError {
        /// 错误详情 / Error detail
        pub detail: Option<String>,
    }
);

impl Display for Bpp3dSolvingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(d) => write!(f, "BPP3D 求解错误: {}", d),
            None => write!(f, "BPP3D 求解错误。"),
        }
    }
}

impl ospf_rust_base::error::Error for Bpp3dSolvingError {
    fn code(&self) -> ErrorCode {
        ErrorCode::ApplicationFailed
    }
    fn msg(&self) -> String {
        format!("{}", self)
    }
}

error_type!(
    /// BPP3D 内部错误 / BPP3D internal error
    #[derive(Clone, Debug)]
    pub struct Bpp3dInternalError {
        /// 错误详情 / Error detail
        pub detail: Option<String>,
    }
);

impl Display for Bpp3dInternalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(d) => write!(f, "BPP3D 内部错误: {}", d),
            None => write!(f, "BPP3D 内部错误。"),
        }
    }
}

impl ospf_rust_base::error::Error for Bpp3dInternalError {
    fn code(&self) -> ErrorCode {
        ErrorCode::ApplicationError
    }
    fn msg(&self) -> String {
        format!("{}", self)
    }
}

error_type!(
    /// BPP3D 参数验证错误 / BPP3D parameter validation error
    #[derive(Clone, Debug)]
    pub struct Bpp3dValidationError {
        /// 错误详情 / Error detail
        pub detail: Option<String>,
    }
);

impl Display for Bpp3dValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(d) => write!(f, "BPP3D 参数验证错误: {}", d),
            None => write!(f, "BPP3D 参数验证错误。"),
        }
    }
}

impl ospf_rust_base::error::Error for Bpp3dValidationError {
    fn code(&self) -> ErrorCode {
        ErrorCode::IllegalArgument
    }
    fn msg(&self) -> String {
        format!("{}", self)
    }
}

error_enum!(
    /// BPP3D 统一错误枚举 / BPP3D unified error enum
    #[derive(Clone)]
    pub enum Bpp3dError {
        Capability(Bpp3dCapabilityError),
        Solving(Bpp3dSolvingError),
        Internal(Bpp3dInternalError),
        Validation(Bpp3dValidationError),
    }
);
