//! 甘特调度领域错误类型
//! Gantt Scheduling domain error types

use std::fmt::{Debug, Display, Formatter};
use ospf_rust_base::error::{ErrorCode, Error, ErrorPosition, WithErrorPosition};
use ospf_rust_base::error_type;
use ospf_rust_base::error_enum;

// ============================================================================
// 甘特调度领域错误
// ============================================================================

// 甘特调度能力错误 / Gantt scheduling capability error
error_type!(
    #[derive(Clone, Debug)]
    pub struct GanttSchedulingCapabilityError {
        /// 错误详情 / Error detail
        pub detail: Option<String>
    }
);

impl Display for GanttSchedulingCapabilityError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(d) => write!(f, "甘特调度能力错误: {}", d),
            None => write!(f, "甘特调度能力错误。"),
        }
    }
}

impl Error for GanttSchedulingCapabilityError {
    fn code(&self) -> ErrorCode { ErrorCode::IllegalArgument }
    fn msg(&self) -> String { format!("{}", self) }
}

// 甘特调度生命周期错误 / Gantt scheduling lifecycle error
error_type!(
    #[derive(Clone, Debug)]
    pub struct GanttSchedulingLifecycleError {
        /// 错误详情 / Error detail
        pub detail: Option<String>
    }
);

impl Display for GanttSchedulingLifecycleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(d) => write!(f, "甘特调度生命周期错误: {}", d),
            None => write!(f, "甘特调度生命周期错误。"),
        }
    }
}

impl Error for GanttSchedulingLifecycleError {
    fn code(&self) -> ErrorCode { ErrorCode::ApplicationError }
    fn msg(&self) -> String { format!("{}", self) }
}

// 甘特调度求解错误 / Gantt scheduling solving error
error_type!(
    #[derive(Clone, Debug)]
    pub struct GanttSchedulingSolvingError {
        /// 错误详情 / Error detail
        pub detail: Option<String>
    }
);

impl Display for GanttSchedulingSolvingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(d) => write!(f, "甘特调度求解错误: {}", d),
            None => write!(f, "甘特调度求解错误。"),
        }
    }
}

impl Error for GanttSchedulingSolvingError {
    fn code(&self) -> ErrorCode { ErrorCode::ApplicationFailed }
    fn msg(&self) -> String { format!("{}", self) }
}

// 甘特调度验证错误 / Gantt scheduling validation error
error_type!(
    #[derive(Clone, Debug)]
    pub struct GanttSchedulingValidationError {
        /// 错误详情 / Error detail
        pub detail: Option<String>
    }
);

impl Display for GanttSchedulingValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(d) => write!(f, "甘特调度验证错误: {}", d),
            None => write!(f, "甘特调度验证错误。"),
        }
    }
}

impl Error for GanttSchedulingValidationError {
    fn code(&self) -> ErrorCode { ErrorCode::IllegalArgument }
    fn msg(&self) -> String { format!("{}", self) }
}

// 甘特调度统一错误枚举 / Gantt scheduling unified error enum
error_enum!(
    #[derive(Clone)]
    pub enum GanttSchedulingError {
        Capability(GanttSchedulingCapabilityError),
        Lifecycle(GanttSchedulingLifecycleError),
        Solving(GanttSchedulingSolvingError),
        Validation(GanttSchedulingValidationError),
    }
);

// 域错误到 crate 级错误的转换
// Domain error to crate-level error conversion
impl From<GanttSchedulingError> for crate::GanttError {
    fn from(err: GanttSchedulingError) -> Self {
        match err {
            GanttSchedulingError::Capability(e) => crate::GanttError::Unsupported { message: e.msg() },
            GanttSchedulingError::Lifecycle(e) => crate::GanttError::Unsupported { message: e.msg() },
            GanttSchedulingError::Solving(e) => crate::GanttError::Calculation { message: e.msg() },
            GanttSchedulingError::Validation(e) => crate::GanttError::InvalidTimeRange { message: e.msg() },
        }
    }
}
