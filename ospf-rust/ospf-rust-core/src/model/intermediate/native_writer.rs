//! 跨求解器原生写入调度 / Cross-solver native write scheduling
//!
//! 求解器适配器在最终列编号之前，需要为每个延迟函数结构决定"用原生接口写入"还是"物化通用
//! fallback"。本模块提供该决策的求解器无关调度层：注册表按注册顺序遍历异构 writer，逐个处理
//! 自己声明支持的结构，返回每个结构的最终去向。
//!
//! 调度层只负责"写什么、谁来写"，**不承载 fallback 语义**：任何 writer 失败都向上返回错误，
//! 由调用方对整模型触发回退；被判定为 fallback 的结构由
//! [`crate::model::mechanism::BasicMechanismModel::materialize_deferred_functions_at`] 物化，
//! 保证 fallback 与 EAGER 展开逐列一致。
//!
//! Before final column numbering a solver adapter must decide, for every deferred function
//! structure, whether to write it through a native interface or to materialize the generic
//! fallback. This module provides the solver-neutral scheduling layer for that decision: the
//! registry walks heterogeneous writers in registration order, each handling the structures it
//! declares support for, and reports the final destination of every structure.
//!
//! The scheduling layer only decides what is written and by whom; it does not carry fallback
//! semantics. Any writer failure propagates as an error so the caller can fall back for the whole
//! model, and structures routed to fallback are materialized by
//! [`crate::model::mechanism::BasicMechanismModel::materialize_deferred_functions_at`], keeping
//! fallback column-identical to eager expansion.

use crate::error::{ModelError, Result};
use crate::model::intermediate::{DeferredFunctionStructure, FunctionUsageSummary};
use std::fmt::Debug;

/// 一个待原生写入的结构 / One structure awaiting a native write.
///
/// 请求携带结构本身与它的使用语境摘要，使 writer 可以在写入前拒绝不安全的场景（例如结果列被
/// 外部引用时不能只写目标项快捷接口）。
///
/// The request carries the structure and its usage summary so a writer can reject unsafe cases
/// before writing (for example an objective-only shortcut when the result column is referenced
/// externally).
pub struct NativeWriteRequest<'a, V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 结构在待展开批次中的位置 / Position of the structure inside the pending batch
    pub index: usize,
    /// 结构描述 / Structure description
    pub structure: &'a dyn DeferredFunctionStructure<V>,
    /// 使用语境摘要 / Usage context summary
    pub usage: FunctionUsageSummary,
}

// 手写 `Clone`/`Copy`：derive 会额外要求 `V: Copy`，而请求只借用与 `V` 无关的数据。
// Manual `Clone`/`Copy`: the derive would additionally require `V: Copy`, while a request only
// borrows data independent of `V`.
impl<'a, V> Clone for NativeWriteRequest<'a, V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, V> Copy for NativeWriteRequest<'a, V> where V: Clone + Debug + Send + Sync + 'static {}

impl<V> Debug for NativeWriteRequest<'_, V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeWriteRequest")
            .field("index", &self.index)
            .field("function", &self.structure.function_name())
            .field("usage", &self.usage)
            .finish()
    }
}

/// 原生写入记录 / Record of one native write.
///
/// 记录随求解结果一起进入报告与恢复链路，因此 writer 名称与 schema 版本必须稳定；schema 变化
/// 表示原生结构的含义发生了变化，恢复阶段应据此拒绝旧指纹。
///
/// The record travels into reporting and recovery, so the writer name and schema version must be
/// stable; a schema change means the native structure's meaning changed and recovery must reject
/// older fingerprints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeWriteRecord {
    /// 写入该结构的 writer 名称 / Name of the writer that wrote the structure
    pub writer: String,
    /// 结构 schema 版本，例如 `functions-abs-1` / Structure schema version
    pub schema: String,
    /// 结构指纹；写入方未提供时由模型按结构内容补齐
    /// Structure fingerprint; the model fills it from the structure when the writer leaves it out.
    pub fingerprint: Option<String>,
    /// 被写入的函数名称，用于把记录关联回具体函数 / Name of the written function so a record can be linked back.
    pub function: String,
}

