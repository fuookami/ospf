//! 求解器无关的延迟函数结构 / Solver-neutral deferred function structures
//!
//! 当 [`crate::model::FunctionExpansionPolicy`] 不为 `Eager` 时，函数符号可以把自身描述为
//! 一个不可变、求解器无关的结构 sidecar，而不是在建模阶段立刻写入通用约束。求解器适配器
//! 可以在最终列编号之前选择原生 lowering；不允许原生接口时，结构自身的 fallback
//! materializer 会生成与即时展开完全相同的通用约束。
//!
//! 结构描述只持有建模层已有的数据（输入多项式、辅助列的令牌 ID、Big-M 及其输入域证明），
//! 不持有求解器 SDK 对象，也不要求完整函数对象贯穿模型各层。
//!
//! When [`crate::model::FunctionExpansionPolicy`] is not `Eager`, a function symbol may describe
//! itself as an immutable solver-neutral structure sidecar instead of writing generic
//! constraints while building the model. A solver adapter can then choose native lowering before
//! final column numbering; when native interfaces are unavailable, the structure's own fallback
//! materializer emits exactly the same generic constraints as eager expansion.
//!
//! A structure only holds data that already exists in the modeling layer (input polynomial,
//! helper-column token IDs, Big-M values and their input-domain proof). It never holds solver SDK
//! objects and never requires a complete function object to travel through every model layer.

use crate::error::{ModelError, Result};
use crate::model::LinearConstraint;
use crate::symbol::IntermediateSymbol;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

/// 输入域证明的校验失败信息前缀 / Failure prefix used by input-domain proof validation.
pub const DOMAIN_PROOF_WIDENED: &str = "input domain proof may be tightened but not widened";
/// 输入域证明丢失的校验失败信息前缀 / Failure prefix used when an input-domain proof is lost.
pub const DOMAIN_PROOF_LOST: &str = "input domain proof must not be dropped";

/// 结构涉及的列与来源符号 / Columns and source symbol a structure touches.
///
/// 使用语境摘要必须在结构落位之后、最终列编号之前计算，因此结构需要向模型说明"哪些列属于我、
/// 哪个符号是我的来源行"。有了这份绑定，机制模型才能区分自身关系行与外部引用，从而给原生
/// writer 提供准确的使用语境。
///
/// The usage summary must be computed after the structure is in place but before final column
/// numbering, so a structure must tell the model which columns belong to it and which symbol owns
/// its relation rows. With this binding the mechanism model can separate its own rows from
/// external references and hand accurate usage context to native writers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureUsageBinding {
    /// 本函数符号的 ID，其关系行不计入外部引用 / ID of the function symbol whose rows are not external references
    pub source_symbol_id: u64,
    /// 结果列 / Result column
    pub result: crate::variable::VariableId,
    /// 辅助列，不含结果列 / Helper columns, excluding the result column
    pub helpers: Vec<crate::variable::VariableId>,
}

impl StructureUsageBinding {
    /// 创建绑定 / Create a binding.
    pub fn new(
        source_symbol_id: u64,
        result: crate::variable::VariableId,
        helpers: Vec<crate::variable::VariableId>,
    ) -> Self {
        Self {
            source_symbol_id,
            result,
            helpers,
        }
    }
}

