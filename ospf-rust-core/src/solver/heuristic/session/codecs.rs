//! 骨架状态编解码 / Skeleton state codecs
//!
//! 与 Kotlin `RunnerStateCodecs` 对齐。Kotlin 把七种骨架状态都编码成同一个 `NativeState` 记录：
//! `individuals`（个体 JSON）、`vectors`（数值矩阵）、`counters`（整数列）、`solutions`（解 JSON）、`children`（嵌套记录）。
//! 本模块保持同一形状与同一数值文本格式，因此两侧快照可以在文本层面互相读取。
//! Mirrors Kotlin's `RunnerStateCodecs`. Kotlin encodes all seven skeleton states as one `NativeState` record:
//! `individuals` (individual JSON), `vectors` (numeric matrix), `counters` (integers), `solutions` (solution JSON), and
//! `children` (nested records). This module keeps the same shape and number formatting so snapshots are text-compatible.
//!
//! 数值一律编码为**字符串**，与 Kotlin `SearchCodecs.flt64` 的 `JsonPrimitive(value.toString())` 一致。
//! Numbers are always encoded as **strings**, matching Kotlin's `SearchCodecs.flt64`.

use serde_json::Value;

use crate::variable::VariableId;

use crate::solver::heuristic::individual::Individual;
use crate::solver::heuristic::model::{HeuristicValue, from_f64, to_f64};
use crate::solver::heuristic::runner::clpso_runner::ClpsoParticle;
use crate::solver::heuristic::runner::particle_swarm_runner::ParticleState;
use crate::solver::heuristic::runner::shade_runner::{ShadeMemory, ShadeState};
use crate::solver::heuristic::session::json::{
    JsonValue, ListCodec, NullableCodec, SearchCodec, field, invalid_state, object,
};
use crate::solver::heuristic::session::protocol::Result;
use crate::solver::heuristic::session::runner_session::{solution_of, values_of};

/// 有限浮点转文本，与 Kotlin `Double.toString()` 同形 / Finite float to text matching Kotlin's `Double.toString()`
///
/// Kotlin 对整数值补 `.0`、指数用大写 `E`；Rust 的 `f64::to_string()` 是同样的 Ryu 最短往返，但省略 `.0` 且用小写 `e`。
/// Kotlin appends `.0` to integral values and uses an uppercase `E` for exponents; Rust's `f64::to_string()` is the same Ryu
/// shortest round-trip but drops `.0` and uses a lowercase `e`.
pub fn format_number(value: f64) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    let magnitude = value.abs();
    // Kotlin 的 `Double.toString()` 在 [1e-3, 1e7) 之外使用科学计数法；Rust 的 Display 只在很大或很小时才用。
    // Rust 的指数形式不带 `.0`，这里统一补成 `1.0E300` 的形状。
    // Kotlin's `Double.toString()` switches to scientific notation outside [1e-3, 1e7), while Rust's Display only does so at
    // extremes. Rust omits `.0` in the mantissa, so it is added back here.
    if magnitude != 0.0 && !(1.0e-3..1.0e7).contains(&magnitude) {
        let text = format!("{value:E}");
        if let Some((mantissa, exponent)) = text.split_once('E') {
            let mantissa = if mantissa.contains('.') {
                mantissa.to_string()
            } else {
                format!("{mantissa}.0")
            };
            let exponent: i64 = exponent.parse().unwrap_or(0);
            return format!("{mantissa}E{exponent}");
        }
        return text;
    }

    let text = format!("{value}");
    if text.contains('.') {
        text
    } else {
        format!("{text}.0")
    }
}

/// 解析文本数值 / Parse a textual number
pub fn parse_number(text: &str) -> Option<f64> {
    text.parse::<f64>().ok().filter(|value| value.is_finite())
}

/// 有限数值编解码（字符串形式）/ Finite-number codec using the textual form
#[derive(Debug, Clone, Copy, Default)]
pub struct TextNumberCodec;

impl SearchCodec<f64> for TextNumberCodec {
    fn encode(&self, value: &f64) -> Result<JsonValue> {
        if !value.is_finite() {
            return Err(invalid_state("non-finite value / 非有限值"));
        }
        Ok(JsonValue::String(format_number(*value)))
    }