impl NativeWriteRecord {
    /// 创建写入记录（不带指纹与函数名）/ Create a write record without fingerprint or function name.
    pub fn new(writer: impl Into<String>, schema: impl Into<String>) -> Self {
        Self {
            writer: writer.into(),
            schema: schema.into(),
            fingerprint: None,
            function: String::new(),
        }
    }

    /// 附上结构指纹 / Attach a structure fingerprint.
    pub fn with_fingerprint(mut self, fingerprint: Option<String>) -> Self {
        self.fingerprint = fingerprint;
        self
    }

    /// 附上函数名称 / Attach the function name.
    pub fn with_function(mut self, function: impl Into<String>) -> Self {
        self.function = function.into();
        self
    }

    /// 校验本记录是否仍然对应当前结构。
    ///
    /// 这是恢复阶段的入口：拿着持久化的原生写入记录与当前重建出的结构调用本方法，两者指纹
    /// 必须都存在且相等。schema 变化或结构语义变化都会让它返回 `false`，调用方应据此拒绝
    /// 复用旧的原生写入并按通用展开重建。
    ///
    /// Validate that this record still corresponds to the current structure.
    ///
    /// This is the recovery entry point: call it with the persisted native write record and the
    /// current rebuilt structure; both fingerprints must be present and equal. A schema change or a
    /// semantic change to the structure makes it return `false`, and the caller should then reject
    /// reusing the old native write and rebuild from the generic expansion.
    pub fn matches_structure<V>(&self, structure: &dyn DeferredFunctionStructure<V>) -> bool
    where
        V: Clone + Debug + Send + Sync + 'static,
    {
        crate::model::intermediate::verify_native_write(self.fingerprint.as_deref(), structure)
    }
}

/// 结构被判定为通用 fallback 的原因 / Why a structure was routed to the generic fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FallbackReason {
    /// 没有 writer 声明支持该结构 / No writer declared support for the structure
    NoWriter,
    /// writer 按使用语境或范围证明明确拒绝 / The writer rejected the structure explicitly
    Rejected(String),
    /// writer 写入失败 / The writer failed while writing
    WriterFailed(String),
}

/// 单个结构的原生写入结果 / Outcome of a single structure's native write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeWriteOutcome {
    /// 已由原生接口写入，调用方删除该结构的通用 fallback 行
    /// Written through a native interface; the caller drops this structure's generic rows.
    Native(NativeWriteRecord),
    /// 未原生写入，调用方必须物化通用 fallback
    /// Not written natively; the caller must materialize the generic fallback.
    Fallback(FallbackReason),
}

impl NativeWriteOutcome {
    /// 是否已原生写入 / Whether the structure was written natively.
    pub fn is_native(&self) -> bool {
        matches!(self, NativeWriteOutcome::Native(_))
    }

    /// 是否必须物化通用 fallback / Whether the generic fallback must be materialized.
    pub fn requires_fallback(&self) -> bool {
        matches!(self, NativeWriteOutcome::Fallback(_))
    }
}

/// 一次原生 lowering 调度的结果 / Result of one native lowering pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeLoweringReport {
    /// 每个待展开结构的最终去向，顺序与 `deferred_functions()` 一致
    /// Final destination of every pending structure, in `deferred_functions()` order.
    pub outcomes: Vec<NativeWriteOutcome>,
    /// 已物化通用 fallback 的结构数量 / Number of structures whose generic fallback was materialized
    pub materialized_fallbacks: usize,
    /// 已由原生接口写入的结构数量 / Number of structures written through a native interface
    pub native_writes: usize,
}

impl NativeLoweringReport {
    /// 是否所有结构都由原生接口写入 / Whether every structure was written natively.
    pub fn is_fully_native(&self) -> bool {
        !self.outcomes.is_empty() && self.materialized_fallbacks == 0
    }

    /// 是否有结构退化为通用 fallback / Whether any structure fell back to the generic expansion.
    pub fn has_fallbacks(&self) -> bool {
        self.materialized_fallbacks > 0
    }
}

