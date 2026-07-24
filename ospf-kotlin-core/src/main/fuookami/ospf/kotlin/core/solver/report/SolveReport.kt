@file:OptIn(kotlin.time.ExperimentalTime::class)
package fuookami.ospf.kotlin.core.solver.report

import java.security.MessageDigest
import java.time.Instant
import java.util.concurrent.atomic.AtomicReference
import kotlin.time.Duration
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.number.UInt64
import fuookami.ospf.kotlin.core.model.basic.Solution
import fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingFeature
import fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSupportLevel
import fuookami.ospf.kotlin.core.solver.output.FeasibleSolverOutput

/** 问题结论 / Problem conclusion */
enum class ProblemStatus {
    Feasible,
    Infeasible,
    Unbounded,
    InfeasibleOrUnbounded,
    Unknown
}

/** 求解终止原因 / Solve termination reason */
enum class TerminationReason {
    Completed,
    TimeLimit,
    NodeLimit,
    IterationLimit,
    SolutionLimit,
    ObjectiveLimit,
    Cancelled,
    Interrupted,
    NumericalFailure,
    BackendFailure
}

/** 解存在性 / Solution presence */
enum class SolutionPresence {
    None,
    Incumbent,
    Optimal
}

/** 证明状态 / Proof status */
enum class ProofStatus {
    None,
    Claimed,
    Verified
}

/** 模型类型 / Model type */
enum class SolverModelType {
    LP,
    MIP,
    QP,
    QCP,
    CP
}

/** 模型元素稳定标识 / Stable model element identifier */
@JvmInline
value class ModelElementId(val value: String)

/** 变量稳定标识 / Stable variable identifier */
@JvmInline
value class VariableId(val value: String)

/** 变量边界方向 / Variable bound side */
enum class BoundSide {
    Lower,
    Upper
}

/** 稳定变量边界引用 / Stable variable-bound reference */
data class VariableBoundRef(
    val variableId: VariableId,
    val side: BoundSide
)

/** 稳定变量值域引用 / Stable variable-domain reference */
data class VariableDomainRef(
    val variableId: VariableId
)

/** 约束稳定标识 / Stable constraint identifier */
@JvmInline
value class ConstraintId(val value: String)

/** 目标稳定标识 / Stable objective identifier */
@JvmInline
value class ObjectiveId(val value: String)

/** 求解运行标识 / Solve run identifier */
@JvmInline
value class SolveRunId(val value: String)

/** 求解尝试标识 / Solve attempt identifier */
@JvmInline
value class SolveAttemptId(val value: String)

/** 诊断或执行问题类别 / Diagnostic or execution issue category */
enum class SolveIssueCategory {
    InvalidInput,
    Environment,
    License,
    Numerical,
    Callback,
    Parsing,
    Backend,
    Protocol,
    Unsupported
}

/**
 * 结构化求解问题。 / Structured solve issue.
 *
 * @property code 稳定错误码 / Stable issue code
 * @property category 问题类别 / Issue category
 * @property message 脱敏消息 / Redacted message
 * @property details 结构化详情 / Structured details
 */
data class SolveIssue(
    val code: String,
    val category: SolveIssueCategory,
    val message: String,
    val details: Map<String, String> = emptyMap()
)

/** 求解器能力声明 / Solver capability declaration */
data class SolverCapabilities(
    val modelTypes: Set<SolverModelType>,
    val nativeIIS: Boolean = false,
    val dual: Boolean = false,
    val farkas: Boolean = false,
    val warmStart: Boolean = false,
    val solutionPool: Boolean = false,
    val callback: Boolean = false,
    val interrupt: Boolean = false,
    val checkpoint: Boolean = false,
    val resume: Boolean = false,
    val constraintProgrammingFeatures: Map<ConstraintProgrammingFeature, ConstraintProgrammingSupportLevel> = emptyMap()
)

/** 求解器描述符 / Solver descriptor */
data class SolverDescriptor(
    val solverId: String,
    val backendName: String,
    val backendVersion: String? = null,
    val pluginVersion: String? = null,
    val capabilities: SolverCapabilities
)

