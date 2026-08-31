//! 远程求解 HTTP 任务客户端
//! Remote solver HTTP task client

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use async_trait::async_trait;
use serde::{Deserialize, Deserializer, Serialize};
use super::domain::{

    BudgetScopeId, ExecutionHandle, HandleId, NodeId, ObjectPath, ObjectRef, OperatorId,
    ReasonCode, RemoteSolverError, RemoteSolverErrorCode, RemoteSolverResult, RequestId,
    SerializedSolution, SliceId, SliceResult, SolvePayload, SolveResult, TaskComplexity, TaskId,
    TaskStatus, TenantId, TimeSensitivity, TraceId, epoch_millis, option_epoch_millis,
};
use super::port::{ObjectStoragePort, SolverExecutionPort};

/// HTTP 传输配置。
/// HTTP transport configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RemoteSolverHttpTransportConfig {
    pub connect_timeout: Option<Duration>,
    pub request_timeout: Option<Duration>,
    pub headers: BTreeMap<String, String>,
    pub properties: BTreeMap<String, String>,
}

/// 远程求解 HTTP 请求。
/// Remote solver HTTP request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteSolverHttpRequest {
    pub method: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub body: Option<String>,
}

/// 远程求解 HTTP 响应。
/// Remote solver HTTP response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteSolverHttpResponse {
    pub status_code: u16,
    pub body: String,
}

/// 远程求解 HTTP 传输。
/// Remote solver HTTP transport.
#[async_trait]
pub trait RemoteSolverHttpTransport: Send + Sync {
    /// 发送 HTTP 请求。
    /// Send an HTTP request.
    async fn send(
        &self,
        request: RemoteSolverHttpRequest,
    ) -> RemoteSolverResult<RemoteSolverHttpResponse>;
}

/// reqwest HTTP 传输。
/// reqwest HTTP transport.
#[cfg(feature = "remote-solver-http-reqwest")]
#[derive(Debug, Clone)]
pub struct ReqwestRemoteSolverHttpTransport {
    client: reqwest::Client,
    config: RemoteSolverHttpTransportConfig,
}

#[cfg(feature = "remote-solver-http-reqwest")]
impl ReqwestRemoteSolverHttpTransport {
    /// 创建 reqwest HTTP 传输。
    /// Create a reqwest HTTP transport.
    pub fn new() -> RemoteSolverResult<Self> {
        Self::with_config(RemoteSolverHttpTransportConfig::default())
    }

    /// 使用配置创建 reqwest HTTP 传输。
    /// Create a reqwest HTTP transport with configuration.
    pub fn with_config(config: RemoteSolverHttpTransportConfig) -> RemoteSolverResult<Self> {
        let mut builder = reqwest::Client::builder();
        if let Some(connect_timeout) = config.connect_timeout {
            builder = builder.connect_timeout(connect_timeout);
        }
        let client = builder.build().map_err(|err| {
            RemoteSolverError::internal(format!("failed to build reqwest client: {err}"))
        })?;
        Ok(Self { client, config })
    }
}

#[cfg(feature = "remote-solver-http-reqwest")]
#[async_trait]
impl RemoteSolverHttpTransport for ReqwestRemoteSolverHttpTransport {
    async fn send(
        &self,
        request: RemoteSolverHttpRequest,
    ) -> RemoteSolverResult<RemoteSolverHttpResponse> {
        let method = reqwest::Method::from_bytes(request.method.as_bytes()).map_err(|err| {
            RemoteSolverError::invalid_argument(format!("invalid HTTP method: {err}"))
        })?;
        let mut builder = self.client.request(method, request.url);
        for (name, value) in &self.config.headers {
            builder = builder.header(name, value);
        }
        for (name, value) in &request.headers {
            builder = builder.header(name, value);
        }
        if let Some(timeout) = self.config.request_timeout {
            builder = builder.timeout(timeout);
        }
        if let Some(body) = request.body {
            builder = builder.body(body);
        }
        let response = builder.send().await.map_err(|err| {
            RemoteSolverError::internal(format!("remote solver HTTP request failed: {err}"))
        })?;
        let status_code = response.status().as_u16();
        let body = response.text().await.map_err(|err| {
            RemoteSolverError::internal(format!("failed to read remote solver response: {err}"))
        })?;
        Ok(RemoteSolverHttpResponse { status_code, body })
    }
}

