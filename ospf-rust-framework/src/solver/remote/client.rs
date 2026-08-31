//! 远程求解客户端
//! Remote solver client

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::domain::{
    NodeId, ObjectRef, RemoteSolverError, RemoteSolverErrorCode, RemoteSolverResult, SliceId,
    SolvePayload, SolveResult, TaskId, TenantId,
};
use super::ospf_serializer::{OspfRemoteModelSerializer, solve_result_to_solver_output};
use super::port::SolverExecutionPort;
use ospf_rust_core::error::{CoreError, SolverError};
use ospf_rust_core::model::intermediate::{LinearTriadModel, QuadraticTetradModel};
use ospf_rust_core::solver::{
    LinearSolver, QuadraticSolver, SolverCapability, SolverInfo, SolverOutput,
};

static REMOTE_CONTEXT_COUNTER: AtomicU64 = AtomicU64::new(1);

/// 远程求解选项。
/// Remote solve options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteSolveOptions {
    pub quantum: Duration,
    pub max_rounds: u64,
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
    pub task_id: TaskId,
    pub slice_id: SliceId,
    pub node_id: NodeId,
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
        let options = options.validate()?;
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
                let slice_result = self
                    .execution_port
                    .await_slice_end(&handle, options.quantum)
                    .await?;
                total_elapsed += slice_result.elapsed;

                if options.export_checkpoint_each_round {
                    latest_checkpoint = self
                        .execution_port
                        .export_checkpoint(&handle)
                        .await?
                        .or(latest_checkpoint);
                }

                if slice_result.completed {
                    return Ok(match self.execution_port.fetch_final_result(&handle).await? {
                        Some(final_result) => final_result,
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
            Ok(result) => {
                let _ = self.execution_port.stop(&handle).await;
                Ok(result)
            }
            Err(error) => {
                let _ = self.execution_port.stop(&handle).await;
                Err(error)
            }
        }
    }
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
        self.remote_client
            .solve(
                normalize_linear_payload(payload)?,
                context.task_id,
                context.slice_id,
                context.node_id,
                context.tenant_id,
                options,
            )
            .await
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
        Ok(solve_result_to_solver_output(&result, None))
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
        self.remote_client
            .solve(
                normalize_quadratic_payload(payload)?,
                context.task_id,
                context.slice_id,
                context.node_id,
                context.tenant_id,
                options,
            )
            .await
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
        Ok(solve_result_to_solver_output(&result, None))
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
                return Err(CoreError::Solver(SolverError::SolveFailed(
                    "remote solver synchronous trait entry cannot block inside a current-thread Tokio runtime; use solve_remote or solve_remote_with_options instead".to_string(),
                )));
            }
            _ => {
                return Err(CoreError::Solver(SolverError::SolveFailed(
                    "remote solver synchronous trait entry cannot block inside this Tokio runtime flavor; use solve_remote or solve_remote_with_options instead".to_string(),
                )));
            }
        },
        Err(_) => tokio::runtime::Runtime::new()
            .map_err(|err| CoreError::Solver(SolverError::SolveFailed(err.to_string())))?
            .block_on(future),
    };
    result.map_err(remote_error_to_core)
}

fn remote_error_to_core(error: RemoteSolverError) -> CoreError {
    CoreError::Solver(SolverError::SolveFailed(format!(
        "{:?}: {}",
        error.code, error.message
    )))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, SystemTime};

    use async_trait::async_trait;

    use super::*;
    use crate::solver::remote::domain::{
        ExecutionHandle, HandleId, ObjectPath, SerializedLinearModel, SliceResult,
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
    }

    impl FakePort {
        fn new(slices: Vec<SliceResult>) -> Self {
            Self {
                events: Arc::new(Mutex::new(Vec::new())),
                slices: Arc::new(Mutex::new(slices)),
                final_result: None,
                checkpoint: None,
                fail_stop: false,
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

        async fn stop(&self, _handle: &ExecutionHandle) -> RemoteSolverResult<bool> {
            self.push(Event::Stop);
            if self.fail_stop {
                return Err(RemoteSolverError::internal("stop failed"));
            }
            Ok(true)
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
    async fn remote_client_resumes_when_snapshot_ref_exists() {
        let snapshot = ObjectRef::new(ObjectPath::of("snapshot.bin").unwrap());
        let port = FakePort::new(vec![slice(true, 1)]);
        let events = port.events.clone();
        let client = RemoteSolverClient::new(port);
        let (task_id, slice_id, node_id, tenant_id) = ids();

        let _ = client
            .solve(
                payload().with_snapshot_ref(snapshot),
                task_id,
                slice_id,
                node_id,
                tenant_id,
                RemoteSolveOptions::new(),
            )
            .await
            .unwrap();

        assert_eq!(events.lock().unwrap()[0], Event::Resume);
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
            result_ref: None,
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

        assert_eq!(result, final_result);
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

        assert!(result.feasible);
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

        assert!(result.feasible);
    }

    #[tokio::test]
    async fn remote_quadratic_solver_fills_target_type() {
        let port = FakePort::new(vec![slice(true, 1)]);
        let solver = RemoteQuadraticSolver::with_execution_port((), port);

        let result = solver.solve_remote(payload(), context()).await.unwrap();

        assert!(result.feasible);
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
            result_ref: None,
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
    fn remote_quadratic_solver_implements_core_quadratic_solver_trait() {
        let final_result = SolveResult {
            feasible: true,
            optimal: true,
            objective_value: Some(11.0),
            gap: Some(0.0),
            elapsed: Duration::from_millis(1),
            checkpoint_ref: None,
            result_ref: None,
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

    #[tokio::test(flavor = "current_thread")]
    async fn remote_linear_solver_sync_trait_reports_current_thread_runtime_risk() {
        let port = FakePort::new(vec![slice(true, 1)]);
        let solver = RemoteLinearSolver::with_execution_port(DummyDelegate, port);
        let model = LinearTriadModel::default();

        let err = LinearSolver::solve_linear(&solver, &model).unwrap_err();

        assert!(err.to_string().contains("current-thread Tokio runtime"));
    }
}