    fn decode(&self, value: &JsonValue) -> Result<f64> {
        value
            .as_str()
            .and_then(parse_number)
            .ok_or_else(|| invalid_state("invalid number / 非法数值"))
    }
}

/// 数值向量编解码（字符串数组）/ Numeric vector codec as an array of strings
#[derive(Debug, Clone, Copy, Default)]
pub struct NumberVectorCodec;

impl SearchCodec<Vec<f64>> for NumberVectorCodec {
    fn encode(&self, value: &Vec<f64>) -> Result<JsonValue> {
        value
            .iter()
            .map(|component| TextNumberCodec.encode(component))
            .collect::<Result<Vec<JsonValue>>>()
            .map(JsonValue::Array)
    }

    fn decode(&self, value: &JsonValue) -> Result<Vec<f64>> {
        let array = value
            .as_array()
            .ok_or_else(|| invalid_state("expected number array / 期望数值数组"))?;
        array.iter().map(|item| TextNumberCodec.decode(item)).collect()
    }
}

/// 数值矩阵编解码 / Numeric matrix codec
#[derive(Debug, Clone, Copy, Default)]
pub struct NumberMatrixCodec;

impl SearchCodec<Vec<Vec<f64>>> for NumberMatrixCodec {
    fn encode(&self, value: &Vec<Vec<f64>>) -> Result<JsonValue> {
        ListCodec::new(NumberVectorCodec).encode(value)
    }

    fn decode(&self, value: &JsonValue) -> Result<Vec<Vec<f64>>> {
        ListCodec::new(NumberVectorCodec).decode(value)
    }
}

/// 原生数值矩阵编解码：输出 **JSON 数字**（而非字符串）/ Raw numeric matrix codec emitting **JSON numbers**, not strings
///
/// Kotlin 的 `NativeState.vectors` 是 `List<List<Double>>`，由 kotlinx 直接序列化为 JSON 数字；只有走问题编解码的
/// `individuals` / `solutions` 才是字符串。两者不能混用。
/// Kotlin's `NativeState.vectors` is `List<List<Double>>`, which kotlinx serializes as JSON numbers; only `individuals` /
/// `solutions`, which go through the problem codecs, are strings. The two must not be mixed.
#[derive(Debug, Clone, Copy, Default)]
pub struct RawNumberMatrixCodec;

impl SearchCodec<Vec<Vec<f64>>> for RawNumberMatrixCodec {
    fn encode(&self, value: &Vec<Vec<f64>>) -> Result<JsonValue> {
        value
            .iter()
            .map(|row| {
                row.iter()
                    .map(|component| {
                        if !component.is_finite() {
                            return Err(invalid_state("non-finite value / 非有限值"));
                        }
                        Ok(JsonValue::from(*component))
                    })
                    .collect::<Result<Vec<JsonValue>>>()
                    .map(JsonValue::Array)
            })
            .collect::<Result<Vec<JsonValue>>>()
            .map(JsonValue::Array)
    }

    fn decode(&self, value: &JsonValue) -> Result<Vec<Vec<f64>>> {
        value
            .as_array()
            .ok_or_else(|| invalid_state("expected matrix / 期望数值矩阵"))?
            .iter()
            .map(|row| {
                row.as_array()
                    .ok_or_else(|| invalid_state("expected matrix row / 期望数值矩阵行"))?
                    .iter()
                    .map(|component| {
                        // 同时接受数字与字符串，便于容忍历史快照。
                        // Accept both numbers and strings to tolerate older snapshots.
                        component
                            .as_f64()
                            .filter(|value| value.is_finite())
                            .or_else(|| component.as_str().and_then(parse_number))
                            .ok_or_else(|| invalid_state("invalid matrix component / 非法矩阵分量"))
                    })
                    .collect()
            })
            .collect()
    }
}