/// 远程求解 HTTP 客户端。
/// Remote solver HTTP client.
#[derive(Clone)]
pub struct RemoteSolverHttpClient<T> {
    base_url: String,
    transport: T,
    tenant_id: Option<TenantId>,
    trace_id_provider: Arc<dyn Fn() -> Option<TraceId> + Send + Sync>,
}

/// HTTP 任务 API 到执行端口的适配器。
/// Adapter from HTTP task API to execution port.
///
/// 当前任务 API 由服务端分配规范 task id。这里把 `SolverExecutionPort::start`
/// 的 `task_id` 作为 `request_id` 和 payload 路径关联键，返回 handle 使用服务端 task id。
/// The current task API assigns the canonical task id on the server side. This adapter
/// uses `SolverExecutionPort::start`'s `task_id` as request id and payload path key,
/// while the returned handle keeps the server task id.
///
/// 当前 `/resume` API 恢复任务自身最新检查点，未提供显式 checkpoint 参数。
/// The current `/resume` API resumes the task's latest checkpoint and has no explicit
/// checkpoint request parameter.
#[derive(Debug, Clone)]
pub struct RemoteSolverHttpExecutionPort<T, S> {
    http_client: RemoteSolverHttpClient<T>,
    object_storage: S,
    payload_prefix: String,
    poll_interval: Duration,
}

impl<T, S> RemoteSolverHttpExecutionPort<T, S> {
    /// 创建 HTTP 执行端口适配器。
    /// Create an HTTP execution-port adapter.
    pub fn new(http_client: RemoteSolverHttpClient<T>, object_storage: S) -> Self {
        Self {
            http_client,
            object_storage,
            payload_prefix: "remote-solver/payloads".to_string(),
            poll_interval: Duration::from_millis(200),
        }
    }

    /// 设置 payload 对象路径前缀。
    /// Set payload object-path prefix.
    pub fn with_payload_prefix(mut self, payload_prefix: impl Into<String>) -> Self {
        self.payload_prefix = payload_prefix.into().trim_matches('/').to_string();
        self
    }

    /// 设置轮询间隔。
    /// Set poll interval.
    pub fn with_poll_interval(mut self, poll_interval: Duration) -> Self {
        self.poll_interval = poll_interval;
        self
    }

    /// 获取 HTTP 客户端。
    /// Get HTTP client.
    pub fn http_client(&self) -> &RemoteSolverHttpClient<T> {
        &self.http_client
    }

    /// 获取对象存储。
    /// Get object storage.
    pub fn object_storage(&self) -> &S {
        &self.object_storage
    }
}

