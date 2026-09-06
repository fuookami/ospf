//! 远程求解 HTTP 任务客户端
//! Remote solver HTTP task client

use super::domain::{
    BudgetScopeId, ExecutionHandle, HandleId, NodeId, ObjectPath, ObjectRef, OperatorId,
    ReasonCode, RemoteSolverError, RemoteSolverErrorCode, RemoteSolverResult, RequestId,
    SerializedSolution, SliceId, SliceResult, SolvePayload, SolveResult, StopAcknowledgement,
    TaskComplexity, TaskId, TaskStatus, TenantId, TimeSensitivity, TraceId, epoch_millis,
    option_epoch_millis,
};
use super::ospf_serializer::{
    CheckpointResumeExpectationWithAttempt, load_checkpoint_artifact_from,
    store_checkpoint_artifact,
};
use super::port::{ObjectStoragePort, SolverExecutionPort};
use super::storage::validate_object_ref_etag;
use async_trait::async_trait;
use ospf_rust_core::solver::{
    AuditFingerprint, CancellationRecord, SolveCheckpoint, SolverProvenance,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

/// HTTP 传输配置。
/// HTTP transport configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RemoteSolverHttpTransportConfig {
    /// 连接超时 / Connect timeout
    pub connect_timeout: Option<Duration>,
    /// 请求超时 / Request timeout
    pub request_timeout: Option<Duration>,
    /// 默认请求头 / Default request headers
    pub headers: BTreeMap<String, String>,
    /// 传输属性 / Transport properties
    pub properties: BTreeMap<String, String>,
}

/// 远程求解 HTTP 请求。
/// Remote solver HTTP request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteSolverHttpRequest {
    /// HTTP 方法 / HTTP method
    pub method: String,
    /// 请求 URL / Request URL
    pub url: String,
    /// 请求头 / Request headers
    pub headers: BTreeMap<String, String>,
    /// 请求体 / Request body
    pub body: Option<String>,
}

