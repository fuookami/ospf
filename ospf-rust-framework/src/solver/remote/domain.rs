//! 远程求解领域模型
//! Remote solver domain models

use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// 远程求解结果。
/// Remote solver result.
pub type RemoteSolverResult<T> = std::result::Result<T, RemoteSolverError>;

macro_rules! remote_string_id {
    ($name:ident) => {
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

remote_string_id!(TaskId);
remote_string_id!(SliceId);
remote_string_id!(NodeId);
remote_string_id!(TenantId);
remote_string_id!(RequestId);
remote_string_id!(HandleId);
remote_string_id!(TraceId);
remote_string_id!(ObjectVersion);
remote_string_id!(ObjectEtag);
remote_string_id!(SolverTypeName);
remote_string_id!(TargetTypeName);
remote_string_id!(BudgetScopeId);
remote_string_id!(OperatorId);
remote_string_id!(OperationSource);
remote_string_id!(ReasonCode);

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

    pub fn serialize<S>(value: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(value.as_millis() as u64)
    }

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

    pub fn serialize<S>(value: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value
            .map(|value| value.as_millis() as u64)
            .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Option::<u64>::deserialize(deserializer)?.map(Duration::from_millis))
    }
}

/// epoch millis SystemTime serde helper。
/// Epoch millis SystemTime serde helper.
pub mod epoch_millis {
    use super::*;
    use serde::ser::Error;

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
    Simple,
    Complex,
}

/// 时间敏感度。
/// Time sensitivity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimeSensitivity {
    Realtime,
    NonRealtime,
}

/// 任务状态。
/// Task status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskStatus {
    Created,
    Accepted,
    Queued,
    Dispatching,
    Running,
    Suspended,
    Completed,
    Stopping,
    Stopped,
    Failed,
    WaitingForBudget,
}

/// 切片状态。
/// Slice status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SliceStatus {
    Planned,
    Running,
    Checkpointing,
    Suspended,
    Completed,
    Failed,
}

/// 求解器类型。
/// Solver type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SolverType {
    Scip,
    Gurobi,
    Auto,
}

/// 标准化模型类型。
/// Normalized model type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NormalizedModelType {
    Linear,
    Quadratic,
    Unknown,
}

/// 对象引用。
/// Object reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectRef {
    pub path: ObjectPath,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<ObjectVersion>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solver_type: Option<SolverTypeName>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_type: Option<TargetTypeName>,
    #[serde(
        rename = "timeLimitMs",
        default,
        with = "option_duration_millis",
        skip_serializing_if = "Option::is_none"
    )]
    pub time_limit: Option<Duration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solution_limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_variable_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_constraint_count: Option<usize>,
    #[serde(
        rename = "historicalRuntimeMs",
        default,
        with = "option_duration_millis",
        skip_serializing_if = "Option::is_none"
    )]
    pub historical_runtime: Option<Duration>,
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
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    pub object_ref: Option<ObjectRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linear_model: Option<SerializedLinearModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quadratic_model: Option<SerializedQuadraticModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_bytes: Option<Vec<u8>>,
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
}

/// 求解配置。
/// Solver config.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SolverConfig {
    #[serde(
        rename = "timeLimitMs",
        default,
        with = "option_duration_millis",
        skip_serializing_if = "Option::is_none"
    )]
    pub time_limit: Option<Duration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solution_limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mip_gap_tolerance: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threads: Option<usize>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub solver_params: BTreeMap<String, String>,
}

/// 求解载荷。
/// Solve payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolvePayload {
    pub model_data: ModelData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_ref: Option<ObjectRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<SolverConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_ref: Option<ObjectRef>,
    #[serde(default)]
    pub task_meta: TaskMeta,
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
}

/// 执行句柄。
/// Execution handle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionHandle {
    pub handle_id: HandleId,
    pub task_id: TaskId,
    pub slice_id: SliceId,
    pub node_id: NodeId,
    #[serde(rename = "startedAtEpochMs", with = "epoch_millis")]
    pub started_at: SystemTime,
}

/// 切片结果。
/// Slice result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SliceResult {
    pub slice_id: SliceId,
    pub completed: bool,
    pub feasible: bool,
    pub objective_value: Option<f64>,
    pub gap: Option<f64>,
    #[serde(rename = "elapsedMs", with = "duration_millis")]
    pub elapsed: Duration,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// 求解结果。
/// Solve result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolveResult {
    pub feasible: bool,
    pub optimal: bool,
    pub objective_value: Option<f64>,
    pub gap: Option<f64>,
    #[serde(rename = "elapsedMs", with = "duration_millis")]
    pub elapsed: Duration,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint_ref: Option<ObjectRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_ref: Option<ObjectRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
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
            feasible: slice_result.feasible,
            optimal: slice_result.gap.unwrap_or(1.0) <= 0.0,
            objective_value: slice_result.objective_value,
            gap: slice_result.gap,
            elapsed,
            checkpoint_ref,
            result_ref: None,
            message: slice_result.message.clone(),
            extension: BTreeMap::new(),
        }
    }
}