/// 延迟物化的函数结构 / A function structure that materializes lazily.
///
/// 实现必须是不可变且可安全共享的；`materialize` 必须是纯函数，同样的列编号必须给出同样的
/// 约束，重复调用不得产生副作用，以便重复 dump、多解和重复求解互不污染。
///
/// Implementations must be immutable and safely shareable. `materialize` must be pure: the same
/// column numbering yields the same constraints, and repeated calls must have no side effects so
/// repeated dumps, multi-solution runs and repeated solves cannot pollute each other.
pub trait DeferredFunctionStructure<V>: Debug + Send + Sync + 'static
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 结构对应的函数符号名称 / Name of the function symbol this structure describes.
    fn function_name(&self) -> &str;

    /// 生成该函数的通用 fallback 约束。
    ///
    /// `symbol_to_index` 与即时展开使用同一套「符号唯一 ID -> 最终列序号」映射，因此
    /// fallback 物化与 EAGER 展开产生相同的稀疏行。任一约束无法生成时必须返回错误且不写入
    /// 任何行。
    ///
    /// Build the generic fallback constraints of this function.
    ///
    /// `symbol_to_index` uses the same "symbol unique ID -> final column index" mapping as eager
    /// expansion, so fallback materialization and eager expansion produce identical sparse rows.
    /// Any failure must return an error without writing a single row.
    fn materialize(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>>;

    /// 以 `Any` 形式返回自身。
    ///
    /// 适配器和 writer 需要检查结构的具体类型（例如读取输入域证明或重算 Big-M），因此结构
    /// 必须可向下转型，而 trait 本身保持最小。
    ///
    /// Return `self` as `Any`.
    ///
    /// Adapters and writers need to inspect the concrete structure type (for example to read the
    /// input-domain proof or recompute a Big-M), so structures must be downcastable while the
    /// trait itself stays minimal.
    fn as_any(&self) -> &dyn std::any::Any;

    /// 返回结构涉及的列与来源符号，用于计算使用语境摘要。
    ///
    /// 默认返回 `None`，表示该结构不参与使用语境判定；此时模型会给出空的摘要，writer 因此看不到
    /// 任何"结果被外部引用"的证据，应当保守地拒绝没有把握的原生写入。
    ///
    /// Return the columns and source symbol the structure touches so the model can compute a usage
    /// summary.
    ///
    /// The default returns `None`, meaning the structure does not take part in usage analysis; the
    /// model then reports an empty summary, so a writer sees no evidence that the result is
    /// externally referenced and should conservatively reject native writes it cannot justify.
    fn usage_binding(&self) -> Option<StructureUsageBinding> {
        None
    }

    /// 返回结构的版本化指纹，用于原生写入的恢复与校验。
    ///
    /// 契约：**内容决定指纹**——同样的结构内容必须给出同样的字符串；任何影响语义的字段变化
    /// （输入多项式、辅助列、Big-M、输入域证明、schema 含义）都必须改变它。默认返回 `None`，
    /// 表示该结构不提供指纹，此时原生写入记录不带指纹、恢复阶段无法校验（
    /// [`verify_native_write`] 会把"一方有指纹、另一方没有"判为不一致）。
    ///
    /// Return the structure's versioned fingerprint for native-write recovery and validation.
    ///
    /// Contract: **content determines the fingerprint** — identical structure content must produce
    /// an identical string, and any semantic field change (input polynomial, helper columns,
    /// Big-M, input-domain proof, schema meaning) must change it. The default returns `None`,
    /// meaning the structure offers no fingerprint; native write records then carry none and
    /// recovery cannot validate ([`verify_native_write`] treats "one side has a fingerprint, the
    /// other does not" as a mismatch).
    fn fingerprint(&self) -> Option<String> {
        None
    }
}

/// 校验原生写入记录是否仍然对应当前结构。
///
/// 使用场景：求解结束后的结果回收，或从持久化状态恢复原生结构时。任一侧缺少指纹都视为不可校验，
/// 只有双方都存在且相等才认为一致——把"没指纹"当成通过会让指纹失去意义。
///
/// Validate that a native write record still corresponds to the current structure.
///
/// Used when recovering results after a solve or when restoring a native structure from persisted
/// state. A missing fingerprint on either side means the pair cannot be validated; only two present
/// and equal fingerprints count as a match, because treating "no fingerprint" as a pass would make
/// fingerprints meaningless.
pub fn verify_native_write<V>(
    recorded: Option<&str>,
    structure: &dyn DeferredFunctionStructure<V>,
) -> bool
where
    V: Clone + Debug + Send + Sync + 'static,
{
    match (recorded, structure.fingerprint()) {
        (Some(recorded), Some(current)) => recorded == current,
        _ => false,
    }
}

/// 结构描述携带的约束来源符号。
///
/// 约束来源只用于报告和残差归属，不参与公式计算；把它抽成独立别名可以让结构描述在不持有
/// 具体函数类型的前提下保持与 EAGER 展开相同的 `from` 归属。
///
/// Source symbol carried by a structure description.
///
/// The constraint source is only used for reporting and residual attribution, never for the
/// formula itself. Aliasing it keeps the same `from` attribution as eager expansion without
/// letting the structure description depend on a concrete function type.
pub type ConstraintSource<V> = Arc<dyn IntermediateSymbol<V>>;