/// 远程求解 HTTP 响应。
/// Remote solver HTTP response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteSolverHttpResponse {
    /// HTTP 状态码 / HTTP status code
    pub status_code: u16,
    /// 响应体 / Response body
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
        if !response.accepted {
            return Err(RemoteSolverError::new(
                RemoteSolverErrorCode::InvalidTaskStateTransition,
                format!("remote task submission was rejected: {}", response.message),
            )
            .with_metadata([
                ("taskId", response.task_id.value().to_owned()),
                ("status", format!("{:?}", response.status)),
            ]));
        }
        Ok(Self::handle(
            response.task_id,
            slice_id.clone(),
            node_id.clone(),
        )?)
    }

    async fn resume(
        &self,
        payload: &SolvePayload,
        checkpoint: &ObjectRef,
        task_id: &TaskId,
        slice_id: &SliceId,
        node_id: &NodeId,
        _tenant_id: &TenantId,
    ) -> RemoteSolverResult<ExecutionHandle> {
        let expected_checkpoint = payload.checkpoint_metadata.as_ref().ok_or_else(|| {
            RemoteSolverError::invalid_argument(
                "resuming an HTTP solve requires checkpoint metadata",
            )
        })?;
        expected_checkpoint.validate().map_err(|error| {
            RemoteSolverError::checkpoint_restore(format!(
                "resume payload checkpoint failed validation: {}",
                error
            ))
        })?;
        if slice_id.value() == expected_checkpoint.attempt_id {
            return Err(RemoteSolverError::checkpoint_restore(
                "resume requires a new child attempt and cannot reuse the source attempt",
            ));
        }
        let expectation = CheckpointResumeExpectationWithAttempt {
            run_id: task_id.value().to_owned(),
            attempt_id: expected_checkpoint.attempt_id.clone(),
            parent_attempt_id: expected_checkpoint.parent_attempt_id.clone(),
            model_fingerprint: expected_checkpoint.model_fingerprint.clone(),
            configuration_fingerprint: expected_checkpoint.configuration_fingerprint.clone(),
            solver_fingerprint: expected_checkpoint.solver_fingerprint.clone(),
            provenance: expected_checkpoint.provenance.clone(),
            cancellation_chain: expected_checkpoint.cancellation_chain.clone(),
        };
        let artifact =
            load_checkpoint_artifact_from(&self.object_storage, checkpoint, &expectation).await?;
        if artifact.checkpoint != *expected_checkpoint {
            return Err(RemoteSolverError::checkpoint_restore(
                "checkpoint artifact identity does not match the resume payload",
            ));
        }
        // 当前服务端按任务恢复最新 checkpoint，trait 参数保留用于非 HTTP port。
        // The current server resumes the latest task checkpoint; trait parameters remain for non-HTTP ports.
        let response = self
            .http_client
            .resume(task_id, &RemoteTaskResumeRequest::default())
            .await?;
        validate_resume_action(&response, expected_checkpoint, slice_id)?;
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
            let bytes = self.object_storage.get(result_ref).await?.ok_or_else(|| {
                RemoteSolverError::new(
                    RemoteSolverErrorCode::SolverExecutionFailed,
                    format!(
                        "remote task {} references missing final result artifact '{}'",
                        handle.task_id, result_ref.path
                    ),
                )
                .with_metadata([("resultRef", result_ref.path.value().to_owned())])
            })?;
            validate_object_ref_etag(result_ref, &bytes)?;
            let solution: SerializedSolution = serde_json::from_slice(&bytes).map_err(|err| {
                RemoteSolverError::invalid_argument(format!(
                    "Failed to decode remote result object '{}': {}",
                    result_ref.path, err
                ))
            })?;
            return Ok(Some(solve_result_from_solution(
                solution,
                view.latest_checkpoint_ref,
                Some(result_ref.clone()),
                &handle.task_id,
                &handle.slice_id,
            )));
        }

        Ok(Some(solve_result_from_view(&view, &handle.slice_id)))
    }

    async fn stop(&self, handle: &ExecutionHandle) -> RemoteSolverResult<StopAcknowledgement> {
        let action = self
            .http_client
            .stop(&handle.task_id, &RemoteTaskStopRequest::default())
            .await?;
        let mut acknowledgement =
            StopAcknowledgement::new(action.task_id, action.accepted, action.status)
                .with_cancellation_origin("REMOTE_STOP");
        acknowledgement.run_id = action.run_id;
        acknowledgement.attempt_id = action.attempt_id;
        acknowledgement.model_fingerprint = action.model_fingerprint;
        acknowledgement.configuration_fingerprint = action.configuration_fingerprint;
        acknowledgement.solver_fingerprint = action.solver_fingerprint;
        acknowledgement.provenance = action.provenance;
        acknowledgement.cancellation_chain = action.cancellation_chain;
        acknowledgement.message = action.message;
        Ok(acknowledgement)
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
    /// 载荷对象路径 / Payload object path
    pub payload_ref: ObjectPath,
    /// 请求 ID / Request ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<RequestId>,
    /// 租户 ID / Tenant ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<TenantId>,
    /// 任务复杂度 / Task complexity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub complexity: Option<TaskComplexity>,
    /// 时间敏感度 / Time sensitivity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_sensitivity: Option<TimeSensitivity>,
    /// 优先级 / Priority
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    /// 预算范围 ID / Budget scope ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_scope: Option<BudgetScopeId>,
    /// 预算上限 / Budget limit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_limit: Option<f64>,
    /// 截止时间 / Deadline
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
    /// 任务 ID / Task ID
    pub task_id: TaskId,
    /// 是否已接受 / Whether accepted
    pub accepted: bool,
    /// 任务状态 / Task status
    pub status: TaskStatus,
    /// 响应消息 / Response message
    pub message: String,
}

