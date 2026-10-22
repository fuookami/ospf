//! 配置与结构校验 / Configuration and structural validation

use std::collections::{BTreeMap, BTreeSet};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use crate::solver::heuristic::result::Result;
use crate::solver::hyper_heuristic::invalid;

/// 显式组件引用，不序列化闭包 / Explicit component reference without serializing closures
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComponentSpec {
    /// 注册标识 / Registered identifier
    pub r#type: String,
    /// 组件版本 / Component version
    #[serde(default = "version")]
    pub version: u32,
    /// 组件参数 / Component parameters
    #[serde(default)]
    pub parameters: Map<String, Value>,
}

impl ComponentSpec {
    /// 创建版本一的组件引用 / Create a version-one component reference
    pub fn new(id: impl Into<String>) -> Self {
        Self { r#type: id.into(), version: 1, parameters: Map::new() }
    }

    /// 添加显式参数 / Add an explicit parameter
    pub fn parameter(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.parameters.insert(key.into(), value.into());
        self
    }

    pub(crate) fn validate_parameters(&self, allowed: &[&str]) -> Result<()> {
        if self.version != 1 || self.parameters.keys().any(|key| !allowed.contains(&key.as_str())) {
            return Err(invalid(format!("unsupported component version or parameter: {}", self.r#type)));
        }
        Ok(())
    }

    pub(crate) fn number(&self, key: &str, default: f64) -> Result<f64> {
        match self.parameters.get(key) {
            None => Ok(default),
            Some(value) => value.as_f64().filter(|number| number.is_finite())
                .ok_or_else(|| invalid(format!("invalid numeric parameter: {key}"))),
        }
    }

    pub(crate) fn integer(&self, key: &str, default: u64) -> Result<u64> {
        match self.parameters.get(key) {
            None => Ok(default),
            Some(value) => value.as_u64().ok_or_else(|| invalid(format!("invalid integer parameter: {key}"))),
        }
    }
}

fn version() -> u32 { 1 }
fn size() -> usize { 20 }
fn round_robin() -> ComponentSpec { ComponentSpec::new("round-robin") }

/// 初始种子来源 / Initial seed source
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum InitializationMode {
    /// 原生模型初始化 / Native model initialization
    #[default]
    Model,
    /// 原生初始化后导入工作集 / Import the working set after native initialization
    WorkingSet,
}

/// 原生导入能力开关 / Native import capability switch
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportMode {
    /// 按原生语义导入 / Import with native semantics
    #[default]
    Native,
    /// 禁止外部导入 / Disable external imports
    Disabled,
}

/// 算法实例定义 / Algorithm instance definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AlgorithmConfig {
    /// 稳定实例标识 / Stable instance identifier
    pub id: String,
    /// 原生算法引用 / Native algorithm reference
    pub algorithm: ComponentSpec,
    /// 种群规模 / Population size
    #[serde(default = "size")]
    pub size: usize,
    /// 更新算子池 / Update operator pool
    #[serde(default)]
    pub updates: Vec<ComponentSpec>,
    /// 变异算子池 / Mutation operator pool
    #[serde(default)]
    pub mutations: Vec<ComponentSpec>,
    /// 邻域算子池 / Neighborhood operator pool
    #[serde(default)]
    pub neighborhoods: Vec<ComponentSpec>,
    /// 参数源池 / Parameter source pools
    #[serde(default)]
    pub parameters: BTreeMap<String, Vec<ComponentSpec>>,
    /// 槽位选择策略 / Slot selection policy
    #[serde(default = "round_robin")]
    pub selection: ComponentSpec,
    /// 显式标量化标识 / Explicit scalarization identifier
    #[serde(default)]
    pub scalarization: Option<String>,
    /// 组合子会话 / Composite child sessions
    #[serde(default)]
    pub children: Vec<AlgorithmConfig>,
    /// 初始种子来源 / Initial seed source
    #[serde(default)]
    pub initialization: InitializationMode,
    /// 导入模式 / Import mode
    #[serde(default)]
    pub import_mode: ImportMode,
}

impl AlgorithmConfig {
    /// 用稳定实例 ID 和算法 ID 创建 / Create from stable instance and algorithm IDs
    pub fn new(id: impl Into<String>, algorithm: impl Into<String>) -> Self {
        Self {
            id: id.into(), algorithm: ComponentSpec::new(algorithm), size: size(),
            updates: Vec::new(), mutations: Vec::new(), neighborhoods: Vec::new(),
            parameters: BTreeMap::new(), selection: round_robin(), scalarization: None,
            children: Vec::new(), initialization: InitializationMode::Model, import_mode: ImportMode::Native,
        }
    }
}

/// 执行模式 / Execution mode
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// 串行推进 / Serial advancement
    Serial,
    /// 并行批次按稳定 ID 归并 / Parallel batches merged by stable ID
    #[default]
    Synchronous,
    /// 按完成顺序归并 / Merge in completion order
    Asynchronous,
}

/// 学习模式 / Learning mode
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LearningMode {
    /// 开始训练 / Start training
    #[default]
    Train,
    /// 延续已有策略 / Continue a learned policy
    Continue,
    /// 冻结学习参数与探索 / Freeze learned parameters and exploration
    Frozen,
}

/// 全局与片段预算 / Global and slice budgets
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct BudgetConfig {
    /// 全局评估请求上限 / Global evaluation request limit
    pub evaluations: Option<u64>,
    /// 全局高层决策上限 / Global decision limit
    pub decisions: Option<u64>,
    /// 全局完整低层步数上限 / Global complete-step limit
    pub steps: Option<u64>,
    /// 有效运行时间上限，毫秒 / Active runtime limit in milliseconds
    pub time_millis: Option<u64>,
    /// 单片段评估额度 / Evaluation allowance per slice
    pub slice_evaluations: u64,
    /// 单片段完整步数额度 / Complete-step allowance per slice
    pub slice_steps: u64,
    /// 预算分配器 / Budget allocator
    pub allocator: ComponentSpec,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self { evaluations: None, decisions: None, steps: None, time_millis: None,
            slice_evaluations: 256, slice_steps: 1, allocator: ComponentSpec::new("fixed") }
    }
}

/// 执行配置 / Execution configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExecutionConfig {
    /// 执行模式 / Execution mode
    pub mode: ExecutionMode,
    /// 最大并发会话数 / Maximum concurrent sessions
    pub parallelism: usize,
    /// 根随机种子，保留 Kotlin 有符号位模式 / Root seed preserving Kotlin signed bit patterns
    pub seed: i64,
}