#[async_trait]
impl<T, S> SolverExecutionPort for RemoteSolverHttpExecutionPort<T, S>
where
    T: RemoteSolverHttpTransport,
    S: ObjectStoragePort,
{
    async fn start(
        &self,
        payload: &SolvePayload,
        task_id: &TaskId,
        slice_id: &SliceId,
        node_id: &NodeId,
        tenant_id: &TenantId,
    ) -> RemoteSolverResult<ExecutionHandle> {
        // 调用方 task_id 在当前 HTTP API 中作为幂等/追踪 request_id。
        // The caller task_id is used as idempotency/tracing request_id for the current HTTP API.
        let payload_ref = self
            .store_payload(payload, task_id, slice_id, tenant_id)
            .await?;
        let response = self
            .http_client
            .submit(&RemoteTaskSubmitRequest {
                payload_ref: payload_ref.path,
                request_id: Some(RequestId::of(task_id.value())?),
                tenant_id: Some(tenant_id.clone()),
                complexity: None,
                time_sensitivity: None,
                priority: None,
                budget_scope: None,
                budget_limit: None,
                deadline: None,
            })
            .await?;
        Ok(Self::handle(
            response.task_id,
            slice_id.clone(),
            node_id.clone(),
        )?)
    }

    async fn resume(
        &self,
        _payload: &SolvePayload,
        _checkpoint: &ObjectRef,
        task_id: &TaskId,
        slice_id: &SliceId,
        node_id: &NodeId,
        _tenant_id: &TenantId,
    ) -> RemoteSolverResult<ExecutionHandle> {
        // 当前服务端按任务恢复最新 checkpoint，trait 参数保留用于非 HTTP port。
        // The current server resumes the latest task checkpoint; trait parameters remain for non-HTTP ports.
        let response = self
            .http_client
            .resume(task_id, &RemoteTaskResumeRequest::default())
            .await?;
        Ok(Self::handle(
            response.task_id,
            slice_id.clone(),
            node_id.clone(),
        )?)
    }

    async fn await_slice_end(
        &self,
        handle: &ExecutionHandle,
        quantum: Duration,
    ) -> RemoteSolverResult<SliceResult> {
        let started = Instant::now();
        loop {
            let Some(view) = self.http_client.get(&handle.task_id).await? else {
                return Err(RemoteSolverError::new(
                    RemoteSolverErrorCode::TaskFailed,
                    format!("Remote task {} was not found.", handle.task_id),
                ));
            };

            if is_terminal_status(view.status) {
                return Ok(slice_result_from_view(
                    &view,
                    &handle.slice_id,
                    true,
                    started.elapsed(),
                ));
            }

            if started.elapsed() >= quantum {
                return Ok(slice_result_from_view(
                    &view,
                    &handle.slice_id,
                    false,
                    started.elapsed(),
                ));
            }

            let remaining = quantum.saturating_sub(started.elapsed());
            tokio::time::sleep(self.poll_interval.min(remaining)).await;
        }
    }

    async fn export_checkpoint(
        &self,
        handle: &ExecutionHandle,
    ) -> RemoteSolverResult<Option<ObjectRef>> {
        Ok(self
            .http_client
            .get(&handle.task_id)
            .await?
            .and_then(|view| view.latest_checkpoint_ref))
    }

    async fn fetch_final_result(
        &self,
        handle: &ExecutionHandle,
    ) -> RemoteSolverResult<Option<SolveResult>> {
        let Some(view) = self.http_client.get(&handle.task_id).await? else {
            return Ok(None);
        };
        if !is_terminal_status(view.status) {
            return Ok(None);
        }

        if let Some(result_ref) = &view.latest_result_ref {
            if let Some(bytes) = self.object_storage.get(result_ref).await? {
                let solution: SerializedSolution =
                    serde_json::from_slice(&bytes).map_err(|err| {
                        RemoteSolverError::invalid_argument(format!(
                            "Failed to decode remote result object '{}': {}",
                            result_ref.path, err
                        ))
                    })?;
                return Ok(Some(solve_result_from_solution(
                    solution,
                    view.latest_checkpoint_ref,
                    Some(result_ref.clone()),
                )));
            }
        }

        Ok(Some(solve_result_from_view(&view)))
    }

    async fn stop(&self, handle: &ExecutionHandle) -> RemoteSolverResult<bool> {
        let _ = self
            .http_client
            .stop(&handle.task_id, &RemoteTaskStopRequest::default())
            .await?;
        Ok(true)
    }
}

impl<T, S> RemoteSolverHttpExecutionPort<T, S>
where
    S: ObjectStoragePort,
{
    async fn store_payload(
        &self,
        payload: &SolvePayload,
        task_id: &TaskId,
        slice_id: &SliceId,
        tenant_id: &TenantId,
    ) -> RemoteSolverResult<ObjectRef> {
        let path = ObjectPath::of(format!(
            "{}/{}/{}/{}.json",
            self.payload_prefix,
            tenant_id.value(),
            task_id.value(),
            slice_id.value()
        ))?;
        let bytes = serde_json::to_vec(payload).map_err(|err| {
            RemoteSolverError::internal(format!("failed to serialize solve payload: {err}"))
        })?;
        let metadata = BTreeMap::from([
            ("contentType".to_string(), "application/json".to_string()),
            ("kind".to_string(), "solvePayload".to_string()),
        ]);
        self.object_storage.put(&path, &bytes, &metadata).await
    }

    fn handle(
        task_id: TaskId,
        slice_id: SliceId,
        node_id: NodeId,
    ) -> RemoteSolverResult<ExecutionHandle> {
        Ok(ExecutionHandle {
            handle_id: HandleId::of(format!("http-{}", task_id.value()))?,
            task_id,
            slice_id,
            node_id,
            started_at: SystemTime::now(),
        })
    }
}

impl<T> std::fmt::Debug for RemoteSolverHttpClient<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteSolverHttpClient")
            .field("base_url", &self.base_url)
            .field("transport", &self.transport)
            .field("tenant_id", &self.tenant_id)
            .finish_non_exhaustive()
    }
}

impl<T> RemoteSolverHttpClient<T> {
    /// 创建 HTTP 客户端。
    /// Create an HTTP client.
    pub fn new(base_url: impl Into<String>, transport: T) -> RemoteSolverResult<Self> {
        let base_url = base_url.into().trim().trim_end_matches('/').to_string();
        if base_url.is_empty() {
            return Err(RemoteSolverError::invalid_argument(
                "base_url must not be blank.",
            ));
        }
        Ok(Self {
            base_url,
            transport,
            tenant_id: None,
            trace_id_provider: Arc::new(|| None),
        })
    }