/// 远程任务视图。
/// Remote task view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteTaskView {
    /// 任务 ID / Task ID
    pub task_id: TaskId,
    /// 租户 ID / Tenant ID
    pub tenant_id: TenantId,
    /// 任务状态 / Task status
    pub status: TaskStatus,
    /// 当前节点 ID / Current node ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_node_id: Option<NodeId>,
    /// 最新检查点引用 / Latest checkpoint reference
    #[serde(
        alias = "latestCheckpointPath",
        default,
        deserialize_with = "deserialize_optional_object_ref",
        skip_serializing_if = "Option::is_none"
    )]
    pub latest_checkpoint_ref: Option<ObjectRef>,
    /// 最新结果引用 / Latest result reference
    #[serde(
        alias = "latestResultPath",
        default,
        deserialize_with = "deserialize_optional_object_ref",
        skip_serializing_if = "Option::is_none"
    )]
    pub latest_result_ref: Option<ObjectRef>,
    /// 已消耗成本 / Consumed cost
    pub consumed_cost: f64,
}

/// 远程任务操作响应。
/// Remote task action response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteTaskAction {
    /// 任务 ID / Task ID
    pub task_id: TaskId,
    /// 是否接受操作 / Whether the operation was accepted.
    #[serde(default = "default_remote_action_accepted")]
    pub accepted: bool,
    /// 任务状态 / Task status
    pub status: TaskStatus,
    /// 求解运行 ID / Solve run identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// 求解 attempt ID / Solve attempt identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<String>,
    /// 模型指纹 / Model fingerprint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_fingerprint: Option<AuditFingerprint>,
    /// 生效配置指纹 / Effective configuration fingerprint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configuration_fingerprint: Option<AuditFingerprint>,
    /// solver 环境指纹 / Solver environment fingerprint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solver_fingerprint: Option<AuditFingerprint>,
    /// solver provenance / Solver provenance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<SolverProvenance>,
    /// 跨 attempt 取消链 / Cross-attempt cancellation chain.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cancellation_chain: Vec<CancellationRecord>,
    /// 服务端附加消息 / Server message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

fn default_remote_action_accepted() -> bool {
    true
}

/// 远程任务停止请求。
/// Remote task stop request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RemoteTaskStopRequest {
    /// 停止原因码 / Stop reason code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<ReasonCode>,
    /// 操作者 ID / Operator ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<OperatorId>,
    /// 操作来源 / Operation source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<super::domain::OperationSource>,
}

/// 远程任务恢复请求。
/// Remote task resume request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RemoteTaskResumeRequest {
    /// 操作者 ID / Operator ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<OperatorId>,
    /// 操作来源 / Operation source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<super::domain::OperationSource>,
    /// 恢复原因码 / Resume reason code
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
            "UNSUPPORTED_PROTOCOL_VERSION" => RemoteSolverErrorCode::UnsupportedProtocolVersion,
            "INVALID_TASK_STATE_TRANSITION" => RemoteSolverErrorCode::InvalidTaskStateTransition,
            "NO_ELIGIBLE_NODE_AVAILABLE" => RemoteSolverErrorCode::NoEligibleNodeAvailable,
            "NODE_OFFLINE" => RemoteSolverErrorCode::NodeOffline,
            "SOLVER_EXECUTION_FAILED" => RemoteSolverErrorCode::SolverExecutionFailed,
            "CHECKPOINT_EXPORT_FAILED" => RemoteSolverErrorCode::CheckpointExportFailed,
            "CHECKPOINT_RESTORE_FAILED" => RemoteSolverErrorCode::CheckpointRestoreFailed,
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

