//! 持久化响应
//! Persistence response

/// 响应 DTO / Response DTO
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ResponseDto<T = (), E = String> {
    /// 请求 ID / Request ID
    pub request_id: String,
    /// 是否成功 / Whether the request succeeded
    pub success: bool,
    /// 成功时的数据 / Data on success
    pub data: Option<T>,
    /// 失败时的错误 / Error on failure
    pub error: Option<E>,
    /// 耗时（毫秒） / Elapsed time in milliseconds
    pub elapsed_ms: u128,
}

impl<T, E> ResponseDto<T, E> {
    /// 创建成功响应 / Create a successful response
    pub fn success(request_id: impl Into<String>, data: T) -> Self {
        Self {
            request_id: request_id.into(),
            success: true,
            data: Some(data),
            error: None,
            elapsed_ms: 0,
        }
    }

    /// 创建失败响应 / Create a failed response
    pub fn failure(request_id: impl Into<String>, error: E) -> Self {
        Self {
            request_id: request_id.into(),
            success: false,
            data: None,
            error: Some(error),
            elapsed_ms: 0,
        }
    }

    /// 设置耗时 / Set elapsed time
    pub fn with_elapsed_ms(mut self, elapsed_ms: u128) -> Self {
        self.elapsed_ms = elapsed_ms;
        self
    }
}
