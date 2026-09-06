//! 远程求解领域模型
//! Remote solver domain models

use ospf_rust_core::solver::{
    AuditFingerprint, CancellationRecord, SolveCheckpoint, SolverProvenance,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 远程求解结果。
/// Remote solver result.
pub type RemoteSolverResult<T> = std::result::Result<T, RemoteSolverError>;

macro_rules! remote_string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// 创建语义 ID。
            /// Create a semantic ID.
            pub fn new(value: impl Into<String>) -> RemoteSolverResult<Self> {
                let value = value.into().trim().to_string();
                if value.is_empty() {
                    return Err(RemoteSolverError::invalid_argument(format!(
                        "{} must not be blank.",
                        stringify!($name)
                    )));
                }
                Ok(Self(value))
            }

            /// 规范化创建语义 ID。
            /// Create a normalized semantic ID.
            pub fn of(value: impl Into<String>) -> RemoteSolverResult<Self> {
                Self::new(value)
            }

            /// 获取原始值。
            /// Get the raw value.
            pub fn value(&self) -> &str {
                &self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

remote_string_id!(
    /// 任务 ID / Task ID.
    TaskId
);
remote_string_id!(
    /// 切片 ID / Slice ID.
    SliceId
);
remote_string_id!(
    /// 节点 ID / Node ID.
    NodeId
);
remote_string_id!(
    /// 租户 ID / Tenant ID.
    TenantId
);
remote_string_id!(
    /// 请求 ID / Request ID.
    RequestId
);
remote_string_id!(
    /// 句柄 ID / Handle ID.
    HandleId
);
remote_string_id!(
    /// 追踪 ID / Trace ID.
    TraceId
);
remote_string_id!(
    /// 对象版本 / Object version.
    ObjectVersion
);
remote_string_id!(
    /// 对象 ETag / Object ETag.
    ObjectEtag
);
remote_string_id!(
    /// 求解器类型名称 / Solver type name.
    SolverTypeName
);
remote_string_id!(
    /// 目标类型名称 / Target type name.
    TargetTypeName
);
remote_string_id!(
    /// 预算范围 ID / Budget scope ID.
    BudgetScopeId
);
remote_string_id!(
    /// 操作者 ID / Operator ID.
    OperatorId
);
remote_string_id!(
    /// 操作来源 / Operation source.
    OperationSource
);
remote_string_id!(
    /// 原因码 / Reason code.
    ReasonCode
);

/// 对象路径。
/// Object path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObjectPath(String);

impl ObjectPath {
    /// 创建对象路径。
    /// Create an object path.
    pub fn new(value: impl Into<String>) -> RemoteSolverResult<Self> {
        let value = value
            .into()
            .trim()
            .replace('\\', "/")
            .trim_start_matches('/')
            .to_string();
        if value.is_empty() {
            return Err(RemoteSolverError::invalid_argument(
                "ObjectPath must not be blank.",
            ));
        }
        if value.contains('\0') {
            return Err(RemoteSolverError::invalid_argument(
                "ObjectPath must not contain NUL.",
            ));
        }
        Ok(Self(value))
    }

    /// 规范化创建对象路径。
    /// Create a normalized object path.
    pub fn of(value: impl Into<String>) -> RemoteSolverResult<Self> {
        Self::new(value)
    }

    /// 获取原始路径。
    /// Get the raw path.
    pub fn value(&self) -> &str {
        &self.0
    }
}

impl Display for ObjectPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// 毫秒 Duration serde helper。
/// Millisecond Duration serde helper.
pub mod duration_millis {
    use super::*;

    /// 序列化 Duration 为毫秒数 / Serialize Duration as milliseconds
    pub fn serialize<S>(value: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(value.as_millis() as u64)
    }

    /// 从毫秒数反序列化 Duration / Deserialize Duration from milliseconds
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Duration::from_millis(u64::deserialize(deserializer)?))
    }
}

/// 可选毫秒 Duration serde helper。
/// Optional millisecond Duration serde helper.
pub mod option_duration_millis {
    use super::*;

    /// 序列化可选 Duration 为毫秒数 / Serialize optional Duration as milliseconds
    pub fn serialize<S>(value: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value
            .map(|value| value.as_millis() as u64)
            .serialize(serializer)
    }

    /// 从毫秒数反序列化可选 Duration / Deserialize optional Duration from milliseconds
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Option::<u64>::deserialize(deserializer)?.map(Duration::from_millis))
    }
}

/// SystemTime 与 epoch milliseconds 的 serde helper / epoch-millis SystemTime serde helper.
pub mod epoch_millis {
    use super::*;
    use serde::ser::Error;

    /// 序列化 SystemTime 为 epoch 毫秒数 / Serialize SystemTime as epoch milliseconds
    pub fn serialize<S>(value: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let millis = value
            .duration_since(UNIX_EPOCH)
            .map_err(S::Error::custom)?
            .as_millis() as u64;
        serializer.serialize_u64(millis)
    }

    /// 从 epoch 毫秒数反序列化 SystemTime / Deserialize SystemTime from epoch milliseconds
    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(UNIX_EPOCH + Duration::from_millis(u64::deserialize(deserializer)?))
    }
}

/// 可选 epoch millis SystemTime serde helper。
/// Optional epoch millis SystemTime serde helper.
pub mod option_epoch_millis {
    use super::*;

    /// 序列化可选 SystemTime 为 epoch 毫秒数 / Serialize optional SystemTime as epoch milliseconds
    pub fn serialize<S>(value: &Option<SystemTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let millis = value.and_then(|value| {
            value
                .duration_since(UNIX_EPOCH)
                .ok()
                .map(|duration| duration.as_millis() as u64)
        });
        millis.serialize(serializer)
    }

    /// 从 epoch 毫秒数反序列化可选 SystemTime / Deserialize optional SystemTime from epoch milliseconds
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<SystemTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Option::<u64>::deserialize(deserializer)?
            .map(|millis| UNIX_EPOCH + Duration::from_millis(millis)))
    }
}