/// 校验 resume action 的接受状态和完整身份链 / Validate the resume action acceptance and identity chain.
fn validate_resume_action(
    action: &RemoteTaskAction,
    source_checkpoint: &SolveCheckpoint,
    child_attempt_id: &SliceId,
) -> RemoteSolverResult<()> {
    if !action.accepted {
        return Err(RemoteSolverError::new(
            RemoteSolverErrorCode::InvalidTaskStateTransition,
            format!(
                "remote task resume was rejected: {}",
                action
                    .message
                    .as_deref()
                    .unwrap_or("the remote service rejected the resume action")
            ),
        )
        .with_metadata([
            ("taskId", action.task_id.value().to_owned()),
            ("status", format!("{:?}", action.status)),
        ]));
    }

    if action.run_id.as_deref() != Some(source_checkpoint.run_id.as_str()) {
        return Err(RemoteSolverError::checkpoint_restore(
            "remote resume action run identity does not match the source checkpoint",
        ));
    }
    if action.attempt_id.as_deref() != Some(child_attempt_id.value()) {
        return Err(RemoteSolverError::checkpoint_restore(
            "remote resume action attempt identity does not match the new child attempt",
        ));
    }
    if action.model_fingerprint.as_ref() != Some(&source_checkpoint.model_fingerprint)
        || action.configuration_fingerprint.as_ref()
            != Some(&source_checkpoint.configuration_fingerprint)
        || action.solver_fingerprint.as_ref() != Some(&source_checkpoint.solver_fingerprint)
    {
        return Err(RemoteSolverError::checkpoint_restore(
            "remote resume action fingerprints do not match the source checkpoint",
        ));
    }
    if action.provenance.as_ref() != Some(&source_checkpoint.provenance) {
        return Err(RemoteSolverError::checkpoint_restore(
            "remote resume action provenance does not match the source checkpoint",
        ));
    }
    if action.cancellation_chain != source_checkpoint.cancellation_chain {
        return Err(RemoteSolverError::checkpoint_restore(
            "remote resume action cancellation chain does not preserve the source checkpoint",
        ));
    }
    Ok(())
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
        // 任务完成只说明执行生命周期结束，不能推导数学可行性 / Task completion only closes
        // the execution lifecycle; it does not establish mathematical feasibility.
        feasible: false,
        objective_value: None,
        gap: None,
        elapsed,
        message: Some(format!("Remote task status is {:?}.", view.status)),
    }
}

fn solve_result_from_view(view: &RemoteTaskView, slice_id: &SliceId) -> SolveResult {
    let mut extension = BTreeMap::new();
    extension.insert("remote.taskStatus".to_owned(), format!("{:?}", view.status));
    SolveResult {
        // 没有结果 artifact 时只能返回未知数学结论 / Without a result artifact, only an
        // unknown mathematical conclusion can be returned.
        feasible: false,
        optimal: false,
        objective_value: None,
        gap: None,
        elapsed: Duration::ZERO,
        checkpoint_ref: view.latest_checkpoint_ref.clone(),
        checkpoint_metadata: None,
        result_ref: view.latest_result_ref.clone(),
        run_id: Some(view.task_id.value().to_owned()),
        attempt_id: Some(slice_id.value().to_owned()),
        artifact_digest: None,
        report: None,
        message: Some(format!("Remote task status is {:?}.", view.status)),
        extension,
    }
}

