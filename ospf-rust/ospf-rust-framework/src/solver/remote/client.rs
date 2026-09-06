//! 远程求解客户端
//! Remote solver client

use super::domain::{
    NodeId, ObjectRef, RemoteSolverError, RemoteSolverErrorCode, RemoteSolverResult, SliceId,
    SolvePayload, SolveResult, StopAcknowledgement, TaskId, TaskStatus, TenantId,
};
use super::ospf_serializer::{
    OspfRemoteModelSerializer, solve_report_to_remote_report, solve_report_to_solver_output,
    solve_result_to_solve_report, solve_result_to_solver_output, validate_solve_result_identity,
};
use super::port::SolverExecutionPort;
use ospf_rust_core::error::{CoreError, SolverError};
use ospf_rust_core::model::intermediate::{LinearTriadModel, QuadraticTetradModel};
use ospf_rust_core::solver::{
    LinearSolver, QuadraticSolver, SolveCheckpoint, SolveFingerprints, SolveHandle, SolveOptions,
    SolveReport, SolverCapability, SolverInfo, SolverOutput, SolverProvenance, SolvingStatus,
    cancelled_solve_report, linear_model_fingerprint, quadratic_model_fingerprint,
};
use std::collections::BTreeMap;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static REMOTE_CONTEXT_COUNTER: AtomicU64 = AtomicU64::new(1);

/// 远程求解选项。
/// Remote solve options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteSolveOptions {
    /// 时间片长度 / Quantum duration
    pub quantum: Duration,
    /// 最大轮数 / Maximum rounds
    pub max_rounds: u64,
    /// 是否每轮导出检查点 / Whether to export checkpoint each round
    pub export_checkpoint_each_round: bool,
}

impl Default for RemoteSolveOptions {
    fn default() -> Self {
        Self {
            quantum: Duration::from_secs(4),
            max_rounds: 64,
            export_checkpoint_each_round: true,
        }
    }
}

impl RemoteSolveOptions {
    /// 创建默认选项。
    /// Create default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// 从载荷创建默认选项。
    /// Create default options from payload.
    pub fn from_payload(_payload: &SolvePayload) -> Self {
        Self::default()
    }

    /// 设置时间片。
    /// Set quantum.
    pub fn with_quantum(mut self, quantum: Duration) -> Self {
        self.quantum = quantum;
        self
    }

    /// 设置最大轮数。
    /// Set maximum rounds.
    pub fn with_max_rounds(mut self, max_rounds: u64) -> Self {
        self.max_rounds = max_rounds;
        self
    }

    /// 设置是否每轮导出检查点。
    /// Set whether checkpoint should be exported each round.
    pub fn with_export_checkpoint_each_round(mut self, export_checkpoint_each_round: bool) -> Self {
        self.export_checkpoint_each_round = export_checkpoint_each_round;
        self
    }

    fn validate(self) -> RemoteSolverResult<Self> {
        if self.quantum.is_zero() {
            return Err(RemoteSolverError::invalid_argument(
                "quantum must be positive.",
            ));
        }
        if self.max_rounds == 0 {
            return Err(RemoteSolverError::invalid_argument(
                "max_rounds must be positive.",
            ));
        }
        Ok(self)
    }
}

/// 远程求解上下文。
/// Remote solve context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteSolveContext {
    /// 任务 ID / Task ID
    pub task_id: TaskId,
    /// 切片 ID / Slice ID
    pub slice_id: SliceId,
    /// 节点 ID / Node ID
    pub node_id: NodeId,
    /// 租户 ID / Tenant ID
    pub tenant_id: TenantId,
}

impl RemoteSolveContext {
    /// 创建远程求解上下文。
    /// Create a remote solve context.
    pub fn new(task_id: TaskId, slice_id: SliceId, node_id: NodeId, tenant_id: TenantId) -> Self {
        Self {
            task_id,
            slice_id,
            node_id,
            tenant_id,
        }
    }
}

/// 远程求解器客户端。
/// Remote solver client.
#[derive(Debug, Clone)]
pub struct RemoteSolverClient<P> {
    execution_port: P,
}

impl<P> RemoteSolverClient<P> {
    /// 创建远程求解器客户端。
    /// Create a remote solver client.
    pub fn new(execution_port: P) -> Self {
        Self { execution_port }
    }

    /// 获取执行端口。
    /// Get execution port.
    pub fn execution_port(&self) -> &P {
        &self.execution_port
    }

    /// 拆出执行端口。
    /// Split into execution port.
    pub fn into_execution_port(self) -> P {
        self.execution_port
    }
}

fn validate_remote_result_identity(
    result: &SolveResult,
    context: &RemoteSolveContext,
) -> RemoteSolverResult<()> {
    validate_solve_result_identity(result)?;
    if let Some(report) = result.report.as_ref() {
        report.validate_identity(
            Some(context.task_id.value()),
            Some(context.slice_id.value()),
            None,
        )?;
    } else {
        for (field, expected, actual) in [
            (
                "run_id",
                Some(context.task_id.value()),
                result.run_id.as_deref(),
            ),
            (
                "attempt_id",
                Some(context.slice_id.value()),
                result.attempt_id.as_deref(),
            ),
        ] {
            if let Some(actual) = actual
                && actual != expected.unwrap_or_default()
            {
                return Err(RemoteSolverError::invalid_argument(format!(
                    "legacy remote solve result {} does not match the expected value",
                    field
                )));
            }
        }
    }
    if let Some(checkpoint) = result.checkpoint_metadata.as_ref() {
        checkpoint
            .validate()
            .map_err(|error| RemoteSolverError::invalid_argument(error.to_string()))?;
        if checkpoint.run_id != context.task_id.value()
            || checkpoint.attempt_id != context.slice_id.value()
        {
            return Err(RemoteSolverError::invalid_argument(
                "remote checkpoint identity does not match the solve context",
            ));
        }
        if let Some(report) = result.report.as_ref() {
            let core_report = &report.report;
            if core_report.fingerprints.model.as_ref() != Some(&checkpoint.model_fingerprint)
                || core_report.fingerprints.configuration.as_ref()
                    != Some(&checkpoint.configuration_fingerprint)
                || core_report.fingerprints.solver.as_ref() != Some(&checkpoint.solver_fingerprint)
                || core_report.provenance != checkpoint.provenance
            {
                return Err(RemoteSolverError::invalid_argument(
                    "remote standalone checkpoint metadata does not match the solve report",
                ));
            }
            if let Some(nested_checkpoint) = report.checkpoint.as_ref() {
                validate_checkpoint_pair(nested_checkpoint, checkpoint)?;
            }
        }
    }
    Ok(())
}

fn validate_checkpoint_pair(
    nested: &SolveCheckpoint,
    standalone: &SolveCheckpoint,
) -> RemoteSolverResult<()> {
    if nested.parent_attempt_id != standalone.parent_attempt_id {
        return Err(RemoteSolverError::invalid_argument(
            "remote nested and standalone checkpoints have different parent identities",
        ));
    }
    if nested.cancellation_chain != standalone.cancellation_chain {
        return Err(RemoteSolverError::invalid_argument(
            "remote nested and standalone checkpoints have different cancellation chains",
        ));
    }
    if nested.state_digest != standalone.state_digest {
        return Err(RemoteSolverError::invalid_argument(
            "remote nested and standalone checkpoints have different state digests",
        ));
    }
    if nested != standalone {
        return Err(RemoteSolverError::invalid_argument(
            "remote nested and standalone checkpoint metadata do not match",
        ));
    }
    Ok(())
}

fn task_status_name(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Created => "CREATED",
        TaskStatus::Accepted => "ACCEPTED",
        TaskStatus::Queued => "QUEUED",
        TaskStatus::Dispatching => "DISPATCHING",
        TaskStatus::Running => "RUNNING",
        TaskStatus::Suspended => "SUSPENDED",
        TaskStatus::Completed => "COMPLETED",
        TaskStatus::Stopping => "STOPPING",
        TaskStatus::Stopped => "STOPPED",
        TaskStatus::Failed => "FAILED",
        TaskStatus::WaitingForBudget => "WAITING_FOR_BUDGET",
    }
}

fn acknowledgement_epoch_ms(acknowledgement: &StopAcknowledgement) -> Option<String> {
    acknowledgement
        .acknowledged_at
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis().min(u64::MAX as u128).to_string())
}