/// 原生状态形状，与 Kotlin `NativeState` 一一对应 / Native state shape mirroring Kotlin's `NativeState`
///
/// 所有字段都有默认值；序列化时省略空字段，与 kotlinx 的 `encodeDefaults = false` 一致。
/// Every field has a default and empty fields are omitted on serialization, matching kotlinx's `encodeDefaults = false`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NativeState {
    /// 个体 JSON / Individual JSON
    pub individuals: JsonValue,
    /// 数值矩阵 / Numeric matrix
    pub vectors: Vec<Vec<f64>>,
    /// 整数列 / Integer columns
    pub counters: Vec<i64>,
    /// 解 JSON / Solution JSON
    pub solutions: JsonValue,
    /// 嵌套状态 / Nested state
    pub children: Vec<NativeState>,
}

impl NativeState {
    /// 编码为 JSON 对象 / Encode into a JSON object
    pub fn encode(&self) -> JsonValue {
        let mut entries: Vec<(&str, JsonValue)> = Vec::new();
        if !is_empty_json(&self.individuals) {
            entries.push(("individuals", self.individuals.clone()));
        }
        if !self.vectors.is_empty() {
            entries.push((
                "vectors",
                RawNumberMatrixCodec
                    .encode(&self.vectors)
                    .unwrap_or(JsonValue::Array(Vec::new())),
            ));
        }
        if !self.counters.is_empty() {
            entries.push((
                "counters",
                JsonValue::Array(
                    self.counters
                        .iter()
                        .map(|counter| JsonValue::from(*counter))
                        .collect(),
                ),
            ));
        }
        if !is_empty_json(&self.solutions) {
            entries.push(("solutions", self.solutions.clone()));
        }
        if !self.children.is_empty() {
            entries.push((
                "children",
                JsonValue::Array(self.children.iter().map(NativeState::encode).collect()),
            ));
        }
        object(entries)
    }

    /// 从 JSON 对象解码 / Decode from a JSON object
    pub fn decode(value: &JsonValue) -> Result<Self> {
        let individuals = field(value, "individuals").cloned().unwrap_or(JsonValue::Null);
        let vectors = match field(value, "vectors") {
            Some(vectors) => RawNumberMatrixCodec.decode(vectors)?,
            None => Vec::new(),
        };
        let counters = match field(value, "counters") {
            Some(counters) => counters
                .as_array()
                .ok_or_else(|| invalid_state("expected counters array / 期望 counters 数组"))?
                .iter()
                .map(|counter| {
                    counter
                        .as_i64()
                        .ok_or_else(|| invalid_state("invalid counter / 非法计数"))
                })
                .collect::<Result<Vec<i64>>>()?,
            None => Vec::new(),
        };
        let solutions = field(value, "solutions").cloned().unwrap_or(JsonValue::Null);
        let children = match field(value, "children") {
            Some(children) => children
                .as_array()
                .ok_or_else(|| invalid_state("expected children array / 期望 children 数组"))?
                .iter()
                .map(NativeState::decode)
                .collect::<Result<Vec<NativeState>>>()?,
            None => Vec::new(),
        };

        Ok(Self {
            individuals,
            vectors,
            counters,
            solutions,
            children,
        })
    }
}

fn is_empty_json(value: &JsonValue) -> bool {
    match value {
        JsonValue::Null => true,
        JsonValue::Array(items) => items.is_empty(),
        JsonValue::Object(entries) => entries.is_empty(),
        _ => false,
    }
}

/// 单个个体的编解码：`{"solution": [...], "fitness": "..."}` / Individual codec
pub struct IndividualCodec {
    order: Vec<VariableId>,
}

impl IndividualCodec {
    /// 以变量顺序创建 / Create from the variable order
    pub fn new(order: Vec<VariableId>) -> Self {
        Self { order }
    }

    /// 变量顺序 / Variable order
    pub fn order(&self) -> &[VariableId] {
        &self.order
    }

    /// 个体列表编解码 / Individual-list codec with the same order
    pub fn list(&self) -> IndividualListCodec {
        IndividualListCodec::new(self.order.clone())
    }
}

impl SearchCodec<Individual<f64, f64>> for IndividualCodec {
    fn encode(&self, value: &Individual<f64, f64>) -> Result<JsonValue> {
        Ok(object([
            (
                "solution",
                NumberVectorCodec.encode(&values_of(&self.order, &value.solution))?,
            ),
            ("fitness", TextNumberCodec.encode(&value.fitness)?),
        ]))
    }