fn solve_result_from_solution(
    solution: SerializedSolution,
    checkpoint_ref: Option<ObjectRef>,
    result_ref: Option<ObjectRef>,
    task_id: &TaskId,
    slice_id: &SliceId,
) -> SolveResult {
    let run_id = solution
        .report
        .as_ref()
        .and_then(|report| report.run_id.clone());
    let attempt_id = solution
        .report
        .as_ref()
        .and_then(|report| report.attempt_id.clone());
    let artifact_digest = solution
        .report
        .as_ref()
        .and_then(|report| report.artifact_digest.clone());
    SolveResult {
        feasible: solution.feasible,
        optimal: solution.optimal,
        objective_value: solution.objective_value,
        gap: solution.gap,
        elapsed: solution.elapsed,
        checkpoint_ref,
        checkpoint_metadata: None,
        result_ref,
        run_id: run_id.or_else(|| Some(task_id.value().to_owned())),
        attempt_id: attempt_id.or_else(|| Some(slice_id.value().to_owned())),
        artifact_digest,
        report: solution.report,
        message: solution.message,
        extension: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use ospf_rust_core::solver::{AuditFingerprint, SolveCheckpoint, SolverProvenance};
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

    fn action_response(action: RemoteTaskAction) -> RemoteSolverHttpResponse {
        let body = serde_json::json!({
            "code": "OK",
            "message": "ok",
            "data": action,
        });
        response(&body.to_string())
    }

    fn accepted_resume_action(
        task_id: &str,
        checkpoint: &SolveCheckpoint,
        attempt_id: &str,
    ) -> RemoteTaskAction {
        RemoteTaskAction {
            task_id: TaskId::of(task_id).expect("task id should be valid"),
            accepted: true,
            status: TaskStatus::Running,
            run_id: Some(checkpoint.run_id.clone()),
            attempt_id: Some(attempt_id.to_owned()),
            model_fingerprint: Some(checkpoint.model_fingerprint.clone()),
            configuration_fingerprint: Some(checkpoint.configuration_fingerprint.clone()),
            solver_fingerprint: Some(checkpoint.solver_fingerprint.clone()),
            provenance: Some(checkpoint.provenance.clone()),
            cancellation_chain: checkpoint.cancellation_chain.clone(),
            message: Some("accepted".to_owned()),
        }
    }

    fn checkpoint_fixture(run_id: &str, attempt_id: &str) -> SolveCheckpoint {
        let fingerprint = |value: &str| AuditFingerprint {
            schema_version: "1.0".to_owned(),
            algorithm: "sha256".to_owned(),
            value: value.to_owned(),
        };
        let state_digest = ospf_rust_core::solver::sha256_fingerprint(
            "ospf.solve.checkpoint.state",
            b"portable-state",
        );
        SolveCheckpoint::new(
            run_id,
            attempt_id,
            None,
            fingerprint("model"),
            fingerprint("config"),
            fingerprint("solver"),
            SolverProvenance {
                solver_id: "fake/1".to_owned(),
                backend_name: "fake".to_owned(),
                ..SolverProvenance::default()
            },
            1,
            None,
            None,
            None,
            state_digest,
        )
        .expect("checkpoint fixture should be valid")
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
    async fn http_execution_port_rejects_submit_response() {
        let transport = FakeTransport::new(vec![response(
            r#"{"code":"OK","message":"ok","data":{"taskId":"task-1","accepted":false,"status":"FAILED","message":"quota rejected"}}"#,
        )]);
        let client = RemoteSolverHttpClient::new("http://localhost", transport).unwrap();
        let port = RemoteSolverHttpExecutionPort::new(client, FakeStorage::default());

        let error = port
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
            .expect_err("a rejected submit response must not create a handle");

        assert_eq!(
            error.code,
            RemoteSolverErrorCode::InvalidTaskStateTransition
        );
        assert!(error.to_string().contains("quota rejected"));
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
        assert!(!slice.feasible);
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

    #[tokio::test]
    async fn http_execution_port_validates_final_result_etag_before_decoding() {
        let bytes = br#"{"feasible":true,"optimal":true,"objectiveValue":5.0,"gap":0.0,"variableValues":[1.0],"elapsedMs":7,"solverStatus":"OPTIMAL"}"#;
        let digest = {
            use sha2::{Digest, Sha256};

            let digest = Sha256::digest(bytes);
            digest
                .iter()
                .map(|byte| format!("{:02x}", byte))
                .collect::<String>()
        };
        let view = |etag: &str| {
            serde_json::to_string(&serde_json::json!({
                "code": "OK",
                "message": "ok",
                "data": {
                    "taskId": "task-1",
                    "tenantId": "tenant-1",
                    "status": "COMPLETED",
                    "latestResultPath": {
                        "path": "result.json",
                        "etag": etag,
                    },
                    "consumedCost": 1.0,
                }
            }))
            .expect("result view should encode")
        };
        let handle = ExecutionHandle {
            handle_id: HandleId::of("handle-1").unwrap(),
            task_id: TaskId::of("task-1").unwrap(),
            slice_id: SliceId::of("slice-1").unwrap(),
            node_id: NodeId::of("node-1").unwrap(),
            started_at: SystemTime::UNIX_EPOCH,
        };

        let good_transport = FakeTransport::new(vec![response(&view(&digest))]);
        let good_storage = FakeStorage::default();
        good_storage
            .objects
            .lock()
            .unwrap()
            .insert("result.json".to_owned(), bytes.to_vec());
        let good_client = RemoteSolverHttpClient::new("http://localhost", good_transport)
            .expect("good HTTP client should be valid");
        let good_port = RemoteSolverHttpExecutionPort::new(good_client, good_storage);
        assert!(
            good_port
                .fetch_final_result(&handle)
                .await
                .expect("matching ETag should be accepted")
                .is_some()
        );

        let bad_transport = FakeTransport::new(vec![response(&view("stale-etag"))]);
        let bad_storage = FakeStorage::default();
        bad_storage
            .objects
            .lock()
            .unwrap()
            .insert("result.json".to_owned(), bytes.to_vec());
        let bad_client = RemoteSolverHttpClient::new("http://localhost", bad_transport)
            .expect("bad HTTP client fixture should be valid");
        let bad_port = RemoteSolverHttpExecutionPort::new(bad_client, bad_storage);
        let error = bad_port
            .fetch_final_result(&handle)
            .await
            .expect_err("stale final-result ETag must be rejected");
        assert!(error.to_string().contains("ETag"));
    }

    #[tokio::test]
    async fn http_execution_port_rejects_missing_final_result_artifact() {
        let transport = FakeTransport::new(vec![response(
            r#"{"code":"OK","message":"ok","data":{"taskId":"task-1","tenantId":"tenant-1","status":"COMPLETED","latestResultPath":"missing-result.json","consumedCost":1.0}}"#,
        )]);
        let client = RemoteSolverHttpClient::new("http://localhost", transport).unwrap();
        let port = RemoteSolverHttpExecutionPort::new(client, FakeStorage::default());
        let handle = ExecutionHandle {
            handle_id: HandleId::of("handle-1").unwrap(),
            task_id: TaskId::of("task-1").unwrap(),
            slice_id: SliceId::of("slice-1").unwrap(),
            node_id: NodeId::of("node-1").unwrap(),
            started_at: SystemTime::UNIX_EPOCH,
        };

        let error = port
            .fetch_final_result(&handle)
            .await
            .expect_err("a dangling result reference must be rejected");

        assert_eq!(error.code, RemoteSolverErrorCode::SolverExecutionFailed);
        assert!(error.to_string().contains("missing final result artifact"));
    }

    #[tokio::test]
    async fn http_execution_port_validates_checkpoint_before_resuming_task() {
        let storage = FakeStorage::default();
        let checkpoint = checkpoint_fixture("task-1", "slice-0");
        let transport = FakeTransport::new(vec![action_response(accepted_resume_action(
            "task-2",
            &checkpoint,
            "slice-1",
        ))]);
        let requests = transport.requests();
        let artifact = checkpoint
            .clone()
            .with_state(b"portable-state".to_vec())
            .expect("checkpoint artifact should be valid");
        let checkpoint_path = ObjectPath::of("checkpoint.json").unwrap();
        let checkpoint_ref = store_checkpoint_artifact(&storage, &checkpoint_path, &artifact)
            .await
            .expect("checkpoint artifact should be stored");
        let payload = SolvePayload::from_linear_model(
            super::super::domain::SerializedLinearModel::empty("m"),
        )
        .with_snapshot_ref(checkpoint_ref.clone())
        .with_checkpoint_metadata(checkpoint);
        let client = RemoteSolverHttpClient::new("http://localhost", transport).unwrap();
        let port = RemoteSolverHttpExecutionPort::new(client, storage);

        let handle = port
            .resume(
                &payload,
                &checkpoint_ref,
                &TaskId::of("task-1").unwrap(),
                &SliceId::of("slice-1").unwrap(),
                &NodeId::of("node-1").unwrap(),
                &TenantId::of("tenant-1").unwrap(),
            )
            .await
            .expect("matching checkpoint should resume the task");

        assert_eq!(handle.task_id.value(), "task-2");
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].url,
            "http://localhost/api/v1/tasks/task-1/resume"
        );
    }

    #[tokio::test]
    async fn http_execution_port_rejects_reusing_source_attempt() {
        let transport = FakeTransport::new(Vec::new());
        let requests = transport.requests();
        let storage = FakeStorage::default();
        let checkpoint = checkpoint_fixture("task-1", "slice-0");
        let artifact = checkpoint
            .clone()
            .with_state(b"portable-state".to_vec())
            .expect("checkpoint artifact should be valid");
        let checkpoint_path = ObjectPath::of("checkpoint.json").unwrap();
        let checkpoint_ref = store_checkpoint_artifact(&storage, &checkpoint_path, &artifact)
            .await
            .expect("checkpoint artifact should be stored");
        let payload = SolvePayload::from_linear_model(
            super::super::domain::SerializedLinearModel::empty("m"),
        )
        .with_snapshot_ref(checkpoint_ref.clone())
        .with_checkpoint_metadata(checkpoint);
        let client = RemoteSolverHttpClient::new("http://localhost", transport).unwrap();
        let port = RemoteSolverHttpExecutionPort::new(client, storage);

        let error = port
            .resume(
                &payload,
                &checkpoint_ref,
                &TaskId::of("task-1").unwrap(),
                &SliceId::of("slice-0").unwrap(),
                &NodeId::of("node-1").unwrap(),
                &TenantId::of("tenant-1").unwrap(),
            )
            .await
            .expect_err("resume must allocate a new child attempt");

        assert_eq!(error.code, RemoteSolverErrorCode::CheckpointRestoreFailed);
        assert!(requests.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn http_execution_port_rejects_resume_action() {
        let transport = FakeTransport::new(vec![response(
            r#"{"code":"OK","message":"ok","data":{"taskId":"task-2","accepted":false,"status":"FAILED","message":"resume rejected"}}"#,
        )]);
        let requests = transport.requests();
        let storage = FakeStorage::default();
        let checkpoint = checkpoint_fixture("task-1", "slice-0");
        let artifact = checkpoint
            .clone()
            .with_state(b"portable-state".to_vec())
            .expect("checkpoint artifact should be valid");
        let checkpoint_path = ObjectPath::of("checkpoint.json").unwrap();
        let checkpoint_ref = store_checkpoint_artifact(&storage, &checkpoint_path, &artifact)
            .await
            .expect("checkpoint artifact should be stored");
        let payload = SolvePayload::from_linear_model(
            super::super::domain::SerializedLinearModel::empty("m"),
        )
        .with_snapshot_ref(checkpoint_ref.clone())
        .with_checkpoint_metadata(checkpoint);
        let client = RemoteSolverHttpClient::new("http://localhost", transport).unwrap();
        let port = RemoteSolverHttpExecutionPort::new(client, storage);

        let error = port
            .resume(
                &payload,
                &checkpoint_ref,
                &TaskId::of("task-1").unwrap(),
                &SliceId::of("slice-1").unwrap(),
                &NodeId::of("node-1").unwrap(),
                &TenantId::of("tenant-1").unwrap(),
            )
            .await
            .expect_err("a rejected resume action must not create a handle");

        assert_eq!(
            error.code,
            RemoteSolverErrorCode::InvalidTaskStateTransition
        );
        assert!(error.to_string().contains("resume rejected"));
        assert_eq!(requests.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn http_execution_port_rejects_resume_action_identity_mismatch() {
        let checkpoint = checkpoint_fixture("task-1", "slice-0");
        let mut action = accepted_resume_action("task-2", &checkpoint, "slice-1");
        action.run_id = Some("wrong-run".to_owned());
        let transport = FakeTransport::new(vec![action_response(action)]);
        let storage = FakeStorage::default();
        let artifact = checkpoint
            .clone()
            .with_state(b"portable-state".to_vec())
            .expect("checkpoint artifact should be valid");
        let checkpoint_path = ObjectPath::of("checkpoint.json").unwrap();
        let checkpoint_ref = store_checkpoint_artifact(&storage, &checkpoint_path, &artifact)
            .await
            .expect("checkpoint artifact should be stored");
        let payload = SolvePayload::from_linear_model(
            super::super::domain::SerializedLinearModel::empty("m"),
        )
        .with_snapshot_ref(checkpoint_ref.clone())
        .with_checkpoint_metadata(checkpoint);
        let client = RemoteSolverHttpClient::new("http://localhost", transport).unwrap();
        let port = RemoteSolverHttpExecutionPort::new(client, storage);

        let error = port
            .resume(
                &payload,
                &checkpoint_ref,
                &TaskId::of("task-1").unwrap(),
                &SliceId::of("slice-1").unwrap(),
                &NodeId::of("node-1").unwrap(),
                &TenantId::of("tenant-1").unwrap(),
            )
            .await
            .expect_err("resume action identity mismatch must be rejected");

        assert_eq!(error.code, RemoteSolverErrorCode::CheckpointRestoreFailed);
        assert!(error.to_string().contains("run identity"));
    }

    #[tokio::test]
    async fn http_execution_port_rejects_checkpoint_identity_mismatch_without_http_call() {
        let transport = FakeTransport::new(vec![response(
            r#"{"code":"OK","message":"ok","data":{"taskId":"task-2","accepted":true,"status":"RUNNING"}}"#,
        )]);
        let requests = transport.requests();
        let storage = FakeStorage::default();
        let artifact_checkpoint = checkpoint_fixture("task-1", "slice-0");
        let artifact = artifact_checkpoint
            .with_state(b"portable-state".to_vec())
            .expect("checkpoint artifact should be valid");
        let checkpoint_path = ObjectPath::of("checkpoint.json").unwrap();
        let checkpoint_ref = store_checkpoint_artifact(&storage, &checkpoint_path, &artifact)
            .await
            .expect("checkpoint artifact should be stored");
        let payload_checkpoint = checkpoint_fixture("task-1", "slice-other");
        let payload = SolvePayload::from_linear_model(
            super::super::domain::SerializedLinearModel::empty("m"),
        )
        .with_snapshot_ref(checkpoint_ref.clone())
        .with_checkpoint_metadata(payload_checkpoint);
        let client = RemoteSolverHttpClient::new("http://localhost", transport).unwrap();
        let port = RemoteSolverHttpExecutionPort::new(client, storage);

        let error = port
            .resume(
                &payload,
                &checkpoint_ref,
                &TaskId::of("task-1").unwrap(),
                &SliceId::of("slice-1").unwrap(),
                &NodeId::of("node-1").unwrap(),
                &TenantId::of("tenant-1").unwrap(),
            )
            .await
            .expect_err("checkpoint identity mismatch must reject resume");

        assert_eq!(error.code, RemoteSolverErrorCode::CheckpointRestoreFailed);
        assert!(requests.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn http_execution_port_rejects_checkpoint_from_another_task() {
        let transport = FakeTransport::new(vec![response(
            r#"{"code":"OK","message":"ok","data":{"taskId":"task-3","accepted":true,"status":"RUNNING"}}"#,
        )]);
        let requests = transport.requests();
        let storage = FakeStorage::default();
        let checkpoint = checkpoint_fixture("task-1", "slice-0");
        let artifact = checkpoint
            .clone()
            .with_state(b"portable-state".to_vec())
            .expect("checkpoint artifact should be valid");
        let checkpoint_path = ObjectPath::of("checkpoint.json").unwrap();
        let checkpoint_ref = store_checkpoint_artifact(&storage, &checkpoint_path, &artifact)
            .await
            .expect("checkpoint artifact should be stored");
        let payload = SolvePayload::from_linear_model(
            super::super::domain::SerializedLinearModel::empty("m"),
        )
        .with_snapshot_ref(checkpoint_ref.clone())
        .with_checkpoint_metadata(checkpoint);
        let client = RemoteSolverHttpClient::new("http://localhost", transport).unwrap();
        let port = RemoteSolverHttpExecutionPort::new(client, storage);

        let error = port
            .resume(
                &payload,
                &checkpoint_ref,
                &TaskId::of("task-2").unwrap(),
                &SliceId::of("slice-1").unwrap(),
                &NodeId::of("node-1").unwrap(),
                &TenantId::of("tenant-1").unwrap(),
            )
            .await
            .expect_err("a checkpoint from another task must not resume");

        assert_eq!(error.code, RemoteSolverErrorCode::CheckpointRestoreFailed);
        assert!(requests.lock().unwrap().is_empty());
    }
}