    /// 设置默认租户 ID。
    /// Set default tenant ID.
    pub fn with_tenant_id(mut self, tenant_id: TenantId) -> Self {
        self.tenant_id = Some(tenant_id);
        self
    }

    /// 设置 trace ID provider。
    /// Set trace ID provider.
    pub fn with_trace_id_provider(
        mut self,
        trace_id_provider: impl Fn() -> Option<TraceId> + Send + Sync + 'static,
    ) -> Self {
        self.trace_id_provider = Arc::new(trace_id_provider);
        self
    }

    /// 获取 transport。
    /// Get transport.
    pub fn transport(&self) -> &T {
        &self.transport
    }
}

impl<T> RemoteSolverHttpClient<T>
where
    T: RemoteSolverHttpTransport,
{
    /// 提交任务。
    /// Submit task.
    pub async fn submit(
        &self,
        request: &RemoteTaskSubmitRequest,
    ) -> RemoteSolverResult<RemoteTaskSubmitResponse> {
        let response = self
            .transport
            .send(self.request(
                "POST",
                "/api/v1/tasks",
                Some(serde_json::to_string(request).map_err(|err| {
                    RemoteSolverError::internal(format!("failed to encode submit request: {err}"))
                })?),
            ))
            .await?;
        self.decode_envelope::<RemoteTaskSubmitResponse>(&response)
    }

    /// 查询任务。
    /// Get task.
    pub async fn get(&self, task_id: &TaskId) -> RemoteSolverResult<Option<RemoteTaskView>> {
        let response = self
            .transport
            .send(self.request("GET", &format!("/api/v1/tasks/{}", task_id.value()), None))
            .await?;
        if response.status_code == 404 {
            return Ok(None);
        }
        self.decode_envelope::<RemoteTaskView>(&response).map(Some)
    }

    /// 停止任务。
    /// Stop task.
    pub async fn stop(
        &self,
        task_id: &TaskId,
        request: &RemoteTaskStopRequest,
    ) -> RemoteSolverResult<RemoteTaskAction> {
        let response = self
            .transport
            .send(self.request(
                "POST",
                &format!("/api/v1/tasks/{}/stop", task_id.value()),
                Some(serde_json::to_string(request).map_err(|err| {
                    RemoteSolverError::internal(format!("failed to encode stop request: {err}"))
                })?),
            ))
            .await?;
        self.decode_envelope::<RemoteTaskAction>(&response)
    }

    /// 恢复任务。
    /// Resume task.
    pub async fn resume(
        &self,
        task_id: &TaskId,
        request: &RemoteTaskResumeRequest,
    ) -> RemoteSolverResult<RemoteTaskAction> {
        let response = self
            .transport
            .send(self.request(
                "POST",
                &format!("/api/v1/tasks/{}/resume", task_id.value()),
                Some(serde_json::to_string(request).map_err(|err| {
                    RemoteSolverError::internal(format!("failed to encode resume request: {err}"))
                })?),
            ))
            .await?;
        self.decode_envelope::<RemoteTaskAction>(&response)
    }

    fn request(&self, method: &str, path: &str, body: Option<String>) -> RemoteSolverHttpRequest {
        let mut headers = BTreeMap::from([("Accept".to_string(), "application/json".to_string())]);
        if body.is_some() {
            headers.insert("Content-Type".to_string(), "application/json".to_string());
        }
        if let Some(tenant_id) = &self.tenant_id {
            headers.insert("X-Tenant-Id".to_string(), tenant_id.value().to_string());
        }
        if let Some(trace_id) = (self.trace_id_provider)() {
            headers.insert("X-Trace-Id".to_string(), trace_id.value().to_string());
        }
        RemoteSolverHttpRequest {
            method: method.to_string(),
            url: format!("{}{}", self.base_url, path),
            headers,
            body,
        }
    }

    fn decode_envelope<D>(&self, response: &RemoteSolverHttpResponse) -> RemoteSolverResult<D>
    where
        D: for<'de> Deserialize<'de>,
    {
        if !(200..=299).contains(&response.status_code) {
            return Err(self.decode_error(response));
        }
        let envelope: ApiEnvelope<D> = serde_json::from_str(&response.body).map_err(|err| {
            RemoteSolverError::internal(format!("failed to decode response envelope: {err}"))
        })?;
        if envelope.code != "OK" {
            return Err(RemoteSolverError::new(
                envelope.code.to_remote_error_code(),
                envelope.message,
            )
            .with_metadata(
                envelope
                    .trace_id
                    .into_iter()
                    .map(|trace_id| ("traceId", trace_id)),
            ));
        }
        envelope.data.ok_or_else(|| {
            RemoteSolverError::internal("Remote solver response data is null.").with_metadata(
                envelope
                    .trace_id
                    .into_iter()
                    .map(|trace_id| ("traceId", trace_id)),
            )
        })
    }

    fn decode_error(&self, response: &RemoteSolverHttpResponse) -> RemoteSolverError {
        let envelope = serde_json::from_str::<ApiEnvelope<serde_json::Value>>(&response.body).ok();
        let mut metadata =
            BTreeMap::from([("status".to_string(), response.status_code.to_string())]);
        if let Some(trace_id) = envelope
            .as_ref()
            .and_then(|envelope| envelope.trace_id.clone())
        {
            metadata.insert("traceId".to_string(), trace_id);
        }
        if !response.body.trim().is_empty() {
            metadata.insert("body".to_string(), response.body.clone());
        }
        let code = envelope
            .as_ref()
            .map(|envelope| envelope.code.to_remote_error_code())
            .unwrap_or(RemoteSolverErrorCode::InternalError);
        let message = envelope
            .map(|envelope| envelope.message)
            .unwrap_or_else(|| {
                format!(
                    "Remote solver HTTP request failed with status {}.",
                    response.status_code
                )
            });
        RemoteSolverError::new(code, message).with_metadata(metadata)
    }
}

