//! 独立版本的策略、检查点与原子文件存储 / Independently versioned policies, checkpoints, and atomic file storage

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use serde_json::Value;
use sha2::{Digest, Sha256};
use crate::solver::heuristic::result::Result;
use crate::solver::heuristic::session::SearchSessionSnapshot;
use crate::solver::hyper_heuristic::invalid;
use crate::solver::hyper_heuristic::selection::{PolicySnapshot, SelectionTransition};

/// 与问题实例状态分离的可迁移策略 / Transferable policy separate from problem instance state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PolicyArchive {
    /// 独立格式版本 / Independent format version
    #[serde(default = "version")]
    pub version: u32,
    /// 算法与学习定义指纹 / Fingerprint of algorithms and learning definitions
    pub definition: String,
    /// 特征协议版本 / Feature protocol version
    pub feature_version: String,
    /// 高层策略 / Upper-level policy
    pub root: PolicySnapshot,
    /// 各算法内层策略 / Inner policies per algorithm
    pub operators: BTreeMap<String, BTreeMap<String, PolicySnapshot>>,
    /// 训练元信息 / Training metadata
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    /// 完整策略定义 / Complete policy definitions
    #[serde(default = "empty_object")]
    pub definitions: Value,
}

/// 一个静止 worker 的完整状态 / Complete state of a quiescent worker
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkerCheckpoint {
    /// 原生会话状态 / Native session state
    pub session: SearchSessionSnapshot,
    /// 实际随机流位模式 / Actual random stream bit pattern
    pub random: i64,
    /// 是否完整初始化 / Whether fully initialized
    pub initialized: bool,
    /// 是否停止派发 / Whether dispatch is disabled
    pub disabled: bool,
    /// 问题回调状态 / Problem callback state
    #[serde(default)]
    pub problem_state: Value,
}

/// 静止边界完整检查点 / Complete checkpoint at a quiescent boundary
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunCheckpoint {
    /// 已消费的调度前缀；恢复后必须重新绑定录制或重放 / Consumed schedule prefix; restore requires rebinding recording or replay
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule: Option<super::schedule::ScheduleCheckpoint>,
    /// 独立格式版本 / Independent format version
    #[serde(default = "version")]
    pub version: u32,
    /// 配置指纹 / Configuration fingerprint
    pub configuration: String,
    /// 问题指纹 / Problem fingerprint
    pub problem: String,
    /// 学习策略 / Learned policy
    pub policy: PolicyArchive,
    /// worker 原生与回调状态 / Worker native and callback states
    pub workers: BTreeMap<String, WorkerCheckpoint>,
    /// 无损有效档案 / Lossless valid archive
    pub archive: Value,
    /// 无损工作集 / Lossless working set
    pub working: Value,
    /// 已消费评估请求 / Consumed evaluation requests
    pub evaluations: u64,
    /// 已派发序号 / Dispatched sequence
    pub dispatched: u64,
    /// 已完成决策 / Completed decisions
    pub completed: u64,
    /// 完整低层步数 / Complete low-level steps
    pub steps: u64,
    /// 全局停滞次数 / Global stagnation
    pub stagnation: u64,
    /// 有效运行时间 / Active runtime
    pub active_nanos: u64,
    /// 协调器随机状态 / Coordinator random state
    pub random: i64,
    /// 失败诊断 / Failure diagnostics
    #[serde(default)]
    pub diagnostics: Vec<String>,
    /// 协调器回调状态 / Coordinator callback state
    #[serde(default)]
    pub problem_state: Value,
    /// 真实目标调用次数 / Actual objective calls
    #[serde(default)]
    pub objective_calls: u64,
    /// 已完成轨迹 / Completed transitions
    #[serde(default)]
    pub transitions: Vec<SelectionTransition>,
    /// 改善次数 / Improvement count
    #[serde(default)]
    pub successes: u64,
    /// 最近动作成本 / Latest action cost
    #[serde(default)]
    pub last_cost: u64,
    /// 完整配置文档 / Complete configuration document
    #[serde(default)]
    pub configuration_data: Value,
    /// 全局可恢复组件状态 / Global recoverable component state
    #[serde(default)]
    pub components: BTreeMap<String, Value>,
}