    fn decode(&self, value: &JsonValue) -> Result<Individual<f64, f64>> {
        let solution = field(value, "solution")
            .ok_or_else(|| invalid_state("missing solution / 缺少 solution 字段"))
            .and_then(|solution| NumberVectorCodec.decode(solution))?;
        if solution.len() != self.order.len() {
            return Err(invalid_state("solution dimension mismatch / 解维度不匹配"));
        }
        let fitness = field(value, "fitness")
            .ok_or_else(|| invalid_state("missing fitness / 缺少 fitness 字段"))
            .and_then(|fitness| TextNumberCodec.decode(fitness))?;
        Ok(Individual::new(
            solution_of(&self.order, &solution),
            from_f64::<f64>(fitness),
        ))
    }
}

/// 个体列表编解码 / Individual list codec
pub struct IndividualListCodec {
    individual: IndividualCodec,
}

impl IndividualListCodec {
    /// 以变量顺序创建 / Create from the variable order
    pub fn new(order: Vec<VariableId>) -> Self {
        Self {
            individual: IndividualCodec::new(order),
        }
    }
}

impl SearchCodec<Vec<Individual<f64, f64>>> for IndividualListCodec {
    fn encode(&self, value: &Vec<Individual<f64, f64>>) -> Result<JsonValue> {
        ListCodec::new(&self.individual).encode(value)
    }

    fn decode(&self, value: &JsonValue) -> Result<Vec<Individual<f64, f64>>> {
        ListCodec::new(&self.individual).decode(value)
    }
}

/// 种群状态编解码：个体列表 / Population-state codec
pub struct PopulationStateCodec {
    individuals: IndividualListCodec,
}

impl PopulationStateCodec {
    /// 以变量顺序创建 / Create from the variable order
    pub fn new(order: Vec<VariableId>) -> Self {
        Self {
            individuals: IndividualListCodec::new(order),
        }
    }
}

impl SearchCodec<Vec<Individual<f64, f64>>> for PopulationStateCodec {
    fn encode(&self, value: &Vec<Individual<f64, f64>>) -> Result<JsonValue> {
        self.individuals.encode(value)
    }

    fn decode(&self, value: &JsonValue) -> Result<Vec<Individual<f64, f64>>> {
        self.individuals.decode(value)
    }
}

/// 岛屿状态编解码：嵌套个体列表 / Island-state codec
pub struct IslandStateCodec {
    individuals: IndividualListCodec,
}

impl IslandStateCodec {
    /// 以变量顺序创建 / Create from the variable order
    pub fn new(order: Vec<VariableId>) -> Self {
        Self {
            individuals: IndividualListCodec::new(order),
        }
    }
}

impl SearchCodec<Vec<Vec<Individual<f64, f64>>>> for IslandStateCodec {
    fn encode(&self, value: &Vec<Vec<Individual<f64, f64>>>) -> Result<JsonValue> {
        ListCodec::new(&self.individuals).encode(value)
    }

    fn decode(&self, value: &JsonValue) -> Result<Vec<Vec<Individual<f64, f64>>>> {
        ListCodec::new(&self.individuals).decode(value)
    }
}

/// 轨迹状态编解码：可空个体 / Trajectory-state codec
pub struct TrajectoryStateCodec {
    individual: IndividualCodec,
}

impl TrajectoryStateCodec {
    /// 以变量顺序创建 / Create from the variable order
    pub fn new(order: Vec<VariableId>) -> Self {
        Self {
            individual: IndividualCodec::new(order),
        }
    }
}

impl SearchCodec<Option<Individual<f64, f64>>> for TrajectoryStateCodec {
    fn encode(&self, value: &Option<Individual<f64, f64>>) -> Result<JsonValue> {
        NullableCodec::new(&self.individual).encode(value)
    }

    fn decode(&self, value: &JsonValue) -> Result<Option<Individual<f64, f64>>> {
        NullableCodec::new(&self.individual).decode(value)
    }
}

/// 粒子群状态编解码 / Particle-state codec
///
/// 与 Kotlin `particles` 同形：`individuals` 按 `individual, best` 交替平铺，`vectors` 保存各粒子速度。
/// Identical to Kotlin's `particles`: `individuals` alternates `individual, best` and `vectors` holds the velocities.
pub struct ParticleStateCodec {
    individual: IndividualCodec,
}