/// 把浮点数格式化为可复现的指纹片段。
///
/// 指纹必须跨进程稳定，因此固定用 `{:.17e}` 的十六进制无关十进制表示，避免默认格式化在不同
/// 版本或精度下产生不同字符串。非有限值单独标记，避免 `NaN` 参与比较。
///
/// Format a float into a reproducible fingerprint fragment.
///
/// Fingerprints must be stable across processes, so `{:.17e}` is used instead of the default
/// formatting, which can vary with precision; non-finite values get their own marker so `NaN`
/// never participates in comparisons.
pub fn fingerprint_float(value: f64) -> String {
    if value.is_nan() {
        "nan".to_string()
    } else if value.is_infinite() {
        if value.is_sign_negative() {
            "-inf".to_string()
        } else {
            "inf".to_string()
        }
    } else {
        format!("{value:.17e}")
    }
}

/// 有限输入域证明 / Proof of a finite input domain.
///
/// 自动推导的 Big-M 与结果范围都建立在"输入落在 `[lower, upper]` 内"这一前提上。把该前提显式
/// 记录成数据，物化阶段与原生 writer 就能重新校验它：**允许收紧**（更窄的区间仍然成立，可以
/// 推出更小的 M），**拒绝扩大**（更宽的区间会让推导出的 M 失效并裁掉原本可行的解），
/// **拒绝丢失**（没有证明就不能声称 M 来自域推断）。
///
/// Automatically derived Big-M values and result ranges rest on the premise that the input stays
/// inside `[lower, upper]`. Recording that premise explicitly lets materialization and native
/// writers re-validate it: **tightening is allowed** (a narrower interval still holds and admits a
/// smaller M), **widening is rejected** (a wider interval invalidates the derived M and would cut
/// off feasible solutions), and **losing the proof is rejected** (without it, an M may not claim
/// to come from domain inference).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InputDomainProof {
    /// 输入域下界 / Input domain lower bound
    lower: f64,
    /// 输入域上界 / Input domain upper bound
    upper: f64,
}

impl InputDomainProof {
    /// 创建输入域证明 / Create an input-domain proof.
    ///
    /// 边界必须有限且下界不大于上界，否则返回错误而不产生一个无法校验的证明。
    ///
    /// Bounds must be finite and the lower bound must not exceed the upper bound; otherwise an
    /// error is returned instead of an unverifiable proof.
    pub fn new(lower: f64, upper: f64) -> Result<Self> {
        if !lower.is_finite() || !upper.is_finite() {
            return Err(ModelError::InvalidConstraint(format!(
                "input domain proof requires finite bounds, got [{lower}, {upper}]"
            ))
            .into());
        }
        if lower > upper {
            return Err(ModelError::InvalidConstraint(format!(
                "input domain proof requires lower <= upper, got [{lower}, {upper}]"
            ))
            .into());
        }
        Ok(Self { lower, upper })
    }

    /// 从可选边界创建证明；任一侧缺失或非有限时返回 `None`。
    /// Build a proof from optional bounds; returns `None` when either side is missing or not finite.
    pub fn from_optional_bounds(lower: Option<f64>, upper: Option<f64>) -> Option<Self> {
        let lower = lower?;
        let upper = upper?;
        Self::new(lower, upper).ok()
    }

    /// 输入域下界 / Input domain lower bound.
    pub fn lower(&self) -> f64 {
        self.lower
    }

    /// 输入域上界 / Input domain upper bound.
    pub fn upper(&self) -> f64 {
        self.upper
    }

    /// 判断本证明是否被另一个证明包含（即是否不宽于对方）。
    ///
    /// 返回 `true` 表示本证明是在对方基础上收紧或持平的结果。
    ///
    /// Whether this proof is contained in another one (that is, not wider than it).
    ///
    /// `true` means this proof is a tightening of, or equal to, the other one.
    pub fn is_contained_in(&self, other: &Self) -> bool {
        self.lower >= other.lower && self.upper <= other.upper
    }

    /// 与另一个证明求交（取更紧的一侧）；无交集时返回 `None`。
    /// Intersect with another proof, taking the tighter side; `None` when they do not overlap.
    pub fn intersect(&self, other: &Self) -> Option<Self> {
        let lower = self.lower.max(other.lower);
        let upper = self.upper.min(other.upper);
        if lower > upper {
            return None;
        }
        Some(Self { lower, upper })
    }
}
