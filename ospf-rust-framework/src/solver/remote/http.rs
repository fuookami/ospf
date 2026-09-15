//! 远程求解 HTTP 任务客户端
//! Remote solver HTTP task client

use super::domain::{
    BudgetScopeId, ExecutionHandle, HandleId, NodeId, ObjectPath, ObjectRef, OperatorId,
    ReasonCode, RemoteProblemStatus, RemoteSolutionPresence, RemoteSolverCapabilities,
    RemoteSolverError, RemoteSolverErrorCode, RemoteSolverResult, RemoteTerminationReason,
    RequestId, SchedulingRequest, SerializedSolution, SliceId, SliceResult, SolvePayload,
    SolveResult, StopAcknowledgement, TaskComplexity, TaskId, TaskStatus, TenantId,
    TimeSensitivity, TraceId, epoch_millis, option_epoch_millis,
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
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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

/// HTTP resume semantics / HTTP 恢复语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RemoteSolverHttpResumeMode {
    /// Resume the task's latest server-side checkpoint / 恢复任务服务端最新 checkpoint。
    Latest,
    /// Require checkpoint-specific resume, which the current API does not expose / 要求指定 checkpoint 恢复；当前 API 不支持。
    #[default]
    StrictCheckpoint,
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
    resume_mode: RemoteSolverHttpResumeMode,
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
            resume_mode: RemoteSolverHttpResumeMode::StrictCheckpoint,
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

    /// 设置 HTTP 恢复语义 / Set HTTP resume semantics.
    pub fn with_resume_mode(mut self, resume_mode: RemoteSolverHttpResumeMode) -> Self {
        self.resume_mode = resume_mode;
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
        self.http_client.ensure_tenant(tenant_id)?;
        if payload.model_data.model_type() == super::domain::NormalizedModelType::Cp {
            let capabilities = self.http_client.probe_capabilities().await?;
            validate_cp_capabilities(&capabilities)?;
        }
        // 调用方 task_id 在当前 HTTP API 中作为幂等/追踪 request_id。
        // The caller task_id is used as idempotency/tracing request_id for the current HTTP API.
        let payload_ref = self
            .store_payload(payload, task_id, slice_id, tenant_id)
            .await?;
        let scheduling = payload.scheduling.as_ref();
        let response = self
            .http_client
            .submit(&RemoteTaskSubmitRequest {
                payload_ref: payload_ref.path,
                request_id: Some(RequestId::of(task_id.value())?),
                tenant_id: Some(tenant_id.clone()),
                complexity: scheduling.and_then(|value| value.complexity),
                time_sensitivity: scheduling.and_then(|value| value.time_sensitivity),
                priority: scheduling.and_then(|value| value.priority),
                budget_scope: scheduling.and_then(|value| value.budget_scope.clone()),
                budget_limit: scheduling.and_then(|value| value.budget_limit),
                deadline: scheduling.and_then(|value| value.deadline),
                scheduling: payload.scheduling.clone(),
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
        tenant_id: &TenantId,
    ) -> RemoteSolverResult<ExecutionHandle> {
        self.http_client.ensure_tenant(tenant_id)?;
        if self.resume_mode == RemoteSolverHttpResumeMode::StrictCheckpoint {
            return Err(RemoteSolverError::invalid_argument(
                "HTTP task resume API does not support checkpoint-specific resume",
            ));
        }
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
        if expected_checkpoint.run_id != task_id.value() {
            return Err(RemoteSolverError::checkpoint_restore(
                "resume checkpoint run identity does not match the requested task",
            ));
        }
        if slice_id.value() == expected_checkpoint.attempt_id {
            return Err(RemoteSolverError::checkpoint_restore(
                "resume requires a new child attempt and cannot reuse the source attempt",
            ));
        }
        let expectation = CheckpointResumeExpectationWithAttempt {
            // The run identity belongs to the source checkpoint.  The task ID here is the
            // caller/request (or an already-known canonical task path), not the solve run ID.
            run_id: expected_checkpoint.run_id.clone(),
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
        validate_resume_action(
            &response,
            expected_checkpoint,
            slice_id,
            task_id,
            tenant_id,
        )?;
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
        if quantum.is_zero() {
            return Err(RemoteSolverError::invalid_argument(
                "quantum must be positive",
            ));
        }
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
                    false,
                )?);
            }

            if view.status == TaskStatus::Suspended {
                return Ok(slice_result_from_view(
                    &view,
                    &handle.slice_id,
                    false,
                    started.elapsed(),
                    false,
                )?);
            }

            // The dispatcher owns the effective quantum and checkpoint/requeue lifecycle.
            // The HTTP port waits for its suspension/terminal observation instead of issuing a
            // second task-level stop that would make a server-managed task non-resumable.
            tokio::time::sleep(self.poll_interval).await;
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
            let solution: SerializedSolution = match serde_json::from_slice(&bytes) {
                Ok(solution) => solution,
                Err(error) if looks_like_typed_cp_result(&bytes) => {
                    // The CP adapter owns exact typed-result validation.  Return the terminal
                    // task metadata while leaving the object reference attached so that it can
                    // reread and validate the typed artifact against the local snapshot.
                    let mut metadata = solve_result_from_view(&view, &handle.slice_id)?;
                    metadata.result_ref = Some(result_ref.clone());
                    metadata.artifact_digest = view.artifact_digest.clone();
                    metadata.message = Some(format!(
                        "Remote typed CP result requires exact adapter validation: {}",
                        error
                    ));
                    return Ok(Some(metadata));
                }
                Err(err) => {
                    return Err(RemoteSolverError::invalid_argument(format!(
                        "Failed to decode remote result object '{}': {}",
                        result_ref.path, err
                    )));
                }
            };
            return Ok(Some(solve_result_from_solution(
                solution,
                &view,
                view.latest_checkpoint_ref.clone(),
                Some(result_ref.clone()),
                &handle.task_id,
                &handle.slice_id,
            )?));
        }

        Ok(Some(solve_result_from_view(&view, &handle.slice_id)?))
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
        acknowledgement.cancellation_chain = action.cancellation_chain.unwrap_or_default();
        acknowledgement.message = action.message;
        Ok(acknowledgement)
    }
}