/// 任务复杂度。
/// Task complexity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskComplexity {
    /// 简单 / Simple
    Simple,
    /// 复杂 / Complex
    Complex,
}

/// 时间敏感度。
/// Time sensitivity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimeSensitivity {
    /// 实时 / Realtime
    Realtime,
    /// 非实时 / Non-realtime
    NonRealtime,
}

/// 任务状态。
/// Task status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskStatus {
    /// 已创建 / Created
    Created,
    /// 已接受 / Accepted
    Accepted,
    /// 已入队 / Queued
    Queued,
    /// 调度中 / Dispatching
    Dispatching,
    /// 运行中 / Running
    Running,
    /// 已挂起 / Suspended
    Suspended,
    /// 已完成 / Completed
    Completed,
    /// 停止中 / Stopping
    Stopping,
    /// 已停止 / Stopped
    Stopped,
    /// 已失败 / Failed
    Failed,
    /// 等待预算 / Waiting for budget
    WaitingForBudget,
}

/// 切片状态。
/// Slice status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SliceStatus {
    /// 已规划 / Planned
    Planned,
    /// 运行中 / Running
    Running,
    /// 检查点导出中 / Checkpointing
    Checkpointing,
    /// 已挂起 / Suspended
    Suspended,
    /// 已完成 / Completed
    Completed,
    /// 已失败 / Failed
    Failed,
}

/// 求解器类型。
/// Solver type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SolverType {
    /// SCIP 求解器 / SCIP solver
    Scip,
    /// Gurobi 求解器 / Gurobi solver
    Gurobi,
    /// 自动选择 / Auto select
    Auto,
}

/// 标准化模型类型。
/// Normalized model type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NormalizedModelType {
    /// 线性 / Linear
    Linear,
    /// 二次 / Quadratic
    Quadratic,
    /// 未知 / Unknown
    Unknown,
}

/// 对象引用。
/// Object reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectRef {
    /// 对象路径 / Object path
    pub path: ObjectPath,
    /// 对象版本 / Object version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<ObjectVersion>,
    /// 对象 ETag / Object ETag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<ObjectEtag>,
}

impl ObjectRef {
    /// 创建对象引用。
    /// Create an object reference.
    pub fn new(path: ObjectPath) -> Self {
        Self {
            path,
            version: None,
            etag: None,
        }
    }

    /// 从字符串创建对象引用。
    /// Create an object reference from strings.
    pub fn of(path: impl Into<String>) -> RemoteSolverResult<Self> {
        Ok(Self::new(ObjectPath::of(path)?))
    }

    /// 设置版本。
    /// Set version.
    pub fn with_version(mut self, version: ObjectVersion) -> Self {
        self.version = Some(version);
        self
    }

    /// 设置 ETag。
    /// Set ETag.
    pub fn with_etag(mut self, etag: ObjectEtag) -> Self {
        self.etag = Some(etag);
        self
    }
}

/// 任务元数据。
/// Task metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TaskMeta {
    /// 求解器类型名称 / Solver type name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solver_type: Option<SolverTypeName>,
    /// 目标类型名称 / Target type name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_type: Option<TargetTypeName>,
    /// 时间限制 / Time limit
    #[serde(
        rename = "timeLimitMs",
        default,
        with = "option_duration_millis",
        skip_serializing_if = "Option::is_none"
    )]
    pub time_limit: Option<Duration>,
    /// 解数量限制 / Solution limit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solution_limit: Option<usize>,
    /// 预估变量数 / Estimated variable count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_variable_count: Option<usize>,
    /// 预估约束数 / Estimated constraint count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_constraint_count: Option<usize>,
    /// 历史运行时间 / Historical runtime
    #[serde(
        rename = "historicalRuntimeMs",
        default,
        with = "option_duration_millis",
        skip_serializing_if = "Option::is_none"
    )]
    pub historical_runtime: Option<Duration>,
    /// 附加元数据 / Additional metadata
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

impl TaskMeta {
    /// 补齐默认 target type。
    /// Fill the default target type.
    pub fn with_default_target_type(mut self, target_type: &str) -> RemoteSolverResult<Self> {
        if self.target_type.is_none() {
            self.target_type = Some(TargetTypeName::of(target_type)?);
        }
        Ok(self)
    }
}

/// 模型数据。
/// Model data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModelData {
    /// 对象引用 / Object reference
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    pub object_ref: Option<ObjectRef>,
    /// 线性模型 / Linear model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linear_model: Option<SerializedLinearModel>,
    /// 二次模型 / Quadratic model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quadratic_model: Option<SerializedQuadraticModel>,
    /// 原始字节 / Raw bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_bytes: Option<Vec<u8>>,
    /// 格式标识 / Format identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

impl ModelData {
    /// 创建引用模型数据。
    /// Create reference model data.
    pub fn reference(object_ref: ObjectRef) -> Self {
        Self {
            object_ref: Some(object_ref),
            ..Self::default()
        }
    }

    /// 创建线性模型数据。
    /// Create linear model data.
    pub fn linear(model: SerializedLinearModel) -> Self {
        Self {
            linear_model: Some(model),
            ..Self::default()
        }
    }

    /// 创建二次模型数据。
    /// Create quadratic model data.
    pub fn quadratic(model: SerializedQuadraticModel) -> Self {
        Self {
            quadratic_model: Some(model),
            ..Self::default()
        }
    }

    /// 创建原始字节模型数据。
    /// Create raw byte model data.
    pub fn raw(bytes: Vec<u8>, format: impl Into<String>) -> Self {
        Self {
            raw_bytes: Some(bytes),
            format: Some(format.into()),
            ..Self::default()
        }
    }

    /// 是否引用模式。
    /// Whether this is reference mode.
    pub fn is_reference(&self) -> bool {
        self.object_ref.is_some()
    }

