//! 远程求解错误详情类型
//! Remote solver error detail types

use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};

use super::domain::{RemoteSolverError, RemoteSolverErrorCode};

/// 远程求解失败详情 / Remote solver failure detail
///
/// 比 `RemoteSolverError` 包含更多调试和可观测性上下文。
/// Extends `RemoteSolverError` with additional context for debugging and observability.
#[derive(Debug, Clone)]
pub struct RemoteSolverFailureDetail {
    /// 错误码 / Error code
    pub code: RemoteSolverErrorCode,
    /// 错误消息 / Error message
    pub message: String,
    /// 附加元数据 / Additional metadata
    pub metadata: HashMap<String, String>,
    /// HTTP 状态码（如适用）/ HTTP status code (if applicable)
    pub http_status: Option<u16>,
    /// 任务 ID（如适用）/ Task ID (if applicable)
    pub task_id: Option<String>,
    /// 切片 ID（如适用）/ Slice ID (if applicable)
    pub slice_id: Option<String>,
    /// 请求 ID（如适用）/ Request ID (if applicable)
    pub request_id: Option<String>,
}

impl RemoteSolverFailureDetail {
    /// 创建新的失败详情 / Create a new failure detail
    pub fn new(code: RemoteSolverErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            metadata: HashMap::new(),
            http_status: None,
            task_id: None,
            slice_id: None,
            request_id: None,
        }
    }

    /// 设置 HTTP 状态码 / Set HTTP status
    pub fn with_http_status(mut self, status: u16) -> Self {
        self.http_status = Some(status);
        self
    }

    /// 设置任务 ID / Set task ID
    pub fn with_task_id(mut self, task_id: impl Into<String>) -> Self {
        self.task_id = Some(task_id.into());
        self
    }

    /// 设置切片 ID / Set slice ID
    pub fn with_slice_id(mut self, slice_id: impl Into<String>) -> Self {
        self.slice_id = Some(slice_id.into());
        self
    }

    /// 设置请求 ID / Set request ID
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// 添加元数据 / Add metadata
    pub fn with_metadata(
        mut self,
        metadata: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        self.metadata = metadata
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        self
    }
}

impl Display for RemoteSolverFailureDetail {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)?;
        if let Some(task_id) = &self.task_id {
            write!(f, " (task: {})", task_id)?;
        }
        if let Some(slice_id) = &self.slice_id {
            write!(f, " (slice: {})", slice_id)?;
        }
        if let Some(request_id) = &self.request_id {
            write!(f, " (request: {})", request_id)?;
        }
        if let Some(status) = self.http_status {
            write!(f, " (HTTP {})", status)?;
        }
        Ok(())
    }
}

impl std::error::Error for RemoteSolverFailureDetail {}

// RemoteSolverFailureDetail → RemoteSolverError 转换（扩展字段写入 metadata 保全）
impl From<RemoteSolverFailureDetail> for RemoteSolverError {
    fn from(detail: RemoteSolverFailureDetail) -> Self {
        let mut metadata: std::collections::BTreeMap<String, String> =
            detail.metadata.into_iter().collect();
        if let Some(status) = detail.http_status {
            metadata.insert("http_status".to_string(), status.to_string());
        }
        if let Some(ref task_id) = detail.task_id {
            metadata.insert("task_id".to_string(), task_id.clone());
        }
        if let Some(ref slice_id) = detail.slice_id {
            metadata.insert("slice_id".to_string(), slice_id.clone());
        }
        if let Some(ref request_id) = detail.request_id {
            metadata.insert("request_id".to_string(), request_id.clone());
        }
        Self {
            code: detail.code,
            message: detail.message,
            metadata,
        }
    }
}

// RemoteSolverError → RemoteSolverFailureDetail 转换（从 metadata 还原 typed 字段）
impl From<RemoteSolverError> for RemoteSolverFailureDetail {
    fn from(err: RemoteSolverError) -> Self {
        let mut metadata: HashMap<String, String> = err.metadata.into_iter().collect();
        let http_status = metadata.remove("http_status").and_then(|s| s.parse().ok());
        let task_id = metadata.remove("task_id");
        let slice_id = metadata.remove("slice_id");
        let request_id = metadata.remove("request_id");
        Self {
            code: err.code,
            message: err.message,
            metadata,
            http_status,
            task_id,
            slice_id,
            request_id,
        }
    }
}