impl Default for ExecutionConfig {
    fn default() -> Self { Self { mode: ExecutionMode::Synchronous, parallelism: 1, seed: 1 } }
}

/// 记忆与交换配置 / Memory and exchange configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryConfig {
    /// 有效档案容量 / Valid archive capacity
    pub archive_capacity: usize,
    /// 已接受工作集容量 / Accepted working-set capacity
    pub working_capacity: usize,
    /// 接受策略 / Acceptance policy
    pub acceptance: ComponentSpec,
    /// 交换策略 / Exchange policy
    pub exchange: ComponentSpec,
}

impl Default for MemoryConfig {
    fn default() -> Self { Self { archive_capacity: 100, working_capacity: 20,
        acceptance: ComponentSpec::new("not-worse"), exchange: ComponentSpec::new("none") } }
}

/// 学习配置 / Learning configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct LearningConfig {
    /// 学习模式 / Learning mode
    pub mode: LearningMode,
    /// 特征提取器 / Feature extractor
    pub features: ComponentSpec,
    /// 奖励函数 / Reward policy
    pub reward: ComponentSpec,
    /// 可选轨迹输出位置 / Optional trajectory destination
    pub trajectory_path: Option<String>,
}

impl Default for LearningConfig {
    fn default() -> Self { Self { mode: LearningMode::Train, features: ComponentSpec::new("progress"),
        reward: ComponentSpec::new("improvement"), trajectory_path: None } }
}