    /// 是否内联模式。
    /// Whether this is inline mode.
    pub fn is_inline(&self) -> bool {
        self.linear_model.is_some() || self.quadratic_model.is_some() || self.raw_bytes.is_some()
    }

    /// 模型类型。
    /// Model type.
    pub fn model_type(&self) -> NormalizedModelType {
        if self.quadratic_model.is_some() {
            NormalizedModelType::Quadratic
        } else if self.linear_model.is_some() {
            NormalizedModelType::Linear
        } else {
            NormalizedModelType::Unknown
        }
    }

    /// 校验内联模型的版本和稳定身份 / Validate inline-model version and stable identities.
    pub fn validate_contract(&self) -> RemoteSolverResult<()> {
        if let Some(model) = self.linear_model.as_ref() {
            model.validate_contract()?;
        }
        if let Some(model) = self.quadratic_model.as_ref() {
            model.validate_contract()?;
        }
        Ok(())
    }
}

/// 求解配置。
/// Solver config.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SolverConfig {
    /// 时间限制 / Time limit
    #[serde(
        rename = "timeLimitMs",
        default,
        with = "option_duration_millis",
        skip_serializing_if = "Option::is_none"
    )]
    pub time_limit: Option<Duration>,
    /// 解数量限制 / Solution limit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solution_limit: Option<usize>,
    /// MIP 间隙容差 / MIP gap tolerance
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mip_gap_tolerance: Option<f64>,
    /// 线程数 / Thread count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threads: Option<usize>,
    /// 求解器参数 / Solver parameters
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub solver_params: BTreeMap<String, String>,
}

/// 求解载荷。
/// Solve payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolvePayload {
    /// 模型数据 / Model data
    pub model_data: ModelData,
    /// 配置引用 / Config reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_ref: Option<ObjectRef>,
    /// 求解配置 / Solver config
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<SolverConfig>,
    /// 快照引用 / Snapshot reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_ref: Option<ObjectRef>,
    /// 快照的可校验身份 / Validatable checkpoint identity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint_metadata: Option<SolveCheckpoint>,
    /// 任务元数据 / Task metadata
    #[serde(default)]
    pub task_meta: TaskMeta,
    /// 扩展字段 / Extension fields
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extension: BTreeMap<String, String>,
}

impl SolvePayload {
    /// 创建载荷。
    /// Create a payload.
    pub fn new(model_data: ModelData) -> Self {
        Self {
            model_data,
            config_ref: None,
            config: None,
            snapshot_ref: None,
            checkpoint_metadata: None,
            task_meta: TaskMeta::default(),
            extension: BTreeMap::new(),
        }
    }

    /// 创建引用模型载荷。
    /// Create a reference-model payload.
    pub fn from_model_ref(model_ref: ObjectRef) -> Self {
        Self::new(ModelData::reference(model_ref))
    }

    /// 创建线性模型载荷。
    /// Create a linear-model payload.
    pub fn from_linear_model(model: SerializedLinearModel) -> Self {
        Self::new(ModelData::linear(model))
    }

    /// 创建二次模型载荷。
    /// Create a quadratic-model payload.
    pub fn from_quadratic_model(model: SerializedQuadraticModel) -> Self {
        Self::new(ModelData::quadratic(model))
    }

    /// 设置任务元数据。
    /// Set task metadata.
    pub fn with_task_meta(mut self, task_meta: TaskMeta) -> Self {
        self.task_meta = task_meta;
        self
    }

    /// 设置快照引用。
    /// Set snapshot reference.
    pub fn with_snapshot_ref(mut self, snapshot_ref: ObjectRef) -> Self {
        self.snapshot_ref = Some(snapshot_ref);
        self
    }

    /// 绑定恢复所需的 checkpoint 身份 / Attach the checkpoint identity required for a resume.
    pub fn with_checkpoint_metadata(mut self, checkpoint: SolveCheckpoint) -> Self {
        self.checkpoint_metadata = Some(checkpoint);
        self
    }

    /// 补齐默认 target type。
    /// Fill the default target type.
    pub fn with_default_target_type(mut self, target_type: &str) -> RemoteSolverResult<Self> {
        self.task_meta = self.task_meta.with_default_target_type(target_type)?;
        Ok(self)
    }

    /// 模型引用。
    /// Model reference.
    pub fn model_ref(&self) -> Option<&ObjectRef> {
        self.model_data.object_ref.as_ref()
    }

    /// 校验远程执行载荷 / Validate the remote execution payload.
    pub fn validate_contract(&self) -> RemoteSolverResult<()> {
        self.model_data.validate_contract()?;
        if self.snapshot_ref.is_none() && self.checkpoint_metadata.is_some() {
            return Err(RemoteSolverError::invalid_argument(
                "checkpoint metadata requires a snapshot reference",
            ));
        }
        if let Some(checkpoint) = self.checkpoint_metadata.as_ref() {
            checkpoint.validate().map_err(|error| {
                RemoteSolverError::invalid_argument(format!(
                    "checkpoint metadata failed validation: {}",
                    error
                ))
            })?;
        }
        Ok(())
    }
}

/// 执行句柄。
/// Execution handle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionHandle {
    /// 句柄 ID / Handle ID
    pub handle_id: HandleId,
    /// 任务 ID / Task ID
    pub task_id: TaskId,
    /// 切片 ID / Slice ID
    pub slice_id: SliceId,
    /// 节点 ID / Node ID
    pub node_id: NodeId,
    /// 启动时间 / Started at
    #[serde(rename = "startedAtEpochMs", with = "epoch_millis")]
    pub started_at: SystemTime,
}