impl ParticleStateCodec {
    /// 以变量顺序创建 / Create from the variable order
    pub fn new(order: Vec<VariableId>) -> Self {
        Self {
            individual: IndividualCodec::new(order),
        }
    }
}

impl SearchCodec<Vec<ParticleState<f64>>> for ParticleStateCodec {
    fn encode(&self, value: &Vec<ParticleState<f64>>) -> Result<JsonValue> {
        let mut flat = Vec::with_capacity(value.len() * 2);
        let mut vectors = Vec::with_capacity(value.len());
        for particle in value {
            flat.push(particle.individual.clone());
            flat.push(particle.best.clone());
            vectors.push(particle.velocity.clone());
        }
        Ok(NativeState {
            individuals: self.individual.list().encode(&flat)?,
            vectors,
            ..NativeState::default()
        }
        .encode())
    }

    fn decode(&self, value: &JsonValue) -> Result<Vec<ParticleState<f64>>> {
        let saved = NativeState::decode(value)?;
        let values = self.individual.list().decode(&saved.individuals)?;
        if values.len() != saved.vectors.len() * 2 {
            return Err(invalid_state("invalid particles / 粒子状态非法"));
        }
        saved
            .vectors
            .iter()
            .enumerate()
            .map(|(index, velocity)| {
                if velocity.len() != values[index * 2].solution.len() {
                    return Err(invalid_state("invalid particle velocity / 粒子速度维度不匹配"));
                }
                Ok(ParticleState {
                    individual: values[index * 2].clone(),
                    velocity: velocity.clone(),
                    best: values[index * 2 + 1].clone(),
                })
            })
            .collect()
    }
}

/// CLPSO 状态编解码 / CLPSO-state codec
///
/// 与 Kotlin `clpso` 同形：`children[i].counters = [stagnation, exemplar...]`。
/// Identical to Kotlin's `clpso`: `children[i].counters = [stagnation, exemplar...]`.
pub struct ClpsoStateCodec {
    individual: IndividualCodec,
}

impl ClpsoStateCodec {
    /// 以变量顺序创建 / Create from the variable order
    pub fn new(order: Vec<VariableId>) -> Self {
        Self {
            individual: IndividualCodec::new(order),
        }
    }
}

impl SearchCodec<Vec<ClpsoParticle<f64>>> for ClpsoStateCodec {
    fn encode(&self, value: &Vec<ClpsoParticle<f64>>) -> Result<JsonValue> {
        let mut flat = Vec::with_capacity(value.len() * 2);
        let mut vectors = Vec::with_capacity(value.len());
        let mut children = Vec::with_capacity(value.len());
        for particle in value {
            flat.push(particle.individual.clone());
            flat.push(particle.best.clone());
            vectors.push(particle.velocity.clone());
            let mut counters = Vec::with_capacity(particle.exemplars.len() + 1);
            counters.push(particle.stagnation as i64);
            counters.extend(particle.exemplars.iter().map(|index| *index as i64));
            children.push(NativeState {
                counters,
                ..NativeState::default()
            });
        }
        Ok(NativeState {
            individuals: self.individual.list().encode(&flat)?,
            vectors,
            children,
            ..NativeState::default()
        }
        .encode())
    }

    fn decode(&self, value: &JsonValue) -> Result<Vec<ClpsoParticle<f64>>> {
        let saved = NativeState::decode(value)?;
        let values = self.individual.list().decode(&saved.individuals)?;
        if values.len() != saved.vectors.len() * 2 || saved.children.len() != saved.vectors.len() {
            return Err(invalid_state("invalid CLPSO state / CLPSO 状态非法"));
        }
        saved
            .vectors
            .iter()
            .enumerate()
            .map(|(index, velocity)| {
                let counters = &saved.children[index].counters;
                if counters.len() != values[index * 2].solution.len() + 1
                    || counters[0] < 0
                    || counters[1..].iter().any(|exemplar| {
                        *exemplar < 0 || *exemplar as usize >= saved.vectors.len()
                    })
                    || velocity.len() != values[index * 2].solution.len()
                {
                    return Err(invalid_state("invalid CLPSO counters / CLPSO 计数非法"));
                }
                Ok(ClpsoParticle {
                    individual: values[index * 2].clone(),
                    velocity: velocity.clone(),
                    best: values[index * 2 + 1].clone(),
                    exemplars: counters[1..].iter().map(|index| *index as usize).collect(),
                    stagnation: counters[0] as u32,
                })
            })
            .collect()
    }
}

