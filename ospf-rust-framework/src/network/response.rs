//! 通用响应类型（Kotlin 对齐）
//! Generic response types (Kotlin-aligned)

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response<T> {
    pub success: bool,
    pub message: Option<String>,
    pub data: Option<T>,
}

impl<T> Response<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            message: None,
            data: Some(data),
        }
    }

    pub fn fail(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: Some(message.into()),
            data: None,
        }
    }
}