/// 远程停止确认。
/// Structured acknowledgement returned by a remote stop request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopAcknowledgement {
    /// 被停止的任务 ID / Task ID being stopped.
    pub task_id: TaskId,
    /// 服务端是否接受停止请求 / Whether the server accepted the stop request.
    pub accepted: bool,
    /// 服务端确认的任务状态 / Task status observed by the server.
    pub status: TaskStatus,
    /// 取消来源 / Cancellation origin recorded by the remote side.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancellation_origin: Option<String>,
    /// 求解运行 ID / Solve run identity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// 求解 attempt ID / Solve attempt identity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<String>,
    /// 模型指纹 / Model fingerprint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_fingerprint: Option<AuditFingerprint>,
    /// 生效配置指纹 / Effective configuration fingerprint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configuration_fingerprint: Option<AuditFingerprint>,
    /// solver 环境指纹 / Solver environment fingerprint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solver_fingerprint: Option<AuditFingerprint>,
    /// 停止时捕获的 solver provenance / Solver provenance captured at stop.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<SolverProvenance>,
    /// 跨 attempt 累积的取消事实 / Cancellation facts accumulated across attempts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cancellation_chain: Vec<CancellationRecord>,
    /// 确认时间 / Acknowledgement time.
    #[serde(rename = "acknowledgedAtEpochMs", with = "epoch_millis")]
    pub acknowledged_at: SystemTime,
    /// 服务端附加消息 / Optional server message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl StopAcknowledgement {
    /// 创建停止确认 / Create a stop acknowledgement.
    pub fn new(task_id: TaskId, accepted: bool, status: TaskStatus) -> Self {
        Self {
            task_id,
            accepted,
            status,
            cancellation_origin: None,
            run_id: None,
            attempt_id: None,
            model_fingerprint: None,
            configuration_fingerprint: None,
            solver_fingerprint: None,
            provenance: None,
            cancellation_chain: Vec::new(),
            acknowledged_at: SystemTime::now(),
            message: None,
        }
    }

    /// 设置取消来源 / Set the cancellation origin.
    pub fn with_cancellation_origin(mut self, origin: impl Into<String>) -> Self {
        self.cancellation_origin = Some(origin.into());
        self
    }

    /// 绑定 checkpoint 的身份链 / Bind the checkpoint identity chain.
    pub fn with_checkpoint_identity(mut self, checkpoint: &SolveCheckpoint) -> Self {
        self.run_id = Some(checkpoint.run_id.clone());
        self.attempt_id = Some(checkpoint.attempt_id.clone());
        self.model_fingerprint = Some(checkpoint.model_fingerprint.clone());
        self.configuration_fingerprint = Some(checkpoint.configuration_fingerprint.clone());
        self.solver_fingerprint = Some(checkpoint.solver_fingerprint.clone());
        self.provenance = Some(checkpoint.provenance.clone());
        self.cancellation_chain = checkpoint.cancellation_chain.clone();
        self
    }

    /// 绑定报告的身份链 / Bind the solve-report identity chain.
    pub fn with_report_identity(
        mut self,
        run_id: impl Into<String>,
        attempt_id: impl Into<String>,
        model_fingerprint: Option<AuditFingerprint>,
        configuration_fingerprint: Option<AuditFingerprint>,
        solver_fingerprint: Option<AuditFingerprint>,
        provenance: SolverProvenance,
    ) -> Self {
        self.run_id = Some(run_id.into());
        self.attempt_id = Some(attempt_id.into());
        self.model_fingerprint = model_fingerprint;
        self.configuration_fingerprint = configuration_fingerprint;
        self.solver_fingerprint = solver_fingerprint;
        self.provenance = Some(provenance);
        self
    }

    /// 设置附加消息 / Set an additional message.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// 切片结果。
/// Slice result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SliceResult {
    /// 切片 ID / Slice ID
    pub slice_id: SliceId,
    /// 是否完成 / Whether completed
    pub completed: bool,
    /// 是否可行 / Whether feasible
    pub feasible: bool,
    /// 目标函数值 / Objective value
    pub objective_value: Option<f64>,
    /// MIP 间隙 / MIP gap
    pub gap: Option<f64>,
    /// 已用时间 / Elapsed time
    #[serde(rename = "elapsedMs", with = "duration_millis")]
    pub elapsed: Duration,
    /// 附加消息 / Additional message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// 求解结果。
/// Solve result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolveResult {
    /// 是否可行 / Whether feasible
    pub feasible: bool,
    /// 是否最优 / Whether optimal
    pub optimal: bool,
    /// 目标函数值 / Objective value
    pub objective_value: Option<f64>,
    /// MIP 间隙 / MIP gap
    pub gap: Option<f64>,
    /// 已用时间 / Elapsed time
    #[serde(rename = "elapsedMs", with = "duration_millis")]
    pub elapsed: Duration,
    /// 检查点引用 / Checkpoint reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint_ref: Option<ObjectRef>,
    /// 检查点身份和 provenance / Checkpoint identity and provenance
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint_metadata: Option<SolveCheckpoint>,
    /// 结果引用 / Result reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_ref: Option<ObjectRef>,
    /// 求解运行 ID（旧协议兼容字段）/ Solve run identity for legacy results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// 求解 attempt ID（旧协议兼容字段）/ Solve attempt identity for legacy results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<String>,
    /// 结果 artifact 摘要（旧协议兼容字段）/ Result-artifact digest for legacy results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_digest: Option<String>,
    /// 版本化统一求解报告 / Versioned unified solve report
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<RemoteSolveReportDto>,
    /// 附加消息 / Additional message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// 扩展字段 / Extension fields
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extension: BTreeMap<String, String>,
}

impl SolveResult {
    /// 从切片结果构造兜底最终结果。
    /// Create a fallback final result from a slice result.
    pub fn from_slice_result(
        slice_result: &SliceResult,
        elapsed: Duration,
        checkpoint_ref: Option<ObjectRef>,
    ) -> Self {
        Self {
            // 没有最终结果 artifact 时，slice 的完成状态不能形成数学结论。
            // Without a final result artifact, slice completion cannot establish a mathematical conclusion.
            feasible: false,
            optimal: false,
            objective_value: None,
            gap: None,
            elapsed,
            checkpoint_ref,
            checkpoint_metadata: None,
            result_ref: None,
            run_id: None,
            attempt_id: None,
            artifact_digest: None,
            report: None,
            message: slice_result.message.clone(),
            extension: BTreeMap::new(),
        }
    }
}