/// SHADE 状态编解码 / SHADE-state codec
///
/// 与 Kotlin `shade` 同形：`vectors = [memoryF, memoryCr]`、`counters = [cursor]`、`solutions` 为外部档案。
/// Identical to Kotlin's `shade`: `vectors = [memoryF, memoryCr]`, `counters = [cursor]`, `solutions` holds the archive.
pub struct ShadeStateCodec {
    individual: IndividualCodec,
}

impl ShadeStateCodec {
    /// 以变量顺序创建 / Create from the variable order
    pub fn new(order: Vec<VariableId>) -> Self {
        Self {
            individual: IndividualCodec::new(order),
        }
    }

    fn solution_list(&self) -> ListCodec<NumberVectorCodec> {
        ListCodec::new(NumberVectorCodec)
    }
}

impl SearchCodec<ShadeState<f64>> for ShadeStateCodec {
    fn encode(&self, value: &ShadeState<f64>) -> Result<JsonValue> {
        let solutions: Vec<Vec<f64>> = value
            .archive
            .iter()
            .map(|solution| values_of(self.individual.order(), solution))
            .collect();
        Ok(NativeState {
            individuals: self.individual.list().encode(&value.population)?,
            vectors: vec![value.memory.memory_f.clone(), value.memory.memory_cr.clone()],
            counters: vec![value.memory.cursor as i64],
            solutions: self.solution_list().encode(&solutions)?,
            children: Vec::new(),
        }
        .encode())
    }