fn stop_acknowledgement_metadata(
    acknowledgement: &StopAcknowledgement,
) -> BTreeMap<String, String> {
    let mut metadata = BTreeMap::from([
        (
            "remote.stop.accepted".to_owned(),
            acknowledgement.accepted.to_string(),
        ),
        (
            "remote.stop.status".to_owned(),
            task_status_name(acknowledgement.status).to_owned(),
        ),
        (
            "remote.stop.taskId".to_owned(),
            acknowledgement.task_id.value().to_owned(),
        ),
    ]);
    if let Some(origin) = acknowledgement.cancellation_origin.as_ref() {
        metadata.insert("remote.stop.cancellationOrigin".to_owned(), origin.clone());
    }
    if let Some(run_id) = acknowledgement.run_id.as_ref() {
        metadata.insert("remote.stop.runId".to_owned(), run_id.clone());
    }
    if let Some(attempt_id) = acknowledgement.attempt_id.as_ref() {
        metadata.insert("remote.stop.attemptId".to_owned(), attempt_id.clone());
    }
    if let Some(fingerprint) = acknowledgement.model_fingerprint.as_ref() {
        metadata.insert(
            "remote.stop.modelFingerprint".to_owned(),
            fingerprint.value.clone(),
        );
    }
    if let Some(fingerprint) = acknowledgement.configuration_fingerprint.as_ref() {
        metadata.insert(
            "remote.stop.configurationFingerprint".to_owned(),
            fingerprint.value.clone(),
        );
    }
    if let Some(fingerprint) = acknowledgement.solver_fingerprint.as_ref() {
        metadata.insert(
            "remote.stop.solverFingerprint".to_owned(),
            fingerprint.value.clone(),
        );
    }
    if let Some(provenance) = acknowledgement.provenance.as_ref() {
        metadata.insert(
            "remote.stop.solverId".to_owned(),
            provenance.solver_id.clone(),
        );
        metadata.insert(
            "remote.stop.backend".to_owned(),
            provenance.backend_name.clone(),
        );
    }
    if !acknowledgement.cancellation_chain.is_empty() {
        metadata.insert(
            "remote.stop.cancellationChainLength".to_owned(),
            acknowledgement.cancellation_chain.len().to_string(),
        );
    }
    if let Some(timestamp) = acknowledgement_epoch_ms(acknowledgement) {
        metadata.insert("remote.stop.acknowledgedAtEpochMs".to_owned(), timestamp);
    }
    if let Some(message) = acknowledgement.message.as_ref() {
        metadata.insert("remote.stop.message".to_owned(), message.clone());
    }
    metadata
}

fn stop_failure_metadata(error: &RemoteSolverError) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("remote.stop.accepted".to_owned(), "false".to_owned()),
        (
            "remote.stop.errorCode".to_owned(),
            format!("{:?}", error.code),
        ),
    ])
}

fn attach_stop_acknowledgement(result: &mut SolveResult, acknowledgement: &StopAcknowledgement) {
    let mut bound = acknowledgement.clone();
    if let Some(checkpoint) = result.checkpoint_metadata.as_ref() {
        bound = bound.with_checkpoint_identity(checkpoint);
    } else if let Some(report) = result.report.as_ref() {
        let run_id = report.run_id.clone().or_else(|| result.run_id.clone());
        let attempt_id = report
            .attempt_id
            .clone()
            .or_else(|| result.attempt_id.clone());
        if let (Some(run_id), Some(attempt_id)) = (run_id, attempt_id) {
            bound = bound.with_report_identity(
                run_id,
                attempt_id,
                report.report.fingerprints.model.clone(),
                report.report.fingerprints.configuration.clone(),
                report.report.fingerprints.solver.clone(),
                report.report.provenance.clone(),
            );
        }
    }
    result
        .extension
        .extend(stop_acknowledgement_metadata(&bound));
}

fn attach_stop_failure(result: &mut SolveResult, error: &RemoteSolverError) {
    result.extension.extend(stop_failure_metadata(error));
}

fn attach_stop_metadata_to_error(
    mut error: RemoteSolverError,
    metadata: BTreeMap<String, String>,
) -> RemoteSolverError {
    error.metadata.extend(metadata);
    error
}

impl<P> RemoteSolverClient<P>
where
    P: SolverExecutionPort,
{
    /// 执行远程求解。
    /// Execute remote solve.
    pub async fn solve(
        &self,
        payload: SolvePayload,
        task_id: TaskId,
        slice_id: SliceId,
        node_id: NodeId,
        tenant_id: TenantId,
        options: RemoteSolveOptions,
    ) -> RemoteSolverResult<SolveResult> {
        self.solve_with_cancellation(
            payload, task_id, slice_id, node_id, tenant_id, options, None,
        )
        .await
    }

    /// 执行远程求解并接入本地取消句柄。
    /// Execute a remote solve with a local cancellation handle.
    #[allow(clippy::too_many_arguments)]
    pub async fn solve_with_cancellation(
        &self,
        payload: SolvePayload,
        task_id: TaskId,
        slice_id: SliceId,
        node_id: NodeId,
        tenant_id: TenantId,
        options: RemoteSolveOptions,
        cancellation_handle: Option<SolveHandle>,
    ) -> RemoteSolverResult<SolveResult> {
        payload.validate_contract()?;
        let options = options.validate()?;
        let resume_checkpoint = match &payload.snapshot_ref {
            Some(_) => {
                let checkpoint = payload.checkpoint_metadata.as_ref().ok_or_else(|| {
                    RemoteSolverError::invalid_argument(
                        "resuming a snapshot requires checkpoint metadata",
                    )
                })?;
                if checkpoint.run_id != task_id.value() {
                    return Err(RemoteSolverError::invalid_argument(
                        "checkpoint run identity does not match the task",
                    ));
                }
                Some(
                    checkpoint
                        .fork_for_resume(slice_id.value())
                        .map_err(|error| {
                            RemoteSolverError::invalid_argument(format!(
                                "checkpoint cannot be resumed: {}",
                                error
                            ))
                        })?,
                )
            }
            None => None,
        };
        if let Some(handle) = cancellation_handle.as_ref()
            && handle.is_cancelled()
        {
            return cancelled_remote_result_with_checkpoint(
                handle,
                &task_id,
                &slice_id,
                payload.snapshot_ref.clone(),
                resume_checkpoint.as_ref(),
            );
        }
        let handle = match &payload.snapshot_ref {
            Some(checkpoint) => {
                self.execution_port
                    .resume(
                        &payload, checkpoint, &task_id, &slice_id, &node_id, &tenant_id,
                    )
                    .await?
            }
            None => {
                self.execution_port
                    .start(&payload, &task_id, &slice_id, &node_id, &tenant_id)
                    .await?
            }
        };

        let mut total_elapsed = Duration::ZERO;
        let mut latest_checkpoint = payload.snapshot_ref.clone();
        let solve_result = async {
            for _round in 0..options.max_rounds {
                if let Some(cancellation) = cancellation_handle.as_ref()
                    && cancellation.is_cancelled()
                {
                    return cancelled_remote_result_with_checkpoint(
                        cancellation,
                        &task_id,
                        &slice_id,
                        latest_checkpoint.clone(),
                        resume_checkpoint.as_ref(),
                    );
                }
                let slice_result = self
                    .execution_port
                    .await_slice_end(&handle, options.quantum)
                    .await?;
                total_elapsed += slice_result.elapsed;

                if let Some(cancellation) = cancellation_handle.as_ref()
                    && cancellation.is_cancelled()
                {
                    return cancelled_remote_result_with_checkpoint(
                        cancellation,
                        &task_id,
                        &slice_id,
                        latest_checkpoint.clone(),
                        resume_checkpoint.as_ref(),
                    );
                }

                if options.export_checkpoint_each_round {
                    latest_checkpoint = self
                        .execution_port
                        .export_checkpoint(&handle)
                        .await?
                        .or(latest_checkpoint);
                }

                if slice_result.completed {
                    return Ok(match self.execution_port.fetch_final_result(&handle).await? {
                        Some(mut final_result) => {
                            if final_result.checkpoint_metadata.is_none() {
                                final_result.checkpoint_metadata = resume_checkpoint.clone();
                            }
                            final_result
                        }
                        None => SolveResult::from_slice_result(
                            &slice_result,
                            total_elapsed,
                            latest_checkpoint,
                        ),
                    });
                }
            }

            Err(RemoteSolverError::new(
                RemoteSolverErrorCode::RemoteSolveNotCompletedWithinMaxRounds,
                format!(
                    "Remote solve does not complete within max_rounds={} (task_id={}, slice_id={}).",
                    options.max_rounds, task_id, slice_id
                ),
            )
            .with_metadata([
                ("taskId", task_id.value()),
                ("sliceId", slice_id.value()),
                ("maxRounds", &options.max_rounds.to_string()),
            ]))
        }
        .await;

        match solve_result {
            Ok(mut result) => {
                match self.execution_port.stop(&handle).await {
                    Ok(acknowledgement) => {
                        attach_stop_acknowledgement(&mut result, &acknowledgement);
                    }
                    Err(stop_error) => {
                        // 停止确认失败不能覆盖已经形成的求解结果；保留结构化失败元数据。
                        // A stop-ack failure must not overwrite an already formed solve result;
                        // retain structured failure metadata instead.
                        attach_stop_failure(&mut result, &stop_error);
                    }
                }
                if result.checkpoint_metadata.is_none() {
                    result.checkpoint_metadata = resume_checkpoint;
                }
                validate_remote_result_identity(
                    &result,
                    &RemoteSolveContext::new(task_id, slice_id, node_id, tenant_id),
                )?;
                Ok(result)
            }
            Err(error) => {
                let error = match self.execution_port.stop(&handle).await {
                    Ok(acknowledgement) => attach_stop_metadata_to_error(
                        error,
                        stop_acknowledgement_metadata(&acknowledgement),
                    ),
                    Err(stop_error) => {
                        attach_stop_metadata_to_error(error, stop_failure_metadata(&stop_error))
                    }
                };
                Err(error)
            }
        }
    }
}

#[cfg(test)]
fn cancelled_remote_result(
    cancellation_handle: &SolveHandle,
    task_id: &TaskId,
    slice_id: &SliceId,
) -> RemoteSolverResult<SolveResult> {
    cancelled_remote_result_with_checkpoint(cancellation_handle, task_id, slice_id, None, None)
}