/**
 * 可审计 backend 配置。 / Auditable backend configuration.
 *
 * 实现不得在公开参数或指纹参数中返回密码、令牌或连接密钥。 / Implementations must not expose passwords, tokens, or connection secrets.
 */
interface BackendConfiguration {
    /** 配置类型标识 / Configuration type identifier */
    val type: String

    /** 脱敏且确定排序的参数 / Redacted and deterministically ordered parameters */
    fun redactedParameters(): Map<String, String>
}

/** 求解器执行来源 / Solver execution provenance */
data class SolverProvenance(
    val descriptor: SolverDescriptor,
    val nativeVersion: String? = null,
    val effectiveParameters: Map<String, String> = emptyMap(),
    val ignoredParameters: Map<String, String> = emptyMap(),
    val threadCount: Int? = null,
    val randomSeed: Long? = null,
    val deterministic: Boolean? = null,
    val environmentSummary: Map<String, String> = emptyMap()
)

/** 求解统计 / Solve statistics */
data class SolveStatistics<V>(
    val solveTime: Duration? = null,
    val iterations: ULong? = null,
    val nodes: ULong? = null,
    val bestBound: V? = null,
    val gap: V? = null
)

/** 求解解及解池 / Solve solution and pool */
data class SolveSolution<V>(
    val values: Solution<V>,
    val objective: V? = null,
    val pool: List<Solution<V>> = emptyList()
)

/** 求解证明 / Solve proof */
data class SolveProof(
    val status: ProofStatus,
    val kind: String? = null,
    val reference: String? = null
)

/** 约束关系 / Constraint relation */
enum class ConstraintRelation {
    LessEqual,
    Equal,
    GreaterEqual,
    Range
}

/** 约束求值 / Constraint evaluation */
data class ConstraintEvaluation<V>(
    val constraintId: ConstraintId,
    val lhs: V,
    val rhs: V,
    val relation: ConstraintRelation,
    val slack: V,
    val violation: V,
    val tolerance: V,
    val satisfied: Boolean,
    val dual: V? = null
)

/** 不可行证据来源 / Infeasibility evidence source */
enum class InfeasibilityEvidenceSource {
    NativeIIS,
    Farkas,
    ConstraintConflict,
    ElasticFilter,
    DeletionFilter,
    None
}

/** 证据有效性 / Evidence validity */
enum class EvidenceValidity {
    Verified,
    Heuristic,
    Unknown
}

/** 证据最小性 / Evidence minimality */
enum class EvidenceMinimality {
    Irreducible,
    Partial,
    NotChecked
}

/** 结构化不可行成员 / Structured infeasibility member */
sealed interface InfeasibilityMember {
    /** 原始约束成员 / Original constraint member */
    data class Constraint(val id: ConstraintId) : InfeasibilityMember

    /** 变量上下界成员 / Variable-bound member */
    data class VariableBound(val ref: VariableBoundRef) : InfeasibilityMember

    /** 稀疏值域成员 / Sparse-domain member */
    data class VariableDomain(val ref: VariableDomainRef) : InfeasibilityMember
}

/** 不可行证据精度 / Infeasibility evidence exactness */
enum class EvidenceExactness {
    Exact,
    Irreducible,
    Heuristic,
    Unknown
}

/** 不可行证据完整度 / Infeasibility evidence completeness */
enum class EvidenceCompleteness {
    Complete,
    Partial,
    Unavailable
}