/// 求解器无关的原生写入器 / Solver-neutral native function writer.
///
/// 类型参数 `C` 是求解器自己的容器类型（例如 SDK 模型句柄的包装）。把容器类型参数化而不是
/// 在核心层引入 SDK 类型，可以容纳异构 writer，同时让核心完全不了解任何 SDK。
///
/// The `C` type parameter is the solver's own container type (for example a wrapper around an SDK
/// model handle). Parameterizing it instead of introducing SDK types into the core keeps the core
/// SDK-free while still allowing heterogeneous writers.
pub trait NativeFunctionWriter<C, V>: Debug + Send + Sync + 'static
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// writer 名称 / Writer name.
    fn name(&self) -> &str;

    /// 该 writer 是否处理给定结构 / Whether this writer handles the given structure.
    fn supports(&self, structure: &dyn DeferredFunctionStructure<V>) -> bool;

    /// 为一个批次写入原生表示。
    ///
    /// 返回 `Ok(None)` 表示本 writer 主动跳过整批（registry 会继续交给后续 writer）；返回
    /// `Ok(Some(outcomes))` 时 `outcomes` 的长度必须与 `requests` 一致，长度不符会被 registry
    /// 判为契约违例。返回 `Err` 表示写入失败，调用方必须对整模型触发 fallback，而不是保留
    /// 半写入状态。
    ///
    /// Write the native representation for one batch.
    ///
    /// `Ok(None)` means this writer skips the whole batch and the registry continues with later
    /// writers. With `Ok(Some(outcomes))` the outcomes must have exactly one entry per request;
    /// the registry treats a length mismatch as a contract violation. `Err` means the write
    /// failed and the caller must fall back for the whole model instead of keeping a half-written
    /// state.
    fn write_batch(
        &self,
        container: &mut C,
        requests: &[NativeWriteRequest<'_, V>],
    ) -> Result<Option<Vec<NativeWriteOutcome>>>;
}

/// 原生写入器注册表 / Registry of native function writers.
///
/// 调度契约（与 Kotlin registry 对齐）：
///
/// 1. 按注册顺序遍历 writer，先注册的 writer 先决定结构的去向；
/// 2. 已被前面 writer 决定的结构不再参与后续 writer 的批次；
/// 3. writer 没有任何可处理的结构时跳过该空批次，不调用它；
/// 4. writer 返回的条目数与批次大小不一致时立即失败，避免静默丢失结构；
/// 5. 任一 writer 返回错误时整体返回错误，由调用方对整模型触发 fallback；
/// 6. 遍历结束后仍未决定的结构按 [`FallbackReason::NoWriter`] 走通用 fallback。
///
/// Scheduling contract (aligned with the Kotlin registry):
///
/// 1. Writers are visited in registration order; an earlier writer decides first.
/// 2. Structures already decided by an earlier writer never enter a later writer's batch.
/// 3. A writer with no handleable structure is skipped without being called.
/// 4. A writer returning a different number of outcomes than its batch size fails immediately so
///    no structure is silently dropped.
/// 5. Any writer error propagates so the caller can fall back for the whole model.
/// 6. Structures left undecided are routed to fallback with [`FallbackReason::NoWriter`].
pub struct NativeFunctionWriterRegistry<C, V>
where
    C: 'static,
    V: Clone + Debug + Send + Sync + 'static,
{
    writers: Vec<Box<dyn NativeFunctionWriter<C, V>>>,
}

impl<C: 'static, V> Default for NativeFunctionWriterRegistry<C, V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<C: 'static, V> Debug for NativeFunctionWriterRegistry<C, V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeFunctionWriterRegistry")
            .field("writers", &self.writer_names())
            .finish()
    }
}

