//! 持久化请求
//! Persistence request

/// 请求 DTO / Request DTO
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RequestDto<P = ()> {
    /// 请求 ID / Request ID
    pub request_id: String,
    /// 请求负载 / Request payload
    pub payload: P,
    /// 追踪 ID / Trace ID
    pub trace_id: Option<String>,
    /// 创建时间（毫秒） / Creation time in milliseconds
    pub created_at_ms: u128,
}

impl<P> RequestDto<P> {
    /// 创建请求 DTO / Create a request DTO
    pub fn new(request_id: impl Into<String>, payload: P) -> Self {
        Self {
            request_id: request_id.into(),
            payload,
            trace_id: None,
            created_at_ms: 0,
        }
    }

    /// 设置 trace id / Set trace id
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// 设置创建时间 / Set creation time
    pub fn with_created_at_ms(mut self, created_at_ms: u128) -> Self {
        self.created_at_ms = created_at_ms;
        self
    }
}

/// 兼容旧命名 / Compatibility alias for the old name
pub type PersistenceRequest = RequestDto<()>;
