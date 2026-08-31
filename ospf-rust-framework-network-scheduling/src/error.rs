//! 网络调度错误 / Network scheduling errors.

use std::fmt::Display;

/// 网络调度框架的统一错误类型 / Unified error type for the network scheduling framework.
#[derive(Debug, thiserror::Error)]
pub enum NetworkSchedulingError {
    /// 输入校验失败 / Input validation failed.
    #[error("操作失败：{message} / Operation failed: {message}")]
    Validation {
        /// 具体错误消息 / Detailed error message.
        message: String,
    },

    /// 图或路线结构无效 / Graph or route structure is invalid.
    #[error("结构无效：{message} / Invalid structure: {message}")]
    Structure {
        /// 具体错误消息 / Detailed error message.
        message: String,
    },

    /// 单位或数值转换失败 / Unit or numeric conversion failed.
    #[error("转换失败：{message} / Conversion failed: {message}")]
    Conversion {
        /// 具体错误消息 / Detailed error message.
        message: String,
    },

    /// 模型注册失败 / Model registration failed.
    #[error("模型注册失败：{message} / Model registration failed: {message}")]
    Model {
        /// 具体错误消息 / Detailed error message.
        message: String,
    },

    /// 定价失败 / Pricing failed.
    #[error("定价失败：{message} / Pricing failed: {message}")]
    Pricing {
        /// 具体错误消息 / Detailed error message.
        message: String,
    },

    /// 求解器调用或证书失败 / Solver call or certificate failed.
    #[error("求解失败：{message} / Solve failed: {message}")]
    Solver {
        /// 具体错误消息 / Detailed error message.
        message: String,
    },

    /// 资源限制或外部取消 / Resource limit or external cancellation.
    #[error("求解被中断：{message} / Solve interrupted: {message}")]
    Interrupted {
        /// 具体错误消息 / Detailed error message.
        message: String,
    },

    /// 内部生命周期合同被破坏 / An internal lifecycle contract was violated.
    #[error("内部合同破坏：{message} / Internal contract violation: {message}")]
    Contract {
        /// 具体错误消息 / Detailed error message.
        message: String,
    },

    /// 依赖组件错误 / Error from a dependent component.
    #[error("依赖组件错误：{0} / Dependency error: {0}")]
    Dependency(String),
}

/// 网络调度结果别名 / Result alias for network scheduling operations.
pub type Result<T> = std::result::Result<T, NetworkSchedulingError>;

impl NetworkSchedulingError {
    /// 创建双语校验错误 / Create a bilingual validation error.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    /// 创建双语结构错误 / Create a bilingual structure error.
    pub fn structure(message: impl Into<String>) -> Self {
        Self::Structure {
            message: message.into(),
        }
    }

    /// 创建双语模型错误 / Create a bilingual model error.
    pub fn model(message: impl Into<String>) -> Self {
        Self::Model {
            message: message.into(),
        }
    }

    /// 创建定价错误 / Create a pricing error.
    pub fn pricing(message: impl Into<String>) -> Self {
        Self::Pricing {
            message: message.into(),
        }
    }

    /// 创建内部合同错误 / Create an internal-contract error.
    pub fn contract(message: impl Into<String>) -> Self {
        Self::Contract {
            message: message.into(),
        }
    }

    /// 创建求解器错误 / Create a solver error.
    pub fn solver(message: impl Into<String>) -> Self {
        Self::Solver {
            message: message.into(),
        }
    }

    /// 创建中断错误 / Create an interruption error.
    pub fn interrupted(message: impl Into<String>) -> Self {
        Self::Interrupted {
            message: message.into(),
        }
    }

    /// 将依赖错误转换为上下文错误 / Convert a dependency error with context.
    pub fn dependency<E: Display>(error: E) -> Self {
        Self::Dependency(error.to_string())
    }
}