fn cancelled_remote_result_with_checkpoint(
    cancellation_handle: &SolveHandle,
    task_id: &TaskId,
    slice_id: &SliceId,
    checkpoint_ref: Option<ObjectRef>,
    checkpoint: Option<&SolveCheckpoint>,
) -> RemoteSolverResult<SolveResult> {
    let report = cancelled_solve_report(
        SolverProvenance {
            solver_id: task_id.value().to_owned(),
            backend_name: "remote".to_owned(),
            ..SolverProvenance::default()
        },
        cancellation_handle,
    )
    .map_err(|error| RemoteSolverError::internal(error.to_string()))?;
    let checkpoint_metadata = checkpoint
        .map(|checkpoint| {
            let mut checkpoint = checkpoint.clone();
            let cancellation = cancellation_handle.cancellation().ok_or_else(|| {
                RemoteSolverError::internal(
                    "cancelled checkpoint is missing its cancellation record",
                )
            })?;
            checkpoint
                .record_cancellation(cancellation)
                .map_err(|error| RemoteSolverError::internal(error.to_string()))?;
            Ok::<_, RemoteSolverError>(checkpoint)
        })
        .transpose()?;
    let mut report = report;
    if let Some(checkpoint) = checkpoint_metadata.as_ref() {
        report.provenance = checkpoint.provenance.clone();
        report.fingerprints = SolveFingerprints {
            model: Some(checkpoint.model_fingerprint.clone()),
            configuration: Some(checkpoint.configuration_fingerprint.clone()),
            solver: Some(checkpoint.solver_fingerprint.clone()),
        };
        report
            .validate()
            .map_err(|error| RemoteSolverError::internal(error.to_string()))?;
    }
    let mut report_dto = solve_report_to_remote_report(
        report,
        Some(task_id.value().to_owned()),
        Some(slice_id.value().to_owned()),
        None,
    );
    report_dto.checkpoint = checkpoint_metadata.clone();
    Ok(SolveResult {
        feasible: false,
        optimal: false,
        objective_value: None,
        gap: None,
        elapsed: Duration::ZERO,
        checkpoint_ref,
        checkpoint_metadata,
        result_ref: None,
        run_id: Some(task_id.value().to_owned()),
        attempt_id: Some(slice_id.value().to_owned()),
        artifact_digest: None,
        report: Some(report_dto),
        message: Some("Remote solve cancelled".to_owned()),
        extension: BTreeMap::new(),
    })
}

/// 远程线性求解载荷工具。
/// Remote linear solve payload helper.
pub fn normalize_linear_payload(payload: SolvePayload) -> RemoteSolverResult<SolvePayload> {
    payload.with_default_target_type("linear")
}

/// 远程二次求解载荷工具。
/// Remote quadratic solve payload helper.
pub fn normalize_quadratic_payload(payload: SolvePayload) -> RemoteSolverResult<SolvePayload> {
    payload.with_default_target_type("quadratic")
}

/// 远程线性求解器。
/// Remote linear solver.
#[derive(Debug, Clone)]
pub struct RemoteLinearSolver<D, P> {
    delegate: D,
    remote_client: RemoteSolverClient<P>,
    context: RemoteSolveContext,
    options: RemoteSolveOptions,
    serializer: OspfRemoteModelSerializer,
}

impl<D, P> RemoteLinearSolver<D, P> {
    /// 创建远程线性求解器。
    /// Create a remote linear solver.
    pub fn new(delegate: D, remote_client: RemoteSolverClient<P>) -> Self {
        Self {
            delegate,
            remote_client,
            context: next_remote_context("linear"),
            options: RemoteSolveOptions::default(),
            serializer: OspfRemoteModelSerializer::new(),
        }
    }

    /// 使用执行端口创建远程线性求解器。
    /// Create a remote linear solver with execution port.
    pub fn with_execution_port(delegate: D, execution_port: P) -> Self {
        Self::new(delegate, RemoteSolverClient::new(execution_port))
    }

    /// 获取本地委托。
    /// Get local delegate.
    pub fn delegate(&self) -> &D {
        &self.delegate
    }

    /// 获取远程客户端。
    /// Get remote client.
    pub fn remote_client(&self) -> &RemoteSolverClient<P> {
        &self.remote_client
    }

    /// 获取默认远程求解上下文。
    /// Get default remote solve context.
    pub fn context(&self) -> &RemoteSolveContext {
        &self.context
    }

    /// 设置默认远程求解上下文。
    /// Set default remote solve context.
    pub fn with_context(mut self, context: RemoteSolveContext) -> Self {
        self.context = context;
        self
    }

    /// 获取默认远程求解选项。
    /// Get default remote solve options.
    pub fn options(&self) -> RemoteSolveOptions {
        self.options
    }

    /// 设置默认远程求解选项。
    /// Set default remote solve options.
    pub fn with_options(mut self, options: RemoteSolveOptions) -> Self {
        self.options = options;
        self
    }
}

impl<D, P> RemoteLinearSolver<D, P>
where
    P: SolverExecutionPort,
{
    /// 执行远程线性求解。
    /// Execute remote linear solve.
    pub async fn solve_remote(
        &self,
        payload: SolvePayload,
        context: RemoteSolveContext,
    ) -> RemoteSolverResult<SolveResult> {
        let options = RemoteSolveOptions::from_payload(&payload);
        self.solve_remote_with_options(payload, context, options)
            .await
    }

    /// 使用选项执行远程线性求解。
    /// Execute remote linear solve with options.
    pub async fn solve_remote_with_options(
        &self,
        payload: SolvePayload,
        context: RemoteSolveContext,
        options: RemoteSolveOptions,
    ) -> RemoteSolverResult<SolveResult> {
        let result = self
            .remote_client
            .solve(
                normalize_linear_payload(payload)?,
                context.task_id.clone(),
                context.slice_id.clone(),
                context.node_id.clone(),
                context.tenant_id.clone(),
                options,
            )
            .await?;
        validate_remote_result_identity(&result, &context)?;
        Ok(result)
    }

    /// 执行远程线性求解并返回统一报告 / Execute remote linear solve and return a unified report.
    pub async fn solve_remote_report(
        &self,
        payload: SolvePayload,
        context: RemoteSolveContext,
    ) -> RemoteSolverResult<SolveReport<f64>> {
        let result = self.solve_remote(payload, context).await?;
        versioned_solve_result_to_solve_report(&result)
    }
}

impl<D, P> SolverInfo for RemoteLinearSolver<D, P>
where
    D: SolverInfo,
    P: Send + Sync,
{
    fn name(&self) -> &str {
        self.delegate.name()
    }

    fn capabilities(&self) -> Vec<SolverCapability> {
        let mut capabilities = self.delegate.capabilities();
        if !capabilities.contains(&SolverCapability::Linear) {
            capabilities.push(SolverCapability::Linear);
        }
        capabilities
    }
}

impl<D, P> LinearSolver for RemoteLinearSolver<D, P>
where
    D: SolverInfo,
    P: SolverExecutionPort,
{
    fn solve_linear(
        &self,
        model: &LinearTriadModel,
    ) -> ospf_rust_core::error::Result<SolverOutput> {
        let payload = SolvePayload::from_linear_model(self.serializer.serialize_linear(model));
        let result = block_on_remote(self.solve_remote_with_options(
            payload,
            self.context.clone(),
            self.options,
        ))?;
        if result.report.is_some() {
            validate_remote_model_fingerprint(&result, &linear_model_fingerprint(model)?)?;
            return solve_result_to_solve_report(&result)
                .map(|report| solve_report_to_solver_output(&report))
                .map_err(remote_error_to_core);
        }
        Ok(solve_result_to_solver_output(&result, None))
    }

    fn solve_linear_report(
        &self,
        model: &LinearTriadModel,
    ) -> ospf_rust_core::error::Result<SolveReport<f64>> {
        let payload = SolvePayload::from_linear_model(self.serializer.serialize_linear(model));
        let report = block_on_remote(self.solve_remote_report(payload, self.context.clone()))?;
        validate_solve_report_model_fingerprint(&report, &linear_model_fingerprint(model)?)?;
        Ok(report)
    }

    fn solve_linear_report_with_options(
        &self,
        model: &LinearTriadModel,
        options: &SolveOptions<'_>,
    ) -> ospf_rust_core::error::Result<SolveReport<f64>> {
        if let Some(handle) = options.cancellation_handle
            && handle.is_cancelled()
        {
            return cancelled_solve_report(
                SolverProvenance {
                    solver_id: self.name().to_owned(),
                    backend_name: "remote".to_owned(),
                    ..SolverProvenance::default()
                },
                handle,
            );
        }
        if let Some(callback) = options.solving_status_callback {
            callback(&SolvingStatus::solving(self.name()))
                .map_err(|error| CoreError::callback_error(error.to_string()))?;
        }
        let payload = SolvePayload::from_linear_model(self.serializer.serialize_linear(model));
        let result = block_on_remote(self.remote_client.solve_with_cancellation(
            normalize_linear_payload(payload).map_err(remote_error_to_core)?,
            self.context.task_id.clone(),
            self.context.slice_id.clone(),
            self.context.node_id.clone(),
            self.context.tenant_id.clone(),
            self.options,
            options.cancellation_handle.cloned(),
        ))?;
        validate_remote_result_identity(&result, &self.context).map_err(remote_error_to_core)?;
        let report =
            versioned_solve_result_to_solve_report(&result).map_err(remote_error_to_core)?;
        validate_remote_model_fingerprint(&result, &linear_model_fingerprint(model)?)?;
        if let Some(callback) = options.solving_status_callback {
            let output = solve_report_to_solver_output(&report);
            callback(&SolvingStatus::from_output(self.name(), &output))
                .map_err(|error| CoreError::callback_error(error.to_string()))?;
        }
        Ok(report)
    }
}