impl RunCheckpoint {
    /// 检查 Kotlin v1 无法保存的已知状态；不认证自定义组件的业务语义。
    /// Check known state unsupported by Kotlin v1; custom component semantics still require independent validation.
    pub fn validate_kotlin_v1(&self) -> Result<()> {
        if self.schedule.is_some() { return Err(invalid("Kotlin 2eb7943 does not support schedule replay state")); }
        if self.version != 1 || self.policy.version != 1 {
            return Err(invalid("Kotlin v1 export requires checkpoint and policy version 1"));
        }
        if self.transitions.iter().any(|event| event.previous_action.is_some()) {
            return Err(invalid("Kotlin 2eb7943 does not support previousAction replay metadata"));
        }
        validate_kotlin_algorithms(&self.configuration_data, "configurationData", 0)?;
        validate_kotlin_algorithms(&self.policy.definitions, "policy/definitions", 0)?;
        for (id, worker) in &self.workers {
            validate_kotlin_session(&worker.session.state, &format!("workers/{id}"), 0)?;
        }
        Ok(())
    }
}

fn validate_kotlin_algorithms(
    document: &Value,
    path: &str,
    depth: usize,
) -> Result<()> {
    if depth > 128 { return Err(invalid("Kotlin algorithm nesting exceeds 128")); }
    for field in ["algorithms", "children"] {
        if let Some(algorithms) = document.get(field).and_then(Value::as_array) {
            for (index, algorithm) in algorithms.iter().enumerate() {
                let path = format!("{path}/{field}/{index}");
                if algorithm.get("algorithm").and_then(|spec| spec.get("parameters"))
                    .is_some_and(|parameters| parameters.get("numerics").is_some()) {
                    return Err(invalid(format!("{path}/algorithm/parameters/numerics: Kotlin 2eb7943 does not support explicit numerical contracts")));
                }
                validate_kotlin_algorithms(algorithm, &path, depth + 1)?;
            }
        }
    }
    Ok(())
}

fn validate_kotlin_session(
    state: &Value,
    path: &str,
    depth: usize,
) -> Result<()> {
    if depth > 128 { return Err(invalid("Kotlin session nesting exceeds 128")); }
    if state.get("slots").is_some() && state.get("localArchive").is_some() {
        if ["rewardState", "rewardStateVersion", "reward"].iter().any(|key| state.get(key).is_some()) {
            return Err(invalid(format!("{path}/rewardState: Kotlin 2eb7943 does not support operator reward-state fields; use the shared reward-state protocol instead of discarding state")));
        }
        if let Some(delegate) = state.get("session").and_then(|session| session.get("state")) {
            validate_kotlin_session(delegate, &format!("{path}/session"), depth + 1)?;
        }
    }
    if let Some(children) = state.get("children").and_then(Value::as_object) {
        for (id, child) in children {
            if let Some(state) = child.get("state") {
                validate_kotlin_session(state, &format!("{path}/children/{id}"), depth + 1)?;
            }
        }
    }
    Ok(())
}

fn version() -> u32 { 1 }
fn empty_object() -> Value { serde_json::json!({}) }

/// 显式旧版本迁移函数 / Explicit source-version migration
pub type ConfigurationMigration = Box<dyn Fn(Value) -> Result<Value> + Send + Sync>;

/// 持久化 SPI / Persistence SPI
pub trait SearchStore {
    /// 保存可迁移策略 / Save a transferable policy
    fn save_policy(&self, key: &str, policy: &PolicyArchive) -> Result<()>;
    /// 加载并校验策略 / Load and validate a policy
    fn load_policy(&self, key: &str) -> Result<PolicyArchive>;
    /// 保存完整检查点 / Save a complete checkpoint
    fn save_checkpoint(&self, key: &str, checkpoint: &RunCheckpoint) -> Result<()>;
    /// 加载并校验检查点 / Load and validate a checkpoint
    fn load_checkpoint(&self, key: &str) -> Result<RunCheckpoint>;
}

