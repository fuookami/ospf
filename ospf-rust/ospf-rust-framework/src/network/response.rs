//! 通用响应类型（Kotlin 对齐）
//! Generic response types (Kotlin-aligned)

/// 通用响应结构体 / Generic response wrapper
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response<T> {
    /// 是否成功 / Whether the operation succeeded
    pub success: bool,
    /// 响应消息 / Response message
    pub message: Option<String>,
    /// 响应数据 / Response payload
    pub data: Option<T>,
}

impl<T> Response<T> {
    /// 构造成功响应 / Construct a success response with data
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            message: None,
            data: Some(data),
        }
    }

    /// 构造失败响应 / Construct a failure response with a message
    pub fn fail(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: Some(message.into()),
            data: None,
        }
    }
}