/// 远程二次求解器。
/// Remote quadratic solver.
#[derive(Debug, Clone)]
pub struct RemoteQuadraticSolver<D, P> {
    delegate: D,
    remote_client: RemoteSolverClient<P>,
    context: RemoteSolveContext,
    options: RemoteSolveOptions,
    serializer: OspfRemoteModelSerializer,
}

impl<D, P> RemoteQuadraticSolver<D, P> {
    /// 创建远程二次求解器。
    /// Create a remote quadratic solver.
    pub fn new(delegate: D, remote_client: RemoteSolverClient<P>) -> Self {
        Self {
            delegate,
            remote_client,
            context: next_remote_context("quadratic"),
            options: RemoteSolveOptions::default(),
            serializer: OspfRemoteModelSerializer::new(),
        }
    }

    /// 使用执行端口创建远程二次求解器。
    /// Create a remote quadratic solver with execution port.
    pub fn with_execution_port(delegate: D, execution_port: P) -> Self {
        Self::new(delegate, RemoteSolverClient::new(execution_port))
    }

    /// 获取本地委托。
    /// Get local delegate.
    pub fn delegate(&self) -> &D {
        &self.delegate
    }

    /// 获取远程客户端。
    /// Get remote client.
    pub fn remote_client(&self) -> &RemoteSolverClient<P> {
        &self.remote_client
    }

    /// 获取默认远程求解上下文。
    /// Get default remote solve context.
    pub fn context(&self) -> &RemoteSolveContext {
        &self.context
    }

    /// 设置默认远程求解上下文。
    /// Set default remote solve context.
    pub fn with_context(mut self, context: RemoteSolveContext) -> Self {
        self.context = context;
        self
    }

    /// 获取默认远程求解选项。
    /// Get default remote solve options.
    pub fn options(&self) -> RemoteSolveOptions {
        self.options
    }

    /// 设置默认远程求解选项。
    /// Set default remote solve options.
    pub fn with_options(mut self, options: RemoteSolveOptions) -> Self {
        self.options = options;
        self
    }
}

impl<D, P> RemoteQuadraticSolver<D, P>
where
    P: SolverExecutionPort,
{
    /// 执行远程二次求解。
    /// Execute remote quadratic solve.
    pub async fn solve_remote(
        &self,
        payload: SolvePayload,
        context: RemoteSolveContext,
    ) -> RemoteSolverResult<SolveResult> {
        let options = RemoteSolveOptions::from_payload(&payload);
        self.solve_remote_with_options(payload, context, options)
            .await
    }

    /// 使用选项执行远程二次求解。
    /// Execute remote quadratic solve with options.
    pub async fn solve_remote_with_options(
        &self,
        payload: SolvePayload,
        context: RemoteSolveContext,
        options: RemoteSolveOptions,
    ) -> RemoteSolverResult<SolveResult> {
        let result = self
            .remote_client
            .solve(
                normalize_quadratic_payload(payload)?,
                context.task_id.clone(),
                context.slice_id.clone(),
                context.node_id.clone(),
                context.tenant_id.clone(),
                options,
            )
            .await?;
        validate_remote_result_identity(&result, &context)?;
        Ok(result)
    }

    /// 执行远程二次求解并返回统一报告 / Execute remote quadratic solve and return a unified report.
    pub async fn solve_remote_report(
        &self,
        payload: SolvePayload,
        context: RemoteSolveContext,
    ) -> RemoteSolverResult<SolveReport<f64>> {
        let result = self.solve_remote(payload, context).await?;
        versioned_solve_result_to_solve_report(&result)
    }
}

impl<D, P> SolverInfo for RemoteQuadraticSolver<D, P>
where
    D: SolverInfo,
    P: Send + Sync,
{
    fn name(&self) -> &str {
        self.delegate.name()
    }

    fn capabilities(&self) -> Vec<SolverCapability> {
        let mut capabilities = self.delegate.capabilities();
        if !capabilities.contains(&SolverCapability::Quadratic) {
            capabilities.push(SolverCapability::Quadratic);
        }
        capabilities
    }
}

impl<D, P> QuadraticSolver for RemoteQuadraticSolver<D, P>
where
    D: SolverInfo,
    P: SolverExecutionPort,
{
    fn solve_quadratic(
        &self,
        model: &QuadraticTetradModel,
    ) -> ospf_rust_core::error::Result<SolverOutput> {
        let payload =
            SolvePayload::from_quadratic_model(self.serializer.serialize_quadratic(model));
        let result = block_on_remote(self.solve_remote_with_options(
            payload,
            self.context.clone(),
            self.options,
        ))?;
        if result.report.is_some() {
            validate_remote_model_fingerprint(&result, &quadratic_model_fingerprint(model)?)?;
            return solve_result_to_solve_report(&result)
                .map(|report| solve_report_to_solver_output(&report))
                .map_err(remote_error_to_core);
        }
        Ok(solve_result_to_solver_output(&result, None))
    }

    fn solve_quadratic_report(
        &self,
        model: &QuadraticTetradModel,
    ) -> ospf_rust_core::error::Result<SolveReport<f64>> {
        let payload =
            SolvePayload::from_quadratic_model(self.serializer.serialize_quadratic(model));
        let report = block_on_remote(self.solve_remote_report(payload, self.context.clone()))?;
        validate_solve_report_model_fingerprint(&report, &quadratic_model_fingerprint(model)?)?;
        Ok(report)
    }

    fn solve_quadratic_report_with_options(
        &self,
        model: &QuadraticTetradModel,
        options: &SolveOptions<'_>,
    ) -> ospf_rust_core::error::Result<SolveReport<f64>> {
        if let Some(handle) = options.cancellation_handle
            && handle.is_cancelled()
        {
            return cancelled_solve_report(
                SolverProvenance {
                    solver_id: self.name().to_owned(),
                    backend_name: "remote".to_owned(),
                    ..SolverProvenance::default()
                },
                handle,
            );
        }
        if let Some(callback) = options.solving_status_callback {
            callback(&SolvingStatus::solving(self.name()))
                .map_err(|error| CoreError::callback_error(error.to_string()))?;
        }
        let payload =
            SolvePayload::from_quadratic_model(self.serializer.serialize_quadratic(model));
        let result = block_on_remote(self.remote_client.solve_with_cancellation(
            normalize_quadratic_payload(payload).map_err(remote_error_to_core)?,
            self.context.task_id.clone(),
            self.context.slice_id.clone(),
            self.context.node_id.clone(),
            self.context.tenant_id.clone(),
            self.options,
            options.cancellation_handle.cloned(),
        ))?;
        validate_remote_result_identity(&result, &self.context).map_err(remote_error_to_core)?;
        let report =
            versioned_solve_result_to_solve_report(&result).map_err(remote_error_to_core)?;
        validate_remote_model_fingerprint(&result, &quadratic_model_fingerprint(model)?)?;
        if let Some(callback) = options.solving_status_callback {
            let output = solve_report_to_solver_output(&report);
            callback(&SolvingStatus::from_output(self.name(), &output))
                .map_err(|error| CoreError::callback_error(error.to_string()))?;
        }
        Ok(report)
    }
}

fn next_remote_context(target: &str) -> RemoteSolveContext {
    let counter = REMOTE_CONTEXT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let prefix = format!("remote-{}-{}-{}", target, millis, counter);
    RemoteSolveContext::new(
        TaskId::of(format!("{}-task", prefix)).expect("generated task id must be valid"),
        SliceId::of(format!("{}-slice", prefix)).expect("generated slice id must be valid"),
        NodeId::of("remote-client").expect("generated node id must be valid"),
        TenantId::of("default").expect("generated tenant id must be valid"),
    )
}

fn validate_remote_model_fingerprint(
    result: &SolveResult,
    expected_model: &ospf_rust_core::solver::ModelFingerprint,
) -> ospf_rust_core::error::Result<()> {
    if let Some(report) = result.report.as_ref() {
        report
            .validate_model_fingerprint(expected_model)
            .map_err(remote_error_to_core)?;
    }
    Ok(())
}

fn versioned_solve_result_to_solve_report(
    result: &SolveResult,
) -> RemoteSolverResult<SolveReport<f64>> {
    if result.report.is_none() {
        return Err(RemoteSolverError::invalid_argument(
            "versioned remote solve report is missing its report envelope",
        ));
    }
    solve_result_to_solve_report(result)
}

fn validate_solve_report_model_fingerprint(
    report: &SolveReport<f64>,
    expected_model: &ospf_rust_core::solver::ModelFingerprint,
) -> ospf_rust_core::error::Result<()> {
    let Some(actual) = report.fingerprints.model.as_ref() else {
        return Err(CoreError::contract_error(
            "versioned remote solve report is missing its model fingerprint",
        ));
    };
    if actual != expected_model {
        return Err(CoreError::contract_error(
            "remote solve report model fingerprint does not match the requested model",
        ));
    }
    Ok(())
}

