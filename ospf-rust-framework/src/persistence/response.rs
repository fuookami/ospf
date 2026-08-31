//! 持久化响应
//! Persistence response

/// 响应 DTO。
/// Response DTO.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ResponseDto<T = (), E = String> {
    pub request_id: String,
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<E>,
    pub elapsed_ms: u128,
}

impl<T, E> ResponseDto<T, E> {
    /// 创建成功响应。
    /// Create a successful response.
    pub fn success(request_id: impl Into<String>, data: T) -> Self {
        Self {
            request_id: request_id.into(),
            success: true,
            data: Some(data),
            error: None,
            elapsed_ms: 0,
        }
    }

    /// 创建失败响应。
    /// Create a failed response.
    pub fn failure(request_id: impl Into<String>, error: E) -> Self {
        Self {
            request_id: request_id.into(),
            success: false,
            data: None,
            error: Some(error),
            elapsed_ms: 0,
        }
    }

    /// 设置耗时。
    /// Set elapsed time.
    pub fn with_elapsed_ms(mut self, elapsed_ms: u128) -> Self {
        self.elapsed_ms = elapsed_ms;
        self
    }
}