/// 完整恢复配置 / Complete recovery configuration
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct PersistenceConfig {
    /// 是否要求完整恢复能力 / Whether complete recovery is required
    pub enabled: bool,
    /// 自动检查点位置 / Automatic checkpoint destination
    pub checkpoint_path: Option<String>,
    /// 自动保存间隔，零为手动 / Automatic interval, zero for manual snapshots
    pub every_decisions: u64,
}

/// 可执行的超启发式配置 / Executable hyper-heuristic configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HyperHeuristicConfig {
    /// 独立配置版本 / Independent configuration version
    #[serde(default = "version")]
    pub version: u32,
    /// 问题工厂引用 / Problem factory reference
    pub problem: ComponentSpec,
    /// 算法实例 / Algorithm instances
    pub algorithms: Vec<AlgorithmConfig>,
    /// 预算定义 / Budget definition
    pub budget: BudgetConfig,
    /// 算法选择策略 / Algorithm selection policy
    #[serde(default = "round_robin")]
    pub selection: ComponentSpec,
    /// 执行配置 / Execution configuration
    #[serde(default)]
    pub execution: ExecutionConfig,
    /// 搜索记忆 / Search memory
    #[serde(default)]
    pub memory: MemoryConfig,
    /// 学习配置 / Learning configuration
    #[serde(default)]
    pub learning: LearningConfig,
    /// 持久化配置 / Persistence configuration
    #[serde(default)]
    pub persistence: PersistenceConfig,
}

impl HyperHeuristicConfig {
    /// 创建有明确评估上限的配置 / Create a configuration with an explicit evaluation limit
    pub fn new(problem: ComponentSpec, evaluations: u64) -> Self {
        Self { version: 1, problem, algorithms: Vec::new(), budget: BudgetConfig {
            evaluations: Some(evaluations), ..BudgetConfig::default()
        }, selection: round_robin(), execution: ExecutionConfig::default(), memory: MemoryConfig::default(),
            learning: LearningConfig::default(), persistence: PersistenceConfig::default() }
    }

    /// 追加算法实例 / Append an algorithm instance
    pub fn algorithm(mut self, algorithm: AlgorithmConfig) -> Self {
        self.algorithms.push(algorithm);
        self
    }

    /// 校验结构，具体组件能力由注册表校验 / Validate structure; the registry validates component capabilities
    pub fn validate(&self) -> Result<()> {
        fn algorithms(values: &[AlgorithmConfig]) -> bool {
            let mut ids = BTreeSet::new();
            !values.is_empty() && values.iter().all(|value| {
                let valid = !value.id.is_empty() && value.id.bytes().next().is_some_and(|byte| byte.is_ascii_alphanumeric())
                    && value.id.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte));
                valid && value.size > 0 && ids.insert(&value.id)
                    && (value.children.is_empty() || algorithms(&value.children))
            })
        }
        if self.version != 1 || !algorithms(&self.algorithms) { return Err(invalid("invalid configuration version or algorithms")); }
        let budget = &self.budget;
        if [budget.evaluations, budget.decisions, budget.steps, budget.time_millis].iter().all(Option::is_none)
            || budget.slice_evaluations == 0 || budget.slice_steps == 0 {
            return Err(invalid("a global stopping limit and positive slice allowances are required"));
        }
        if self.execution.parallelism == 0 || self.memory.archive_capacity == 0 || self.memory.working_capacity == 0 {
            return Err(invalid("capacities and parallelism must be positive"));
        }
        if self.persistence.every_decisions > 0 && (!self.persistence.enabled || self.persistence.checkpoint_path.is_none()) {
            return Err(invalid("automatic checkpoints require persistence and a destination"));
        }
        Ok(())
    }
}

/// JSON 配置格式 / JSON configuration format
pub struct ConfigurationFormats;

