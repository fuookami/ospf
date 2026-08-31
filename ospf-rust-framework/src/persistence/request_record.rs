//! 请求记录
//! Request record

/// 请求记录 PO。
/// Request record PO.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RequestRecordPo<M = ()> {
    pub request_id: String,
    pub created_at_ms: u128,
    pub trace_id: Option<String>,
    pub metadata: M,
}

impl<M> RequestRecordPo<M> {
    /// 创建请求记录。
    /// Create a request record.
    pub fn new(request_id: impl Into<String>, created_at_ms: u128, metadata: M) -> Self {
        Self {
            request_id: request_id.into(),
            created_at_ms,
            trace_id: None,
            metadata,
        }
    }

    /// 设置 trace id。
    /// Set trace id.
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }
}

/// 兼容旧命名。
/// Compatibility alias for the old name.
pub type RequestRecord = RequestRecordPo<()>;