fn looks_like_typed_cp_result(bytes: &[u8]) -> bool {
    serde_json::from_slice::<serde_json::Value>(bytes)
        .ok()
        .and_then(|value| value.as_object().cloned())
        .is_some_and(|object| {
            object.contains_key("snapshot")
                && object.contains_key("report")
                && object.contains_key("artifactDigest")
        })
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
            scheduling: None,
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
        let value = self.decode_envelope::<RemoteTaskSubmitResponse>(&response)?;
        reject_unknown_task_status(value.status, "submit")?;
        Ok(value)
    }

    /// 探测远程求解器能力 / Probe remote solver capabilities.
    pub async fn probe_capabilities(&self) -> RemoteSolverResult<RemoteSolverCapabilities> {
        let response = self
            .transport
            .send(self.request("GET", "/api/v1/capabilities", None))
            .await?;
        self.decode_envelope::<RemoteSolverCapabilities>(&response)
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
        let value = self.decode_envelope::<RemoteTaskView>(&response)?;
        self.validate_response_tenant(&value.tenant_id)?;
        reject_unknown_task_status(value.status, "get")?;
        Ok(Some(value))
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
        let value = self.decode_envelope::<RemoteTaskAction>(&response)?;
        if let Some(tenant_id) = value.tenant_id.as_ref() {
            self.validate_response_tenant(tenant_id)?;
        }
        value.validate_status("stop")?;
        Ok(value)
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
        let value = self.decode_envelope::<RemoteTaskAction>(&response)?;
        if let Some(tenant_id) = value.tenant_id.as_ref() {
            self.validate_response_tenant(tenant_id)?;
        }
        value.validate_status("resume")?;
        Ok(value)
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

    fn ensure_tenant(&self, tenant_id: &TenantId) -> RemoteSolverResult<()> {
        if let Some(configured) = self.tenant_id.as_ref()
            && configured != tenant_id
        {
            return Err(RemoteSolverError::invalid_argument(
                "HTTP client tenant does not match the execution tenant",
            ));
        }
        Ok(())
    }

    fn validate_response_tenant(&self, tenant_id: &TenantId) -> RemoteSolverResult<()> {
        if let Some(configured) = self.tenant_id.as_ref()
            && configured != tenant_id
        {
            return Err(RemoteSolverError::new(
                RemoteSolverErrorCode::InvalidArgument,
                "remote response tenant identity does not match the configured tenant",
            ));
        }
        Ok(())
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
    /// 完整调度请求 / Complete scheduling request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduling: Option<SchedulingRequest>,
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
    /// Task-view/result wire schema. V2 views must carry canonical identities.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<String>,
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
    #[serde(default)]
    pub consumed_cost: f64,
    /// 当前切片 ID / Current slice ID.
    #[serde(
        alias = "currentSliceId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub slice_id: Option<SliceId>,
    /// 当前浮点目标值 / Current floating-point objective value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub objective_value: Option<f64>,
    /// 当前精确整数目标值 / Current exact integer objective value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub objective_value_int64: Option<i64>,
    /// 当前最优下界 / Current best bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub best_bound: Option<f64>,
    /// 兼容旧服务端的 bound 字段 / Legacy bound field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound: Option<f64>,
    /// 当前 gap / Current gap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap: Option<f64>,
    /// 当前进度 / Current progress.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<f64>,
    /// 截止时间风险 / Deadline risk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline_risk: Option<f64>,
    /// 截止时间 / Deadline.
    #[serde(
        rename = "deadlineEpochMs",
        alias = "deadline",
        default,
        with = "option_epoch_millis",
        skip_serializing_if = "Option::is_none"
    )]
    pub deadline: Option<SystemTime>,
    /// 调度分发 ID / Dispatch ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dispatch_id: Option<super::domain::DispatchId>,
    /// 求解运行 ID / Solve run ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// 求解 attempt ID / Solve attempt ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<String>,
    /// 结果 artifact 摘要 / Result artifact digest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_digest: Option<String>,
    /// 当前 incumbent 引用 / Current incumbent reference.
    #[serde(
        default,
        deserialize_with = "deserialize_optional_object_ref",
        skip_serializing_if = "Option::is_none"
    )]
    pub incumbent_ref: Option<ObjectRef>,
    /// 模型指纹 / Model fingerprint.
    #[serde(
        default,
        deserialize_with = "deserialize_optional_string_or_fingerprint",
        skip_serializing_if = "Option::is_none"
    )]
    pub model_fingerprint: Option<String>,
    /// 模型指纹 schema / Model fingerprint schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_fingerprint_schema: Option<String>,
    /// 配置指纹 / Configuration fingerprint.
    #[serde(
        default,
        deserialize_with = "deserialize_optional_string_or_fingerprint",
        skip_serializing_if = "Option::is_none"
    )]
    pub configuration_fingerprint: Option<String>,
    /// 配置指纹 schema / Configuration fingerprint schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configuration_fingerprint_schema: Option<String>,
    /// solver 指纹 / Solver fingerprint.
    #[serde(
        default,
        deserialize_with = "deserialize_optional_string_or_fingerprint",
        skip_serializing_if = "Option::is_none"
    )]
    pub solver_fingerprint: Option<String>,
    /// solver 指纹 schema / Solver fingerprint schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solver_fingerprint_schema: Option<String>,
    /// 审计指纹集合 / Audit fingerprints.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fingerprints: BTreeMap<String, String>,
    /// 执行来源 / Execution provenance.
    #[serde(
        default,
        deserialize_with = "deserialize_provenance_map",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub provenance: BTreeMap<String, String>,
    /// Complete cancellation chain retained across attempts.
    #[serde(
        default,
        deserialize_with = "deserialize_cancellation_chain"
    )]
    pub cancellation_chain: Vec<CancellationRecord>,
    /// 实际生效调度信息 / Effective scheduling information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduling: Option<super::domain::SchedulingDecision>,
    /// 当前切片结果 / Current slice outcome.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<super::domain::SliceOutcome>,
    /// 数学问题结论 / Mathematical problem status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem_status: Option<RemoteProblemStatus>,
    /// 终止原因 / Termination reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub termination_reason: Option<RemoteTerminationReason>,
    /// 解存在性 / Solution presence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solution_presence: Option<RemoteSolutionPresence>,
    /// 证明状态 / Proof status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proof_status: Option<super::domain::RemoteProofStatus>,
    /// 求解统计 / Solve statistics.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub statistics: BTreeMap<String, String>,
    /// 结构化诊断 / Structured diagnostics.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub diagnostics: BTreeMap<String, String>,
    /// 指纹 schema 清单 / Fingerprint schema map.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fingerprint_schemas: BTreeMap<String, String>,
    /// 嵌套切片 / Nested slice view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slice: Option<SliceResult>,
}

/// 远程任务操作响应。
/// Remote task action response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteTaskAction {
    /// Task-action wire schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<String>,
    /// 任务 ID / Task ID
    pub task_id: TaskId,
    /// 服务端租户 ID / Tenant ID returned by the server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<TenantId>,
    /// 服务端确认的切片 ID / Slice ID acknowledged by the server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slice_id: Option<SliceId>,
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
    #[serde(
        default,
        deserialize_with = "deserialize_optional_audit_fingerprint",
        skip_serializing_if = "Option::is_none"
    )]
    pub model_fingerprint: Option<AuditFingerprint>,
    /// 生效配置指纹 / Effective configuration fingerprint.
    #[serde(
        default,
        deserialize_with = "deserialize_optional_audit_fingerprint",
        skip_serializing_if = "Option::is_none"
    )]
    pub configuration_fingerprint: Option<AuditFingerprint>,
    /// solver 环境指纹 / Solver environment fingerprint.
    #[serde(
        default,
        deserialize_with = "deserialize_optional_audit_fingerprint",
        skip_serializing_if = "Option::is_none"
    )]
    pub solver_fingerprint: Option<AuditFingerprint>,
    /// solver provenance / Solver provenance.
    #[serde(
        default,
        serialize_with = "serialize_optional_provenance",
        deserialize_with = "deserialize_optional_provenance",
        skip_serializing_if = "Option::is_none"
    )]
    pub provenance: Option<SolverProvenance>,
    /// 跨 attempt 取消链；恢复响应必须显式携带该字段 / Cross-attempt cancellation chain; resume responses must carry it explicitly.
    #[serde(
        default,
        serialize_with = "serialize_optional_cancellation_chain",
        deserialize_with = "deserialize_optional_cancellation_chain",
        skip_serializing_if = "Option::is_none"
    )]
    pub cancellation_chain: Option<Vec<CancellationRecord>>,
    /// 服务端附加消息 / Server message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl RemoteTaskAction {
    fn validate_status(&self, operation: &str) -> RemoteSolverResult<()> {
        reject_unknown_task_status(self.status, operation)
    }
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

/// Decode a fingerprint that may be represented as either a plain value or an object.
/// Kotlin protocol versions have used both forms for task-view fields.
fn deserialize_optional_string_or_fingerprint<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
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
    if let Some(value) = value.as_str() {
        return Ok(Some(value.to_owned()));
    }
    if let Some(value) = value.get("value").and_then(serde_json::Value::as_str) {
        return Ok(Some(value.to_owned()));
    }
    Err(serde::de::Error::custom(
        "fingerprint must be a string or an object containing value",
    ))
}

fn deserialize_optional_audit_fingerprint<'de, D>(
    deserializer: D,
) -> Result<Option<AuditFingerprint>, D::Error>
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
    if let Some(value) = value.as_str() {
        return Ok(Some(AuditFingerprint {
            // The Kotlin wire form can carry the schema/algorithm in sibling fields.  Keep
            // those components unconstrained here; resume validation still compares the digest
            // value and validates them when the object form supplies them inline.
            schema_version: String::new(),
            algorithm: String::new(),
            value: value.to_owned(),
        }));
    }
    serde_json::from_value(value)
        .map(Some)
        .map_err(serde::de::Error::custom)
}