/// 远程任务提交请求。
/// Remote task submit request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteTaskSubmitRequest {
    pub payload_ref: ObjectPath,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<RequestId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<TenantId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub complexity: Option<TaskComplexity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_sensitivity: Option<TimeSensitivity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_scope: Option<BudgetScopeId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_limit: Option<f64>,
    #[serde(
        rename = "deadlineEpochMs",
        default,
        with = "option_epoch_millis",
        skip_serializing_if = "Option::is_none"
    )]
    pub deadline: Option<SystemTime>,
}

/// 远程任务提交响应。
/// Remote task submit response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteTaskSubmitResponse {
    pub task_id: TaskId,
    pub accepted: bool,
    pub status: TaskStatus,
    pub message: String,
}

/// 远程任务视图。
/// Remote task view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteTaskView {
    pub task_id: TaskId,
    pub tenant_id: TenantId,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_node_id: Option<NodeId>,
    #[serde(
        alias = "latestCheckpointPath",
        default,
        deserialize_with = "deserialize_optional_object_ref",
        skip_serializing_if = "Option::is_none"
    )]
    pub latest_checkpoint_ref: Option<ObjectRef>,
    #[serde(
        alias = "latestResultPath",
        default,
        deserialize_with = "deserialize_optional_object_ref",
        skip_serializing_if = "Option::is_none"
    )]
    pub latest_result_ref: Option<ObjectRef>,
    pub consumed_cost: f64,
}

/// 远程任务操作响应。
/// Remote task action response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteTaskAction {
    pub task_id: TaskId,
    pub status: TaskStatus,
}

/// 远程任务停止请求。
/// Remote task stop request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RemoteTaskStopRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<ReasonCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<OperatorId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<super::domain::OperationSource>,
}

/// 远程任务恢复请求。
/// Remote task resume request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RemoteTaskResumeRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<OperatorId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<super::domain::OperationSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<ReasonCode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiEnvelope<T> {
    code: String,
    message: String,
    trace_id: Option<String>,
    data: Option<T>,
}

trait RemoteErrorCodeName {
    fn to_remote_error_code(&self) -> RemoteSolverErrorCode;
}