impl<C: 'static, V> NativeFunctionWriterRegistry<C, V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建空注册表 / Create an empty registry.
    pub fn new() -> Self {
        Self {
            writers: Vec::new(),
        }
    }

    /// 按注册顺序登记一个 writer / Register a writer in registration order.
    pub fn register(&mut self, writer: Box<dyn NativeFunctionWriter<C, V>>) {
        self.writers.push(writer);
    }

    /// 已登记 writer 的名称，按注册顺序 / Names of the registered writers, in registration order.
    pub fn writer_names(&self) -> Vec<&str> {
        self.writers.iter().map(|writer| writer.name()).collect()
    }

    /// 是否没有登记任何 writer / Whether no writer is registered.
    pub fn is_empty(&self) -> bool {
        self.writers.is_empty()
    }

    /// 为一批结构调度原生写入，并返回每个结构的最终去向。
    ///
    /// 返回值的顺序与 `requests` 一致，调用方据此物化 fallback 子集。
    ///
    /// Schedule native writes for a batch of structures and report each structure's final
    /// destination. The result follows the order of `requests` so the caller can materialize the
    /// fallback subset.
    pub fn write_batch(
        &self,
        container: &mut C,
        requests: &[NativeWriteRequest<'_, V>],
    ) -> Result<Vec<NativeWriteOutcome>> {
        let mut outcomes: Vec<Option<NativeWriteOutcome>> = vec![None; requests.len()];

        for writer in &self.writers {
            let claimed: Vec<usize> = requests
                .iter()
                .enumerate()
                .filter(|(index, request)| {
                    // 前面 writer 已经决定的结构不再参与后续批次。
                    // Structures already decided by an earlier writer are not re-offered.
                    outcomes[*index].is_none() && writer.supports(request.structure)
                })
                .map(|(index, _)| index)
                .collect();
            if claimed.is_empty() {
                // 空批次跳过，不调用 writer。
                // Empty batches are skipped without calling the writer.
                continue;
            }

            let batch: Vec<NativeWriteRequest<'_, V>> = claimed
                .iter()
                .map(|index| requests[*index])
                .collect();
            let written = writer.write_batch(container, &batch)?;
            let Some(written) = written else {
                // writer 主动跳过整批：交给后续 writer，而不是当作 fallback。
                // The writer skipped the whole batch explicitly: hand it to later writers instead
                // of treating it as fallback.
                continue;
            };

            if written.len() != batch.len() {
                return Err(ModelError::InvalidConstraint(format!(
                    "native writer `{}` returned {} outcomes for a batch of {} structures; batch sizes must match",
                    writer.name(),
                    written.len(),
                    batch.len()
                ))
                .into());
            }

            for (index, outcome) in claimed.into_iter().zip(written) {
                outcomes[index] = Some(outcome);
            }
        }

        Ok(outcomes
            .into_iter()
            .map(|outcome| outcome.unwrap_or(NativeWriteOutcome::Fallback(FallbackReason::NoWriter)))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::flatten::Linear;
    use crate::model::mechanism::{ConstraintRelation, LinearConstraint, LinearInequality};
    use std::collections::HashMap;
    use std::sync::Arc;

    /// 测试用结构描述 / Structure description used by the tests.
    #[derive(Debug, Clone)]
    struct FakeStructure {
        name: String,
        kind: &'static str,
        rows: usize,
        materialize_fails: bool,
    }

    impl FakeStructure {
        fn new(name: &str, kind: &'static str) -> Self {
            Self {
                name: name.to_string(),
                kind,
                rows: 1,
                materialize_fails: false,
            }
        }
    }

    impl DeferredFunctionStructure<f64> for FakeStructure {
        fn function_name(&self) -> &str {
            &self.name
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn materialize(
            &self,
            _symbol_to_index: &HashMap<usize, usize>,
        ) -> Result<Vec<LinearConstraint<f64>>> {
            if self.materialize_fails {
                return Err(ModelError::InvalidConstraint(format!(
                    "fake structure `{}` cannot materialize",
                    self.name
                ))
                .into());
            }
            Ok((0..self.rows)
                .map(|index| {
                    LinearConstraint::new(
                        LinearInequality::new(
                            Linear::new(Vec::new(), 0.0),
                            ConstraintRelation::LessEqual,
                            0.0,
                        ),
                        &format!("{}_row_{}", self.name, index),
                    )
                })
                .collect())
        }
    }

    /// 测试用求解器容器 / Solver container used by the tests.
    #[derive(Debug, Default)]
    struct FakeContainer {
        writes: Vec<String>,
    }

    /// writer 在测试中的行为 / Behaviour of a writer under test.
    #[derive(Debug, Clone)]
    enum FakeBehaviour {
        /// 正常写入 / Write normally
        Native,
        /// 整批拒绝 / Reject the whole batch
        Fallback(FallbackReason),
        /// 跳过整批 / Skip the whole batch
        Skip,
        /// 写入失败 / Fail the write
        Fail,
        /// 返回错误长度的结果 / Return a mismatched number of outcomes
        WrongLength(usize),
    }

    #[derive(Debug)]
    struct FakeWriter {
        name: &'static str,
        kinds: Vec<&'static str>,
        behaviour: FakeBehaviour,
    }

    impl FakeWriter {
        fn new(name: &'static str, kinds: &[&'static str], behaviour: FakeBehaviour) -> Self {
            Self {
                name,
                kinds: kinds.to_vec(),
                behaviour,
            }
        }
    }

    impl NativeFunctionWriter<FakeContainer, f64> for FakeWriter {
        fn name(&self) -> &str {
            self.name
        }

        fn supports(&self, structure: &dyn DeferredFunctionStructure<f64>) -> bool {
            let structure = structure
                .as_any()
                .downcast_ref::<FakeStructure>()
                .expect("test structures are fake structures");
            self.kinds.contains(&structure.kind)
        }

        fn write_batch(
            &self,
            container: &mut FakeContainer,
            requests: &[NativeWriteRequest<'_, f64>],
        ) -> Result<Option<Vec<NativeWriteOutcome>>> {
            for request in requests {
                container.writes.push(format!(
                    "{}:{}",
                    self.name,
                    request.structure.function_name()
                ));
            }
            match &self.behaviour {
                FakeBehaviour::Skip => Ok(None),
                FakeBehaviour::Fail => Err(ModelError::InvalidConstraint(format!(
                    "fake writer `{}` failed",
                    self.name
                ))
                .into()),
                FakeBehaviour::WrongLength(length) => {
                    Ok(Some(vec![NativeWriteOutcome::Native(NativeWriteRecord::new(
                        self.name, "functions-fake-1",
                    )); *length]))
                }
                FakeBehaviour::Native => Ok(Some(
                    requests
                        .iter()
                        .map(|_| {
                            NativeWriteOutcome::Native(NativeWriteRecord::new(
                                self.name,
                                "functions-fake-1",
                            ))
                        })
                        .collect(),
                )),
                FakeBehaviour::Fallback(reason) => Ok(Some(
                    requests
                        .iter()
                        .map(|_| NativeWriteOutcome::Fallback(reason.clone()))
                        .collect(),
                )),
            }
        }
    }

    fn requests<'a>(
        structures: &'a [FakeStructure],
    ) -> Vec<NativeWriteRequest<'a, f64>> {
        structures
            .iter()
            .enumerate()
            .map(|(index, structure)| NativeWriteRequest {
                index,
                structure,
                usage: FunctionUsageSummary::default(),
            })
            .collect()
    }

    #[test]
    fn registry_visits_writers_in_registration_order_and_skips_empty_batches() {
        let structures = vec![FakeStructure::new("abs_a", "abs")];
        let batch = requests(&structures);

        let mut registry = NativeFunctionWriterRegistry::<FakeContainer, f64>::new();
        registry.register(Box::new(FakeWriter::new(
            "first",
            &["abs"],
            FakeBehaviour::Native,
        )));
        registry.register(Box::new(FakeWriter::new(
            "second",
            &["abs"],
            FakeBehaviour::Native,
        )));
        assert_eq!(registry.writer_names(), vec!["first", "second"]);

        let mut container = FakeContainer::default();
        let outcomes = registry.write_batch(&mut container, &batch).unwrap();

        // 先注册的 writer 决定去向，后注册的 writer 批次为空因此不被调用。
        // The earlier writer decides, and the later writer is not called for an empty batch.
        assert_eq!(container.writes, vec!["first:abs_a"]);
        assert_eq!(outcomes.len(), 1);
        assert_eq!(
            outcomes[0],
            NativeWriteOutcome::Native(NativeWriteRecord::new("first", "functions-fake-1"))
        );
    }

    #[test]
    fn registry_routes_structures_without_a_writer_to_fallback() {
        let structures = vec![
            FakeStructure::new("abs_a", "abs"),
            FakeStructure::new("max_b", "max"),
        ];
        let batch = requests(&structures);

        let mut registry = NativeFunctionWriterRegistry::<FakeContainer, f64>::new();
        registry.register(Box::new(FakeWriter::new(
            "abs_writer",
            &["abs"],
            FakeBehaviour::Native,
        )));

        let mut container = FakeContainer::default();
        let outcomes = registry.write_batch(&mut container, &batch).unwrap();

        assert!(outcomes[0].is_native());
        assert_eq!(
            outcomes[1],
            NativeWriteOutcome::Fallback(FallbackReason::NoWriter)
        );
        assert!(outcomes[1].requires_fallback());
        // 未被支持的结构不会进入任何 writer 的批次。
        // Unsupported structures never enter any writer's batch.
        assert_eq!(container.writes, vec!["abs_writer:abs_a"]);
    }

    #[test]
    fn registry_hands_a_skipped_batch_to_the_next_writer() {
        let structures = vec![FakeStructure::new("abs_a", "abs")];
        let batch = requests(&structures);

        let mut registry = NativeFunctionWriterRegistry::<FakeContainer, f64>::new();
        registry.register(Box::new(FakeWriter::new(
            "skip_writer",
            &["abs"],
            FakeBehaviour::Skip,
        )));
        registry.register(Box::new(FakeWriter::new(
            "real_writer",
            &["abs"],
            FakeBehaviour::Native,
        )));

        let mut container = FakeContainer::default();
        let outcomes = registry.write_batch(&mut container, &batch).unwrap();

        assert_eq!(container.writes, vec!["skip_writer:abs_a", "real_writer:abs_a"]);
        assert_eq!(
            outcomes[0],
            NativeWriteOutcome::Native(NativeWriteRecord::new(
                "real_writer",
                "functions-fake-1"
            ))
        );
    }

    #[test]
    fn registry_keeps_an_explicit_rejection_as_fallback() {
        let structures = vec![FakeStructure::new("abs_a", "abs")];
        let batch = requests(&structures);

        let mut registry = NativeFunctionWriterRegistry::<FakeContainer, f64>::new();
        registry.register(Box::new(FakeWriter::new(
            "rejecting",
            &["abs"],
            FakeBehaviour::Fallback(FallbackReason::Rejected("result in objective".to_string())),
        )));
        // 后面的 writer 不会重新接管已经被拒绝的结构，避免同一个结构出现两种去向。
        // A later writer does not take over an already rejected structure, so one structure never
        // gets two destinations.
        registry.register(Box::new(FakeWriter::new(
            "late",
            &["abs"],
            FakeBehaviour::Native,
        )));

        let mut container = FakeContainer::default();
        let outcomes = registry.write_batch(&mut container, &batch).unwrap();

        assert_eq!(container.writes, vec!["rejecting:abs_a"]);
        assert_eq!(
            outcomes[0],
            NativeWriteOutcome::Fallback(FallbackReason::Rejected(
                "result in objective".to_string()
            ))
        );
    }

    #[test]
    fn registry_fails_when_a_writer_returns_a_mismatched_batch() {
        let structures = vec![
            FakeStructure::new("abs_a", "abs"),
            FakeStructure::new("abs_b", "abs"),
        ];
        let batch = requests(&structures);

        let mut registry = NativeFunctionWriterRegistry::<FakeContainer, f64>::new();
        registry.register(Box::new(FakeWriter::new(
            "short",
            &["abs"],
            FakeBehaviour::WrongLength(1),
        )));

        let mut container = FakeContainer::default();
        let error = registry.write_batch(&mut container, &batch).unwrap_err();
        assert!(error.to_string().contains("batch sizes must match"));
    }

    #[test]
    fn registry_propagates_writer_failures_for_a_whole_model_fallback() {
        let structures = vec![FakeStructure::new("abs_a", "abs")];
        let batch = requests(&structures);

        let mut registry = NativeFunctionWriterRegistry::<FakeContainer, f64>::new();
        registry.register(Box::new(FakeWriter::new(
            "broken",
            &["abs"],
            FakeBehaviour::Fail,
        )));

        let mut container = FakeContainer::default();
        let error = registry.write_batch(&mut container, &batch).unwrap_err();
        assert!(error.to_string().contains("fake writer `broken` failed"));
    }

    #[test]
    fn registry_without_writers_routes_everything_to_fallback() {
        let structures = vec![FakeStructure::new("abs_a", "abs")];
        let batch = requests(&structures);

        let registry = NativeFunctionWriterRegistry::<FakeContainer, f64>::new();
        assert!(registry.is_empty());

        let mut container = FakeContainer::default();
        let outcomes = registry.write_batch(&mut container, &batch).unwrap();
        assert_eq!(
            outcomes,
            vec![NativeWriteOutcome::Fallback(FallbackReason::NoWriter)]
        );
    }

    #[test]
    fn fake_structures_materialize_fallback_rows() {
        let structure = FakeStructure {
            name: "abs_materialized".to_string(),
            kind: "abs",
            rows: 2,
            materialize_fails: false,
        };
        let rows = structure.materialize(&HashMap::new()).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "abs_materialized_row_0");

        let failing = FakeStructure {
            materialize_fails: true,
            ..structure.clone()
        };
        assert!(failing.materialize(&HashMap::new()).is_err());
    }

    /// 支持 ABS 结构的假 writer / Fake writer that supports ABS structures.
    #[derive(Debug)]
    struct FakeAbsWriter {
        name: &'static str,
        native_functions: Vec<String>,
        fail: bool,
    }

    impl FakeAbsWriter {
        fn new(name: &'static str, native_functions: &[&str], fail: bool) -> Self {
            Self {
                name,
                native_functions: native_functions.iter().map(|name| name.to_string()).collect(),
                fail,
            }
        }
    }

    impl NativeFunctionWriter<FakeContainer, f64> for FakeAbsWriter {
        fn name(&self) -> &str {
            self.name
        }

        fn supports(&self, structure: &dyn DeferredFunctionStructure<f64>) -> bool {
            structure
                .as_any()
                .downcast_ref::<crate::symbol::function::AbsStructure<f64>>()
                .is_some()
        }

        fn write_batch(
            &self,
            container: &mut FakeContainer,
            requests: &[NativeWriteRequest<'_, f64>],
        ) -> Result<Option<Vec<NativeWriteOutcome>>> {
            if self.fail {
                return Err(ModelError::InvalidConstraint(format!(
                    "fake abs writer `{}` failed",
                    self.name
                ))
                .into());
            }
            Ok(Some(
                requests
                    .iter()
                    .map(|request| {
                        let function = request.structure.function_name().to_string();
                        // 结果被外部引用时拒绝原生写入，模拟真实 writer 的使用语境门。
                        // Reject native writes when the result is referenced externally, mimicking
                        // the usage gate of a real writer.
                        if self.native_functions.contains(&function) && !request.usage.in_constraint
                        {
                            container.writes.push(format!("{function}"));
                            NativeWriteOutcome::Native(NativeWriteRecord::new(
                                self.name,
                                "functions-abs-1",
                            ))
                        } else {
                            NativeWriteOutcome::Fallback(FallbackReason::Rejected(
                                "result is referenced by other rows".to_string(),
                            ))
                        }
                    })
                    .collect(),
            ))
        }
    }

    fn deferred_abs_model() -> crate::model::MetaModel<f64> {
        use crate::model::flatten::{Linear, LinearMonomial};
        use crate::model::{FunctionExpansionPolicy, MetaModel};
        use crate::symbol::function::AbsFunction;
        use crate::variable::{ContinuousVariableItem, VariableId, VariableRange};

        let mut model = MetaModel::<f64>::new("native_lowering");
        model.set_function_expansion_policy(FunctionExpansionPolicy::DeferredNativeFirst);
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(94_000),
            "x",
            VariableRange::bounded(-3.0, 7.0),
        );
        let x_index = model.register_variable(x).unwrap();

        let first = AbsFunction::new(
            980,
            "abs_native",
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
        );
        model.add_symbol(Arc::new(first)).unwrap();

        // 第二个函数消费第一个函数的列，使"结果被外部引用"的场景真实存在。
        // The second function consumes the first one's column so the "result referenced
        // externally" case really occurs.
        let second = AbsFunction::new(
            981,
            "abs_fallback",
            Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0),
        );
        model.add_symbol(Arc::new(second)).unwrap();
        model
    }

    #[test]
    fn model_level_lowering_applies_native_writes_and_materializes_the_rest() {
        let mut mechanism = deferred_abs_model().try_into_mechanism_model().unwrap();
        assert_eq!(mechanism.as_basic().deferred_functions().len(), 2);

        // 只允许第二个结构原生写入；第一个结构的结果被第二个函数引用，writer 必须拒绝。
        // Only the second structure may be written natively: the first one's result is referenced
        // by the second function, so the writer must reject it.
        let mut registry = NativeFunctionWriterRegistry::<FakeContainer, f64>::new();
        registry.register(Box::new(FakeAbsWriter::new(
            "fake_gurobi_abs",
            &["abs_fallback"],
            false,
        )));

        let mut container = FakeContainer::default();
        let report = mechanism
            .lower_deferred_functions(&registry, &mut container)
            .unwrap();

        assert_eq!(container.writes, vec!["abs_fallback"]);
        assert_eq!(report.native_writes, 1);
        assert_eq!(report.materialized_fallbacks, 1);
        assert!(report.has_fallbacks());
        assert!(!report.is_fully_native());
        assert_eq!(report.outcomes.len(), 2);
        assert!(report.outcomes[0].requires_fallback());
        assert!(report.outcomes[1].is_native());

        // 原生结构被丢弃、fallback 结构被物化，两者都不会重复展开。
        // The native structure is dropped and the fallback structure is materialized, so neither is
        // expanded twice.
        assert!(mechanism.as_basic().deferred_functions().is_empty());
        assert_eq!(mechanism.as_basic().constraints().len(), 4);
        assert!(
            mechanism
                .as_basic()
                .constraints()
                .iter()
                .all(|constraint| constraint.name.starts_with("abs_native"))
        );
        assert_eq!(mechanism.as_basic().native_writes().len(), 1);
        let record = &mechanism.as_basic().native_writes()[0];
        assert_eq!(record.writer, "fake_gurobi_abs");
        assert_eq!(record.schema, "functions-abs-1");
        // 模型按结构内容补齐函数名与指纹，恢复阶段可据此把记录关联回具体函数。
        // The model fills the function name and fingerprint from the structure so recovery can link
        // a record back to a concrete function.
        assert_eq!(record.function, "abs_fallback");
        assert!(record.fingerprint.is_some());

        // 之后的转换不会补出原生结构的行。
        // A later conversion does not add rows for the natively written structure.
        let linear = mechanism.into_linear_triad_model();
        assert_eq!(linear.basic.constraint_names.len(), 4);
    }

    #[test]
    fn model_level_lowering_failure_keeps_the_model_unchanged() {
        let mut mechanism = deferred_abs_model().try_into_mechanism_model().unwrap();
        let mut registry = NativeFunctionWriterRegistry::<FakeContainer, f64>::new();
        registry.register(Box::new(FakeAbsWriter::new("broken_abs", &["abs_native"], true)));

        let mut container = FakeContainer::default();
        assert!(
            mechanism
                .lower_deferred_functions(&registry, &mut container)
                .is_err()
        );

        // 失败必须原子：不动待展开列表、不写行、不记录原生写入。
        // A failure must be atomic: no pending structure is dropped, no row is written and no
        // native write is recorded.
        assert_eq!(mechanism.as_basic().deferred_functions().len(), 2);
        assert!(mechanism.as_basic().constraints().is_empty());
        assert!(mechanism.as_basic().native_writes().is_empty());
    }

    #[test]
    fn lowering_without_pending_structures_is_a_no_op() {
        let mut mechanism = crate::model::mechanism::MechanismModel::<f64>::new("empty");
        let registry = NativeFunctionWriterRegistry::<FakeContainer, f64>::new();
        let mut container = FakeContainer::default();

        let report = mechanism
            .lower_deferred_functions(&registry, &mut container)
            .unwrap();
        assert!(report.outcomes.is_empty());
        assert_eq!(report.native_writes, 0);
        assert_eq!(report.materialized_fallbacks, 0);
        assert!(!report.is_fully_native());
        assert!(container.writes.is_empty());
    }
}