fn deserialize_optional_provenance<'de, D>(
    deserializer: D,
) -> Result<Option<SolverProvenance>, D::Error>
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

    // Rust peers may send the fully typed provenance object.  Kotlin's HTTP adapter sends the
    // flattened string map, so accept that representation as well.
    if let Ok(provenance) = serde_json::from_value::<SolverProvenance>(value.clone()) {
        return Ok(Some(provenance));
    }
    let Some(object) = value.as_object() else {
        return Err(serde::de::Error::custom(
            "provenance must be an object",
        ));
    };
    let text = |keys: &[&str]| {
        keys.iter()
            .find_map(|key| object.get(*key).and_then(serde_json::Value::as_str))
            .map(str::to_owned)
    };
    let parse_usize = |keys: &[&str]| {
        text(keys).and_then(|value| value.parse::<usize>().ok()).or_else(|| {
            keys.iter().find_map(|key| {
                object
                    .get(*key)
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|value| usize::try_from(value).ok())
            })
        })
    };
    let parse_u64 = |keys: &[&str]| {
        text(keys).and_then(|value| value.parse::<u64>().ok()).or_else(|| {
            keys.iter()
                .find_map(|key| object.get(*key).and_then(serde_json::Value::as_u64))
        })
    };
    let parse_bool = |keys: &[&str]| {
        text(keys)
            .and_then(|value| value.parse::<bool>().ok())
            .or_else(|| keys.iter().find_map(|key| object.get(*key).and_then(serde_json::Value::as_bool)))
    };
    let mut environment_summary = ["environmentSummary", "environment_summary"]
        .iter()
        .find_map(|key| {
            object
                .get(*key)
                .and_then(|value| serde_json::from_value::<BTreeMap<String, String>>(value.clone()).ok())
        })
        .unwrap_or_default();
    for (key, value) in object {
        if !matches!(
            key.as_str(),
            "solverId"
                | "solver_id"
                | "solver"
                | "descriptor"
                | "backendName"
                | "backend_name"
                | "backend"
                | "backendVersion"
                | "backend_version"
                | "pluginVersion"
                | "plugin_version"
                | "threadCount"
                | "thread_count"
                | "threads"
                | "randomSeed"
                | "random_seed"
                | "deterministic"
                | "requestedConfiguration"
                | "requested_configuration"
                | "effectiveConfiguration"
                | "effective_configuration"
                | "environmentSummary"
                | "environment_summary"
        ) && let Some(value) = value.as_str()
        {
            environment_summary
                .entry(key.clone())
                .or_insert_with(|| value.to_owned());
        }
    }
    Ok(Some(SolverProvenance {
        solver_id: text(&["solverId", "solver_id", "solver", "descriptor"])
            .unwrap_or_else(|| "unknown".to_owned()),
        backend_name: text(&["backendName", "backend_name", "backend"])
            .unwrap_or_else(|| "unknown".to_owned()),
        backend_version: text(&["backendVersion", "backend_version"]),
        plugin_version: text(&["pluginVersion", "plugin_version"]),
        requested_configuration: object
            .get("requestedConfiguration")
            .or_else(|| object.get("requested_configuration"))
            .and_then(|value| serde_json::from_value(value.clone()).ok())
            .unwrap_or_default(),
        effective_configuration: object
            .get("effectiveConfiguration")
            .or_else(|| object.get("effective_configuration"))
            .and_then(|value| serde_json::from_value(value.clone()).ok())
            .unwrap_or_default(),
        thread_count: parse_usize(&["threadCount", "thread_count", "threads"]),
        random_seed: parse_u64(&["randomSeed", "random_seed"]),
        deterministic: parse_bool(&["deterministic"]),
        environment_summary,
    }))
}

fn deserialize_provenance_map<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    if value.is_null() {
        return Ok(BTreeMap::new());
    }
    if let Ok(provenance) = serde_json::from_value::<SolverProvenance>(value.clone()) {
        return Ok(provenance_to_map(&provenance));
    }
    let Some(object) = value.as_object() else {
        return Err(serde::de::Error::custom("provenance must be an object"));
    };
    let mut result = BTreeMap::new();
    for (key, value) in object {
        match value {
            serde_json::Value::String(value) => {
                result.insert(key.clone(), value.clone());
            }
            serde_json::Value::Bool(value) => {
                result.insert(key.clone(), value.to_string());
            }
            serde_json::Value::Number(value) => {
                result.insert(key.clone(), value.to_string());
            }
            _ => {}
        }
    }
    Ok(result)
}

fn provenance_to_map(provenance: &SolverProvenance) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    values.insert("solverId".to_owned(), provenance.solver_id.clone());
    values.insert("backendName".to_owned(), provenance.backend_name.clone());
    if let Some(value) = provenance.backend_version.as_ref() {
        values.insert("backendVersion".to_owned(), value.clone());
    }
    if let Some(value) = provenance.plugin_version.as_ref() {
        values.insert("pluginVersion".to_owned(), value.clone());
    }
    if let Some(value) = provenance.thread_count {
        values.insert("threadCount".to_owned(), value.to_string());
    }
    if let Some(value) = provenance.random_seed {
        values.insert("randomSeed".to_owned(), value.to_string());
    }
    if let Some(value) = provenance.deterministic {
        values.insert("deterministic".to_owned(), value.to_string());
    }
    values.extend(
        provenance
            .environment_summary
            .iter()
            .map(|(key, value)| (key.clone(), value.clone())),
    );
    values
}

fn serialize_optional_provenance<S>(
    value: &Option<SolverProvenance>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let Some(provenance) = value else {
        return serializer.serialize_none();
    };
    // Kotlin's action wire keeps typed nested maps and numeric fields.  Preserve every field so
    // a decode/encode cycle does not flatten or stringify provenance data.
    let map = serde_json::json!({
        "solverId": &provenance.solver_id,
        "backendName": &provenance.backend_name,
        "backendVersion": &provenance.backend_version,
        "pluginVersion": &provenance.plugin_version,
        "requestedConfiguration": &provenance.requested_configuration,
        "effectiveConfiguration": &provenance.effective_configuration,
        "threadCount": &provenance.thread_count,
        "randomSeed": &provenance.random_seed,
        "deterministic": &provenance.deterministic,
        "environmentSummary": &provenance.environment_summary,
    });
    map.serialize(serializer)
}

fn deserialize_optional_cancellation_chain<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<CancellationRecord>>, D::Error>
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
    if let Ok(chain) = serde_json::from_value::<Vec<CancellationRecord>>(value.clone()) {
        return Ok(Some(chain));
    }
    let Some(entries) = value.as_array() else {
        return Err(serde::de::Error::custom(
            "cancellationChain must be an array",
        ));
    };
    let mut chain = Vec::with_capacity(entries.len());
    for entry in entries {
        let Some(object) = entry.as_object() else {
            return Err(serde::de::Error::custom(
                "cancellationChain entries must be objects",
            ));
        };
        let origin = object
            .get("origin")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| serde::de::Error::custom("cancellationChain entry is missing origin"))?;
        let requested_at_epoch_ms = object
            .get("requestedAtEpochMs")
            .or_else(|| object.get("requested_at_epoch_ms"))
            .and_then(|value| {
                value
                    .as_u64()
                    .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
            })
            .ok_or_else(|| {
                serde::de::Error::custom(
                    "cancellationChain entry is missing requestedAtEpochMs",
                )
            })?;
        chain.push(CancellationRecord {
            origin: origin.to_owned().into(),
            requested_at_epoch_ms,
        });
    }
    Ok(Some(chain))
}

fn deserialize_cancellation_chain<'de, D>(
    deserializer: D,
) -> Result<Vec<CancellationRecord>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(deserialize_optional_cancellation_chain(deserializer)?.unwrap_or_default())
}

fn serialize_optional_cancellation_chain<S>(
    value: &Option<Vec<CancellationRecord>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let Some(chain) = value else {
        return serializer.serialize_none();
    };
    // Kotlin's `requestedAtEpochMs` is a Long, so do not encode it as a quoted string.
    let entries: Vec<serde_json::Value> = chain
        .iter()
        .map(|record| {
            serde_json::json!({
                "origin": record.origin.to_string(),
                "requestedAtEpochMs": record.requested_at_epoch_ms,
            })
        })
        .collect();
    entries.serialize(serializer)
}

fn reject_unknown_task_status(status: TaskStatus, operation: &str) -> RemoteSolverResult<()> {
    if status == TaskStatus::Unknown {
        return Err(RemoteSolverError::invalid_argument(format!(
            "remote {} response contains an unknown task status",
            operation
        )));
    }
    Ok(())
}

fn is_terminal_status(status: TaskStatus) -> bool {
    matches!(
        status,
        TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Stopped
    )
}