/** 不可行证据 / Infeasibility evidence */
data class InfeasibilityEvidence(
    val source: InfeasibilityEvidenceSource,
    val exactness: EvidenceExactness = EvidenceExactness.Unknown,
    val completeness: EvidenceCompleteness = EvidenceCompleteness.Unavailable,
    val constraintIds: Set<ConstraintId> = emptySet(),
    val variableBoundIds: Set<VariableId> = emptySet(),
    val elapsed: Duration? = null,
    val unavailableReason: SolveIssue? = null,
    val validity: EvidenceValidity = EvidenceValidity.Unknown,
    val minimality: EvidenceMinimality = EvidenceMinimality.NotChecked,
    val variableBoundRefs: Set<VariableBoundRef> = emptySet(),
    val variableDomainRefs: Set<VariableDomainRef> = emptySet(),
    val members: Set<InfeasibilityMember> = emptySet(),
    val reference: String? = null,
    /** 后端诊断使用的稳定假设成员 ID。 / Stable assumption-member IDs used by backend diagnostics. */
    val assumptionIds: Set<VariableId> = emptySet(),
    /** 诊断复验执行次数。 / Number of verification checks performed by the diagnostic. */
    val verificationChecks: UInt64? = null,
    /** 诊断复验的终止原因。 / Termination reason of diagnostic verification. */
    val terminationReason: TerminationReason? = null
)

/** 求解诊断 / Solve diagnostics */
data class SolveDiagnostics<V>(
    val constraintEvaluations: List<ConstraintEvaluation<V>> = emptyList(),
    val infeasibilityEvidence: InfeasibilityEvidence? = null,
    val warnings: List<SolveIssue> = emptyList(),
    val errors: List<SolveIssue> = emptyList()
)

/** 带算法和版本信息的加密指纹 / Cryptographic fingerprint with algorithm and schema version */
data class AuditFingerprint(
    val schemaVersion: String,
    val algorithm: String,
    val value: String
)

/** 模型指纹 / Model fingerprint */
typealias ModelFingerprint = AuditFingerprint

/** 配置指纹 / Configuration fingerprint */
typealias ConfigurationFingerprint = AuditFingerprint

/** 求解器指纹 / Solver fingerprint */
typealias SolverFingerprint = AuditFingerprint

/** 求解指纹集合 / Solve fingerprint set */
data class SolveFingerprints(
    val model: AuditFingerprint? = null,
    val configuration: AuditFingerprint? = null,
    val solver: AuditFingerprint? = null
)

/** 统一求解报告 / Unified solve report */
data class SolveReport<V>(
    val schemaVersion: String = CURRENT_SCHEMA_VERSION,
    val runId: SolveRunId? = null,
    val problemStatus: ProblemStatus,
    val terminationReason: TerminationReason,
    val solutionPresence: SolutionPresence,
    val solution: SolveSolution<V>? = null,
    val proof: SolveProof = SolveProof(ProofStatus.None),
    val statistics: SolveStatistics<V> = SolveStatistics(),
    val diagnostics: SolveDiagnostics<V> = SolveDiagnostics(),
    val provenance: SolverProvenance? = null,
    val fingerprints: SolveFingerprints = SolveFingerprints()
) {
    companion object {
        const val CURRENT_SCHEMA_VERSION: String = "1.0"
    }
}

/** 组合求解选择依据 / Combinatorial solve selection reason */
enum class SolveSelectionReason {
    FirstFeasible,
    BestObjective,
    BestBound,
    PreferredBackend,
    NoSuccessfulAttempt
}

/** 单次 backend 尝试轨迹 / Single backend attempt trace */
data class SolveAttemptTrace<V>(
    val attemptId: SolveAttemptId,
    val parentAttemptId: SolveAttemptId? = null,
    val backendId: String,
    val report: SolveReport<V>? = null,
    val elapsed: Duration? = null,
    val errors: List<SolveIssue> = emptyList(),
    val cancellationReason: String? = null
)

/** 组合求解报告 / Combinatorial solve report */
data class CombinatorialSolveReport<V>(
    val finalReport: SolveReport<V>?,
    val attempts: List<SolveAttemptTrace<V>>,
    val selectedAttemptId: SolveAttemptId?,
    val selectionReason: SolveSelectionReason
)

/** 取消来源 / Cancellation source */
enum class CancellationSource {
    Caller,
    Coroutine,
    Future,
    Combinatorial,
    Remote,
    Timeout
}

/** 取消事实 / Cancellation fact */
data class CancellationRecord(
    val source: CancellationSource,
    val requestedAt: Instant,
    val reason: String? = null
)