/// 当前远程报告协议版本 / Current remote-report protocol version.
pub const CURRENT_REMOTE_SOLVE_REPORT_SCHEMA_VERSION: &str = "1.0";

/// 版本化远程统一报告包装 / Versioned remote unified-report envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSolveReportDto {
    /// 远程协议 schema 版本 / Remote protocol schema version.
    pub schema_version: String,
    /// 求解运行 ID / Solve run identity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// 求解尝试 ID / Solve attempt identity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<String>,
    /// 统一求解报告 / Unified solve report.
    pub report: ospf_rust_core::solver::SolveReport<f64>,
    /// 结果 artifact 摘要 / Result-artifact digest.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_digest: Option<String>,
    /// 与报告关联的 portable checkpoint / Portable checkpoint associated with the report.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint: Option<SolveCheckpoint>,
}

impl RemoteSolveReportDto {
    /// 创建版本化报告包装 / Create a versioned report envelope.
    pub fn new(report: ospf_rust_core::solver::SolveReport<f64>) -> Self {
        Self {
            schema_version: CURRENT_REMOTE_SOLVE_REPORT_SCHEMA_VERSION.to_owned(),
            run_id: None,
            attempt_id: None,
            report,
            artifact_digest: None,
            checkpoint: None,
        }
    }

    /// 校验远程报告版本和核心报告版本 / Validate remote and core report versions.
    pub fn validate_schema_version(&self) -> RemoteSolverResult<()> {
        if self.schema_version != CURRENT_REMOTE_SOLVE_REPORT_SCHEMA_VERSION {
            return Err(RemoteSolverError::new(
                RemoteSolverErrorCode::UnsupportedProtocolVersion,
                format!(
                    "unsupported remote solve report schema version '{}', expected '{}'.",
                    self.schema_version, CURRENT_REMOTE_SOLVE_REPORT_SCHEMA_VERSION
                ),
            ));
        }
        if self.report.schema_version != ospf_rust_core::solver::CURRENT_SOLVE_REPORT_SCHEMA_VERSION
        {
            return Err(RemoteSolverError::new(
                RemoteSolverErrorCode::UnsupportedProtocolVersion,
                format!(
                    "unsupported core solve report schema version '{}'.",
                    self.report.schema_version
                ),
            ));
        }
        for (field, value) in [
            ("run_id", self.run_id.as_deref()),
            ("attempt_id", self.attempt_id.as_deref()),
            ("artifact_digest", self.artifact_digest.as_deref()),
        ] {
            if value.is_some_and(|value| value.trim().is_empty()) {
                return Err(RemoteSolverError::invalid_argument(format!(
                    "remote solve report {} cannot be blank",
                    field
                )));
            }
        }
        if let Some(checkpoint) = self.checkpoint.as_ref() {
            checkpoint.validate().map_err(|error| {
                RemoteSolverError::invalid_argument(format!(
                    "remote checkpoint failed validation: {}",
                    error
                ))
            })?;
            if self.run_id.as_deref() != Some(checkpoint.run_id.as_str())
                || self.attempt_id.as_deref() != Some(checkpoint.attempt_id.as_str())
                || self.report.fingerprints.model.as_ref() != Some(&checkpoint.model_fingerprint)
                || self.report.fingerprints.configuration.as_ref()
                    != Some(&checkpoint.configuration_fingerprint)
                || self.report.fingerprints.solver.as_ref() != Some(&checkpoint.solver_fingerprint)
                || self.report.provenance != checkpoint.provenance
            {
                return Err(RemoteSolverError::invalid_argument(
                    "remote report and checkpoint identities or provenance do not match",
                ));
            }
        }
        self.report.validate().map_err(|error| {
            RemoteSolverError::invalid_argument(format!(
                "remote solve report failed invariant validation: {}",
                error
            ))
        })
    }

    /// 校验报告与调用方期望的身份和 artifact 摘要一致。
    /// Validate report identity and artifact digest against caller expectations.
    pub fn validate_identity(
        &self,
        expected_run_id: Option<&str>,
        expected_attempt_id: Option<&str>,
        expected_artifact_digest: Option<&str>,
    ) -> RemoteSolverResult<()> {
        self.validate_schema_version()?;
        for (field, expected, actual) in [
            ("run_id", expected_run_id, self.run_id.as_deref()),
            (
                "attempt_id",
                expected_attempt_id,
                self.attempt_id.as_deref(),
            ),
            (
                "artifact_digest",
                expected_artifact_digest,
                self.artifact_digest.as_deref(),
            ),
        ] {
            if let Some(expected) = expected
                && actual != Some(expected)
            {
                return Err(RemoteSolverError::invalid_argument(format!(
                    "remote solve report {} does not match the expected value",
                    field
                )));
            }
        }
        Ok(())
    }

    /// 校验报告绑定的模型指纹 / Validate the model fingerprint bound to the report.
    pub fn validate_model_fingerprint(
        &self,
        expected_model: &ospf_rust_core::solver::ModelFingerprint,
    ) -> RemoteSolverResult<()> {
        self.validate_schema_version()?;
        let actual = self.report.fingerprints.model.as_ref().ok_or_else(|| {
            RemoteSolverError::invalid_argument(
                "versioned remote solve report is missing its model fingerprint",
            )
        })?;
        if actual != expected_model {
            return Err(RemoteSolverError::invalid_argument(
                "versioned remote solve report model fingerprint does not match the requested model",
            ));
        }
        Ok(())
    }
}

/// 序列化变量类型。
/// Serialized variable type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SerializedVariableType {
    /// 连续 / Continuous
    Continuous,
    /// 二元 / Binary
    Binary,
    /// 整数 / Integer
    Integer,
    /// 半连续 / Semi-continuous
    SemiContinuous,
    /// 半整数 / Semi-integer
    SemiInteger,
}