fn block_on_remote<F, T>(future: F) -> ospf_rust_core::error::Result<T>
where
    F: Future<Output = RemoteSolverResult<T>>,
{
    let result = match tokio::runtime::Handle::try_current() {
        Ok(handle) => match handle.runtime_flavor() {
            tokio::runtime::RuntimeFlavor::MultiThread => {
                tokio::task::block_in_place(|| handle.block_on(future))
            }
            tokio::runtime::RuntimeFlavor::CurrentThread => {
                return Err(CoreError::Solver(SolverError::ContractViolation(
                    "remote solver synchronous trait entry cannot block inside a current-thread Tokio runtime; use solve_remote or solve_remote_with_options instead".to_string(),
                )));
            }
            _ => {
                return Err(CoreError::Solver(SolverError::ContractViolation(
                    "remote solver synchronous trait entry cannot block inside this Tokio runtime flavor; use solve_remote or solve_remote_with_options instead".to_string(),
                )));
            }
        },
        Err(_) => tokio::runtime::Runtime::new()
            .map_err(|err| CoreError::Solver(SolverError::NotAvailable(err.to_string())))?
            .block_on(future),
    };
    result.map_err(remote_error_to_core)
}

fn remote_error_to_core(error: RemoteSolverError) -> CoreError {
    let message = format!("{:?}: {}", error.code, error.message);
    let solver_error = match error.code {
        RemoteSolverErrorCode::InvalidArgument => SolverError::InvalidInput(message),
        RemoteSolverErrorCode::UnsupportedProtocolVersion
        | RemoteSolverErrorCode::CheckpointExportFailed
        | RemoteSolverErrorCode::CheckpointRestoreFailed => SolverError::Parsing(message),
        RemoteSolverErrorCode::StorageIoFailed
        | RemoteSolverErrorCode::NoEligibleNodeAvailable
        | RemoteSolverErrorCode::NodeOffline
        | RemoteSolverErrorCode::NoCompatibleNodeAvailable => SolverError::NotAvailable(message),
        RemoteSolverErrorCode::InvalidTaskStateTransition
        | RemoteSolverErrorCode::EventPublishFailed
        | RemoteSolverErrorCode::InternalError => SolverError::ContractViolation(message),
        RemoteSolverErrorCode::SolverExecutionFailed | RemoteSolverErrorCode::TaskFailed => {
            SolverError::SolveFailed(message)
        }
        RemoteSolverErrorCode::TaskFailedHardTimeout
        | RemoteSolverErrorCode::TaskFailedSliceTimeout => SolverError::Timeout(Duration::ZERO),
        RemoteSolverErrorCode::TaskFailedBudgetExceeded
        | RemoteSolverErrorCode::TaskNotTerminalWithinMaxRounds
        | RemoteSolverErrorCode::RemoteSolveNotCompletedWithinMaxRounds => SolverError::NoSolution,
    };
    CoreError::Solver(solver_error)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, SystemTime};

    use async_trait::async_trait;
    use ospf_rust_core::solver::{AuditFingerprint, CancellationOrigin, CancellationRecord};

    use super::*;
    use crate::solver::remote::domain::{
        ExecutionHandle, HandleId, ObjectPath, SerializedLinearModel, SliceResult,
        StopAcknowledgement, TaskStatus,
    };

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Event {
        Start,
        Resume,
        Await,
        Checkpoint,
        Final,
        Stop,
    }

    #[derive(Debug)]
    struct FakePort {
        events: Arc<Mutex<Vec<Event>>>,
        slices: Arc<Mutex<Vec<SliceResult>>>,
        final_result: Option<SolveResult>,
        checkpoint: Option<ObjectRef>,
        fail_stop: bool,
        cancel_on_await: Option<SolveHandle>,
    }

    impl FakePort {
        fn new(slices: Vec<SliceResult>) -> Self {
            Self {
                events: Arc::new(Mutex::new(Vec::new())),
                slices: Arc::new(Mutex::new(slices)),
                final_result: None,
                checkpoint: None,
                fail_stop: false,
                cancel_on_await: None,
            }
        }

        fn with_final_result(mut self, final_result: SolveResult) -> Self {
            self.final_result = Some(final_result);
            self
        }

        fn with_checkpoint(mut self, checkpoint: ObjectRef) -> Self {
            self.checkpoint = Some(checkpoint);
            self
        }

        fn with_stop_failure(mut self) -> Self {
            self.fail_stop = true;
            self
        }

        fn with_cancel_on_await(mut self, handle: SolveHandle) -> Self {
            self.cancel_on_await = Some(handle);
            self
        }

        fn handle(task_id: &TaskId, slice_id: &SliceId, node_id: &NodeId) -> ExecutionHandle {
            ExecutionHandle {
                handle_id: HandleId::of("handle-1").unwrap(),
                task_id: task_id.clone(),
                slice_id: slice_id.clone(),
                node_id: node_id.clone(),
                started_at: SystemTime::UNIX_EPOCH,
            }
        }

        fn push(&self, event: Event) {
            self.events.lock().unwrap().push(event);
        }
    }

    #[async_trait]
    impl SolverExecutionPort for FakePort {
        async fn start(
            &self,
            _payload: &SolvePayload,
            task_id: &TaskId,
            slice_id: &SliceId,
            node_id: &NodeId,
            _tenant_id: &TenantId,
        ) -> RemoteSolverResult<ExecutionHandle> {
            self.push(Event::Start);
            Ok(Self::handle(task_id, slice_id, node_id))
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
            self.push(Event::Resume);
            Ok(Self::handle(task_id, slice_id, node_id))
        }

        async fn await_slice_end(
            &self,
            _handle: &ExecutionHandle,
            _quantum: Duration,
        ) -> RemoteSolverResult<SliceResult> {
            self.push(Event::Await);
            if let Some(handle) = self.cancel_on_await.as_ref() {
                handle.cancel(ospf_rust_core::solver::CancellationOrigin::User);
            }
            let mut slices = self.slices.lock().unwrap();
            if slices.is_empty() {
                return Err(RemoteSolverError::internal("no fake slice result"));
            }
            Ok(slices.remove(0))
        }

        async fn export_checkpoint(
            &self,
            _handle: &ExecutionHandle,
        ) -> RemoteSolverResult<Option<ObjectRef>> {
            self.push(Event::Checkpoint);
            Ok(self.checkpoint.clone())
        }

        async fn fetch_final_result(
            &self,
            _handle: &ExecutionHandle,
        ) -> RemoteSolverResult<Option<SolveResult>> {
            self.push(Event::Final);
            Ok(self.final_result.clone())
        }

        async fn stop(&self, handle: &ExecutionHandle) -> RemoteSolverResult<StopAcknowledgement> {
            self.push(Event::Stop);
            if self.fail_stop {
                return Err(RemoteSolverError::internal("stop failed"));
            }
            Ok(
                StopAcknowledgement::new(handle.task_id.clone(), true, TaskStatus::Stopped)
                    .with_cancellation_origin("REMOTE_STOP"),
            )
        }
    }

    #[derive(Debug, Clone)]
    struct DummyDelegate;

    impl SolverInfo for DummyDelegate {
        fn name(&self) -> &str {
            "dummy-remote"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            Vec::new()
        }
    }

    fn ids() -> (TaskId, SliceId, NodeId, TenantId) {
        (
            TaskId::of("task-1").unwrap(),
            SliceId::of("slice-1").unwrap(),
            NodeId::of("node-1").unwrap(),
            TenantId::of("tenant-1").unwrap(),
        )
    }

    fn context() -> RemoteSolveContext {
        let (task_id, slice_id, node_id, tenant_id) = ids();
        RemoteSolveContext::new(task_id, slice_id, node_id, tenant_id)
    }

    fn payload() -> SolvePayload {
        SolvePayload::from_linear_model(SerializedLinearModel::empty("m"))
    }

    fn checkpoint_fixture() -> SolveCheckpoint {
        let fingerprint = |value: &str| AuditFingerprint {
            schema_version: "1.0".to_owned(),
            algorithm: "sha256".to_owned(),
            value: value.to_owned(),
        };
        SolveCheckpoint::new(
            "task-1",
            "slice-1",
            Some("slice-0".to_owned()),
            fingerprint("model"),
            fingerprint("config"),
            fingerprint("solver"),
            SolverProvenance {
                solver_id: "fake/1".to_owned(),
                backend_name: "fake".to_owned(),
                ..SolverProvenance::default()
            },
            2,
            None,
            None,
            None,
            fingerprint("state"),
        )
        .expect("checkpoint fixture should be valid")
    }

    #[test]
    fn remote_errors_keep_input_parsing_and_backend_categories() {
        assert_eq!(
            remote_error_to_core(RemoteSolverError::invalid_argument("bad payload"))
                .solver_error_class(),
            ospf_rust_core::error::SolverErrorClass::Input
        );
        assert_eq!(
            remote_error_to_core(RemoteSolverError::new(
                RemoteSolverErrorCode::UnsupportedProtocolVersion,
                "future schema",
            ))
            .solver_error_class(),
            ospf_rust_core::error::SolverErrorClass::Parsing
        );
        assert_eq!(
            remote_error_to_core(RemoteSolverError::new(
                RemoteSolverErrorCode::SolverExecutionFailed,
                "backend stopped",
            ))
            .solver_error_class(),
            ospf_rust_core::error::SolverErrorClass::Backend
        );
        assert!(
            !remote_error_to_core(RemoteSolverError::new(
                RemoteSolverErrorCode::SolverExecutionFailed,
                "backend stopped",
            ))
            .is_terminal_projection()
        );
        assert!(
            remote_error_to_core(RemoteSolverError::new(
                RemoteSolverErrorCode::TaskFailedHardTimeout,
                "time limit",
            ))
            .is_normal_terminal()
        );
        assert!(
            remote_error_to_core(RemoteSolverError::new(
                RemoteSolverErrorCode::RemoteSolveNotCompletedWithinMaxRounds,
                "round limit",
            ))
            .is_normal_terminal()
        );
    }

    fn slice(completed: bool, elapsed_ms: u64) -> SliceResult {
        SliceResult {
            slice_id: SliceId::of("slice-1").unwrap(),
            completed,
            feasible: completed,
            objective_value: Some(3.0),
            gap: Some(0.0),
            elapsed: Duration::from_millis(elapsed_ms),
            message: None,
        }
    }

    #[tokio::test]
    async fn stop_acknowledgement_keeps_origin_and_legacy_bool_facade() {
        let port = FakePort::new(vec![slice(true, 1)]);
        let (task_id, slice_id, node_id, tenant_id) = ids();
        let handle = port
            .start(&payload(), &task_id, &slice_id, &node_id, &tenant_id)
            .await
            .expect("fake task should start");

        let acknowledgement = port
            .stop(&handle)
            .await
            .expect("stop should return a structured acknowledgement");
        assert!(acknowledgement.accepted);
        assert_eq!(acknowledgement.task_id, task_id);
        assert_eq!(acknowledgement.status, TaskStatus::Stopped);
        assert_eq!(
            acknowledgement.cancellation_origin.as_deref(),
            Some("REMOTE_STOP")
        );
        assert!(
            port.stop_legacy(&handle)
                .await
                .expect("legacy stop facade should project acknowledgement to bool")
        );
    }

    #[test]
    fn stop_acknowledgement_metadata_keeps_checkpoint_identity_chain() {
        let mut checkpoint = checkpoint_fixture();
        checkpoint
            .record_cancellation(CancellationRecord {
                origin: CancellationOrigin::RemoteStop,
                requested_at_epoch_ms: 10,
            })
            .expect("checkpoint cancellation should be valid");
        let mut result = SolveResult::from_slice_result(&slice(false, 0), Duration::ZERO, None);
        result.checkpoint_metadata = Some(checkpoint);

        let acknowledgement =
            StopAcknowledgement::new(TaskId::of("task-1").unwrap(), true, TaskStatus::Stopped)
                .with_cancellation_origin("REMOTE_STOP");
        attach_stop_acknowledgement(&mut result, &acknowledgement);

        assert_eq!(
            result.extension.get("remote.stop.runId"),
            Some(&"task-1".to_owned())
        );
        assert_eq!(
            result.extension.get("remote.stop.attemptId"),
            Some(&"slice-1".to_owned())
        );
        assert_eq!(
            result.extension.get("remote.stop.cancellationChainLength"),
            Some(&"1".to_owned())
        );
        assert_eq!(
            result.extension.get("remote.stop.modelFingerprint"),
            Some(&"model".to_owned())
        );
    }

    #[test]
    fn remote_solve_options_from_payload_keeps_default_quantum() {
        let mut payload = payload();
        payload.task_meta.time_limit = Some(Duration::from_millis(7));

        let options = RemoteSolveOptions::from_payload(&payload);

        assert_eq!(options.quantum, RemoteSolveOptions::default().quantum);
    }

    #[tokio::test]
    async fn remote_client_starts_new_task_and_uses_fallback_final_result() {
        let checkpoint = ObjectRef::new(ObjectPath::of("checkpoint.bin").unwrap());
        let port = FakePort::new(vec![slice(false, 10), slice(true, 20)])
            .with_checkpoint(checkpoint.clone());
        let events = port.events.clone();
        let client = RemoteSolverClient::new(port);
        let (task_id, slice_id, node_id, tenant_id) = ids();

        let result = client
            .solve(
                payload(),
                task_id,
                slice_id,
                node_id,
                tenant_id,
                RemoteSolveOptions::new().with_max_rounds(3),
            )
            .await
            .unwrap();

        assert_eq!(result.elapsed, Duration::from_millis(30));
        assert_eq!(result.checkpoint_ref, Some(checkpoint));
        assert_eq!(
            *events.lock().unwrap(),
            vec![
                Event::Start,
                Event::Await,
                Event::Checkpoint,
                Event::Await,
                Event::Checkpoint,
                Event::Final,
                Event::Stop
            ]
        );
    }

    #[tokio::test]
    async fn remote_client_stops_after_mid_solve_cancellation_and_returns_report() {
        let cancellation = SolveHandle::new();
        let port = FakePort::new(vec![slice(false, 1), slice(true, 1)])
            .with_cancel_on_await(cancellation.clone());
        let events = port.events.clone();
        let client = RemoteSolverClient::new(port);
        let (task_id, slice_id, node_id, tenant_id) = ids();

        let result = client
            .solve_with_cancellation(
                payload(),
                task_id,
                slice_id,
                node_id,
                tenant_id,
                RemoteSolveOptions::new(),
                Some(cancellation),
            )
            .await
            .expect("remote cancellation should be a normal result");
        let report = solve_result_to_solve_report(&result).expect("cancel report should decode");

        assert_eq!(
            report.termination_reason,
            ospf_rust_core::solver::TerminationReason::Cancelled
        );
        assert_eq!(
            result.extension.get("remote.stop.accepted"),
            Some(&"true".to_owned())
        );
        assert_eq!(
            result.extension.get("remote.stop.cancellationOrigin"),
            Some(&"REMOTE_STOP".to_owned())
        );
        assert_eq!(
            report.diagnostics.extensions.get("remote.stop.status"),
            Some(&"STOPPED".to_owned())
        );
        assert_eq!(
            events.lock().unwrap().as_slice(),
            &[Event::Start, Event::Await, Event::Stop]
        );
    }

    #[tokio::test]
    async fn remote_client_rejects_report_with_mismatched_identity() {
        let cancellation = SolveHandle::new();
        assert!(cancellation.cancel(ospf_rust_core::solver::CancellationOrigin::User));
        let (task_id, slice_id, node_id, tenant_id) = ids();
        let mut final_result = cancelled_remote_result(&cancellation, &task_id, &slice_id)
            .expect("cancelled result fixture should be valid");
        final_result
            .report
            .as_mut()
            .expect("cancelled result should carry a report")
            .run_id = Some("task-from-another-run".to_owned());

        let client = RemoteSolverClient::new(
            FakePort::new(vec![slice(true, 1)]).with_final_result(final_result),
        );
        let result = client
            .solve(
                payload(),
                task_id,
                slice_id,
                node_id,
                tenant_id,
                RemoteSolveOptions::new(),
            )
            .await;

        let error = result.expect_err("a report from another run must be rejected");
        assert!(error.to_string().contains("run_id"));
    }

    #[test]
    fn remote_client_pairs_standalone_checkpoint_metadata_with_the_report() {
        let cancellation = SolveHandle::new();
        assert!(cancellation.cancel(CancellationOrigin::User));
        let (task_id, slice_id, _, _) = ids();
        let checkpoint = checkpoint_fixture();
        let result = cancelled_remote_result_with_checkpoint(
            &cancellation,
            &task_id,
            &slice_id,
            None,
            Some(&checkpoint),
        )
        .expect("matching result fixture");
        validate_remote_result_identity(&result, &context())
            .expect("matching standalone and nested identities should pass");

        let mut standalone_mismatch = result.clone();
        standalone_mismatch
            .report
            .as_mut()
            .expect("report")
            .checkpoint = None;
        standalone_mismatch
            .checkpoint_metadata
            .as_mut()
            .expect("standalone checkpoint")
            .model_fingerprint
            .value = "other-model".to_owned();
        assert!(
            validate_remote_result_identity(&standalone_mismatch, &context())
                .expect_err("standalone metadata mismatch must be rejected")
                .to_string()
                .contains("standalone checkpoint")
        );

        let mut parent_mismatch = result.clone();
        parent_mismatch
            .checkpoint_metadata
            .as_mut()
            .expect("standalone checkpoint")
            .parent_attempt_id = Some("different-parent".to_owned());
        assert!(
            validate_remote_result_identity(&parent_mismatch, &context())
                .expect_err("checkpoint parent mismatch must be rejected")
                .to_string()
                .contains("parent")
        );

        let mut cancellation_mismatch = result.clone();
        cancellation_mismatch
            .checkpoint_metadata
            .as_mut()
            .expect("standalone checkpoint")
            .cancellation_chain[0]
            .origin = CancellationOrigin::RemoteStop;
        assert!(
            validate_remote_result_identity(&cancellation_mismatch, &context())
                .expect_err("checkpoint cancellation mismatch must be rejected")
                .to_string()
                .contains("cancellation")
        );

        let mut state_digest_mismatch = result.clone();
        state_digest_mismatch
            .checkpoint_metadata
            .as_mut()
            .expect("standalone checkpoint")
            .state_digest
            .value = "different-state".to_owned();
        assert!(
            validate_remote_result_identity(&state_digest_mismatch, &context())
                .expect_err("checkpoint state digest mismatch must be rejected")
                .to_string()
                .contains("state digest")
        );

        let mut artifact_digest_mismatch = result.clone();
        artifact_digest_mismatch.artifact_digest = Some("different-artifact".to_owned());
        assert!(
            validate_remote_result_identity(&artifact_digest_mismatch, &context())
                .expect_err("report artifact digest mismatch must be rejected")
                .to_string()
                .contains("artifact digest")
        );

        let mut nested_and_standalone_conflict = result;
        nested_and_standalone_conflict
            .checkpoint_metadata
            .as_mut()
            .expect("standalone checkpoint")
            .configuration_fingerprint
            .value = "other-config".to_owned();
        assert!(
            validate_remote_result_identity(&nested_and_standalone_conflict, &context()).is_err()
        );
    }

    #[tokio::test]
    async fn remote_client_rejects_legacy_result_with_mismatched_identity() {
        let final_result = SolveResult {
            feasible: false,
            optimal: false,
            objective_value: None,
            gap: None,
            elapsed: Duration::ZERO,
            checkpoint_ref: None,
            checkpoint_metadata: None,
            result_ref: None,
            run_id: Some("task-from-another-run".to_owned()),
            attempt_id: Some("slice-1".to_owned()),
            artifact_digest: None,
            report: None,
            message: None,
            extension: BTreeMap::new(),
        };
        let client = RemoteSolverClient::new(
            FakePort::new(vec![slice(true, 1)]).with_final_result(final_result),
        );
        let (task_id, slice_id, node_id, tenant_id) = ids();
        let result = client
            .solve(
                payload(),
                task_id,
                slice_id,
                node_id,
                tenant_id,
                RemoteSolveOptions::new(),
            )
            .await;

        let error = result.expect_err("legacy result from another run must be rejected");
        assert!(error.to_string().contains("run_id"));
    }

    #[tokio::test]
    async fn remote_client_rejects_snapshot_without_checkpoint_metadata() {
        let snapshot = ObjectRef::new(ObjectPath::of("snapshot.bin").unwrap());
        let port = FakePort::new(vec![slice(true, 1)]);
        let events = port.events.clone();
        let client = RemoteSolverClient::new(port);
        let (task_id, slice_id, node_id, tenant_id) = ids();

        let error = client
            .solve(
                payload().with_snapshot_ref(snapshot),
                task_id,
                slice_id,
                node_id,
                tenant_id,
                RemoteSolveOptions::new(),
            )
            .await
            .expect_err("snapshot resume without metadata must be rejected");

        assert_eq!(error.code, RemoteSolverErrorCode::InvalidArgument);
        assert!(events.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn remote_client_resumes_when_snapshot_ref_exists() {
        let snapshot = ObjectRef::new(ObjectPath::of("snapshot.bin").unwrap());
        let checkpoint = SolveCheckpoint::new(
            "task-1",
            "slice-0",
            None,
            ospf_rust_core::solver::AuditFingerprint {
                schema_version: "1.0".to_owned(),
                algorithm: "sha256".to_owned(),
                value: "model".to_owned(),
            },
            ospf_rust_core::solver::AuditFingerprint {
                schema_version: "1.0".to_owned(),
                algorithm: "sha256".to_owned(),
                value: "config".to_owned(),
            },
            ospf_rust_core::solver::AuditFingerprint {
                schema_version: "1.0".to_owned(),
                algorithm: "sha256".to_owned(),
                value: "solver".to_owned(),
            },
            ospf_rust_core::solver::SolverProvenance {
                solver_id: "fake/1".to_owned(),
                backend_name: "fake".to_owned(),
                ..Default::default()
            },
            1,
            None,
            None,
            None,
            ospf_rust_core::solver::AuditFingerprint {
                schema_version: "1.0".to_owned(),
                algorithm: "sha256".to_owned(),
                value: "state".to_owned(),
            },
        )
        .expect("checkpoint identity should be valid");
        let port = FakePort::new(vec![slice(true, 1)]);
        let events = port.events.clone();
        let client = RemoteSolverClient::new(port);
        let (task_id, slice_id, node_id, tenant_id) = ids();

        let result = client
            .solve(
                payload()
                    .with_snapshot_ref(snapshot)
                    .with_checkpoint_metadata(checkpoint),
                task_id,
                slice_id,
                node_id,
                tenant_id,
                RemoteSolveOptions::new(),
            )
            .await
            .unwrap();

        assert_eq!(events.lock().unwrap()[0], Event::Resume);
        let resumed_checkpoint = result
            .checkpoint_metadata
            .expect("resume result should preserve checkpoint metadata");
        assert_eq!(resumed_checkpoint.run_id, "task-1");
        assert_eq!(resumed_checkpoint.attempt_id, "slice-1");
        assert_eq!(
            resumed_checkpoint.parent_attempt_id.as_deref(),
            Some("slice-0")
        );
        assert_eq!(resumed_checkpoint.provenance.solver_id, "fake/1");
    }

    #[tokio::test]
    async fn remote_client_cancelled_resume_preserves_cancellation_chain() {
        let snapshot = ObjectRef::new(ObjectPath::of("snapshot.bin").unwrap());
        let checkpoint = SolveCheckpoint::new(
            "task-1",
            "slice-0",
            None,
            ospf_rust_core::solver::AuditFingerprint {
                schema_version: "1.0".to_owned(),
                algorithm: "sha256".to_owned(),
                value: "model".to_owned(),
            },
            ospf_rust_core::solver::AuditFingerprint {
                schema_version: "1.0".to_owned(),
                algorithm: "sha256".to_owned(),
                value: "config".to_owned(),
            },
            ospf_rust_core::solver::AuditFingerprint {
                schema_version: "1.0".to_owned(),
                algorithm: "sha256".to_owned(),
                value: "solver".to_owned(),
            },
            ospf_rust_core::solver::SolverProvenance {
                solver_id: "fake/1".to_owned(),
                backend_name: "fake".to_owned(),
                ..Default::default()
            },
            1,
            None,
            None,
            None,
            ospf_rust_core::solver::AuditFingerprint {
                schema_version: "1.0".to_owned(),
                algorithm: "sha256".to_owned(),
                value: "state".to_owned(),
            },
        )
        .expect("checkpoint identity should be valid");
        let cancellation = SolveHandle::new();
        let port = FakePort::new(vec![slice(false, 1)]).with_cancel_on_await(cancellation.clone());
        let client = RemoteSolverClient::new(port);
        let (task_id, slice_id, node_id, tenant_id) = ids();

        let result = client
            .solve_with_cancellation(
                payload()
                    .with_snapshot_ref(snapshot)
                    .with_checkpoint_metadata(checkpoint),
                task_id,
                slice_id,
                node_id,
                tenant_id,
                RemoteSolveOptions::new(),
                Some(cancellation),
            )
            .await
            .expect("cancelled resume should return a structured result");

        let checkpoint = result
            .checkpoint_metadata
            .expect("cancelled resume should retain checkpoint metadata");
        assert_eq!(checkpoint.cancellation_chain.len(), 1);
        assert_eq!(
            checkpoint.cancellation_chain[0].origin,
            ospf_rust_core::solver::CancellationOrigin::User
        );
        assert_eq!(
            result
                .report
                .as_ref()
                .and_then(|report| report.checkpoint.as_ref()),
            Some(&checkpoint)
        );
    }

    #[tokio::test]
    async fn remote_client_uses_final_result_when_available() {
        let final_result = SolveResult {
            feasible: true,
            optimal: true,
            objective_value: Some(9.0),
            gap: Some(0.0),
            elapsed: Duration::from_millis(99),
            checkpoint_ref: None,
            checkpoint_metadata: None,
            result_ref: None,
            run_id: None,
            attempt_id: None,
            artifact_digest: None,
            report: None,
            message: Some("final".to_string()),
            extension: BTreeMap::new(),
        };
        let port = FakePort::new(vec![slice(true, 1)]).with_final_result(final_result.clone());
        let client = RemoteSolverClient::new(port);
        let (task_id, slice_id, node_id, tenant_id) = ids();

        let result = client
            .solve(
                payload(),
                task_id,
                slice_id,
                node_id,
                tenant_id,
                RemoteSolveOptions::new(),
            )
            .await
            .unwrap();

        assert_eq!(result.feasible, final_result.feasible);
        assert_eq!(result.optimal, final_result.optimal);
        assert_eq!(result.objective_value, final_result.objective_value);
        assert_eq!(
            result.extension.get("remote.stop.accepted"),
            Some(&"true".to_owned())
        );
    }

    #[tokio::test]
    async fn remote_client_does_not_let_stop_failure_overwrite_success() {
        let port = FakePort::new(vec![slice(true, 1)]).with_stop_failure();
        let client = RemoteSolverClient::new(port);
        let (task_id, slice_id, node_id, tenant_id) = ids();

        let result = client
            .solve(
                payload(),
                task_id,
                slice_id,
                node_id,
                tenant_id,
                RemoteSolveOptions::new(),
            )
            .await
            .unwrap();

        assert!(
            !result.feasible,
            "task completion without a result artifact must not imply feasibility"
        );
        assert_eq!(
            result.extension.get("remote.stop.accepted"),
            Some(&"false".to_owned())
        );
        assert_eq!(
            result.extension.get("remote.stop.errorCode"),
            Some(&"InternalError".to_owned())
        );
    }

    #[tokio::test]
    async fn remote_client_stops_and_reports_timeout_after_max_rounds() {
        let port = FakePort::new(vec![slice(false, 1), slice(false, 1)]);
        let events = port.events.clone();
        let client = RemoteSolverClient::new(port);
        let (task_id, slice_id, node_id, tenant_id) = ids();

        let err = client
            .solve(
                payload(),
                task_id,
                slice_id,
                node_id,
                tenant_id,
                RemoteSolveOptions::new().with_max_rounds(2),
            )
            .await
            .unwrap_err();

        assert_eq!(
            err.code,
            RemoteSolverErrorCode::RemoteSolveNotCompletedWithinMaxRounds
        );
        assert_eq!(events.lock().unwrap().last(), Some(&Event::Stop));
    }

    #[test]
    fn payload_normalizers_fill_default_target_type() {
        let linear = normalize_linear_payload(payload()).unwrap();
        let quadratic = normalize_quadratic_payload(payload()).unwrap();

        assert_eq!(linear.task_meta.target_type.unwrap().value(), "linear");
        assert_eq!(
            quadratic.task_meta.target_type.unwrap().value(),
            "quadratic"
        );
    }

    #[tokio::test]
    async fn remote_linear_solver_fills_target_type() {
        let mut payload = payload();
        payload.task_meta.time_limit = Some(Duration::from_millis(7));
        let port = FakePort::new(vec![slice(true, 1)]);
        let solver = RemoteLinearSolver::with_execution_port((), port);

        let result = solver.solve_remote(payload, context()).await.unwrap();

        assert!(
            !result.feasible,
            "task completion without a result artifact must not imply feasibility"
        );
    }

    #[tokio::test]
    async fn remote_quadratic_solver_fills_target_type() {
        let port = FakePort::new(vec![slice(true, 1)]);
        let solver = RemoteQuadraticSolver::with_execution_port((), port);

        let result = solver.solve_remote(payload(), context()).await.unwrap();

        assert!(
            !result.feasible,
            "task completion without a result artifact must not imply feasibility"
        );
    }

    #[test]
    fn remote_linear_solver_implements_core_linear_solver_trait() {
        let final_result = SolveResult {
            feasible: true,
            optimal: true,
            objective_value: Some(9.0),
            gap: Some(0.0),
            elapsed: Duration::from_millis(1),
            checkpoint_ref: None,
            checkpoint_metadata: None,
            result_ref: None,
            run_id: None,
            attempt_id: None,
            artifact_digest: None,
            report: None,
            message: None,
            extension: BTreeMap::new(),
        };
        let port = FakePort::new(vec![slice(true, 1)]).with_final_result(final_result);
        let solver = RemoteLinearSolver::with_execution_port(DummyDelegate, port);
        let model = LinearTriadModel::default();

        let output = LinearSolver::solve_linear(&solver, &model).unwrap();

        assert_eq!(solver.name(), "dummy-remote");
        assert!(solver.supports(SolverCapability::Linear));
        assert_eq!(output.objective_value, Some(9.0));
    }

    #[test]
    fn remote_linear_solver_report_trait_preserves_remote_terminal_semantics() {
        let model = LinearTriadModel::default();
        let report = SolveReport::builder(
            ospf_rust_core::solver::ProblemStatus::Feasible,
            ospf_rust_core::solver::TerminationReason::NodeLimit,
        )
        .solution(ospf_rust_core::solver::SolveSolution {
            objective: Some(1.0),
            objective_value: Some(1.0),
            ..ospf_rust_core::solver::SolveSolution::vector(vec![1.0])
        })
        .statistics(ospf_rust_core::solver::SolveStatistics {
            best_bound_value: Some(0.5),
            relative_gap: Some(0.5),
            ..ospf_rust_core::solver::SolveStatistics::default()
        })
        .fingerprints(ospf_rust_core::solver::SolveFingerprints {
            model: Some(linear_model_fingerprint(&model).expect("model fingerprint")),
            ..ospf_rust_core::solver::SolveFingerprints::default()
        })
        .build()
        .expect("remote report fixture should be valid");
        let final_result = SolveResult {
            feasible: true,
            optimal: false,
            objective_value: Some(1.0),
            gap: Some(0.5),
            elapsed: Duration::from_millis(1),
            checkpoint_ref: None,
            checkpoint_metadata: None,
            result_ref: None,
            run_id: None,
            attempt_id: None,
            artifact_digest: None,
            report: Some(
                super::super::ospf_serializer::solve_report_to_remote_report(
                    report,
                    Some("task-1".to_owned()),
                    Some("slice-1".to_owned()),
                    None,
                ),
            ),
            message: None,
            extension: BTreeMap::new(),
        };
        let port = FakePort::new(vec![slice(true, 1)]).with_final_result(final_result);
        let solver =
            RemoteLinearSolver::with_execution_port(DummyDelegate, port).with_context(context());

        let report = LinearSolver::solve_linear_report(&solver, &model).unwrap();

        assert_eq!(
            report.termination_reason,
            ospf_rust_core::solver::TerminationReason::NodeLimit
        );
        assert_eq!(report.statistics.best_bound_value, Some(0.5));
        assert_eq!(report.solution.unwrap().values, vec![1.0]);
    }

    #[test]
    fn remote_linear_solver_report_rejects_missing_model_fingerprint() {
        let model = LinearTriadModel::default();
        let report = SolveReport::builder(
            ospf_rust_core::solver::ProblemStatus::Feasible,
            ospf_rust_core::solver::TerminationReason::Completed,
        )
        .solution(ospf_rust_core::solver::SolveSolution {
            objective: Some(1.0),
            objective_value: Some(1.0),
            ..ospf_rust_core::solver::SolveSolution::vector(vec![1.0])
        })
        .build()
        .expect("report fixture should be valid");
        let final_result = SolveResult {
            feasible: true,
            optimal: false,
            objective_value: Some(1.0),
            gap: None,
            elapsed: Duration::from_millis(1),
            checkpoint_ref: None,
            checkpoint_metadata: None,
            result_ref: None,
            run_id: None,
            attempt_id: None,
            artifact_digest: None,
            report: Some(
                super::super::ospf_serializer::solve_report_to_remote_report(
                    report,
                    Some("task-1".to_owned()),
                    Some("slice-1".to_owned()),
                    None,
                ),
            ),
            message: None,
            extension: BTreeMap::new(),
        };
        let solver = RemoteLinearSolver::with_execution_port(
            DummyDelegate,
            FakePort::new(vec![slice(true, 1)]).with_final_result(final_result),
        )
        .with_context(context());

        let error = LinearSolver::solve_linear_report(&solver, &model)
            .expect_err("report entry must reject an unbound remote report");
        assert!(error.to_string().contains("model fingerprint"));
    }

    #[test]
    fn remote_linear_solver_report_rejects_legacy_result_without_envelope() {
        let final_result = SolveResult {
            feasible: true,
            optimal: true,
            objective_value: Some(11.0),
            gap: Some(0.0),
            elapsed: Duration::from_millis(1),
            checkpoint_ref: None,
            checkpoint_metadata: None,
            result_ref: None,
            run_id: None,
            attempt_id: None,
            artifact_digest: None,
            report: None,
            message: None,
            extension: BTreeMap::new(),
        };
        let solver = RemoteLinearSolver::with_execution_port(
            DummyDelegate,
            FakePort::new(vec![slice(true, 1)]).with_final_result(final_result),
        );

        let error = LinearSolver::solve_linear_report(&solver, &LinearTriadModel::default())
            .expect_err("report entry must reject legacy result payloads");
        assert!(error.to_string().contains("report envelope"));
    }

    #[test]
    fn remote_quadratic_solver_implements_core_quadratic_solver_trait() {
        let final_result = SolveResult {
            feasible: true,
            optimal: true,
            objective_value: Some(11.0),
            gap: Some(0.0),
            elapsed: Duration::from_millis(1),
            checkpoint_ref: None,
            checkpoint_metadata: None,
            result_ref: None,
            run_id: None,
            attempt_id: None,
            artifact_digest: None,
            report: None,
            message: None,
            extension: BTreeMap::new(),
        };
        let port = FakePort::new(vec![slice(true, 1)]).with_final_result(final_result);
        let solver = RemoteQuadraticSolver::with_execution_port(DummyDelegate, port);
        let model = QuadraticTetradModel::default();

        let output = QuadraticSolver::solve_quadratic(&solver, &model).unwrap();

        assert_eq!(solver.name(), "dummy-remote");
        assert!(solver.supports(SolverCapability::Quadratic));
        assert_eq!(output.objective_value, Some(11.0));
    }

    #[test]
    fn remote_quadratic_solver_report_rejects_legacy_result_without_envelope() {
        let final_result = SolveResult {
            feasible: true,
            optimal: true,
            objective_value: Some(11.0),
            gap: Some(0.0),
            elapsed: Duration::from_millis(1),
            checkpoint_ref: None,
            checkpoint_metadata: None,
            result_ref: None,
            run_id: None,
            attempt_id: None,
            artifact_digest: None,
            report: None,
            message: None,
            extension: BTreeMap::new(),
        };
        let solver = RemoteQuadraticSolver::with_execution_port(
            DummyDelegate,
            FakePort::new(vec![slice(true, 1)]).with_final_result(final_result),
        );

        let error =
            QuadraticSolver::solve_quadratic_report(&solver, &QuadraticTetradModel::default())
                .expect_err("report entry must reject legacy result payloads");
        assert!(error.to_string().contains("report envelope"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn remote_linear_solver_sync_trait_reports_current_thread_runtime_risk() {
        let port = FakePort::new(vec![slice(true, 1)]);
        let solver = RemoteLinearSolver::with_execution_port(DummyDelegate, port);
        let model = LinearTriadModel::default();

        let err = LinearSolver::solve_linear(&solver, &model).unwrap_err();

        assert!(err.to_string().contains("current-thread Tokio runtime"));
    }
}