/// 配置文本格式 / Configuration text format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigurationFormat {
    /// JSON 文本 / JSON text
    Json,
    /// YAML 文本 / YAML text
    Yaml,
}

impl ConfigurationFormats {
    /// 编码并校验 JSON 配置 / Validate and encode JSON configuration
    pub fn encode(config: &HyperHeuristicConfig) -> Result<String> {
        config.validate()?;
        serde_json::to_string_pretty(config).map_err(|error| invalid(error.to_string()))
    }

    /// 解码并校验 JSON 配置 / Decode and validate JSON configuration
    pub fn decode(text: &str) -> Result<HyperHeuristicConfig> {
        Self::decode_format(text, ConfigurationFormat::Json)
    }

    /// 用指定格式编码同一个配置模型 / Encode the same configuration model in the selected format
    pub fn encode_format(config: &HyperHeuristicConfig, format: ConfigurationFormat) -> Result<String> {
        config.validate()?;
        match format {
            ConfigurationFormat::Json => Self::encode(config),
            ConfigurationFormat::Yaml => serde_yaml_ng::to_string(config).map_err(|error| invalid(error.to_string())),
        }
    }

    /// 用指定格式读取配置，不静默接受未知字段 / Decode the selected format without silently accepting unknown fields
    pub fn decode_format(text: &str, format: ConfigurationFormat) -> Result<HyperHeuristicConfig> {
        Self::decode_migrating(text, format, &BTreeMap::new())
    }

    /// 仅通过调用方显式提供的映射迁移旧版本 / Migrate old versions only through caller-supplied explicit mappings
    pub fn decode_migrating(
        text: &str,
        format: ConfigurationFormat,
        migrations: &BTreeMap<u32, super::persistence::ConfigurationMigration>,
    ) -> Result<HyperHeuristicConfig> {
        if text.len() > 4 * 1024 * 1024 { return Err(invalid("configuration exceeds 4 MiB")); }
        let document: StrictValue = match format {
            ConfigurationFormat::Json => serde_json::from_str(text).map_err(|error| invalid(error.to_string()))?,
            ConfigurationFormat::Yaml => serde_yaml_ng::from_str(text).map_err(|error| invalid(error.to_string()))?,
        };
        let document = super::persistence::migrate_document(document.0, migrations)?;
        let config: HyperHeuristicConfig = serde_json::from_value(document).map_err(|error| invalid(error.to_string()))?;
        config.validate()?;
        Ok(config)
    }
}