/// 序列化约束符号。
/// Serialized constraint sign.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SerializedConstraintSign {
    /// 小于等于 / Less than or equal
    LessEqual,
    /// 大于等于 / Greater than or equal
    GreaterEqual,
    /// 等于 / Equal
    Equal,
}

/// 序列化目标类型。
/// Serialized objective category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SerializedObjectiveCategory {
    /// 最小化 / Minimize
    Minimize,
    /// 最大化 / Maximize
    Maximize,
}

/// 当前远程模型 schema 版本 / Current remote model schema version.
pub const CURRENT_REMOTE_MODEL_SCHEMA_VERSION: &str = "1.0";

fn default_remote_model_schema_version() -> String {
    "0.legacy".to_owned()
}

/// 约束矩阵单元。
/// Constraint matrix cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedConstraintCell {
    /// 行索引 / Row index
    pub row_index: usize,
    /// 列索引 / Column index
    pub col_index: usize,
    /// 系数 / Coefficient
    pub coefficient: f64,
}

/// 目标函数单元。
/// Objective cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedObjectiveCell {
    /// 列索引 / Column index
    pub col_index: usize,
    /// 系数 / Coefficient
    pub coefficient: f64,
}

/// 序列化变量。
/// Serialized variable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedVariable {
    /// 报告级稳定变量 ID / Report-level stable variable ID
    #[serde(default)]
    pub stable_id: String,
    /// 变量索引 / Variable index
    pub index: usize,
    /// 变量名称 / Variable name
    pub name: String,
    /// 下界 / Lower bound
    pub lower_bound: f64,
    /// 上界 / Upper bound
    pub upper_bound: f64,
    /// 变量类型 / Variable type
    #[serde(rename = "type")]
    pub variable_type: SerializedVariableType,
}

/// 序列化线性约束。
/// Serialized linear constraint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedConstraint {
    /// 报告级稳定约束 ID / Report-level stable constraint ID
    #[serde(default)]
    pub stable_id: String,
    /// 约束矩阵单元 / Constraint matrix cells
    pub cells: Vec<SerializedConstraintCell>,
    /// 约束符号 / Constraint sign
    pub sign: SerializedConstraintSign,
    /// 右端项 / Right-hand side
    pub rhs: f64,
    /// 约束名称 / Constraint name
    pub name: String,
}

/// 序列化线性目标。
/// Serialized linear objective.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedObjective {
    /// 报告级稳定目标 ID / Report-level stable objective ID
    #[serde(default)]
    pub stable_id: String,
    /// 目标类型 / Objective category
    pub category: SerializedObjectiveCategory,
    /// 目标函数单元 / Objective cells
    pub cells: Vec<SerializedObjectiveCell>,
    /// 常数项 / Constant term
    pub constant: f64,
}

/// 序列化线性模型。
/// Serialized linear model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedLinearModel {
    /// 模型 schema 版本 / Model schema version
    #[serde(default = "default_remote_model_schema_version")]
    pub schema_version: String,
    /// 模型名称 / Model name
    pub name: String,
    /// 变量列表 / Variables
    pub variables: Vec<SerializedVariable>,
    /// 约束列表 / Constraints
    pub constraints: Vec<SerializedConstraint>,
    /// 目标函数 / Objective
    pub objective: SerializedObjective,
}

impl SerializedLinearModel {
    /// 创建空线性模型。
    /// Create an empty linear model.
    pub fn empty(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            schema_version: CURRENT_REMOTE_MODEL_SCHEMA_VERSION.to_owned(),
            name: name.clone(),
            variables: Vec::new(),
            constraints: Vec::new(),
            objective: SerializedObjective {
                stable_id: format!("linear-objective:{}", name),
                category: SerializedObjectiveCategory::Minimize,
                cells: Vec::new(),
                constant: 0.0,
            },
        }
    }

    /// 变量数量。
    /// Variable count.
    pub fn variable_count(&self) -> usize {
        self.variables.len()
    }

    /// 约束数量。
    /// Constraint count.
    pub fn constraint_count(&self) -> usize {
        self.constraints.len()
    }

    /// 校验当前 schema 的稳定模型身份 / Validate stable model identities for the current schema.
    pub fn validate_contract(&self) -> RemoteSolverResult<()> {
        validate_model_schema(&self.schema_version)?;
        if self.schema_version == "0.legacy" {
            return Ok(());
        }
        validate_stable_ids(
            "linear variable",
            self.variables
                .iter()
                .map(|variable| variable.stable_id.as_str()),
        )?;
        validate_stable_ids(
            "linear constraint",
            self.constraints
                .iter()
                .map(|constraint| constraint.stable_id.as_str()),
        )?;
        validate_non_blank_id("linear objective", &self.objective.stable_id)
    }
}

/// 二次约束单元。
/// Quadratic constraint cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedQuadraticConstraintCell {
    /// 行索引 / Row index
    pub row_index: usize,
    /// 列索引1 / Column index 1
    pub col_index1: usize,
    /// 列索引2 / Column index 2
    pub col_index2: usize,
    /// 系数 / Coefficient
    pub coefficient: f64,
}

/// 序列化二次约束。
/// Serialized quadratic constraint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedQuadraticConstraint {
    /// 报告级稳定二次约束 ID / Report-level stable quadratic-constraint ID
    #[serde(default)]
    pub stable_id: String,
    /// 线性部分单元 / Linear part cells
    pub linear_cells: Vec<SerializedConstraintCell>,
    /// 二次部分单元 / Quadratic part cells
    pub quadratic_cells: Vec<SerializedQuadraticConstraintCell>,
    /// 约束符号 / Constraint sign
    pub sign: SerializedConstraintSign,
    /// 右端项 / Right-hand side
    pub rhs: f64,
    /// 约束名称 / Constraint name
    pub name: String,
}