/// 带校验和、安全键和原子替换的本地文件存储 / Local storage with checksums, safe keys, and atomic replacement
pub struct FileSearchStore {
    root: PathBuf,
    policy_migrations: BTreeMap<u32, ConfigurationMigration>,
    checkpoint_migrations: BTreeMap<u32, ConfigurationMigration>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredDocument {
    #[serde(default = "version")]
    version: u32,
    kind: String,
    checksum: String,
    payload: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStoredDocument {
    #[serde(default = "version")]
    version: u32,
    kind: String,
    checksum: String,
    payload: Box<serde_json::value::RawValue>,
}

impl FileSearchStore {
    /// 保存已校验的完整调度日志 / Save a validated complete schedule log
    pub fn save_schedule(&self, key: &str, log: &super::schedule::ScheduleLog) -> Result<()> {
        log.validate()?; self.write(key, "schedule", log)
    }

    /// 载入并校验完整调度日志 / Load and validate a complete schedule log
    pub fn load_schedule(&self, key: &str) -> Result<super::schedule::ScheduleLog> {
        let log: super::schedule::ScheduleLog = self.read(key, "schedule")?;
        log.validate()?; Ok(log)
    }

    /// 创建并解析本地存储目录 / Create and resolve the local storage directory
    pub fn new(directory: impl AsRef<Path>) -> Result<Self> {
        fs::create_dir_all(directory.as_ref()).map_err(io_error)?;
        Ok(Self { root: fs::canonicalize(directory.as_ref()).map_err(io_error)?,
            policy_migrations: BTreeMap::new(), checkpoint_migrations: BTreeMap::new() })
    }

    /// 预检 Kotlin v1 已知状态限制后原子保存；失败不覆盖已有文件。
    /// Atomically save after checking known Kotlin v1 state restrictions; failure leaves an existing file untouched.
    pub fn save_kotlin_checkpoint(&self, key: &str, checkpoint: &RunCheckpoint) -> Result<()> {
        checkpoint.validate_kotlin_v1()?;
        self.save_checkpoint(key, checkpoint)
    }

    /// 注册独立策略或检查点迁移 / Register an independent policy or checkpoint migration
    pub fn register_migration(
        &mut self,
        kind: &str,
        version: u32,
        migration: ConfigurationMigration,
    ) -> Result<()> {
        let migrations = match kind { "policy" => &mut self.policy_migrations, "checkpoint" => &mut self.checkpoint_migrations,
            _ => return Err(invalid("unknown document kind")) };
        if version == 1 || migrations.contains_key(&version) { return Err(invalid("invalid or duplicate migration version")); }
        migrations.insert(version, migration);
        Ok(())
    }

    fn target(&self, key: &str) -> Result<PathBuf> {
        let valid = !key.is_empty() && key.len() <= 200 && key.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
            && key != "." && key != ".." && !key.ends_with('.')
            && !["CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
                "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"].contains(&key.split('.').next().unwrap_or("").to_ascii_uppercase().as_str());
        if !valid { return Err(invalid("unsafe storage key")); }
        let target = self.root.join(key);
        if fs::symlink_metadata(&target).is_ok_and(|metadata| metadata.file_type().is_symlink()) { return Err(invalid("symbolic-link storage entries are unsupported")); }
        Ok(target)
    }

    fn write<T: Serialize>(&self, key: &str, kind: &str, value: &T) -> Result<()> {
        let target = self.target(key)?;
        let payload = serde_json::to_value(value).map_err(json_error)?;
        let document = StoredDocument { version: 1, kind: kind.into(), checksum: fingerprint(&payload), payload };
        let text = serde_json::to_vec(&document).map_err(json_error)?;
        if text.len() > 64 * 1024 * 1024 { return Err(invalid("stored document exceeds 64 MiB")); }
        atomic_write(&target, &text)
    }

    fn read<T: DeserializeOwned>(&self, key: &str, kind: &str) -> Result<T> {
        let source = self.target(key)?;
        let file = fs::File::open(source).map_err(io_error)?;
        let mut bytes = Vec::new();
        file.take(64 * 1024 * 1024 + 1).read_to_end(&mut bytes).map_err(io_error)?;
        if bytes.len() > 64 * 1024 * 1024 { return Err(invalid("stored document exceeds 64 MiB")); }
        let document: RawStoredDocument = serde_json::from_slice(&bytes).map_err(json_error)?;
        if document.version != 1 || document.kind != kind || document.checksum != raw_fingerprint(&document.payload)? {
            return Err(invalid("incompatible or corrupt stored document"));
        }
        let migrations = if kind == "policy" { &self.policy_migrations } else { &self.checkpoint_migrations };
        let payload = serde_json::from_str(document.payload.get()).map_err(json_error)?;
        let payload = migrate_document(payload, migrations)?;
        serde_json::from_value(payload).map_err(json_error)
    }
}

impl SearchStore for FileSearchStore {
    fn save_policy(&self, key: &str, policy: &PolicyArchive) -> Result<()> { self.write(key, "policy", policy) }
    fn load_policy(&self, key: &str) -> Result<PolicyArchive> { self.read(key, "policy") }
    fn save_checkpoint(&self, key: &str, checkpoint: &RunCheckpoint) -> Result<()> { self.write(key, "checkpoint", checkpoint) }
    fn load_checkpoint(&self, key: &str) -> Result<RunCheckpoint> { self.read(key, "checkpoint") }
}

pub(crate) fn migrate_document(value: Value, migrations: &BTreeMap<u32, ConfigurationMigration>) -> Result<Value> {
    if !value.is_object() { return Err(invalid("document must be an object")); }
    let version = value.get("version").and_then(Value::as_u64).unwrap_or(1);
    if version == 1 { return Ok(value); }
    let migration = u32::try_from(version).ok().and_then(|version| migrations.get(&version))
        .ok_or_else(|| invalid(format!("unsupported document version: {version}")))?;
    let migrated = super::boundary(|| migration(value))?;
    if migrated.get("version").and_then(Value::as_u64) != Some(1) { return Err(invalid("migration must produce version 1")); }
    Ok(migrated)
}

pub(crate) fn fingerprint(value: &Value) -> String {
    fn canonical(value: &Value) -> String {
        match value {
            Value::Object(entries) => {
                let mut keys: Vec<_> = entries.keys().collect();
                keys.sort_by(|lhs, rhs| lhs.encode_utf16().cmp(rhs.encode_utf16()));
                format!("{{{}}}", keys.iter().map(|key| format!("{}:{}", Value::String((*key).clone()), canonical(&entries[*key]))).collect::<Vec<_>>().join(", "))
            }
            Value::Array(values) => format!("[{}]", values.iter().map(canonical).collect::<Vec<_>>().join(", ")),
            _ => value.to_string(),
        }
    }
    Sha256::digest(canonical(value).as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn raw_fingerprint(value: &serde_json::value::RawValue) -> Result<String> {
    fn canonical(value: &serde_json::value::RawValue, depth: usize) -> Result<String> {
        if depth > 128 { return Err(invalid("stored document nesting exceeds 128")); }
        let text = value.get();
        match text.as_bytes().first() {
            Some(b'{') => {
                let entries: BTreeMap<String, &serde_json::value::RawValue> = serde_json::from_str(text).map_err(json_error)?;
                let mut keys: Vec<_> = entries.keys().collect();
                keys.sort_by(|lhs, rhs| lhs.encode_utf16().cmp(rhs.encode_utf16()));
                let values = keys.into_iter().map(|key| Ok(format!("{}:{}", Value::String(key.clone()), canonical(entries[key], depth + 1)?)))
                    .collect::<Result<Vec<_>>>()?;
                Ok(format!("{{{}}}", values.join(", ")))
            }
            Some(b'[') => {
                let entries: Vec<&serde_json::value::RawValue> = serde_json::from_str(text).map_err(json_error)?;
                let values = entries.into_iter().map(|value| canonical(value, depth + 1)).collect::<Result<Vec<_>>>()?;
                Ok(format!("[{}]", values.join(", ")))
            }
            Some(b'"') => {
                let value: String = serde_json::from_str(text).map_err(json_error)?;
                Ok(Value::String(value).to_string())
            }
            // Kotlin 的校验和保留原始指数拼写，不经过 f64 重排版。
            // Kotlin checksums retain the original exponent spelling without f64 reformatting.
            _ => Ok(text.into()),
        }
    }
    Ok(Sha256::digest(canonical(value, 0)?.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect())
}

pub(crate) fn atomic_write(destination: &Path, bytes: &[u8]) -> Result<()> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let parent = destination.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(io_error)?;
    let temp = parent.join(format!(".search-{}-{}.tmp", std::process::id(), COUNTER.fetch_add(1, Ordering::Relaxed)));
    let mut file = OpenOptions::new().create_new(true).write(true).open(&temp).map_err(io_error)?;
    let result = (|| {
        file.write_all(bytes).map_err(io_error)?;
        file.sync_all().map_err(io_error)?;
        drop(file);
        fs::rename(&temp, destination).map_err(io_error)
    })();
    if result.is_err() { let _ = fs::remove_file(&temp); }
    result
}

fn io_error(error: std::io::Error) -> crate::solver::heuristic::result::HeuristicError { invalid(format!("search storage: {error}")) }
fn json_error(error: serde_json::Error) -> crate::solver::heuristic::result::HeuristicError { invalid(format!("search document: {error}")) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_checksums_preserve_numeric_spelling_and_canonicalize_structure() {
        for number in ["1.0E-7", "1e-07", "0.0000001", "-0.22322009677671706"] {
            let text = format!("{{\"b\": [ {number} ], \"a\": 0}}");
            let raw: Box<serde_json::value::RawValue> = serde_json::from_str(&text).unwrap();
            let expected = format!("{{\"a\":0, \"b\":[{number}]}}");
            let checksum: String = Sha256::digest(expected.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect();
            assert_eq!(raw_fingerprint(&raw).unwrap(), checksum);
        }
        let expected = -0.22322009677671706_f64;
        let actual: f64 = serde_json::from_str("-0.22322009677671706").unwrap();
        assert_eq!(actual.to_bits(), expected.to_bits());
    }
}
