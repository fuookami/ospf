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

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 全部远程错误码，用于遍历断言。
    /// Every remote error code, used for exhaustive assertions.
    const ALL_CODES: [RemoteSolverErrorCode; 18] = [
        RemoteSolverErrorCode::InvalidArgument,
        RemoteSolverErrorCode::UnsupportedProtocolVersion,
        RemoteSolverErrorCode::InvalidTaskStateTransition,
        RemoteSolverErrorCode::NoEligibleNodeAvailable,
        RemoteSolverErrorCode::NodeOffline,
        RemoteSolverErrorCode::SolverExecutionFailed,
        RemoteSolverErrorCode::CheckpointExportFailed,
        RemoteSolverErrorCode::CheckpointRestoreFailed,
        RemoteSolverErrorCode::EventPublishFailed,
        RemoteSolverErrorCode::StorageIoFailed,
        RemoteSolverErrorCode::TaskNotTerminalWithinMaxRounds,
        RemoteSolverErrorCode::NoCompatibleNodeAvailable,
        RemoteSolverErrorCode::TaskFailed,
        RemoteSolverErrorCode::TaskFailedHardTimeout,
        RemoteSolverErrorCode::TaskFailedSliceTimeout,
        RemoteSolverErrorCode::TaskFailedBudgetExceeded,
        RemoteSolverErrorCode::RemoteSolveNotCompletedWithinMaxRounds,
        RemoteSolverErrorCode::InternalError,
    ];

    #[test]
    fn new_starts_without_any_optional_context() {
        // 新建详情必须不带任何可选上下文：让"缺失"保持显式，而不是空串。
        // A fresh detail must carry no optional context, keeping "absent" explicit
        // rather than represented by empty strings.
        let detail = RemoteSolverFailureDetail::new(RemoteSolverErrorCode::NodeOffline, "no node");

        assert_eq!(detail.code, RemoteSolverErrorCode::NodeOffline);
        assert_eq!(detail.message, "no node");
        assert!(detail.metadata.is_empty());
        assert!(detail.http_status.is_none());
        assert!(detail.task_id.is_none());
        assert!(detail.slice_id.is_none());
        assert!(detail.request_id.is_none());
    }

    #[test]
    fn builders_set_every_optional_field() {
        // 构建器必须逐个写入对应字段，不得互相覆盖。
        // Each builder must populate its own field without clobbering the others.
        let detail = RemoteSolverFailureDetail::new(RemoteSolverErrorCode::TaskFailed, "failed")
            .with_http_status(503)
            .with_task_id("task-1")
            .with_slice_id("slice-2")
            .with_request_id("req-3")
            .with_metadata([("region", "cn-north"), ("attempt", "2")]);

        assert_eq!(detail.http_status, Some(503));
        assert_eq!(detail.task_id.as_deref(), Some("task-1"));
        assert_eq!(detail.slice_id.as_deref(), Some("slice-2"));
        assert_eq!(detail.request_id.as_deref(), Some("req-3"));
        assert_eq!(detail.metadata.get("region").map(String::as_str), Some("cn-north"));
        assert_eq!(detail.metadata.get("attempt").map(String::as_str), Some("2"));
    }

    #[test]
    fn display_reports_code_message_and_present_context_only() {
        // Display 只应渲染已存在的上下文；缺失项不得留下空括号噪音。
        // Display must render only present context; absent fields must not leave
        // empty-parenthesis noise behind.
        let bare = RemoteSolverFailureDetail::new(RemoteSolverErrorCode::NodeOffline, "no node");
        let rendered = bare.to_string();

        assert!(rendered.starts_with("[NodeOffline] no node"), "got {rendered}");
        assert!(!rendered.contains("task:"), "缺失 task 不得渲染");
        assert!(!rendered.contains("slice:"), "缺失 slice 不得渲染");
        assert!(!rendered.contains("request:"), "缺失 request 不得渲染");
        assert!(!rendered.contains("HTTP"), "缺失状态码不得渲染");

        let full = bare
            .clone()
            .with_task_id("task-1")
            .with_slice_id("slice-2")
            .with_request_id("req-3")
            .with_http_status(503);
        let rendered = full.to_string();

        assert!(rendered.contains("(task: task-1)"), "got {rendered}");
        assert!(rendered.contains("(slice: slice-2)"), "got {rendered}");
        assert!(rendered.contains("(request: req-3)"), "got {rendered}");
        assert!(rendered.contains("(HTTP 503)"), "got {rendered}");
    }

    #[test]
    fn detail_to_error_promotes_context_into_metadata() {
        // 详情转错误时，typed 上下文必须落入 metadata，否则错误链会丢掉定位信息。
        // When a detail becomes an error, typed context must land in metadata;
        // otherwise the error chain loses the information needed to locate the failure.
        let detail = RemoteSolverFailureDetail::new(RemoteSolverErrorCode::TaskFailed, "failed")
            .with_http_status(503)
            .with_task_id("task-1")
            .with_slice_id("slice-2")
            .with_request_id("req-3")
            .with_metadata([("region", "cn-north")]);

        let error: RemoteSolverError = detail.into();

        assert_eq!(error.code, RemoteSolverErrorCode::TaskFailed);
        assert_eq!(error.message, "failed");
        assert_eq!(error.metadata.get("http_status").map(String::as_str), Some("503"));
        assert_eq!(error.metadata.get("task_id").map(String::as_str), Some("task-1"));
        assert_eq!(error.metadata.get("slice_id").map(String::as_str), Some("slice-2"));
        assert_eq!(error.metadata.get("request_id").map(String::as_str), Some("req-3"));
        assert_eq!(error.metadata.get("region").map(String::as_str), Some("cn-north"));
    }

    #[test]
    fn detail_round_trips_through_error_without_loss() {
        // 往返必须无损：详情 → 错误 → 详情 后每个字段原样恢复。
        // The round trip must be lossless: detail → error → detail restores every field.
        let original = RemoteSolverFailureDetail::new(
            RemoteSolverErrorCode::CheckpointRestoreFailed,
            "restore failed",
        )
        .with_http_status(500)
        .with_task_id("task-9")
        .with_slice_id("slice-9")
        .with_request_id("req-9")
        .with_metadata([("solver", "gurobi"), ("attempt", "3")]);

        let error: RemoteSolverError = original.clone().into();
        let restored: RemoteSolverFailureDetail = error.into();

        assert_eq!(restored.code, original.code);
        assert_eq!(restored.message, original.message);
        assert_eq!(restored.http_status, original.http_status);
        assert_eq!(restored.task_id, original.task_id);
        assert_eq!(restored.slice_id, original.slice_id);
        assert_eq!(restored.request_id, original.request_id);
        assert_eq!(restored.metadata, original.metadata, "业务 metadata 必须原样保留");
        assert_eq!(restored.to_string(), original.to_string());
    }

    #[test]
    fn error_to_detail_extracts_context_out_of_metadata() {
        // 反向转换必须把 typed 上下文从 metadata 中摘出，避免同一信息重复存在。
        // The reverse conversion must pull typed context out of metadata so the same
        // information does not exist twice.
        let error = RemoteSolverError {
            code: RemoteSolverErrorCode::EventPublishFailed,
            message: "publish failed".to_string(),
            metadata: [
                ("http_status".to_string(), "504".to_string()),
                ("task_id".to_string(), "task-7".to_string()),
                ("slice_id".to_string(), "slice-7".to_string()),
                ("request_id".to_string(), "req-7".to_string()),
                ("region".to_string(), "cn-north".to_string()),
            ]
            .into_iter()
            .collect(),
        };

        let detail: RemoteSolverFailureDetail = error.into();

        assert_eq!(detail.http_status, Some(504));
        assert_eq!(detail.task_id.as_deref(), Some("task-7"));
        assert_eq!(detail.slice_id.as_deref(), Some("slice-7"));
        assert_eq!(detail.request_id.as_deref(), Some("req-7"));
        assert_eq!(detail.metadata.len(), 1, "typed 上下文必须已从 metadata 移出");
        assert_eq!(detail.metadata.get("region").map(String::as_str), Some("cn-north"));
    }

    #[test]
    fn a_non_numeric_status_in_metadata_is_dropped_rather_than_crashing() {
        // 记录当前行为：无法解析为 u16 的 http_status 会被静默丢弃。
        // 这是有意的容错（远端可能回填非法值），但意味着该字段不可恢复，
        // 调用方不得假设 metadata 里的 http_status 一定能往返。
        //
        // Documents current behavior: an http_status that does not parse as u16 is
        // silently dropped. The tolerance is intentional (a peer may emit a bad value),
        // but the field is then unrecoverable, so callers must not assume it round-trips.
        let error = RemoteSolverError {
            code: RemoteSolverErrorCode::InternalError,
            message: "boom".to_string(),
            metadata: [("http_status".to_string(), "not-a-number".to_string())]
                .into_iter()
                .collect(),
        };

        let detail: RemoteSolverFailureDetail = error.into();

        assert!(detail.http_status.is_none());
        assert!(
            detail.metadata.is_empty(),
            "无法解析的状态码既未进入 typed 字段，也不应残留在 metadata 中"
        );
    }

    #[test]
    fn error_to_detail_leaves_unrelated_metadata_untouched() {
        // 不含 typed 键时，metadata 必须逐条原样保留。
        // Without typed keys, metadata must be preserved entry by entry.
        let error = RemoteSolverError {
            code: RemoteSolverErrorCode::StorageIoFailed,
            message: "io".to_string(),
            metadata: [
                ("bucket".to_string(), "b1".to_string()),
                ("key".to_string(), "k1".to_string()),
            ]
            .into_iter()
            .collect(),
        };

        let detail: RemoteSolverFailureDetail = error.into();

        assert_eq!(detail.http_status, None);
        assert_eq!(detail.task_id, None);
        assert_eq!(detail.metadata.len(), 2);
        assert_eq!(detail.metadata.get("bucket").map(String::as_str), Some("b1"));
        assert_eq!(detail.metadata.get("key").map(String::as_str), Some("k1"));
    }

    #[test]
    fn every_error_code_survives_both_conversion_directions() {
        // 穷举全部错误码，确保没有任何一个在转换中被替换或丢失。
        // Exhaustively walk every error code to ensure none is substituted or lost.
        for code in ALL_CODES {
            let detail = RemoteSolverFailureDetail::new(code, "message").with_task_id("t");
            let error: RemoteSolverError = detail.clone().into();
            assert_eq!(error.code, code, "detail→error 必须保留错误码");

            let restored: RemoteSolverFailureDetail = error.into();
            assert_eq!(restored.code, code, "error→detail 必须保留错误码");
            assert_eq!(restored.message, detail.message);
            assert_eq!(restored.task_id, detail.task_id);
            // Display 必须可渲染且非空，保证日志链路可用。
            assert!(!restored.to_string().is_empty());
        }
    }

    #[test]
    fn it_is_usable_as_a_std_error() {
        // 必须能作为 std::error::Error 使用，否则无法接入 ? 与错误链。
        // It must be usable as std::error::Error so it can flow through `?` and chains.
        let detail = RemoteSolverFailureDetail::new(RemoteSolverErrorCode::TaskFailed, "failed");
        let erased: &dyn std::error::Error = &detail;

        assert_eq!(erased.to_string(), detail.to_string());
        assert!(erased.source().is_none(), "当前没有下层错误来源");
    }
}