/// 二次目标单元。
/// Quadratic objective cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedQuadraticObjectiveCell {
    /// 列索引1 / Column index 1
    pub col_index1: usize,
    /// 列索引2 / Column index 2
    pub col_index2: usize,
    /// 系数 / Coefficient
    pub coefficient: f64,
}

/// 序列化二次目标。
/// Serialized quadratic objective.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedQuadraticObjective {
    /// 报告级稳定目标 ID / Report-level stable objective ID
    #[serde(default)]
    pub stable_id: String,
    /// 目标类型 / Objective category
    pub category: SerializedObjectiveCategory,
    /// 线性部分单元 / Linear part cells
    pub linear_cells: Vec<SerializedObjectiveCell>,
    /// 二次部分单元 / Quadratic part cells
    pub quadratic_cells: Vec<SerializedQuadraticObjectiveCell>,
    /// 常数项 / Constant term
    pub constant: f64,
}

/// 序列化二次模型。
/// Serialized quadratic model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedQuadraticModel {
    /// 模型 schema 版本 / Model schema version
    #[serde(default = "default_remote_model_schema_version")]
    pub schema_version: String,
    /// 模型名称 / Model name
    pub name: String,
    /// 变量列表 / Variables
    pub variables: Vec<SerializedVariable>,
    /// 线性约束列表 / Linear constraints
    pub linear_constraints: Vec<SerializedConstraint>,
    /// 二次约束列表 / Quadratic constraints
    pub quadratic_constraints: Vec<SerializedQuadraticConstraint>,
    /// 目标函数 / Objective
    pub objective: SerializedQuadraticObjective,
}

impl SerializedQuadraticModel {
    /// 校验当前 schema 的稳定模型身份 / Validate stable model identities for the current schema.
    pub fn validate_contract(&self) -> RemoteSolverResult<()> {
        validate_model_schema(&self.schema_version)?;
        if self.schema_version == "0.legacy" {
            return Ok(());
        }
        validate_stable_ids(
            "quadratic variable",
            self.variables
                .iter()
                .map(|variable| variable.stable_id.as_str()),
        )?;
        validate_stable_ids(
            "linear constraint",
            self.linear_constraints
                .iter()
                .map(|constraint| constraint.stable_id.as_str()),
        )?;
        validate_stable_ids(
            "quadratic constraint",
            self.quadratic_constraints
                .iter()
                .map(|constraint| constraint.stable_id.as_str()),
        )?;
        validate_non_blank_id("quadratic objective", &self.objective.stable_id)
    }
}

fn validate_model_schema(schema_version: &str) -> RemoteSolverResult<()> {
    if schema_version == CURRENT_REMOTE_MODEL_SCHEMA_VERSION || schema_version == "0.legacy" {
        Ok(())
    } else {
        Err(RemoteSolverError::new(
            RemoteSolverErrorCode::UnsupportedProtocolVersion,
            format!(
                "unsupported remote model schema version '{}', expected '{}' or '0.legacy'.",
                schema_version, CURRENT_REMOTE_MODEL_SCHEMA_VERSION
            ),
        ))
    }
}

fn validate_stable_ids<'a>(
    kind: &str,
    ids: impl IntoIterator<Item = &'a str>,
) -> RemoteSolverResult<()> {
    let mut seen = std::collections::BTreeSet::new();
    for id in ids {
        validate_non_blank_id(kind, id)?;
        if !seen.insert(id) {
            return Err(RemoteSolverError::invalid_argument(format!(
                "{} stable ID '{}' is duplicated",
                kind, id
            )));
        }
    }
    Ok(())
}

fn validate_non_blank_id(kind: &str, id: &str) -> RemoteSolverResult<()> {
    if id.trim().is_empty() {
        return Err(RemoteSolverError::invalid_argument(format!(
            "{} stable ID cannot be blank",
            kind
        )));
    }
    Ok(())
}

/// 序列化解。
/// Serialized solution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedSolution {
    /// 是否可行 / Whether feasible
    pub feasible: bool,
    /// 是否最优 / Whether optimal
    pub optimal: bool,
    /// 目标函数值 / Objective value
    pub objective_value: Option<f64>,
    /// MIP 间隙 / MIP gap
    pub gap: Option<f64>,
    /// 变量取值 / Variable values
    #[serde(default)]
    pub variable_values: Vec<f64>,
    /// 已用时间 / Elapsed time
    #[serde(rename = "elapsedMs", with = "duration_millis")]
    pub elapsed: Duration,
    /// 求解器状态 / Solver status
    #[serde(default)]
    pub solver_status: String,
    /// 版本化统一求解报告 / Versioned unified solve report
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<RemoteSolveReportDto>,
    /// 附加消息 / Additional message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl SerializedSolution {
    /// 创建不可行解。
    /// Create an infeasible solution.
    pub fn infeasible(message: Option<String>) -> Self {
        Self {
            feasible: false,
            optimal: false,
            objective_value: None,
            gap: None,
            variable_values: Vec::new(),
            elapsed: Duration::ZERO,
            solver_status: String::new(),
            report: None,
            message: Some(message.unwrap_or_else(|| "Model is infeasible".to_string())),
        }
    }

    /// 创建无界解。
    /// Create an unbounded solution.
    pub fn unbounded(message: Option<String>) -> Self {
        Self {
            feasible: false,
            optimal: false,
            objective_value: None,
            gap: None,
            variable_values: Vec::new(),
            elapsed: Duration::ZERO,
            solver_status: String::new(),
            report: None,
            message: Some(message.unwrap_or_else(|| "Model is unbounded".to_string())),
        }
    }

    /// 创建错误解。
    /// Create an error solution.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            feasible: false,
            optimal: false,
            objective_value: None,
            gap: None,
            variable_values: Vec::new(),
            elapsed: Duration::ZERO,
            solver_status: String::new(),
            report: None,
            message: Some(message.into()),
        }
    }
}