/** 求解取消令牌 / Solve cancellation token */
class CancellationToken internal constructor(
    private val cancellation: AtomicReference<CancellationRecord?>
) {
    /** 是否已经请求取消 / Whether cancellation has been requested */
    val isCancellationRequested: Boolean get() = cancellation.get() != null

    /** 首次取消事实 / First cancellation record */
    val record: CancellationRecord? get() = cancellation.get()
}

/**
 * 单次求解句柄。 / Per-solve handle.
 *
 * @property token 取消令牌 / Cancellation token
 */
class SolveHandle private constructor(
    val token: CancellationToken,
    private val cancellation: AtomicReference<CancellationRecord?>,
    private val interrupt: (CancellationRecord) -> Try
) {
    /** 幂等请求取消；仅首次请求调用 backend 中断 / Request cancellation idempotently */
    fun cancel(
        source: CancellationSource = CancellationSource.Caller,
        reason: String? = null
    ): Try {
        val record = CancellationRecord(source, Instant.now(), reason)
        return if (cancellation.compareAndSet(null, record)) {
            interrupt(record)
        } else {
            ok
        }
    }

    companion object {
        /** 创建独立求解句柄 / Create an independent solve handle */
        fun create(interrupt: (CancellationRecord) -> Try = { ok }): SolveHandle {
            val cancellation = AtomicReference<CancellationRecord?>(null)
            return SolveHandle(
                token = CancellationToken(cancellation),
                cancellation = cancellation,
                interrupt = interrupt
            )
        }
    }
}

/** 确定性 SHA-256 指纹工具 / Deterministic SHA-256 fingerprint utility */
object SolveFingerprinting {
    /** 对已规范化的 UTF-8 内容生成指纹 / Fingerprint canonical UTF-8 content */
    fun sha256(canonicalContent: String, schemaVersion: String = "1.0"): AuditFingerprint {
        val bytes = MessageDigest.getInstance("SHA-256")
            .digest(canonicalContent.toByteArray(Charsets.UTF_8))
        return AuditFingerprint(
            schemaVersion = schemaVersion,
            algorithm = "SHA-256",
            value = bytes.joinToString(separator = "") { byte -> "%02x".format(byte) }
        )
    }

    /** 规范化键值配置后生成指纹 / Fingerprint a canonicalized key-value configuration */
    fun configuration(parameters: Map<String, String>, schemaVersion: String = "1.0"): AuditFingerprint {
        val canonical = parameters.toSortedMap().entries.joinToString(separator = "\n") { (key, value) ->
            "${escape(key)}=${escape(value)}"
        }
        return sha256(canonical, schemaVersion)
    }

    private fun escape(value: String): String {
        return value.replace("\\", "\\\\").replace("\n", "\\n").replace("=", "\\=")
    }
}

/** 将旧可行输出无损包装为统一报告 / Wrap a legacy feasible output in a unified report */
fun <V> FeasibleSolverOutput<V>.toSolveReport(
    runId: SolveRunId? = null,
    provenance: SolverProvenance? = null,
    fingerprints: SolveFingerprints = SolveFingerprints()
): SolveReport<V> {
    return SolveReport(
        runId = runId,
        problemStatus = ProblemStatus.Feasible,
        terminationReason = TerminationReason.Completed,
        solutionPresence = if (gap == Flt64.zero) {
            SolutionPresence.Optimal
        } else {
            SolutionPresence.Incumbent
        },
        solution = SolveSolution(
            values = solution,
            objective = objValueOrNull
        ),
        proof = SolveProof(
            status = if (gap == Flt64.zero) {
                ProofStatus.Claimed
            } else {
                ProofStatus.None
            },
            kind = "legacy-feasible-output"
        ),
        statistics = SolveStatistics(
            solveTime = solveTime,
            iterations = iterations?.toULong(),
            nodes = nodeCount?.toULong(),
            bestBound = bestBoundValueOrNull,
            gap = null
        ),
        provenance = provenance,
        fingerprints = fingerprints
    )
}