/// 序列化变量类型。
/// Serialized variable type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SerializedVariableType {
    Continuous,
    Binary,
    Integer,
    SemiContinuous,
    SemiInteger,
}

/// 序列化约束符号。
/// Serialized constraint sign.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SerializedConstraintSign {
    LessEqual,
    GreaterEqual,
    Equal,
}

/// 序列化目标类型。
/// Serialized objective category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SerializedObjectiveCategory {
    Minimize,
    Maximize,
}

/// 约束矩阵单元。
/// Constraint matrix cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedConstraintCell {
    pub row_index: usize,
    pub col_index: usize,
    pub coefficient: f64,
}

/// 目标函数单元。
/// Objective cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedObjectiveCell {
    pub col_index: usize,
    pub coefficient: f64,
}

/// 序列化变量。
/// Serialized variable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedVariable {
    pub index: usize,
    pub name: String,
    pub lower_bound: f64,
    pub upper_bound: f64,
    #[serde(rename = "type")]
    pub variable_type: SerializedVariableType,
}

/// 序列化线性约束。
/// Serialized linear constraint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedConstraint {
    pub cells: Vec<SerializedConstraintCell>,
    pub sign: SerializedConstraintSign,
    pub rhs: f64,
    pub name: String,
}

/// 序列化线性目标。
/// Serialized linear objective.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedObjective {
    pub category: SerializedObjectiveCategory,
    pub cells: Vec<SerializedObjectiveCell>,
    pub constant: f64,
}

/// 序列化线性模型。
/// Serialized linear model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedLinearModel {
    pub name: String,
    pub variables: Vec<SerializedVariable>,
    pub constraints: Vec<SerializedConstraint>,
    pub objective: SerializedObjective,
}

impl SerializedLinearModel {
    /// 创建空线性模型。
    /// Create an empty linear model.
    pub fn empty(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            variables: Vec::new(),
            constraints: Vec::new(),
            objective: SerializedObjective {
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
}

/// 二次约束单元。
/// Quadratic constraint cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedQuadraticConstraintCell {
    pub row_index: usize,
    pub col_index1: usize,
    pub col_index2: usize,
    pub coefficient: f64,
}

/// 序列化二次约束。
/// Serialized quadratic constraint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedQuadraticConstraint {
    pub linear_cells: Vec<SerializedConstraintCell>,
    pub quadratic_cells: Vec<SerializedQuadraticConstraintCell>,
    pub sign: SerializedConstraintSign,
    pub rhs: f64,
    pub name: String,
}

/// 二次目标单元。
/// Quadratic objective cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedQuadraticObjectiveCell {
    pub col_index1: usize,
    pub col_index2: usize,
    pub coefficient: f64,
}

/// 序列化二次目标。
/// Serialized quadratic objective.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedQuadraticObjective {
    pub category: SerializedObjectiveCategory,
    pub linear_cells: Vec<SerializedObjectiveCell>,
    pub quadratic_cells: Vec<SerializedQuadraticObjectiveCell>,
    pub constant: f64,
}

/// 序列化二次模型。
/// Serialized quadratic model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedQuadraticModel {
    pub name: String,
    pub variables: Vec<SerializedVariable>,
    pub linear_constraints: Vec<SerializedConstraint>,
    pub quadratic_constraints: Vec<SerializedQuadraticConstraint>,
    pub objective: SerializedQuadraticObjective,
}

/// 序列化解。
/// Serialized solution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedSolution {
    pub feasible: bool,
    pub optimal: bool,
    pub objective_value: Option<f64>,
    pub gap: Option<f64>,
    #[serde(default)]
    pub variable_values: Vec<f64>,
    #[serde(rename = "elapsedMs", with = "duration_millis")]
    pub elapsed: Duration,
    #[serde(default)]
    pub solver_status: String,
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
            message: Some(message.into()),
        }
    }
}

/// 远程求解错误码。
/// Remote solver error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RemoteSolverErrorCode {
    InvalidArgument,
    InvalidTaskStateTransition,
    NoEligibleNodeAvailable,
    NodeOffline,
    SolverExecutionFailed,
    CheckpointExportFailed,
    EventPublishFailed,
    StorageIoFailed,
    TaskNotTerminalWithinMaxRounds,
    NoCompatibleNodeAvailable,
    TaskFailed,
    TaskFailedHardTimeout,
    TaskFailedSliceTimeout,
    TaskFailedBudgetExceeded,
    RemoteSolveNotCompletedWithinMaxRounds,
    InternalError,
}

/// 远程求解错误。
/// Remote solver error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct RemoteSolverError {
    pub code: RemoteSolverErrorCode,
    pub message: String,
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
    }

    #[test]
    fn task_meta_fills_default_target_type_only_when_missing() {
        let meta = TaskMeta::default()
            .with_default_target_type("linear")
            .unwrap();

        assert_eq!(meta.target_type.unwrap().value(), "linear");
    }
}