impl RemoteErrorCodeName for str {
    fn to_remote_error_code(&self) -> RemoteSolverErrorCode {
        match self {
            "INVALID_ARGUMENT" => RemoteSolverErrorCode::InvalidArgument,
            "INVALID_TASK_STATE_TRANSITION" => RemoteSolverErrorCode::InvalidTaskStateTransition,
            "NO_ELIGIBLE_NODE_AVAILABLE" => RemoteSolverErrorCode::NoEligibleNodeAvailable,
            "NODE_OFFLINE" => RemoteSolverErrorCode::NodeOffline,
            "SOLVER_EXECUTION_FAILED" => RemoteSolverErrorCode::SolverExecutionFailed,
            "CHECKPOINT_EXPORT_FAILED" => RemoteSolverErrorCode::CheckpointExportFailed,
            "EVENT_PUBLISH_FAILED" => RemoteSolverErrorCode::EventPublishFailed,
            "STORAGE_IO_FAILED" => RemoteSolverErrorCode::StorageIoFailed,
            "TASK_NOT_TERMINAL_WITHIN_MAX_ROUNDS" => {
                RemoteSolverErrorCode::TaskNotTerminalWithinMaxRounds
            }
            "NO_COMPATIBLE_NODE_AVAILABLE" => RemoteSolverErrorCode::NoCompatibleNodeAvailable,
            "TASK_FAILED" => RemoteSolverErrorCode::TaskFailed,
            "TASK_FAILED_HARD_TIMEOUT" => RemoteSolverErrorCode::TaskFailedHardTimeout,
            "TASK_FAILED_SLICE_TIMEOUT" => RemoteSolverErrorCode::TaskFailedSliceTimeout,
            "TASK_FAILED_BUDGET_EXCEEDED" => RemoteSolverErrorCode::TaskFailedBudgetExceeded,
            "REMOTE_SOLVE_NOT_COMPLETED_WITHIN_MAX_ROUNDS" => {
                RemoteSolverErrorCode::RemoteSolveNotCompletedWithinMaxRounds
            }
            _ => RemoteSolverErrorCode::InternalError,
        }
    }
}

fn deserialize_optional_object_ref<'de, D>(deserializer: D) -> Result<Option<ObjectRef>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    if let Some(path) = value.as_str() {
        return ObjectRef::of(path)
            .map(Some)
            .map_err(serde::de::Error::custom);
    }
    serde_json::from_value(value)
        .map(Some)
        .map_err(serde::de::Error::custom)
}

fn is_terminal_status(status: TaskStatus) -> bool {
    matches!(
        status,
        TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Stopped
    )
}

fn slice_result_from_view(
    view: &RemoteTaskView,
    slice_id: &SliceId,
    completed: bool,
    elapsed: Duration,
) -> SliceResult {
    SliceResult {
        slice_id: slice_id.clone(),
        completed,
        feasible: view.status == TaskStatus::Completed,
        objective_value: None,
        gap: None,
        elapsed,
        message: Some(format!("Remote task status is {:?}.", view.status)),
    }
}

fn solve_result_from_view(view: &RemoteTaskView) -> SolveResult {
    SolveResult {
        feasible: view.status == TaskStatus::Completed,
        optimal: view.status == TaskStatus::Completed,
        objective_value: None,
        gap: None,
        elapsed: Duration::ZERO,
        checkpoint_ref: view.latest_checkpoint_ref.clone(),
        result_ref: view.latest_result_ref.clone(),
        message: Some(format!("Remote task status is {:?}.", view.status)),
        extension: BTreeMap::new(),
    }
}