/// 远程求解错误码。
/// Remote solver error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RemoteSolverErrorCode {
    /// 无效参数 / Invalid argument
    InvalidArgument,
    /// 不支持的协议版本 / Unsupported protocol version
    UnsupportedProtocolVersion,
    /// 无效任务状态转换 / Invalid task state transition
    InvalidTaskStateTransition,
    /// 无可用合格节点 / No eligible node available
    NoEligibleNodeAvailable,
    /// 节点离线 / Node offline
    NodeOffline,
    /// 求解器执行失败 / Solver execution failed
    SolverExecutionFailed,
    /// 检查点导出失败 / Checkpoint export failed
    CheckpointExportFailed,
    /// 检查点恢复失败 / Checkpoint restore failed
    CheckpointRestoreFailed,
    /// 事件发布失败 / Event publish failed
    EventPublishFailed,
    /// 存储 IO 失败 / Storage I/O failed
    StorageIoFailed,
    /// 任务在最大轮次内未终止 / Task not terminal within max rounds
    TaskNotTerminalWithinMaxRounds,
    /// 无兼容节点可用 / No compatible node available
    NoCompatibleNodeAvailable,
    /// 任务失败 / Task failed
    TaskFailed,
    /// 任务硬超时失败 / Task failed hard timeout
    TaskFailedHardTimeout,
    /// 任务切片超时失败 / Task failed slice timeout
    TaskFailedSliceTimeout,
    /// 任务预算超限失败 / Task failed budget exceeded
    TaskFailedBudgetExceeded,
    /// 远程求解在最大轮次内未完成 / Remote solve not completed within max rounds
    RemoteSolveNotCompletedWithinMaxRounds,
    /// 内部错误 / Internal error
    InternalError,
}

/// 远程求解错误。
/// Remote solver error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct RemoteSolverError {
    /// 错误码 / Error code
    pub code: RemoteSolverErrorCode,
    /// 错误消息 / Error message
    pub message: String,
    /// 附加元数据 / Additional metadata
    pub metadata: BTreeMap<String, String>,
}

impl RemoteSolverError {
    /// 创建远程求解错误。
    /// Create a remote solver error.
    pub fn new(code: RemoteSolverErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            metadata: BTreeMap::new(),
        }
    }

    /// 创建带元数据的远程求解错误。
    /// Create a remote solver error with metadata.
    pub fn with_metadata(
        mut self,
        metadata: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        self.metadata = metadata
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect();
        self
    }

    /// 创建无效参数错误。
    /// Create an invalid-argument error.
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::new(RemoteSolverErrorCode::InvalidArgument, message)
    }

    /// 创建内部错误。
    /// Create an internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(RemoteSolverErrorCode::InternalError, message)
    }

    /// 创建检查点恢复错误。
    /// Create a checkpoint-restore error.
    pub fn checkpoint_restore(message: impl Into<String>) -> Self {
        Self::new(RemoteSolverErrorCode::CheckpointRestoreFailed, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_ids_trim_and_reject_blank_values() {
        assert_eq!(TaskId::of(" task-1 ").unwrap().value(), "task-1");
        assert!(TaskId::of(" ").is_err());
    }

    #[test]
    fn object_path_normalizes_slashes_and_rejects_nul() {
        assert_eq!(ObjectPath::of("\\root\\a").unwrap().value(), "root/a");
        assert!(ObjectPath::of("a\0b").is_err());
    }

    #[test]
    fn model_data_reports_model_type() {
        let model = SerializedLinearModel::empty("empty");
        let data = ModelData::linear(model);

        assert!(data.is_inline());
        assert_eq!(data.model_type(), NormalizedModelType::Linear);
        assert!(data.validate_contract().is_ok());
    }

    #[test]
    fn current_model_schema_rejects_missing_or_unknown_stable_identity() {
        let mut model = SerializedLinearModel::empty("identity");
        model.objective.stable_id.clear();
        let error = model
            .validate_contract()
            .expect_err("current models must carry an objective identity");
        assert_eq!(error.code, RemoteSolverErrorCode::InvalidArgument);

        let mut model = SerializedLinearModel::empty("version");
        model.schema_version = "9.0".to_owned();
        let error = model
            .validate_contract()
            .expect_err("future model schemas must be rejected");
        assert_eq!(
            error.code,
            RemoteSolverErrorCode::UnsupportedProtocolVersion
        );
    }

    #[test]
    fn task_meta_fills_default_target_type_only_when_missing() {
        let meta = TaskMeta::default()
            .with_default_target_type("linear")
            .unwrap();

        assert_eq!(meta.target_type.unwrap().value(), "linear");
    }

    #[test]
    fn stop_acknowledgement_binds_checkpoint_identity_and_cancellation_chain() {
        let fingerprint = |value: &str| AuditFingerprint {
            schema_version: "1.0".to_owned(),
            algorithm: "sha256".to_owned(),
            value: value.to_owned(),
        };
        let mut checkpoint = SolveCheckpoint::new(
            "run-1",
            "attempt-2",
            Some("attempt-1".to_owned()),
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
        .expect("checkpoint fixture should be valid");
        checkpoint
            .record_cancellation(CancellationRecord {
                origin: ospf_rust_core::solver::CancellationOrigin::RemoteStop,
                requested_at_epoch_ms: 10,
            })
            .expect("checkpoint cancellation should be valid");

        let acknowledgement =
            StopAcknowledgement::new(TaskId::of("run-1").unwrap(), true, TaskStatus::Stopped)
                .with_checkpoint_identity(&checkpoint);

        assert_eq!(acknowledgement.run_id.as_deref(), Some("run-1"));
        assert_eq!(acknowledgement.attempt_id.as_deref(), Some("attempt-2"));
        assert_eq!(
            acknowledgement.model_fingerprint,
            Some(fingerprint("model"))
        );
        assert_eq!(acknowledgement.cancellation_chain.len(), 1);
        assert_eq!(
            acknowledgement
                .provenance
                .as_ref()
                .map(|provenance| provenance.solver_id.as_str()),
            Some("fake/1")
        );
    }
}