struct StrictValue(Value);

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = StrictValue;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a JSON-compatible value without duplicate keys")
            }
            fn visit_bool<E: serde::de::Error>(self, value: bool) -> std::result::Result<StrictValue, E> { Ok(StrictValue(value.into())) }
            fn visit_i64<E: serde::de::Error>(self, value: i64) -> std::result::Result<StrictValue, E> { Ok(StrictValue(value.into())) }
            fn visit_u64<E: serde::de::Error>(self, value: u64) -> std::result::Result<StrictValue, E> { Ok(StrictValue(value.into())) }
            fn visit_f64<E: serde::de::Error>(self, value: f64) -> std::result::Result<StrictValue, E> {
                serde_json::Number::from_f64(value).map(|number| StrictValue(Value::Number(number)))
                    .ok_or_else(|| E::custom("non-finite configuration number"))
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> std::result::Result<StrictValue, E> { Ok(StrictValue(value.into())) }
            fn visit_string<E: serde::de::Error>(self, value: String) -> std::result::Result<StrictValue, E> { Ok(StrictValue(value.into())) }
            fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<StrictValue, E> { Ok(StrictValue(Value::Null)) }
            fn visit_none<E: serde::de::Error>(self) -> std::result::Result<StrictValue, E> { Ok(StrictValue(Value::Null)) }
            fn visit_some<D: serde::Deserializer<'de>>(self, value: D) -> std::result::Result<StrictValue, D::Error> { StrictValue::deserialize(value) }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut sequence: A) -> std::result::Result<StrictValue, A::Error> {
                let mut values = Vec::new();
                while let Some(StrictValue(value)) = sequence.next_element()? { values.push(value); }
                Ok(StrictValue(Value::Array(values)))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut object: A) -> std::result::Result<StrictValue, A::Error> {
                let mut values = Map::new();
                while let Some(key) = object.next_key::<String>()? {
                    if values.contains_key(&key) { return Err(serde::de::Error::custom(format!("duplicate key: {key}"))); }
                    let StrictValue(value) = object.next_value()?;
                    values.insert(key, value);
                }
                Ok(StrictValue(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

pub(crate) fn compact_algorithm(algorithm: &AlgorithmConfig) -> Result<Value> {
    let mut value = serde_json::to_value(algorithm).map_err(|error| invalid(error.to_string()))?;
    let defaults = serde_json::to_value(AlgorithmConfig::new("", "")).map_err(|error| invalid(error.to_string()))?;
    remove_defaults(&mut value, &defaults);
    if let Some(spec) = value.get_mut("algorithm") { compact_component(spec); }
    if let Some(spec) = value.get_mut("selection") { compact_component(spec); }
    for field in ["updates", "mutations", "neighborhoods"] {
        if let Some(Value::Array(specs)) = value.get_mut(field) { for spec in specs { compact_component(spec); } }
    }
    if let Some(Value::Object(parameters)) = value.get_mut("parameters") {
        for specs in parameters.values_mut() { if let Value::Array(specs) = specs { for spec in specs { compact_component(spec); } } }
    }
    if !algorithm.children.is_empty() {
        value["children"] = Value::Array(algorithm.children.iter().map(compact_algorithm).collect::<Result<_>>()?);
    }
    Ok(value)
}

pub(crate) fn compact_component(value: &mut Value) {
    if let Some(fields) = value.as_object_mut() {
        if fields.get("version") == Some(&Value::from(1)) { fields.remove("version"); }
        if fields.get("parameters").and_then(Value::as_object).is_some_and(Map::is_empty) { fields.remove("parameters"); }
    }
}

fn remove_defaults(value: &mut Value, defaults: &Value) {
    if let Some(fields) = value.as_object_mut() { fields.retain(|key, value| defaults.get(key) != Some(value)); }
}

pub(crate) fn checkpoint_configuration(config: &HyperHeuristicConfig) -> Result<Value> {
    let mut value = serde_json::to_value(config).map_err(|error| invalid(error.to_string()))?;
    let fields = value.as_object_mut().ok_or_else(|| invalid("configuration must be an object"))?;
    for (name, defaults) in [
        ("execution", serde_json::to_value(ExecutionConfig::default())),
        ("memory", serde_json::to_value(MemoryConfig::default())),
        ("learning", serde_json::to_value(LearningConfig::default())),
        ("persistence", serde_json::to_value(PersistenceConfig::default())),
    ] {
        let defaults = defaults.map_err(|error| invalid(error.to_string()))?;
        if fields.get(name) == Some(&defaults) { fields.remove(name); }
        else if let Some(value) = fields.get_mut(name) { remove_defaults(value, &defaults); }
    }
    if config.version == 1 { fields.remove("version"); }
    if config.selection == round_robin() { fields.remove("selection"); }
    remove_defaults(&mut value["budget"], &serde_json::to_value(BudgetConfig::default()).map_err(|error| invalid(error.to_string()))?);
    compact_component(&mut value["problem"]);
    if let Some(spec) = value.get_mut("selection") { compact_component(spec); }
    for (field, names) in [("budget", &["allocator"][..]), ("memory", &["acceptance", "exchange"][..]), ("learning", &["features", "reward"][..])] {
        if let Some(record) = value.get_mut(field) { for name in names { if let Some(spec) = record.get_mut(*name) { compact_component(spec); } } }
    }
    value["algorithms"] = Value::Array(config.algorithms.iter().map(compact_algorithm).collect::<Result<_>>()?);
    Ok(value)
}