fn solve_result_from_solution(
    solution: SerializedSolution,
    checkpoint_ref: Option<ObjectRef>,
    result_ref: Option<ObjectRef>,
) -> SolveResult {
    SolveResult {
        feasible: solution.feasible,
        optimal: solution.optimal,
        objective_value: solution.objective_value,
        gap: solution.gap,
        elapsed: solution.elapsed,
        checkpoint_ref,
        result_ref,
        message: solution.message,
        extension: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use super::*;

    #[derive(Debug)]
    struct FakeTransport {
        requests: Arc<Mutex<Vec<RemoteSolverHttpRequest>>>,
        responses: Arc<Mutex<Vec<RemoteSolverHttpResponse>>>,
    }

    impl FakeTransport {
        fn new(responses: Vec<RemoteSolverHttpResponse>) -> Self {
            Self {
                requests: Arc::new(Mutex::new(Vec::new())),
                responses: Arc::new(Mutex::new(responses)),
            }
        }

        fn requests(&self) -> Arc<Mutex<Vec<RemoteSolverHttpRequest>>> {
            self.requests.clone()
        }
    }

    #[async_trait]
    impl RemoteSolverHttpTransport for FakeTransport {
        async fn send(
            &self,
            request: RemoteSolverHttpRequest,
        ) -> RemoteSolverResult<RemoteSolverHttpResponse> {
            self.requests.lock().unwrap().push(request);
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                return Err(RemoteSolverError::internal("no fake HTTP response"));
            }
            Ok(responses.remove(0))
        }
    }

    #[derive(Debug, Clone, Default)]
    struct FakeStorage {
        objects: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    }

    #[async_trait]
    impl ObjectStoragePort for FakeStorage {
        async fn put(
            &self,
            path: &ObjectPath,
            bytes: &[u8],
            _metadata: &BTreeMap<String, String>,
        ) -> RemoteSolverResult<ObjectRef> {
            self.objects
                .lock()
                .unwrap()
                .insert(path.value().to_string(), bytes.to_vec());
            Ok(ObjectRef::new(path.clone()))
        }

        async fn get(&self, object_ref: &ObjectRef) -> RemoteSolverResult<Option<Vec<u8>>> {
            Ok(self
                .objects
                .lock()
                .unwrap()
                .get(object_ref.path.value())
                .cloned())
        }

        async fn delete(&self, object_ref: &ObjectRef) -> RemoteSolverResult<bool> {
            Ok(self
                .objects
                .lock()
                .unwrap()
                .remove(object_ref.path.value())
                .is_some())
        }

        async fn exists(&self, object_ref: &ObjectRef) -> RemoteSolverResult<bool> {
            Ok(self
                .objects
                .lock()
                .unwrap()
                .contains_key(object_ref.path.value()))
        }
    }

    fn response(body: &str) -> RemoteSolverHttpResponse {
        RemoteSolverHttpResponse {
            status_code: 200,
            body: body.to_string(),
        }
    }

    #[tokio::test]
    async fn http_client_submits_task_with_headers() {
        let transport = FakeTransport::new(vec![response(
            r#"{"code":"OK","message":"ok","data":{"taskId":"task-1","accepted":true,"status":"ACCEPTED","message":"accepted"}}"#,
        )]);
        let requests = transport.requests();
        let client = RemoteSolverHttpClient::new("http://localhost/", transport)
            .unwrap()
            .with_tenant_id(TenantId::of("tenant-1").unwrap())
            .with_trace_id_provider(|| Some(TraceId::of("trace-1").unwrap()));

        let result = client
            .submit(&RemoteTaskSubmitRequest {
                payload_ref: ObjectPath::of("payload.json").unwrap(),
                request_id: None,
                tenant_id: None,
                complexity: None,
                time_sensitivity: None,
                priority: None,
                budget_scope: None,
                budget_limit: None,
                deadline: None,
            })
            .await
            .unwrap();

        assert_eq!(result.task_id.value(), "task-1");
        let requests = requests.lock().unwrap();
        assert_eq!(requests[0].method, "POST");
        assert_eq!(requests[0].url, "http://localhost/api/v1/tasks");
        assert_eq!(
            requests[0].headers.get("X-Tenant-Id").map(String::as_str),
            Some("tenant-1")
        );
        assert_eq!(
            requests[0].headers.get("X-Trace-Id").map(String::as_str),
            Some("trace-1")
        );
    }

    #[tokio::test]
    async fn http_client_get_returns_none_on_404() {
        let transport = FakeTransport::new(vec![RemoteSolverHttpResponse {
            status_code: 404,
            body: String::new(),
        }]);
        let client = RemoteSolverHttpClient::new("http://localhost", transport).unwrap();

        let result = client.get(&TaskId::of("missing").unwrap()).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn http_client_get_accepts_kotlin_path_fields_as_object_refs() {
        let transport = FakeTransport::new(vec![response(
            r#"{"code":"OK","message":"ok","data":{"taskId":"task-1","tenantId":"tenant-1","status":"COMPLETED","latestCheckpointPath":"checkpoint.bin","latestResultPath":"result.json","consumedCost":1.5}}"#,
        )]);
        let client = RemoteSolverHttpClient::new("http://localhost", transport).unwrap();

        let result = client
            .get(&TaskId::of("task-1").unwrap())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            result
                .latest_checkpoint_ref
                .as_ref()
                .map(|object_ref| object_ref.path.value()),
            Some("checkpoint.bin")
        );
        assert_eq!(
            result
                .latest_result_ref
                .as_ref()
                .map(|object_ref| object_ref.path.value()),
            Some("result.json")
        );
    }

    #[tokio::test]
    async fn http_client_maps_non_ok_envelope_to_error() {
        let transport = FakeTransport::new(vec![response(
            r#"{"code":"INVALID_ARGUMENT","message":"bad","traceId":"trace-1","data":null}"#,
        )]);
        let client = RemoteSolverHttpClient::new("http://localhost", transport).unwrap();

        let err = client
            .get(&TaskId::of("task-1").unwrap())
            .await
            .unwrap_err();

        assert_eq!(err.code, RemoteSolverErrorCode::InvalidArgument);
        assert_eq!(
            err.metadata.get("traceId").map(String::as_str),
            Some("trace-1")
        );
    }

    #[tokio::test]
    async fn http_execution_port_stores_payload_and_submits_task() {
        let transport = FakeTransport::new(vec![response(
            r#"{"code":"OK","message":"ok","data":{"taskId":"task-1","accepted":true,"status":"ACCEPTED","message":"accepted"}}"#,
        )]);
        let requests = transport.requests();
        let storage = FakeStorage::default();
        let stored = storage.objects.clone();
        let client = RemoteSolverHttpClient::new("http://localhost", transport).unwrap();
        let port = RemoteSolverHttpExecutionPort::new(client, storage);

        let handle = port
            .start(
                &SolvePayload::from_linear_model(
                    super::super::domain::SerializedLinearModel::empty("m"),
                ),
                &TaskId::of("task-1").unwrap(),
                &SliceId::of("slice-1").unwrap(),
                &NodeId::of("node-1").unwrap(),
                &TenantId::of("tenant-1").unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(handle.task_id.value(), "task-1");
        let requests = requests.lock().unwrap();
        assert_eq!(requests[0].method, "POST");
        let body: serde_json::Value =
            serde_json::from_str(requests[0].body.as_deref().unwrap()).unwrap();
        assert_eq!(body["requestId"], "task-1");
        assert!(
            stored
                .lock()
                .unwrap()
                .contains_key("remote-solver/payloads/tenant-1/task-1/slice-1.json")
        );
    }

    #[tokio::test]
    async fn http_execution_port_awaits_until_terminal_status() {
        let transport = FakeTransport::new(vec![
            response(
                r#"{"code":"OK","message":"ok","data":{"taskId":"task-1","tenantId":"tenant-1","status":"RUNNING","consumedCost":0.0}}"#,
            ),
            response(
                r#"{"code":"OK","message":"ok","data":{"taskId":"task-1","tenantId":"tenant-1","status":"COMPLETED","consumedCost":1.0}}"#,
            ),
        ]);
        let client = RemoteSolverHttpClient::new("http://localhost", transport).unwrap();
        let port = RemoteSolverHttpExecutionPort::new(client, FakeStorage::default())
            .with_poll_interval(Duration::from_millis(1));
        let handle = ExecutionHandle {
            handle_id: HandleId::of("handle-1").unwrap(),
            task_id: TaskId::of("task-1").unwrap(),
            slice_id: SliceId::of("slice-1").unwrap(),
            node_id: NodeId::of("node-1").unwrap(),
            started_at: SystemTime::UNIX_EPOCH,
        };

        let slice = port
            .await_slice_end(&handle, Duration::from_millis(50))
            .await
            .unwrap();

        assert!(slice.completed);
        assert!(slice.feasible);
    }

    #[tokio::test]
    async fn http_execution_port_fetches_final_result_object() {
        let transport = FakeTransport::new(vec![response(
            r#"{"code":"OK","message":"ok","data":{"taskId":"task-1","tenantId":"tenant-1","status":"COMPLETED","latestResultPath":"result.json","consumedCost":1.0}}"#,
        )]);
        let storage = FakeStorage::default();
        storage
            .objects
            .lock()
            .unwrap()
            .insert(
                "result.json".to_string(),
                br#"{"feasible":true,"optimal":true,"objectiveValue":5.0,"gap":0.0,"variableValues":[1.0],"elapsedMs":7,"solverStatus":"OPTIMAL"}"#.to_vec(),
            );
        let client = RemoteSolverHttpClient::new("http://localhost", transport).unwrap();
        let port = RemoteSolverHttpExecutionPort::new(client, storage);
        let handle = ExecutionHandle {
            handle_id: HandleId::of("handle-1").unwrap(),
            task_id: TaskId::of("task-1").unwrap(),
            slice_id: SliceId::of("slice-1").unwrap(),
            node_id: NodeId::of("node-1").unwrap(),
            started_at: SystemTime::UNIX_EPOCH,
        };

        let result = port.fetch_final_result(&handle).await.unwrap().unwrap();

        assert!(result.optimal);
        assert_eq!(result.objective_value, Some(5.0));
        assert_eq!(result.elapsed, Duration::from_millis(7));
    }
}