/// 校验远程 CP 提交所需的能力 / Validate capabilities required by a remote CP submission.
fn validate_cp_capabilities(capabilities: &RemoteSolverCapabilities) -> RemoteSolverResult<()> {
    let schema_major = capabilities
        .schema_version
        .split('.')
        .next()
        .and_then(|value| value.parse::<u64>().ok());
    let supports_schema = schema_major == Some(1);
    let supports_protocol = capabilities.protocol_versions.contains("2.0");
    let supports_cp = capabilities
        .supported_model_types
        .iter()
        .any(|model_type| model_type.eq_ignore_ascii_case("CP"));
    let supports_portable_checkpoint = capabilities.supports_portable_checkpoint;
    if supports_schema && supports_protocol && supports_cp && supports_portable_checkpoint {
        return Ok(());
    }

    Err(RemoteSolverError::invalid_argument(
        "remote solver does not advertise the required CP protocol, model, and portable-checkpoint capabilities",
    )
    .with_metadata([
        ("capabilitySchemaVersion", capabilities.schema_version.clone()),
        ("requiredCapabilitySchemaMajor", "1".to_owned()),
        ("requiredProtocol", "2.0".to_owned()),
        (
            "protocolVersions",
            capabilities
                .protocol_versions
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(","),
        ),
        (
            "supportedModelTypes",
            capabilities
                .supported_model_types
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(","),
        ),
        (
            "supportsPortableCheckpoint",
            capabilities.supports_portable_checkpoint.to_string(),
        ),
    ]))
}