    fn decode(&self, value: &JsonValue) -> Result<ShadeState<f64>> {
        let saved = NativeState::decode(value)?;
        if saved.vectors.len() != 2
            || saved.vectors[0].is_empty()
            || saved.vectors[0].len() != saved.vectors[1].len()
            || saved.counters.len() != 1
            || saved.counters[0] < 0
            || saved.counters[0] as usize >= saved.vectors[0].len()
            || saved
                .vectors
                .iter()
                .flatten()
                .any(|component| !(0.0..=1.0).contains(component))
        {
            return Err(invalid_state("invalid SHADE memory / SHADE 记忆库非法"));
        }
        let population = self.individual.list().decode(&saved.individuals)?;
        let solutions = self.solution_list().decode(&saved.solutions)?;
        let archive = solutions
            .iter()
            .map(|values| {
                if values.len() != self.individual.order().len() {
                    Err(invalid_state("invalid SHADE archive dimensions / SHADE 档案维度非法"))
                } else {
                    Ok(solution_of(self.individual.order(), values))
                }
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(ShadeState {
            population,
            memory: ShadeMemory {
                memory_f: saved.vectors[0].clone(),
                memory_cr: saved.vectors[1].clone(),
                cursor: saved.counters[0] as usize,
            },
            archive,
        })
    }
}

/// CMA-ES 状态编解码 / CMA-ES-state codec
///
/// 与 Kotlin `cmaEs` 同形：`vectors = [mean, [sigma], pathSigma, pathCovariance, covariance...]`、`counters = [generation]`。
/// Identical to Kotlin's `cmaEs`: `vectors = [mean, [sigma], pathSigma, pathCovariance, covariance...]`,
/// `counters = [generation]`.
pub struct CmaEsStateCodec {
    individual: IndividualCodec,
}

impl CmaEsStateCodec {
    /// 以变量顺序创建 / Create from the variable order
    pub fn new(order: Vec<VariableId>) -> Self {
        Self {
            individual: IndividualCodec::new(order),
        }
    }
}

impl SearchCodec<crate::solver::heuristic::runner::cmaes_runner::CmaEsState<f64>>
    for CmaEsStateCodec
{
    fn encode(
        &self,
        value: &crate::solver::heuristic::runner::cmaes_runner::CmaEsState<f64>,
    ) -> Result<JsonValue> {
        let mut vectors = Vec::with_capacity(4 + value.covariance.len());
        vectors.push(value.mean.clone());
        vectors.push(vec![value.sigma]);
        vectors.push(value.path_sigma.clone());
        vectors.push(value.path_covariance.clone());
        vectors.extend(value.covariance.iter().cloned());

        Ok(NativeState {
            individuals: TrajectoryStateCodec::new(self.individual.order().to_vec())
                .encode(&value.best)?,
            vectors,
            counters: vec![value.generation as i64],
            ..NativeState::default()
        }
        .encode())
    }

    fn decode(
        &self,
        value: &JsonValue,
    ) -> Result<crate::solver::heuristic::runner::cmaes_runner::CmaEsState<f64>> {
        let saved = NativeState::decode(value)?;
        let vectors = &saved.vectors;
        if vectors.len() < 4
            || vectors[0].is_empty()
            || vectors[1].len() != 1
            || vectors[1][0] <= 0.0
            || saved.counters.len() != 1
            || saved.counters[0] < 0
            || vectors[2].len() != vectors[0].len()
            || vectors[3].len() != vectors[0].len()
            || vectors.len() - 4 != vectors[0].len()
            || vectors[4..].iter().any(|row| row.len() != vectors[0].len())
        {
            return Err(invalid_state("invalid CMA-ES distribution / CMA-ES 分布非法"));
        }

        let best = TrajectoryStateCodec::new(self.individual.order().to_vec())
            .decode(&saved.individuals)?;
        Ok(crate::solver::heuristic::runner::cmaes_runner::CmaEsState {
            mean: vectors[0].clone(),
            sigma: vectors[1][0],
            covariance: vectors[4..].to_vec(),
            path_sigma: vectors[2].clone(),
            path_covariance: vectors[3].clone(),
            generation: saved.counters[0] as u32,
            best,
        })
    }
}

/// 目标向量编解码（字符串数组）/ Objective-vector codec
#[derive(Debug, Clone, Copy, Default)]
pub struct ObjectiveVectorCodec;

impl SearchCodec<Vec<f64>> for ObjectiveVectorCodec {
    fn encode(&self, value: &Vec<f64>) -> Result<JsonValue> {
        if value.is_empty() {
            return Err(invalid_state("empty objective vector / 目标向量为空"));
        }
        NumberVectorCodec.encode(value)
    }

    fn decode(&self, value: &JsonValue) -> Result<Vec<f64>> {
        let decoded = NumberVectorCodec.decode(value)?;
        if decoded.is_empty() {
            return Err(invalid_state("empty objective vector / 目标向量为空"));
        }
        Ok(decoded)
    }
}

/// 多目标个体编解码 / Multi-objective individual codec
pub struct MultiObjectiveIndividualCodec {
    order: Vec<VariableId>,
}

impl MultiObjectiveIndividualCodec {
    /// 以变量顺序创建 / Create from the variable order
    pub fn new(order: Vec<VariableId>) -> Self {
        Self { order }
    }
}

impl SearchCodec<Individual<Vec<f64>, f64>> for MultiObjectiveIndividualCodec {
    fn encode(&self, value: &Individual<Vec<f64>, f64>) -> Result<JsonValue> {
        if value.fitness.is_empty() {
            return Err(invalid_state("empty objective vector / 目标向量为空"));
        }
        Ok(object([
            (
                "solution",
                NumberVectorCodec.encode(&values_of(&self.order, &value.solution))?,
            ),
            ("fitness", NumberVectorCodec.encode(&value.fitness)?),
        ]))
    }

    fn decode(&self, value: &JsonValue) -> Result<Individual<Vec<f64>, f64>> {
        let solution = field(value, "solution")
            .ok_or_else(|| invalid_state("missing solution / 缺少 solution 字段"))
            .and_then(|solution| NumberVectorCodec.decode(solution))?;
        if solution.len() != self.order.len() {
            return Err(invalid_state("solution dimension mismatch / 解维度不匹配"));
        }
        let fitness = field(value, "fitness")
            .ok_or_else(|| invalid_state("missing fitness / 缺少 fitness 字段"))
            .and_then(|fitness| NumberVectorCodec.decode(fitness))?;
        Ok(Individual::new(solution_of(&self.order, &solution), fitness))
    }
}

/// 多目标档案编解码 / Multi-objective archive codec
pub struct MultiObjectiveArchiveCodec {
    individual: MultiObjectiveIndividualCodec,
}

impl MultiObjectiveArchiveCodec {
    /// 以变量顺序创建 / Create from the variable order
    pub fn new(order: Vec<VariableId>) -> Self {
        Self {
            individual: MultiObjectiveIndividualCodec::new(order),
        }
    }
}

impl SearchCodec<Vec<Individual<Vec<f64>, f64>>> for MultiObjectiveArchiveCodec {
    fn encode(&self, value: &Vec<Individual<Vec<f64>, f64>>) -> Result<JsonValue> {
        ListCodec::new(&self.individual).encode(value)
    }

    fn decode(&self, value: &JsonValue) -> Result<Vec<Individual<Vec<f64>, f64>>> {
        ListCodec::new(&self.individual).decode(value)
    }
}

/// 目标值转文本，供诊断复用 / Objective to text, reused by diagnostics
pub fn objective_text<V: HeuristicValue>(value: &V) -> String {
    format_number(to_f64(value))
}

/// 允许外部以 `Value` 读取字段（保持与 `serde_json` 的直接互操作）/ Re-export for direct `serde_json` interop
pub type RawJsonValue = Value;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::callback::Solution;
    use crate::variable::new_standalone_id;

    #[test]
    fn numbers_use_kotlin_double_to_string_shape() {
        assert_eq!(format_number(1.0), "1.0");
        assert_eq!(format_number(-0.0), "-0.0");
        assert_eq!(format_number(123.5), "123.5");
        assert_eq!(format_number(1.0e300), "1.0E300");
        assert_eq!(format_number(1.0e7), "1.0E7");
        assert_eq!(format_number(1.0e-4), "1.0E-4");
        assert_eq!(format_number(0.0), "0.0");
        assert_eq!(parse_number("1.0E300"), Some(1.0e300));
        assert_eq!(parse_number("not-a-number"), None);
    }

    #[test]
    fn number_vector_codec_round_trips_and_rejects_garbage() {
        let codec = NumberVectorCodec;
        let encoded = codec.encode(&vec![0.0, 1.5, -2.0]).expect("encodable");
        assert_eq!(
            encoded,
            JsonValue::Array(vec![
                JsonValue::from("0.0"),
                JsonValue::from("1.5"),
                JsonValue::from("-2.0"),
            ])
        );
        assert_eq!(
            codec.decode(&encoded).expect("decodable"),
            vec![0.0, 1.5, -2.0]
        );
        assert!(codec.encode(&vec![f64::NAN]).is_err());
        assert!(codec
            .decode(&JsonValue::Array(vec![JsonValue::from("x")]))
            .is_err());
    }

    #[test]
    fn population_codec_round_trips() {
        let x = new_standalone_id();
        let codec = PopulationStateCodec::new(vec![x]);
        let population = vec![
            Individual::new(Solution::from([(x, 1.0)]), 1.0),
            Individual::new(Solution::from([(x, 2.5)]), 2.5),
        ];
        let encoded = codec.encode(&population).expect("encodable");
        let decoded = codec.decode(&encoded).expect("decodable");
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].fitness, 1.0);
        assert_eq!(decoded[1].solution.get(&x).copied(), Some(2.5));
    }

    #[test]
    fn native_state_omits_default_fields() {
        let empty = NativeState::default().encode();
        assert_eq!(empty, JsonValue::Object(Default::default()));

        let state = NativeState {
            vectors: vec![vec![1.0]],
            counters: vec![2],
            ..NativeState::default()
        };
        let encoded = state.encode();
        assert!(encoded.get("individuals").is_none());
        assert!(encoded.get("solutions").is_none());
        assert!(encoded.get("children").is_none());
        assert_eq!(
            encoded.get("counters"),
            Some(&JsonValue::Array(vec![JsonValue::from(2)]))
        );
        assert_eq!(
            encoded.get("vectors"),
            Some(&serde_json::json!([[1.0]]))
        );
        assert_eq!(NativeState::decode(&encoded).expect("decodable"), state);
    }
}
