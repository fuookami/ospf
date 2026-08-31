//! 响应记录
//! Response record

/// 响应记录 PO / Response record PO
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ResponseRecordPo<M = ()> {
    /// 请求 ID / Request ID
    pub request_id: String,
    /// 是否成功 / Whether the request succeeded
    pub success: bool,
    /// 创建时间（毫秒） / Creation time in milliseconds
    pub created_at_ms: u128,
    /// 耗时（毫秒） / Elapsed time in milliseconds
    pub elapsed_ms: u128,
    /// 错误消息 / Error message
    pub error_message: Option<String>,
    /// 元数据 / Metadata
    pub metadata: M,
}

impl<M> ResponseRecordPo<M> {
    /// 创建响应记录 / Create a response record
    pub fn new(
        request_id: impl Into<String>,
        success: bool,
        created_at_ms: u128,
        elapsed_ms: u128,
        metadata: M,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            success,
            created_at_ms,
            elapsed_ms,
            error_message: None,
            metadata,
        }
    }

    /// 设置错误消息 / Set error message
    pub fn with_error_message(mut self, error_message: impl Into<String>) -> Self {
        self.error_message = Some(error_message.into());
        self
    }
}