/// 校验 resume action 的接受状态和完整身份链 / Validate the resume action acceptance and identity chain.
fn validate_resume_action(
    action: &RemoteTaskAction,
    source_checkpoint: &SolveCheckpoint,
    child_attempt_id: &SliceId,
    expected_task_id: &TaskId,
    expected_tenant_id: &TenantId,
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
    if action.task_id != *expected_task_id {
        return Err(RemoteSolverError::checkpoint_restore(
            "remote resume action task identity does not match the requested task",
        ));
    }
    if action.tenant_id.as_ref() != Some(expected_tenant_id) {
        return Err(RemoteSolverError::checkpoint_restore(
            "remote resume action tenant identity does not match the requested tenant",
        ));
    }

    action.validate_status("resume")?;

    if action.run_id.as_deref() != Some(source_checkpoint.run_id.as_str()) {
        return Err(RemoteSolverError::checkpoint_restore(
            "remote resume action run identity does not match the source checkpoint",
        ));
    }
    if action.attempt_id.as_deref() != Some(source_checkpoint.attempt_id.as_str()) {
        return Err(RemoteSolverError::checkpoint_restore(
            "remote resume action attempt identity does not match the source checkpoint",
        ));
    }
    if let Some(action_slice_id) = action.slice_id.as_ref()
        && action_slice_id != child_attempt_id
    {
        return Err(RemoteSolverError::checkpoint_restore(
            "remote resume action slice identity does not match the requested child attempt",
        ));
    }
    for (actual, expected) in [
        (
            action.model_fingerprint.as_ref(),
            &source_checkpoint.model_fingerprint,
        ),
        (
            action.configuration_fingerprint.as_ref(),
            &source_checkpoint.configuration_fingerprint,
        ),
        (
            action.solver_fingerprint.as_ref(),
            &source_checkpoint.solver_fingerprint,
        ),
    ] {
        let matches = actual.is_some_and(|actual| {
            actual.value == expected.value
                && (actual.schema_version.is_empty()
                    || actual.schema_version == expected.schema_version)
                && (actual.algorithm.is_empty() || actual.algorithm.eq_ignore_ascii_case(&expected.algorithm))
        });
        if !matches {
            return Err(RemoteSolverError::checkpoint_restore(
                "remote resume action fingerprints do not match the source checkpoint",
            ));
        }
    }
    if action.provenance.as_ref() != Some(&source_checkpoint.provenance) {
        return Err(RemoteSolverError::checkpoint_restore(
            "remote resume action provenance does not match the source checkpoint",
        ));
    }
    if action.cancellation_chain.as_ref() != Some(&source_checkpoint.cancellation_chain) {
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
    force_timeout: bool,
) -> RemoteSolverResult<SliceResult> {
    let nested = view.slice.as_ref();
    let schema_version = nested
        .and_then(|slice| slice.schema_version.clone())
        .or_else(|| view.schema_version.clone());
    let run_id = nested
        .and_then(|slice| slice.run_id.clone())
        .or_else(|| view.run_id.clone());
    let attempt_id = nested
        .and_then(|slice| slice.attempt_id.clone())
        .or_else(|| view.attempt_id.clone());
    let is_v2 = schema_version
        .as_deref()
        .is_some_and(|version| version.split('.').next() == Some("2"));
    if is_v2 && (run_id.as_deref().is_none_or(str::is_empty)
        || attempt_id.as_deref().is_none_or(str::is_empty))
    {
        return Err(RemoteSolverError::invalid_argument(
            "remote V2 slice view is missing canonical runId or attemptId",
        ));
    }
    let effective_slice_id = view
        .slice_id
        .clone()
        .or_else(|| nested.map(|slice| slice.slice_id.clone()))
        .or_else(|| attempt_id.as_deref().map(SliceId::of).transpose().ok().flatten())
        .unwrap_or_else(|| slice_id.clone());
    let checkpoint_ref = nested
        .and_then(|slice| slice.checkpoint_ref.clone())
        .or_else(|| view.latest_checkpoint_ref.clone())
        .or_else(|| {
            view.scheduling
                .as_ref()
                .and_then(|s| s.checkpoint_ref.clone())
        });
    let incumbent_ref = nested
        .and_then(|slice| slice.incumbent_ref.clone())
        .or_else(|| view.incumbent_ref.clone())
        .or_else(|| {
            view.scheduling
                .as_ref()
                .and_then(|s| s.incumbent_ref.clone())
        });
    let result_ref = nested
        .and_then(|slice| slice.result_ref.clone())
        .or_else(|| view.latest_result_ref.clone());
    let feasible = nested.map(|slice| slice.feasible).unwrap_or_else(|| {
        matches!(
            view.solution_presence,
            Some(RemoteSolutionPresence::Incumbent | RemoteSolutionPresence::Optimal)
        ) || incumbent_ref.is_some()
    });
    let outcome = nested
        .and_then(|slice| slice.outcome)
        .or(view.outcome)
        .or_else(|| {
            if force_timeout {
                Some(super::domain::SliceOutcome::Preempted)
            } else {
                match view.status {
                    TaskStatus::Stopped => Some(super::domain::SliceOutcome::Cancelled),
                    TaskStatus::Failed => Some(super::domain::SliceOutcome::Failed),
                    TaskStatus::Completed => Some(super::domain::SliceOutcome::Completed),
                    _ if checkpoint_ref.is_some() => {
                        Some(super::domain::SliceOutcome::Checkpointed)
                    }
                    _ if feasible => Some(super::domain::SliceOutcome::Resumable),
                    _ => Some(super::domain::SliceOutcome::Preempted),
                }
            }
        });
    let termination_reason = if force_timeout {
        Some(RemoteTerminationReason::TimeLimit)
    } else {
        nested
            .and_then(|slice| slice.termination_reason)
            .or(view.termination_reason)
            .or_else(|| match view.status {
                TaskStatus::Stopped => Some(RemoteTerminationReason::Cancelled),
                TaskStatus::Failed => Some(RemoteTerminationReason::BackendFailure),
                TaskStatus::Completed => Some(RemoteTerminationReason::Completed),
                _ => Some(RemoteTerminationReason::TimeLimit),
            })
    };
    let mut scheduling = nested
        .and_then(|slice| slice.scheduling.clone())
        .or_else(|| view.scheduling.clone());
    if let Some(decision) = scheduling.as_mut() {
        if decision.task_id.is_none() {
            decision.task_id = Some(view.task_id.clone());
        }
        if decision.slice_id.is_none() {
            decision.slice_id = Some(effective_slice_id.clone());
        }
        if decision.node_id.is_none() {
            decision.node_id = view.current_node_id.clone();
        }
        if decision.checkpoint_ref.is_none() {
            decision.checkpoint_ref = checkpoint_ref.clone();
        }
        if decision.incumbent_ref.is_none() {
            decision.incumbent_ref = incumbent_ref.clone();
        }
        if decision.model_fingerprint.is_none() {
            decision.model_fingerprint = nested
                .and_then(|slice| slice.model_fingerprint.clone())
                .or_else(|| view.model_fingerprint.clone());
        }
        if decision.outcome.is_none() {
            decision.outcome = outcome;
        }
    }
    let mut statistics = view.statistics.clone();
    if let Some(slice) = nested {
        statistics.extend(slice.statistics.clone());
    }
    if let Some(best_bound) = nested.and_then(|slice| slice.statistics.get("bestBound")) {
        statistics.insert("bestBound".to_owned(), best_bound.to_owned());
    } else if let Some(value) = view.best_bound.or(view.bound) {
        statistics.insert("bestBound".to_owned(), value.to_string());
    }
    let mut diagnostics = view.diagnostics.clone();
    if let Some(slice) = nested {
        diagnostics.extend(slice.diagnostics.clone());
    }
    let mut fingerprints = view.fingerprints.clone();
    if let Some(value) = view.model_fingerprint.clone() {
        fingerprints.entry("model".to_owned()).or_insert(value);
    }
    if let Some(value) = view.configuration_fingerprint.clone() {
        fingerprints
            .entry("configuration".to_owned())
            .or_insert(value);
    }
    if let Some(value) = view.solver_fingerprint.clone() {
        fingerprints.entry("solver".to_owned()).or_insert(value);
    }
    let mut fingerprint_schemas = view.fingerprint_schemas.clone();
    if let Some(value) = view.model_fingerprint_schema.clone() {
        fingerprint_schemas
            .entry("model".to_owned())
            .or_insert(value);
    }
    if let Some(value) = view.configuration_fingerprint_schema.clone() {
        fingerprint_schemas
            .entry("configuration".to_owned())
            .or_insert(value);
    }
    if let Some(value) = view.solver_fingerprint_schema.clone() {
        fingerprint_schemas
            .entry("solver".to_owned())
            .or_insert(value);
    }
    Ok(SliceResult {
        slice_id: effective_slice_id,
        completed,
        feasible,
        objective_value: nested
            .and_then(|slice| slice.objective_value)
            .or(view.objective_value),
        gap: nested.and_then(|slice| slice.gap).or(view.gap),
        elapsed: nested.map(|slice| slice.elapsed).unwrap_or(elapsed),
        message: nested
            .and_then(|slice| slice.message.clone())
            .or_else(|| {
                force_timeout.then(|| {
                    format!(
                        "Remote slice quantum expired while task status was {:?}.",
                        view.status
                    )
                })
            })
            .or_else(|| Some(format!("Remote task status is {:?}.", view.status))),
        schema_version,
        problem_status: nested
            .and_then(|slice| slice.problem_status)
            .or(view.problem_status)
            .or_else(|| feasible.then_some(RemoteProblemStatus::Feasible)),
        termination_reason,
        solution_presence: nested
            .and_then(|slice| slice.solution_presence)
            .or(view.solution_presence)
            .or_else(|| {
                Some(if feasible {
                    RemoteSolutionPresence::Incumbent
                } else {
                    RemoteSolutionPresence::None
                })
            }),
        proof_status: nested
            .and_then(|slice| slice.proof_status)
            .or(view.proof_status),
        result_ref,
        provenance: if nested.is_some_and(|slice| !slice.provenance.is_empty()) {
            nested
                .map(|slice| slice.provenance.clone())
                .unwrap_or_default()
        } else {
            view.provenance.clone()
        },
        fingerprints: if nested.is_some_and(|slice| !slice.fingerprints.is_empty()) {
            nested
                .map(|slice| slice.fingerprints.clone())
                .unwrap_or_default()
        } else {
            fingerprints
        },
        fingerprint_schemas: if nested.is_some_and(|slice| !slice.fingerprint_schemas.is_empty()) {
            nested
                .map(|slice| slice.fingerprint_schemas.clone())
                .unwrap_or_default()
        } else {
            fingerprint_schemas
        },
        statistics,
        diagnostics,
        run_id,
        attempt_id,
        artifact_digest: nested
            .and_then(|slice| slice.artifact_digest.clone())
            .or_else(|| view.artifact_digest.clone()),
        objective_value_int64: nested
            .and_then(|slice| slice.objective_value_int64)
            .or(view.objective_value_int64),
        checkpoint_ref,
        incumbent_ref,
        model_fingerprint: nested
            .and_then(|slice| slice.model_fingerprint.clone())
            .or_else(|| view.model_fingerprint.clone()),
        scheduling,
        outcome,
        cancellation_chain: nested
            .and_then(|slice| (!slice.cancellation_chain.is_empty()).then(|| slice.cancellation_chain.clone()))
            .unwrap_or_else(|| view.cancellation_chain.clone()),
    })
}

fn solve_result_from_view(
    view: &RemoteTaskView,
    slice_id: &SliceId,
) -> RemoteSolverResult<SolveResult> {
    let mut extension = BTreeMap::new();
    extension.insert("remote.taskStatus".to_owned(), format!("{:?}", view.status));
    extension.insert(
        "remote.consumedCost".to_owned(),
        view.consumed_cost.to_string(),
    );
    if let Some(progress) = view.progress {
        extension.insert("remote.progress".to_owned(), progress.to_string());
    }
    let checkpoint_ref = view.latest_checkpoint_ref.clone().or_else(|| {
        view.scheduling
            .as_ref()
            .and_then(|s| s.checkpoint_ref.clone())
    });
    let incumbent_ref = view.incumbent_ref.clone().or_else(|| {
        view.scheduling
            .as_ref()
            .and_then(|s| s.incumbent_ref.clone())
    });
    let feasible = matches!(
        view.solution_presence,
        Some(RemoteSolutionPresence::Incumbent | RemoteSolutionPresence::Optimal)
    ) || incumbent_ref.is_some();
    let outcome = view.outcome.or_else(|| match view.status {
        TaskStatus::Completed => Some(super::domain::SliceOutcome::Completed),
        TaskStatus::Stopped => Some(super::domain::SliceOutcome::Cancelled),
        TaskStatus::Failed => Some(super::domain::SliceOutcome::Failed),
        _ if checkpoint_ref.is_some() => Some(super::domain::SliceOutcome::Checkpointed),
        _ if feasible => Some(super::domain::SliceOutcome::Resumable),
        _ => None,
    });
    let termination_reason = view.termination_reason.or_else(|| match view.status {
        TaskStatus::Completed => Some(RemoteTerminationReason::Completed),
        TaskStatus::Stopped => Some(RemoteTerminationReason::Cancelled),
        TaskStatus::Failed => Some(RemoteTerminationReason::BackendFailure),
        _ => None,
    });
    let mut scheduling = view.scheduling.clone();
    if let Some(decision) = scheduling.as_mut() {
        decision.task_id.get_or_insert_with(|| view.task_id.clone());
        decision
            .slice_id
            .get_or_insert_with(|| view.slice_id.clone().unwrap_or_else(|| slice_id.clone()));
        if decision.node_id.is_none() {
            decision.node_id = view.current_node_id.clone();
        }
        if decision.checkpoint_ref.is_none() {
            decision.checkpoint_ref = checkpoint_ref.clone();
        }
        if decision.incumbent_ref.is_none() {
            decision.incumbent_ref = incumbent_ref.clone();
        }
        if decision.model_fingerprint.is_none() {
            decision.model_fingerprint = view.model_fingerprint.clone();
        }
        if decision.outcome.is_none() {
            decision.outcome = outcome;
        }
    }
    let elapsed = view
        .slice
        .as_ref()
        .map(|slice| slice.elapsed)
        .unwrap_or_default();
    let mut statistics = view.statistics.clone();
    if let Some(value) = view.best_bound.or(view.bound) {
        statistics
            .entry("bestBound".to_owned())
            .or_insert_with(|| value.to_string());
    }
    let schema_version = view
        .schema_version
        .clone()
        .or_else(|| {
            view
                .slice
                .as_ref()
                .and_then(|slice| slice.schema_version.clone())
        });
    let is_v2 = schema_version
        .as_deref()
        .is_some_and(|version| version.split('.').next() == Some("2"));
    let run_id = view
        .run_id
        .clone()
        .or_else(|| view.slice.as_ref().and_then(|slice| slice.run_id.clone()));
    let attempt_id = view
        .attempt_id
        .clone()
        .or_else(|| view.slice.as_ref().and_then(|slice| slice.attempt_id.clone()));
    if is_v2 && (run_id.as_deref().is_none_or(str::is_empty)
        || attempt_id.as_deref().is_none_or(str::is_empty))
    {
        return Err(RemoteSolverError::invalid_argument(
            "remote V2 task view is missing canonical runId or attemptId",
        ));
    }
    let schema_version = view
        .slice
        .as_ref()
        .and_then(|slice| slice.schema_version.clone())
        .or(schema_version);
    let mut fingerprints = view.fingerprints.clone();
    if let Some(value) = view.model_fingerprint.clone() {
        fingerprints.entry("model".to_owned()).or_insert(value);
    }
    if let Some(value) = view.configuration_fingerprint.clone() {
        fingerprints
            .entry("configuration".to_owned())
            .or_insert(value);
    }
    if let Some(value) = view.solver_fingerprint.clone() {
        fingerprints.entry("solver".to_owned()).or_insert(value);
    }
    let mut fingerprint_schemas = view.fingerprint_schemas.clone();
    if let Some(value) = view.model_fingerprint_schema.clone() {
        fingerprint_schemas
            .entry("model".to_owned())
            .or_insert(value);
    }
    if let Some(value) = view.configuration_fingerprint_schema.clone() {
        fingerprint_schemas
            .entry("configuration".to_owned())
            .or_insert(value);
    }
    if let Some(value) = view.solver_fingerprint_schema.clone() {
        fingerprint_schemas
            .entry("solver".to_owned())
            .or_insert(value);
    }
    Ok(SolveResult {
        // 没有结果 artifact 时只能返回未知数学结论 / Without a result artifact, only an
        // unknown mathematical conclusion can be returned.
        feasible,
        optimal: view.solution_presence == Some(RemoteSolutionPresence::Optimal),
        objective_value: view.objective_value,
        gap: view.gap,
        elapsed,
        checkpoint_ref,
        checkpoint_metadata: None,
        result_ref: view.latest_result_ref.clone(),
        run_id,
        attempt_id,
        artifact_digest: view.artifact_digest.clone(),
        report: None,
        message: view
            .slice
            .as_ref()
            .and_then(|slice| slice.message.clone())
            .or_else(|| Some(format!("Remote task status is {:?}.", view.status))),
        extension,
        schema_version,
        problem_status: view
            .problem_status
            .or_else(|| feasible.then_some(RemoteProblemStatus::Feasible)),
        termination_reason,
        solution_presence: view.solution_presence.or_else(|| {
            Some(if feasible {
                RemoteSolutionPresence::Incumbent
            } else {
                RemoteSolutionPresence::None
            })
        }),
        proof_status: view.proof_status,
        provenance: view.provenance.clone(),
        fingerprints,
        fingerprint_schemas,
        statistics,
        diagnostics: view.diagnostics.clone(),
        objective_value_int64: view.objective_value_int64,
        incumbent_ref,
        model_fingerprint: view.model_fingerprint.clone(),
        scheduling,
        outcome,
        cancellation_chain: view.cancellation_chain.clone(),
    })
}

fn solve_result_from_solution(
    solution: SerializedSolution,
    view: &RemoteTaskView,
    checkpoint_ref: Option<ObjectRef>,
    result_ref: Option<ObjectRef>,
    task_id: &TaskId,
    _slice_id: &SliceId,
) -> RemoteSolverResult<SolveResult> {
    let schema_version = solution
        .schema_version
        .clone()
        .or_else(|| view.schema_version.clone());
    let is_v2 = schema_version
        .as_deref()
        .is_some_and(|version| version.split('.').next() == Some("2"));
    let run_id = solution.run_id.clone().or_else(|| {
        solution
            .report
            .as_ref()
            .and_then(|report| report.run_id.clone())
    });
    let attempt_id = solution.attempt_id.clone().or_else(|| {
        solution
            .report
            .as_ref()
            .and_then(|report| report.attempt_id.clone())
    });
    let run_id = run_id.or_else(|| view.run_id.clone());
    let attempt_id = attempt_id.or_else(|| view.attempt_id.clone());
    if is_v2 && (run_id.as_deref().is_none_or(str::is_empty)
        || attempt_id.as_deref().is_none_or(str::is_empty))
    {
        return Err(RemoteSolverError::invalid_argument(
            "remote V2 result artifact is missing canonical runId or attemptId",
        ));
    }
    let artifact_digest = solution
        .artifact_digest
        .clone()
        .or_else(|| {
            solution
                .report
                .as_ref()
                .and_then(|report| report.artifact_digest.clone())
        })
        .or_else(|| view.artifact_digest.clone());
    let mut statistics = view.statistics.clone();
    statistics.extend(solution.statistics.clone());
    let mut diagnostics = view.diagnostics.clone();
    diagnostics.extend(solution.diagnostics.clone());
    Ok(SolveResult {
        feasible: solution.feasible,
        optimal: solution.optimal,
        objective_value: solution.objective_value.or(view.objective_value),
        gap: solution.gap.or(view.gap),
        elapsed: solution.elapsed,
        checkpoint_ref,
        checkpoint_metadata: None,
        result_ref,
        run_id,
        attempt_id,
        artifact_digest,
        report: solution.report,
        message: solution.message,
        extension: BTreeMap::new(),
        schema_version,
        problem_status: solution.problem_status.or(view.problem_status),
        termination_reason: solution.termination_reason.or(view.termination_reason),
        solution_presence: solution.solution_presence.or(view.solution_presence),
        proof_status: solution.proof_status.or(view.proof_status),
        provenance: if solution.provenance.is_empty() {
            view.provenance.clone()
        } else {
            solution.provenance
        },
        fingerprints: if solution.fingerprints.is_empty() {
            view.fingerprints.clone()
        } else {
            solution.fingerprints
        },
        fingerprint_schemas: if solution.fingerprint_schemas.is_empty() {
            view.fingerprint_schemas.clone()
        } else {
            solution.fingerprint_schemas
        },
        statistics,
        diagnostics,
        objective_value_int64: solution
            .objective_value_int64
            .or(view.objective_value_int64),
        incumbent_ref: view.incumbent_ref.clone().or_else(|| {
            view.scheduling
                .as_ref()
                .and_then(|s| s.incumbent_ref.clone())
        }),
        model_fingerprint: view.model_fingerprint.clone().or_else(|| {
            view.scheduling
                .as_ref()
                .and_then(|s| s.model_fingerprint.clone())
        }),
        scheduling: view.scheduling.clone(),
        outcome: view.outcome,
        cancellation_chain: view.cancellation_chain.clone(),
    })
}

#[cfg(test)]
mod tests {
    use ospf_rust_core::solver::{AuditFingerprint, SolveCheckpoint, SolverProvenance};
    use std::collections::HashMap;
    use std::sync::Mutex;

    use super::*;
    use crate::solver::remote::domain::{ModelData, SerializedLinearModel};

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
        slice_id: &str,
    ) -> RemoteTaskAction {
        RemoteTaskAction {
            task_id: TaskId::of(task_id).expect("task id should be valid"),
            tenant_id: Some(TenantId::of("tenant-1").expect("tenant id should be valid")),
            schema_version: Some("2.0".to_owned()),
            slice_id: Some(SliceId::of(slice_id).expect("slice id should be valid")),
            accepted: true,
            status: TaskStatus::Running,
            run_id: Some(checkpoint.run_id.clone()),
            attempt_id: Some(checkpoint.attempt_id.clone()),
            model_fingerprint: Some(checkpoint.model_fingerprint.clone()),
            configuration_fingerprint: Some(checkpoint.configuration_fingerprint.clone()),
            solver_fingerprint: Some(checkpoint.solver_fingerprint.clone()),
            provenance: Some(checkpoint.provenance.clone()),
            cancellation_chain: Some(checkpoint.cancellation_chain.clone()),
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
                scheduling: None,
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
    async fn http_client_probes_kotlin_capabilities_endpoint() {
        let transport = FakeTransport::new(vec![response(
            r#"{"code":"OK","message":"success","data":{"schemaVersion":"1.0","protocolVersions":["2.0"],"supportedModelTypes":["CP","LINEAR"],"supportsPortableCheckpoint":true,"supportsNativeCheckpoint":false}}"#,
        )]);
        let requests = transport.requests();
        let client = RemoteSolverHttpClient::new("http://localhost", transport).unwrap();

        let capabilities = client.probe_capabilities().await.unwrap();

        assert_eq!(capabilities.schema_version, "1.0");
        assert!(capabilities.protocol_versions.contains("2.0"));
        assert!(capabilities.supported_model_types.contains("CP"));
        assert!(capabilities.supports_portable_checkpoint);
        assert!(!capabilities.supports_native_checkpoint);
        let requests = requests.lock().unwrap();
        assert_eq!(requests[0].method, "GET");
        assert_eq!(requests[0].url, "http://localhost/api/v1/capabilities");
        assert!(requests[0].body.is_none());
    }

    #[test]
    fn resume_action_without_identity_fields_is_rejected() {
        let checkpoint = checkpoint_fixture("task-1", "slice-0");
        let action: RemoteTaskAction =
            serde_json::from_str(
                r#"{"taskId":"task-1","tenantId":"tenant-1","status":"RUNNING"}"#,
            )
            .unwrap();

        let error = validate_resume_action(
            &action,
            &checkpoint,
            &SliceId::of("slice-1").unwrap(),
            &TaskId::of("task-1").unwrap(),
            &TenantId::of("tenant-1").unwrap(),
        )
        .expect_err("resume identity fields are required for strict validation");
        assert!(error.to_string().contains("run identity"));
    }

    #[test]
    fn resume_action_tenant_mismatch_is_rejected() {
        let checkpoint = checkpoint_fixture("task-1", "slice-0");
        let mut action = accepted_resume_action("task-1", &checkpoint, "slice-1");
        action.tenant_id = Some(TenantId::of("tenant-2").unwrap());

        let error = validate_resume_action(
            &action,
            &checkpoint,
            &SliceId::of("slice-1").unwrap(),
            &TaskId::of("task-1").unwrap(),
            &TenantId::of("tenant-1").unwrap(),
        )
        .expect_err("resume action from another tenant must be rejected");

        assert_eq!(error.code, RemoteSolverErrorCode::CheckpointRestoreFailed);
        assert!(error.to_string().contains("tenant identity"));
    }

    #[test]
    fn task_action_wire_round_trip_preserves_nested_provenance_and_cancellation() {
        let encoded = r#"{
            "taskId":"task-1",
            "tenantId":"tenant-1",
            "status":"RUNNING",
            "provenance":{
                "solverId":"scip-cp",
                "backendName":"SCIP",
                "backendVersion":"9.2.4",
                "pluginVersion":"ospf-scip-1",
                "requestedConfiguration":{"threads":"4","seed":"17"},
                "effectiveConfiguration":{"threads":"2","seed":"17"},
                "threadCount":2,
                "randomSeed":17,
                "deterministic":true,
                "environmentSummary":{"os":"linux","arch":"x86_64"}
            },
            "cancellationChain":[
                {"origin":"USER","requestedAtEpochMs":10},
                {"origin":"REMOTE_STOP","requestedAtEpochMs":20}
            ]
        }"#;
        let action: RemoteTaskAction = serde_json::from_str(encoded).unwrap();
        let provenance = action.provenance.as_ref().expect("provenance");
        assert_eq!(provenance.requested_configuration["threads"], "4");
        assert_eq!(provenance.effective_configuration["threads"], "2");
        assert_eq!(provenance.environment_summary["arch"], "x86_64");
        assert_eq!(action.cancellation_chain.as_ref().unwrap().len(), 2);
        assert_eq!(
            action.cancellation_chain.as_ref().unwrap()[1].requested_at_epoch_ms,
            20
        );

        let round_trip = serde_json::to_value(&action).unwrap();
        assert_eq!(round_trip["provenance"]["requestedConfiguration"]["seed"], "17");
        assert_eq!(round_trip["provenance"]["environmentSummary"]["os"], "linux");
        assert_eq!(
            round_trip["cancellationChain"][1]["requestedAtEpochMs"],
            20
        );
        let decoded: RemoteTaskAction = serde_json::from_value(round_trip).unwrap();
        assert_eq!(decoded, action);
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
        let payload = SolvePayload::from_linear_model(
            super::super::domain::SerializedLinearModel::empty("m"),
        )
        .with_scheduling(SchedulingRequest {
            complexity: Some(TaskComplexity::Complex),
            time_sensitivity: Some(TimeSensitivity::Realtime),
            priority: Some(7),
            deadline: Some(SystemTime::UNIX_EPOCH + Duration::from_millis(1_700_000_000_123)),
            budget_scope: Some(BudgetScopeId::of("budget-1").unwrap()),
            budget_limit: Some(4.5),
            ..SchedulingRequest::default()
        });

        let handle = port
            .start(
                &payload,
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
        assert_eq!(body["complexity"], "COMPLEX");
        assert_eq!(body["timeSensitivity"], "REALTIME");
        assert_eq!(body["priority"], 7);
        assert_eq!(body["deadlineEpochMs"], 1_700_000_000_123u64);
        assert_eq!(body["budgetScope"], "budget-1");
        assert_eq!(body["budgetLimit"], 4.5);
        assert!(
            stored
                .lock()
                .unwrap()
                .contains_key("remote-solver/payloads/tenant-1/task-1/slice-1.json")
        );
    }

    #[tokio::test]
    async fn http_execution_port_preserves_canonical_task_id_after_request_id_differs() {
        let transport = FakeTransport::new(vec![
            response(
                r#"{"code":"OK","message":"ok","data":{"taskId":"server-task-9","accepted":true,"status":"ACCEPTED","message":"accepted"}}"#,
            ),
            response(
                r#"{"code":"OK","message":"ok","data":{"taskId":"server-task-9","tenantId":"tenant-1","status":"COMPLETED","sliceId":"attempt-9","runId":"run-9","attemptId":"attempt-9","consumedCost":1.0}}"#,
            ),
            response(
                r#"{"code":"OK","message":"ok","data":{"taskId":"server-task-9","tenantId":"tenant-1","status":"COMPLETED","sliceId":"attempt-9","runId":"run-9","attemptId":"attempt-9","consumedCost":1.0}}"#,
            ),
            response(
                r#"{"code":"OK","message":"ok","data":{"taskId":"server-task-9","tenantId":"tenant-1","accepted":true,"status":"STOPPED","runId":"run-9","attemptId":"attempt-9"}}"#,
            ),
        ]);
        let requests = transport.requests();
        let client = RemoteSolverHttpClient::new("http://localhost", transport).unwrap();
        let port = RemoteSolverHttpExecutionPort::new(client, FakeStorage::default());
        let caller_task_id = TaskId::of("caller-task-9").unwrap();
        let handle = port
            .start(
                &SolvePayload::from_linear_model(SerializedLinearModel::empty("m")),
                &caller_task_id,
                &SliceId::of("slice-9").unwrap(),
                &NodeId::of("node-9").unwrap(),
                &TenantId::of("tenant-1").unwrap(),
            )
            .await
            .expect("server should accept the request");

        assert_eq!(handle.task_id.value(), "server-task-9");
        let slice = port
            .await_slice_end(&handle, Duration::from_millis(20))
            .await
            .expect("canonical task should be queried");
        assert!(slice.completed);
        let result = port
            .fetch_final_result(&handle)
            .await
            .expect("canonical result should be fetched")
            .expect("completed task should have a result projection");
        assert_eq!(result.run_id.as_deref(), Some("run-9"));
        assert_eq!(result.attempt_id.as_deref(), Some("attempt-9"));
        let acknowledgement = port
            .stop(&handle)
            .await
            .expect("canonical task should be stopped");
        assert_eq!(acknowledgement.task_id.value(), "server-task-9");

        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 4);
        assert_eq!(requests[0].url, "http://localhost/api/v1/tasks");
        let submit_body: serde_json::Value =
            serde_json::from_str(requests[0].body.as_deref().unwrap()).unwrap();
        assert_eq!(submit_body["requestId"], "caller-task-9");
        assert_eq!(requests[1].url, "http://localhost/api/v1/tasks/server-task-9");
        assert_eq!(requests[2].url, "http://localhost/api/v1/tasks/server-task-9");
        assert_eq!(
            requests[3].url,
            "http://localhost/api/v1/tasks/server-task-9/stop"
        );
    }

    #[tokio::test]
    async fn http_execution_port_probes_cp_capabilities_before_storing_or_submitting() {
        let transport = FakeTransport::new(vec![response(
            r#"{"code":"OK","message":"success","data":{"schemaVersion":"1.0","protocolVersions":["2.0"],"supportedModelTypes":["LINEAR"],"supportsPortableCheckpoint":true,"supportsNativeCheckpoint":false}}"#,
        )]);
        let requests = transport.requests();
        let storage = FakeStorage::default();
        let stored = storage.objects.clone();
        let client = RemoteSolverHttpClient::new("http://localhost", transport).unwrap();
        let port = RemoteSolverHttpExecutionPort::new(client, storage)
            .with_resume_mode(RemoteSolverHttpResumeMode::Latest);
        let payload = SolvePayload::new(ModelData::raw(b"{}".to_vec(), "ospf-cp-snapshot-json"));

        let error = port
            .start(
                &payload,
                &TaskId::of("task-cp").unwrap(),
                &SliceId::of("slice-1").unwrap(),
                &NodeId::of("node-1").unwrap(),
                &TenantId::of("tenant-1").unwrap(),
            )
            .await
            .expect_err("CP must be rejected when the server omits CP capability");

        assert_eq!(error.code, RemoteSolverErrorCode::InvalidArgument);
        assert_eq!(requests.lock().unwrap().len(), 1);
        let requests = requests.lock().unwrap();
        assert_eq!(requests[0].method, "GET");
        assert_eq!(requests[0].url, "http://localhost/api/v1/capabilities");
        assert!(stored.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn http_execution_port_probes_cp_capabilities_before_task_post() {
        let transport = FakeTransport::new(vec![
            response(
                r#"{"code":"OK","message":"success","data":{"schemaVersion":"1.0","protocolVersions":["2.0"],"supportedModelTypes":["CP"],"supportsPortableCheckpoint":true,"supportsNativeCheckpoint":false}}"#,
            ),
            response(
                r#"{"code":"OK","message":"ok","data":{"taskId":"task-cp","accepted":true,"status":"ACCEPTED","message":"accepted"}}"#,
            ),
        ]);
        let requests = transport.requests();
        let storage = FakeStorage::default();
        let stored = storage.objects.clone();
        let client = RemoteSolverHttpClient::new("http://localhost", transport).unwrap();
        let port = RemoteSolverHttpExecutionPort::new(client, storage);
        let payload = SolvePayload::new(ModelData::raw(b"{}".to_vec(), "ospf-cp-snapshot-json"));

        let handle = port
            .start(
                &payload,
                &TaskId::of("task-cp").unwrap(),
                &SliceId::of("slice-1").unwrap(),
                &NodeId::of("node-1").unwrap(),
                &TenantId::of("tenant-1").unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(handle.task_id.value(), "task-cp");
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].method, "GET");
        assert_eq!(requests[0].url, "http://localhost/api/v1/capabilities");
        assert_eq!(requests[1].method, "POST");
        assert_eq!(requests[1].url, "http://localhost/api/v1/tasks");
        assert!(
            stored
                .lock()
                .unwrap()
                .contains_key("remote-solver/payloads/tenant-1/task-cp/slice-1.json")
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
            scheduling: None,
        };

        let slice = port
            .await_slice_end(&handle, Duration::from_millis(50))
            .await
            .unwrap();

        assert!(slice.completed);
        assert!(!slice.feasible);
    }

    #[tokio::test]
    async fn http_execution_port_returns_server_suspension_without_local_stop() {
        let transport = FakeTransport::new(vec![
            response(
                r#"{"code":"OK","message":"ok","data":{"taskId":"task-1","tenantId":"tenant-1","status":"RUNNING","consumedCost":0.0}}"#,
            ),
            response(
                r#"{"code":"OK","message":"ok","data":{"taskId":"task-1","tenantId":"tenant-1","status":"SUSPENDED","latestCheckpointPath":"checkpoint.json","consumedCost":0.5}}"#,
            ),
        ]);
        let requests = transport.requests();
        let client = RemoteSolverHttpClient::new("http://localhost", transport).unwrap();
        let port = RemoteSolverHttpExecutionPort::new(client, FakeStorage::default())
            .with_poll_interval(Duration::from_millis(1));
        let handle = ExecutionHandle {
            handle_id: HandleId::of("handle-1").unwrap(),
            task_id: TaskId::of("task-1").unwrap(),
            slice_id: SliceId::of("slice-1").unwrap(),
            node_id: NodeId::of("node-1").unwrap(),
            started_at: SystemTime::UNIX_EPOCH,
            scheduling: None,
        };

        let slice = port
            .await_slice_end(&handle, Duration::from_millis(50))
            .await
            .unwrap();

        assert!(!slice.completed);
        assert_eq!(slice.slice_id.value(), "slice-1");
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests.iter().all(|request| request.method == "GET"));
        assert!(
            !requests
                .iter()
                .any(|request| request.url.ends_with("/stop"))
        );
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
            scheduling: None,
        };

        let result = port.fetch_final_result(&handle).await.unwrap().unwrap();

        assert!(result.optimal);
        assert_eq!(result.objective_value, Some(5.0));
        assert_eq!(result.elapsed, Duration::from_millis(7));
    }

    #[tokio::test]
    async fn http_execution_port_maps_v12_solution_audit_fields_and_cp_objective() {
        let transport = FakeTransport::new(vec![response(
            r#"{"code":"OK","message":"ok","data":{"taskId":"task-1","tenantId":"tenant-1","status":"COMPLETED","latestResultPath":"result.json","consumedCost":1.0}}"#,
        )]);
        let storage = FakeStorage::default();
        let bytes = serde_json::to_vec(&serde_json::json!({
            "feasible": true,
            "optimal": false,
            "objectiveValue": null,
            "objectiveValueInt64": 9223372036854775807i64,
            "gap": null,
            "variableValues": [],
            "variableValuesById": {"x-1": 3},
            "intervalValues": {},
            "problemStatus": "FEASIBLE",
            "solutionPresence": "INCUMBENT",
            "proofStatus": "CLAIMED",
            "terminationReason": "TIME_LIMIT",
            "schemaVersion": "1.2",
            "provenance": {"backend": "cp"},
            "fingerprints": {"model": "model-fp"},
            "fingerprintSchemas": {"model": "sha256"},
            "elapsedMs": 17,
            "solverStatus": "TIME_LIMIT",
            "statistics": {"nodes": "4"},
            "diagnostics": {"warning": "partial"},
            "runId": "run-v12",
            "attemptId": "attempt-v12",
            "artifactDigest": "digest-v12"
        }))
        .unwrap();
        storage
            .objects
            .lock()
            .unwrap()
            .insert("result.json".to_owned(), bytes);
        let client = RemoteSolverHttpClient::new("http://localhost", transport).unwrap();
        let port = RemoteSolverHttpExecutionPort::new(client, storage);
        let handle = ExecutionHandle {
            handle_id: HandleId::of("handle-1").unwrap(),
            task_id: TaskId::of("task-1").unwrap(),
            slice_id: SliceId::of("slice-1").unwrap(),
            node_id: NodeId::of("node-1").unwrap(),
            started_at: SystemTime::UNIX_EPOCH,
            scheduling: None,
        };

        let result = port.fetch_final_result(&handle).await.unwrap().unwrap();

        assert_eq!(result.schema_version.as_deref(), Some("1.2"));
        assert_eq!(result.problem_status, Some(RemoteProblemStatus::Feasible));
        assert_eq!(
            result.termination_reason,
            Some(RemoteTerminationReason::TimeLimit)
        );
        assert_eq!(
            result.solution_presence,
            Some(RemoteSolutionPresence::Incumbent)
        );
        assert_eq!(result.objective_value_int64, Some(i64::MAX));
        assert_eq!(result.run_id.as_deref(), Some("run-v12"));
        assert_eq!(result.attempt_id.as_deref(), Some("attempt-v12"));
        assert_eq!(result.artifact_digest.as_deref(), Some("digest-v12"));
        assert_eq!(
            result.statistics.get("nodes").map(String::as_str),
            Some("4")
        );
        assert_eq!(
            result.diagnostics.get("warning").map(String::as_str),
            Some("partial")
        );
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
            scheduling: None,
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
            scheduling: None,
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
            "task-1",
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
        let port = RemoteSolverHttpExecutionPort::new(client, storage)
            .with_resume_mode(RemoteSolverHttpResumeMode::Latest);

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

        assert_eq!(handle.task_id.value(), "task-1");
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
        let port = RemoteSolverHttpExecutionPort::new(client, storage)
            .with_resume_mode(RemoteSolverHttpResumeMode::Latest);

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
        let port = RemoteSolverHttpExecutionPort::new(client, storage)
            .with_resume_mode(RemoteSolverHttpResumeMode::Latest);

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
        let mut action = accepted_resume_action("task-1", &checkpoint, "slice-1");
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
        let port = RemoteSolverHttpExecutionPort::new(client, storage)
            .with_resume_mode(RemoteSolverHttpResumeMode::Latest);

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
        let port = RemoteSolverHttpExecutionPort::new(client, storage)
            .with_resume_mode(RemoteSolverHttpResumeMode::Latest);

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
        let port = RemoteSolverHttpExecutionPort::new(client, storage)
            .with_resume_mode(RemoteSolverHttpResumeMode::Latest);

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
