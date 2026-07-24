/** Logic-Based Benders contracts and engine. / Logic-Based Benders 契约与迭代引擎。 */
package fuookami.ospf.kotlin.framework.solver

import kotlin.time.Duration
import kotlin.time.Duration.Companion.ZERO
import kotlin.time.TimeSource
import fuookami.ospf.kotlin.core.model.constraint_programming.BooleanLiteral
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingModel
import fuookami.ospf.kotlin.core.model.constraint_programming.IntegerDomain
import fuookami.ospf.kotlin.core.model.intermediate.LinearTriadModelView
import fuookami.ospf.kotlin.core.model.mechanism.LinearMetaModel
import fuookami.ospf.kotlin.core.solver.AbstractLinearSolver
import fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSession
import fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSolveOptions
import fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSolver
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingConflict
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingFeasibleOutput
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingInfeasibleOutput
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingSolverOutput
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingUnknownOutput
import fuookami.ospf.kotlin.core.solver.output.FeasibleSolverOutput
import fuookami.ospf.kotlin.core.solver.output.SolverStatus
import fuookami.ospf.kotlin.core.solver.report.CancellationToken
import fuookami.ospf.kotlin.core.solver.report.ProblemStatus
import fuookami.ospf.kotlin.core.solver.report.ProofStatus
import fuookami.ospf.kotlin.core.solver.report.SolveDiagnostics
import fuookami.ospf.kotlin.core.solver.report.SolveIssue
import fuookami.ospf.kotlin.core.solver.report.SolveIssueCategory
import fuookami.ospf.kotlin.core.solver.report.SolveProof
import fuookami.ospf.kotlin.core.solver.report.SolveReport
import fuookami.ospf.kotlin.core.solver.report.SolveSolution
import fuookami.ospf.kotlin.core.solver.report.SolutionPresence
import fuookami.ospf.kotlin.core.solver.report.TerminationReason
import fuookami.ospf.kotlin.core.solver.report.VariableId
import fuookami.ospf.kotlin.core.variable.AbstractVariableItem
import fuookami.ospf.kotlin.core.variable.BinVar
import fuookami.ospf.kotlin.core.variable.BinVariable
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.number.Int64
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.inequality.LinearInequality
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.utils.error.ErrorCode
import fuookami.ospf.kotlin.utils.functional.Failed
import fuookami.ospf.kotlin.utils.functional.Fatal
import fuookami.ospf.kotlin.utils.functional.Ok
import fuookami.ospf.kotlin.utils.functional.Ret
import fuookami.ospf.kotlin.utils.functional.Try
import fuookami.ospf.kotlin.utils.functional.ok

/** Engine proof mode. / 引擎证明模式。 */
enum class BendersProofMode {
    /** Every accepted terminal certificate must be verified. / 所有接受的终态证书都必须已验证。 */
    Exact,

    /** Feasible incumbents may be used without a global optimality proof. / 允许使用未完成全局最优证明的可行解。 */
    Heuristic
}

/** Master-cut category. / 主问题割类别。 */
enum class BendersCutKind {
    Feasibility,
    Conflict,
    NoGood,
    Optimality,
    Heuristic
}

/** Cut validity scope. / 割的有效性范围。 */
enum class BendersCutValidity {
    /** Valid for the complete master feasible region. / 对整个主问题可行域全局有效。 */
    Global,

    /** Valid only at the current assignment. / 仅对当前赋值有效。 */
    Assignment,

    /** Heuristic and not suitable for exact mode. / 启发式有效性，不能用于精确模式。 */
    Heuristic
}

/** Stable value source used by variable bindings. / 变量绑定使用的稳定值源。 */
interface ConstraintProgrammingValueSource {
    /** Read a value by a stable domain key. / 按稳定领域键读取值。 */
    fun value(key: String): Ret<Flt64>

    /** Return all values for diagnostics and trace output. / 返回全部值以供诊断和轨迹使用。 */
    fun values(): Map<String, Flt64>

    /** Read a value by an OSPF variable identity. / 按 OSPF 变量身份读取值。 */
    fun value(variable: AbstractVariableItem<*, *>): Ret<Flt64> {
        return value(variable.bendersStableKey())
    }

    companion object {
        /** Create a map-backed source. / 创建基于映射的值源。 */
        fun of(values: Map<String, Flt64>): ConstraintProgrammingValueSource {
            return MapConstraintProgrammingValueSource(values)
        }
    }
}

/** Map-backed value source. / 基于映射的值源。 */
data class MapConstraintProgrammingValueSource(
    private val assignment: Map<String, Flt64>
) : ConstraintProgrammingValueSource {
    override fun value(key: String): Ret<Flt64> {
        return assignment[key]?.let(::ok) ?: Failed(
            ErrorCode.DataNotFound,
            "缺少主问题变量值：$key / Missing master variable value: $key"
        )
    }

    override fun values(): Map<String, Flt64> {
        return assignment.toMap()
    }
}

/** Build a source from a legacy solver vector and optional variable list. / 从旧求解器向量和可选变量列表创建值源。 */
fun masterSolutionValueSource(
    output: FeasibleSolverOutput<Flt64>,
    variables: List<AbstractVariableItem<*, *>> = emptyList()
): ConstraintProgrammingValueSource {
    val values = LinkedHashMap<String, Flt64>()
    output.solution.forEachIndexed { index, value ->
        values["index:$index"] = value
        values[index.toString()] = value
    }
    variables.forEach { variable ->
        output.solution.getOrNull(variable.index)?.let { value ->
            values[variable.bendersStableKey()] = value
        }
    }
    return MapConstraintProgrammingValueSource(values)
}

/** One bound master value and its CP projection. / 一个主问题值及其 CP 投影。 */
data class BendersMasterAssignment(
    val key: String,
    val variable: AbstractVariableItem<*, *>,
    val value: Flt64
)

/** Master assignment fixed for one CP subproblem solve. / 一轮 CP 子问题固定的主问题赋值。 */
data class BendersSubproblemAssignment(
    val masterValues: Map<String, Flt64>,
    val masterVariables: Map<String, AbstractVariableItem<*, *>>,
    val fixedValues: Map<VariableId, Int64>,
    val assumptions: List<BooleanLiteral>,
    val assumptionToMaster: Map<String, String> = emptyMap()
) {
    /** Compatibility alias for callers using the plan terminology. / 计划术语兼容别名。 */
    val masterAssignment: Map<String, Flt64>
        get() = masterValues

    /** Compatibility alias for CP fixed values. / CP 固定值兼容别名。 */
    val cpValues: Map<VariableId, Int64>
        get() = fixedValues

    /** Reverse assumption mapping keyed by CP variable ID. / 按 CP 变量 ID 索引的反向映射。 */
    val assumptionOrigins: Map<String, String>
        get() = assumptionToMaster
}

/** Explicit master-to-CP binding contract. / 显式主问题到 CP 绑定契约。 */
interface BendersVariableBinding {
    /** Bind one master solution to one static CP model. / 将一轮主问题解绑定到静态 CP 模型。 */
    fun bind(
        masterSolution: ConstraintProgrammingValueSource,
        subproblem: ConstraintProgrammingModel
    ): Ret<BendersSubproblemAssignment>
}

/** A binary master variable mapped to a binary CP variable. / 主问题二值变量到 CP 二值变量的映射。 */
data class BinaryBendersVariable(
    val key: String,
    val masterVariable: AbstractVariableItem<*, *>,
    val subproblemVariable: BinVariable
)

/** A bounded integer master variable mapped to an integer CP variable. / 有界整数主问题变量到 CP 整数变量的映射。 */
data class IntegerBendersVariable(
    val key: String,
    val masterVariable: AbstractVariableItem<*, *>,
    val subproblemVariable: AbstractVariableItem<*, *>,
    val domain: IntegerDomain
)

/** Default binding for binary master assignments. / 二值主问题赋值的默认绑定。 */
class BinaryBendersVariableBinding(
    private val variables: List<BinaryBendersVariable>
) : BendersVariableBinding {
    override fun bind(
        masterSolution: ConstraintProgrammingValueSource,
        subproblem: ConstraintProgrammingModel
    ): Ret<BendersSubproblemAssignment> {
        val snapshot = subproblem.snapshot()
        if (snapshot.failed) {
            return propagate(snapshot)
        }
        val current = snapshot.value!!
        val masterValues = LinkedHashMap<String, Flt64>()
        val masterVariables = LinkedHashMap<String, AbstractVariableItem<*, *>>()
        val fixedValues = LinkedHashMap<VariableId, Int64>()
        val assumptions = ArrayList<BooleanLiteral>()
        val assumptionToMaster = LinkedHashMap<String, String>()

        for (binding in variables) {
            if (!binding.masterVariable.type.isBinaryType || !binding.subproblemVariable.type.isBinaryType) {
                return Failed(
                    ErrorCode.IllegalArgument,
                    "Benders 绑定要求主问题和 CP 变量均为二值：${binding.key} / " +
                        "Benders binding requires binary master and CP variables: ${binding.key}"
                )
            }
            val value = masterSolution.value(binding.key)
            if (value.failed) {
                return propagate(value)
            }
            val raw = value.value!!
            if (raw != Flt64.zero && raw != Flt64.one) {
                return Failed(
                    ErrorCode.IllegalArgument,
                    "Benders 主问题变量必须为 0 或 1：${binding.key}=$raw / " +
                        "Benders master variable must be 0 or 1: ${binding.key}=$raw"
                )
            }
            val cpId = VariableId("${binding.subproblemVariable.identifier}:${binding.subproblemVariable.index}")
            if (current.variable(cpId) == null) {
                return Failed(
                    ErrorCode.DataNotFound,
                    "CP 子问题缺少绑定变量：$cpId / CP subproblem is missing bound variable: $cpId"
                )
            }
            val integer = if (raw == Flt64.one) Int64.one else Int64.zero
            val assumption = BooleanLiteral(binding.subproblemVariable, negated = integer == Int64.zero)
            masterValues[binding.key] = raw
            masterVariables[binding.key] = binding.masterVariable
            fixedValues[cpId] = integer
            assumptions += assumption
            assumptionToMaster[cpId.value] = binding.key
        }

        return ok(
            BendersSubproblemAssignment(
                masterValues = masterValues,
                masterVariables = masterVariables,
                fixedValues = fixedValues,
                assumptions = assumptions,
                assumptionToMaster = assumptionToMaster
            )
        )
    }
}

/** Binding for bounded integer master assignments. / 有界整数主问题赋值绑定。 */
class IntegerBendersVariableBinding(
    private val variables: List<IntegerBendersVariable>
) : BendersVariableBinding {
    override fun bind(
        masterSolution: ConstraintProgrammingValueSource,
        subproblem: ConstraintProgrammingModel
    ): Ret<BendersSubproblemAssignment> {
        val snapshot = subproblem.snapshot()
        if (snapshot.failed) {
            return propagate(snapshot)
        }
        val current = snapshot.value!!
        val masterValues = LinkedHashMap<String, Flt64>()
        val masterVariables = LinkedHashMap<String, AbstractVariableItem<*, *>>()
        val fixedValues = LinkedHashMap<VariableId, Int64>()

        for (binding in variables) {
            if (!binding.masterVariable.type.isIntegerType || !binding.subproblemVariable.type.isIntegerType) {
                return Failed(
                    ErrorCode.IllegalArgument,
                    "Benders 绑定要求主问题和 CP 变量均为整数：${binding.key} / " +
                        "Benders binding requires integer master and CP variables: ${binding.key}"
                )
            }
            val rawResult = masterSolution.value(binding.key)
            if (rawResult.failed) {
                return propagate(rawResult)
            }
            val raw = rawResult.value!!
            if (!raw.toDouble().isFinite() || raw != raw.round()) {
                return Failed(
                    ErrorCode.IllegalArgument,
                    "Benders 主问题变量必须为整数：${binding.key}=$raw / " +
                        "Benders master variable must have an integral value: ${binding.key}=$raw"
                )
            }
            val integer = raw.toInt64()
            if (!binding.domain.contains(integer)) {
                return Failed(
                    ErrorCode.ORSolutionInvalid,
                    "Benders 主问题赋值超出声明值域：${binding.key}=$integer / " +
                        "Benders master assignment is outside the declared domain: ${binding.key}=$integer"
                )
            }
            val cpId = VariableId("${binding.subproblemVariable.identifier}:${binding.subproblemVariable.index}")
            val cpDefinition = current.variable(cpId)
                ?: return Failed(
                    ErrorCode.DataNotFound,
                    "CP 子问题缺少绑定变量：$cpId / CP subproblem is missing bound variable: $cpId"
                )
            if (!cpDefinition.domain.contains(integer)) {
                return Failed(
                    ErrorCode.ORSolutionInvalid,
                    "绑定值超出 CP 子问题值域：$cpId=$integer / " +
                        "Binding value is outside the CP subproblem domain: $cpId=$integer"
                )
            }
            masterValues[binding.key] = raw
            masterVariables[binding.key] = binding.masterVariable
            fixedValues[cpId] = integer
        }

        return ok(
            BendersSubproblemAssignment(
                masterValues = masterValues,
                masterVariables = masterVariables,
                fixedValues = fixedValues,
                assumptions = emptyList(),
                assumptionToMaster = emptyMap()
            )
        )
    }
}

/** Subproblem result contract. / 子问题结果契约。 */
sealed interface LogicBasedBendersSubproblemResult {
    val assignment: BendersSubproblemAssignment
}

/** Proven or accepted feasible CP result. / 已证明或已接受的 CP 可行结果。 */
data class FeasibleSubproblemResult(
    override val assignment: BendersSubproblemAssignment,
    val output: ConstraintProgrammingFeasibleOutput
) : LogicBasedBendersSubproblemResult

/** Proven CP infeasibility result. / 已证明 CP 不可行结果。 */
data class InfeasibleSubproblemResult(
    override val assignment: BendersSubproblemAssignment,
    val conflict: ConstraintProgrammingConflict?,
    val proofStatus: ProofStatus = ProofStatus.Verified
) : LogicBasedBendersSubproblemResult

/** Unknown, limited, or cancelled CP result. / 未知、受限或取消的 CP 结果。 */
data class UnknownSubproblemResult(
    override val assignment: BendersSubproblemAssignment,
    val terminationReason: TerminationReason
) : LogicBasedBendersSubproblemResult

/** Context supplied to cut oracles. / 提供给割预言器的上下文。 */
data class BendersCutContext(
    val master: LinearMetaModel<Flt64>,
    val subproblem: ConstraintProgrammingModel,
    val assignment: BendersSubproblemAssignment,
    val iteration: Int,
    val proofMode: BendersProofMode
)

/** Master cut contract. / 主问题割契约。 */
data class BendersMasterCut(
    val inequality: LinearInequality<Flt64>,
    val kind: BendersCutKind,
    val validity: BendersCutValidity,
    val proofStatus: ProofStatus,
    val source: String,
    val name: String? = null,
    val additionalInequalities: List<LinearInequality<Flt64>> = emptyList(),
    val auxiliaryVariables: List<BinVar> = emptyList()
) {
    /** Stable deduplication key. / 稳定去重键。 */
    val key: String
        get() = buildString {
            append(source)
            append('|')
            append(kind)
            append('|')
            append(inequality)
            additionalInequalities.forEach {
                append('|')
                append(it)
            }
            auxiliaryVariables.forEach {
                append('|')
                append(it.identifier)
                append(':')
                append(it.index)
            }
        }
}

/** Domain cut oracle. / 领域割预言器。 */
interface BendersCutOracle {
    /** Generate feasibility/conflict cuts. / 生成可行性或冲突割。 */
    fun feasibilityCuts(
        result: InfeasibleSubproblemResult,
        context: BendersCutContext
    ): Ret<List<BendersMasterCut>>

    /** Generate optional optimality cuts. / 生成可选最优性割。 */
    fun optimalityCuts(
        result: FeasibleSubproblemResult,
        context: BendersCutContext
    ): Ret<List<BendersMasterCut>>
}

/** Default binary no-good/conflict cut oracle. / 默认二值 no-good/conflict 割预言器。 */
class BinaryNoGoodCutOracle : BendersCutOracle {
    override fun feasibilityCuts(
        result: InfeasibleSubproblemResult,
        context: BendersCutContext
    ): Ret<List<BendersMasterCut>> {
        val candidateKeys = linkedSetOf<String>()
        val conflict = result.conflict
        // A shrunk core is only safe to project when its final revalidation was
        // verified.  Otherwise fall back to the complete assignment: the
        // proven infeasibility of that fixed assignment still yields a valid
        // global binary no-good cut.
        val useVerifiedCore = conflict?.validity == fuookami.ospf.kotlin.core.solver.report.EvidenceValidity.Verified
        if (conflict != null && useVerifiedCore) {
            conflict.assumptions.forEach { assumption ->
                assumption.variableId?.value?.let { id ->
                    result.assignment.assumptionToMaster[id]?.let(candidateKeys::add)
                }
            }
            conflict.variableIds.forEach { id ->
                result.assignment.assumptionToMaster[id.value]?.let(candidateKeys::add)
            }
        }
        if (candidateKeys.isEmpty()) {
            candidateKeys += result.assignment.masterValues.keys
        }
        if (candidateKeys.isEmpty()) {
            return Failed(
                ErrorCode.IllegalArgument,
                "无法为无变量赋值生成 no-good cut / Cannot generate a no-good cut without an assignment"
            )
        }
        return noGoodCut(result.assignment, candidateKeys, conflict != null && useVerifiedCore)
    }

    override fun optimalityCuts(
        result: FeasibleSubproblemResult,
        context: BendersCutContext
    ): Ret<List<BendersMasterCut>> {
        // A CP point objective is not globally valid without a domain oracle. / CP 点目标没有领域证明时不具备全局有效性。
        return ok(emptyList())
    }

    private fun noGoodCut(
        assignment: BendersSubproblemAssignment,
        keys: Set<String>,
        conflict: Boolean
    ): Ret<List<BendersMasterCut>> {
        val terms = ArrayList<LinearMonomial<Flt64>>()
        var constant = Flt64.zero
        for (key in keys) {
            val variable = assignment.masterVariables[key]
                ?: return Failed(
                    ErrorCode.DataNotFound,
                    "no-good cut 缺少主问题变量：$key / No-good cut is missing master variable: $key"
                )
            val value = assignment.masterValues[key]
                ?: return Failed(
                    ErrorCode.DataNotFound,
                    "no-good cut 缺少主问题赋值：$key / No-good cut is missing master assignment: $key"
                )
            if (value != Flt64.zero && value != Flt64.one) {
                return Failed(
                    ErrorCode.IllegalArgument,
                    "no-good cut 只支持二值赋值：$key=$value / No-good cut only supports binary assignments: $key=$value"
                )
            }
            if (value == Flt64.one) {
                terms += LinearMonomial(-Flt64.one, variable)
                constant += Flt64.one
            } else {
                terms += LinearMonomial(Flt64.one, variable)
            }
        }
        val lhs = LinearPolynomial(terms, constant)
        val rhs = LinearPolynomial<Flt64>(emptyList(), Flt64.one)
        return ok(
            listOf(
                BendersMasterCut(
                    inequality = LinearInequality(lhs, rhs, Comparison.GE),
                    kind = if (conflict) BendersCutKind.Conflict else BendersCutKind.NoGood,
                    validity = BendersCutValidity.Global,
                    proofStatus = ProofStatus.Verified,
                    source = if (conflict) "binary-conflict-core" else "binary-no-good"
                )
            )
        )
    }
}

/** Exact no-good oracle for bounded integer master assignments. / 有界整数主问题的精确 no-good 割预言器。 */
class IntegerNoGoodCutOracle(
    private val variables: List<IntegerBendersVariable>
) : BendersCutOracle {
    override fun feasibilityCuts(
        result: InfeasibleSubproblemResult,
        context: BendersCutContext
    ): Ret<List<BendersMasterCut>> {
        if (variables.isEmpty()) {
            return Failed(
                ErrorCode.IllegalArgument,
                "整数 no-good 绑定不能为空 / Integer no-good binding must not be empty"
            )
        }
        val items = ArrayList<IntegerNoGoodVariable>(variables.size)
        for (binding in variables) {
            val value = result.assignment.masterValues[binding.key]
                ?: return Failed(
                    ErrorCode.DataNotFound,
                    "整数 no-good 缺少主问题赋值：${binding.key} / Integer no-good is missing master assignment: ${binding.key}"
                )
            if (value != value.round() || !value.toDouble().isFinite()) {
                return Failed(
                    ErrorCode.ORSolutionInvalid,
                    "整数 no-good 收到非整数赋值：${binding.key}=$value / " +
                        "Integer no-good received a non-integral assignment: ${binding.key}=$value"
                )
            }
            items += IntegerNoGoodVariable(
                key = binding.key,
                variable = binding.masterVariable,
                value = value.toInt64(),
                domain = binding.domain
            )
        }
        val encoding = IntegerNoGoodCutEncoder.encode(
            variables = items,
            name = "integer-no-good-${context.iteration}"
        )
        if (encoding.failed) {
            return propagate(encoding)
        }
        val value = encoding.value!!
        val constraints = value.constraints
        if (constraints.isEmpty()) {
            return Failed(
                ErrorCode.ApplicationError,
                "整数 no-good 编码未生成约束 / Integer no-good encoding produced no constraints"
            )
        }
        return ok(
            listOf(
                BendersMasterCut(
                    inequality = constraints.first(),
                    additionalInequalities = constraints.drop(1),
                    auxiliaryVariables = value.auxiliaryVariables,
                    kind = BendersCutKind.NoGood,
                    validity = BendersCutValidity.Global,
                    proofStatus = ProofStatus.Verified,
                    source = "integer-no-good"
                )
            )
        )
    }

    override fun optimalityCuts(
        result: FeasibleSubproblemResult,
        context: BendersCutContext
    ): Ret<List<BendersMasterCut>> {
        return ok(emptyList())
    }
}

/** Iteration-level Benders trace. / Benders 迭代轨迹。 */
data class BendersIterationTrace(
    val iteration: Int,
    val masterObjective: Flt64? = null,
    val masterBestBound: Flt64? = null,
    val masterStatus: SolverStatus? = null,
    val subproblemStatus: BendersSubproblemStatus,
    val subproblemObjective: Flt64? = null,
    val cutCount: Int = 0,
    val conflictCoreSize: Int = 0,
    val elapsed: Duration = ZERO,
    val proofMode: BendersProofMode,
    val convergenceGap: Flt64? = null
)

/**
 * Evaluates the complete Benders incumbent objective for an assignment. /
 * 计算给定赋值下完整 Benders incumbent 目标值。
 *
 * The returned value must use the same objective sense and constant convention as the master output. /
 * 返回值必须与主问题输出使用相同的优化方向和常数项口径。
 */
typealias BendersCompleteObjectiveEvaluator = (
    masterOutput: FeasibleSolverOutput<Flt64>,
    assignment: BendersSubproblemAssignment,
    subproblemOutput: ConstraintProgrammingFeasibleOutput
) -> Ret<Flt64>

/** CP subproblem trace status. / CP 子问题轨迹状态。 */
enum class BendersSubproblemStatus {
    Feasible,
    Infeasible,
    Unknown
}

/** Engine options. / 引擎选项。 */
data class LogicBasedBendersOptions(
    val proofMode: BendersProofMode = BendersProofMode.Exact,
    val maxIterations: Int = 100,
    val stallIterationLimit: Int = 1,
    /**
     * Absolute objective and master incumbent/bound gaps accepted by Exact mode. The master objective
     * must represent the complete Benders objective at the current assignment. / Exact 模式接受的
     * 子问题目标与主问题目标差值、以及主问题 incumbent/bound 绝对间隙；主问题目标必须表示当前赋值下的完整 Benders 目标。
     */
    val optimalityTolerance: Flt64 = Flt64.zero,
    /**
     * Evaluates first-stage cost plus the subproblem contribution. Required by Exact mode for an
     * optimizing CP subproblem because the master objective may contain theta and other terms. /
     * 计算第一阶段成本与子问题贡献之和。Exact 模式下带目标 CP 子问题必须提供该契约，
     * 因为主问题目标可能包含 theta 和其他项。
     */
    val completeObjectiveEvaluator: BendersCompleteObjectiveEvaluator? = null,
    val masterVariables: List<AbstractVariableItem<*, *>> = emptyList(),
    val masterSolutionSource: ((FeasibleSolverOutput<Flt64>) -> Ret<ConstraintProgrammingValueSource>)? = null,
    val constraintProgrammingOptions: ConstraintProgrammingSolveOptions = ConstraintProgrammingSolveOptions(),
    val cancellationToken: CancellationToken? = null,
    val progressReporter: ((BendersIterationTrace) -> Try)? = null
)

/** Final Logic-Based Benders report. / Logic-Based Benders 最终报告。 */
data class LogicBasedBendersReport(
    val problemStatus: ProblemStatus,
    val terminationReason: TerminationReason,
    val proof: SolveProof,
    val masterOutput: FeasibleSolverOutput<Flt64>? = null,
    val assignment: BendersSubproblemAssignment? = null,
    val subproblemResult: LogicBasedBendersSubproblemResult? = null,
    val cuts: List<BendersMasterCut> = emptyList(),
    val iterations: List<BendersIterationTrace> = emptyList(),
    val diagnostics: List<SolveIssue> = emptyList()
) {
    /** Compatibility alias. / 兼容别名。 */
    val status: ProblemStatus
        get() = problemStatus

    /** Convert to the core report contract. / 转换为 core 统一报告契约。 */
    fun toSolveReport(): SolveReport<Flt64> {
        val output = masterOutput
        return SolveReport(
            problemStatus = problemStatus,
            terminationReason = terminationReason,
            solutionPresence = when {
                output == null -> SolutionPresence.None
                proof.status == ProofStatus.Verified && problemStatus == ProblemStatus.Feasible -> SolutionPresence.Optimal
                else -> SolutionPresence.Incumbent
            },
            solution = output?.let {
                SolveSolution(
                    values = it.solution,
                    objective = it.objValueOrNull
                )
            },
            proof = proof,
            statistics = fuookami.ospf.kotlin.core.solver.report.SolveStatistics(
                solveTime = output?.solveTime,
                iterations = iterations.size.toULong()
            ),
            diagnostics = SolveDiagnostics(errors = diagnostics)
        )
    }
}

/** Master-solve adapter used by the engine. / 引擎使用的主问题求解适配器。 */
fun interface BendersMasterProblemSolver {
    /** Solve the current mutable master model. / 求解当前可变主问题模型。 */
    suspend fun solve(master: LinearMetaModel<Flt64>): Ret<FeasibleSolverOutput<Flt64>>
}

/** Logic-Based Benders engine. / Logic-Based Benders 迭代引擎。 */
class LogicBasedBendersEngine(
    private val masterSolver: BendersMasterProblemSolver,
    private val subproblemSolver: ConstraintProgrammingSolver,
    private val binding: BendersVariableBinding,
    private val cutOracle: BendersCutOracle = BinaryNoGoodCutOracle(),
    private val options: LogicBasedBendersOptions = LogicBasedBendersOptions()
) {
    /** Adapt the existing framework Benders solver contract. / 适配现有 framework Benders 求解器契约。 */
    constructor(
        masterSolver: LinearBendersDecompositionSolver,
        subproblemSolver: ConstraintProgrammingSolver,
        binding: BendersVariableBinding,
        cutOracle: BendersCutOracle = BinaryNoGoodCutOracle(),
        options: LogicBasedBendersOptions = LogicBasedBendersOptions()
    ) : this(
        masterSolver = BendersMasterProblemSolver { master ->
            when (val result = masterSolver.solveMaster(master, FrameworkSolveOptions())) {
                is Ok -> {
                    val output = result.value as? FeasibleSolverOutput<Flt64>
                    output?.let(::ok) ?: Failed(
                        ErrorCode.Other,
                        "主问题求解器未返回可行输出 / Master solver did not return a feasible output"
                    )
                }

                is Failed -> Failed(result.error)
                is Fatal -> Fatal(result.errors)
            }
        },
        subproblemSolver = subproblemSolver,
        binding = binding,
        cutOracle = cutOracle,
        options = options
    )

    /** Solve a linear master and static CP subproblem. / 求解线性主问题和静态 CP 子问题。 */
    suspend fun solve(
        master: LinearMetaModel<Flt64>,
        subproblem: ConstraintProgrammingModel
    ): Ret<LogicBasedBendersReport> {
        val optionError = validateOptions()
        if (optionError != null) {
            return Failed(ErrorCode.IllegalArgument, optionError)
        }
        val session = subproblemSolver.createSession(
            subproblem,
            options.constraintProgrammingOptions
        )
        if (session.failed) {
            return propagate(session)
        }
        return try {
            iterate(master, subproblem, session.value!!)
        } finally {
            session.value?.close()
        }
    }

    /** Solve and directly return the unified core report. / 求解并直接返回 core 统一报告。 */
    suspend fun solveReport(
        master: LinearMetaModel<Flt64>,
        subproblem: ConstraintProgrammingModel
    ): Ret<SolveReport<Flt64>> {
        return solve(master, subproblem).map { it.toSolveReport() }
    }

    private suspend fun iterate(
        master: LinearMetaModel<Flt64>,
        subproblem: ConstraintProgrammingModel,
        session: ConstraintProgrammingSession
    ): Ret<LogicBasedBendersReport> {
        val subproblemSnapshot = subproblem.snapshot()
        if (subproblemSnapshot.failed) {
            return propagate(subproblemSnapshot)
        }
        val hasSubproblemObjective = subproblemSnapshot.value!!.objectives.isNotEmpty()
        val started = TimeSource.Monotonic.markNow()
        val cuts = ArrayList<BendersMasterCut>()
        val traces = ArrayList<BendersIterationTrace>()
        val knownCuts = HashSet<String>()
        var lastMaster: FeasibleSolverOutput<Flt64>? = null
        var lastAssignment: BendersSubproblemAssignment? = null
        var lastSubproblem: LogicBasedBendersSubproblemResult? = null
        var stallIterations = 0

        for (iteration in 0 until options.maxIterations) {
            if (options.cancellationToken?.isCancellationRequested == true) {
                return ok(
                    report(
                        status = ProblemStatus.Unknown,
                        termination = TerminationReason.Cancelled,
                        proof = ProofStatus.None,
                        master = lastMaster,
                        assignment = lastAssignment,
                        subproblem = lastSubproblem,
                        cuts = cuts,
                        traces = traces,
                        diagnostics = listOf(cancelledIssue())
                    )
                )
            }
            val masterResult = try {
                masterSolver.solve(master)
            } catch (error: Throwable) {
                return Failed(
                    ErrorCode.OREngineSolvingException,
                    "Benders 主问题求解失败：${error.message ?: error::class.simpleName} / " +
                        "Benders master solve failed: ${error.message ?: error::class.simpleName}"
                )
            }
            when (masterResult) {
                is Failed -> {
                    return if (masterResult.error.code == ErrorCode.ORModelInfeasible) {
                        ok(
                            report(
                                status = ProblemStatus.Infeasible,
                                termination = TerminationReason.Completed,
                                proof = ProofStatus.Verified,
                                master = lastMaster,
                                assignment = lastAssignment,
                                subproblem = lastSubproblem,
                                cuts = cuts,
                                traces = traces
                            )
                        )
                    } else {
                        Failed(masterResult.error)
                    }
                }

                is Fatal -> return Fatal(masterResult.errors)
                is Ok -> {}
            }
            val masterOutput = (masterResult as Ok).value
            lastMaster = masterOutput
            if (options.proofMode == BendersProofMode.Exact && masterOutput.status != SolverStatus.Optimal) {
                return ok(
                    report(
                        status = ProblemStatus.Unknown,
                        termination = TerminationReason.BackendFailure,
                        proof = ProofStatus.None,
                        master = masterOutput,
                        assignment = lastAssignment,
                        subproblem = lastSubproblem,
                        cuts = cuts,
                        traces = traces,
                        diagnostics = listOf(
                            SolveIssue(
                                code = "benders-master-not-proven-optimal",
                                category = SolveIssueCategory.Backend,
                                message = "Exact 模式要求主问题最优证明 / Exact mode requires a proven master optimum"
                            )
                        )
                    )
                )
            }

            val source = options.masterSolutionSource?.invoke(masterOutput)
                ?: ok(masterSolutionValueSource(masterOutput, options.masterVariables))
            if (source.failed) {
                return propagate(source)
            }
            val assignmentResult = binding.bind(source.value!!, subproblem)
            if (assignmentResult.failed) {
                return propagate(assignmentResult)
            }
            val assignment = assignmentResult.value!!
            lastAssignment = assignment
            val subproblemOutput = try {
                session.solve(
                    assumptions = assignment.assumptions,
                    fixedValues = assignment.fixedValues
                )
            } catch (error: Throwable) {
                return Failed(
                    ErrorCode.OREngineSolvingException,
                    "Benders CP 子问题求解失败：${error.message ?: error::class.simpleName} / " +
                        "Benders CP subproblem solve failed: ${error.message ?: error::class.simpleName}"
                )
            }
            if (subproblemOutput.failed) {
                return propagate(subproblemOutput)
            }
            val result = classifySubproblem(subproblemOutput.value!!, assignment)
            lastSubproblem = result
            when (result) {
                is FeasibleSubproblemResult -> {
                    if (options.proofMode == BendersProofMode.Exact &&
                        (result.output.proofStatus != ProofStatus.Verified || result.output.status != SolverStatus.Optimal)
                    ) {
                        return ok(
                            report(
                                status = ProblemStatus.Unknown,
                                termination = TerminationReason.BackendFailure,
                                proof = ProofStatus.None,
                                master = masterOutput,
                                assignment = assignment,
                                subproblem = result,
                                cuts = cuts,
                                traces = traces,
                                diagnostics = listOf(
                                    SolveIssue(
                                        code = "benders-subproblem-not-proven",
                                        category = SolveIssueCategory.Backend,
                                        message = "Exact 模式要求 CP 子问题已证明最优或不可行 / " +
                                            "Exact mode requires the CP subproblem to be proven optimal or infeasible"
                                    )
                                )
                            )
                        )
                    }
                    if (options.proofMode == BendersProofMode.Exact &&
                        hasSubproblemObjective &&
                        result.output.objective == null
                    ) {
                        return ok(
                            report(
                                status = ProblemStatus.Unknown,
                                termination = TerminationReason.BackendFailure,
                                proof = ProofStatus.None,
                                master = masterOutput,
                                assignment = assignment,
                                subproblem = result,
                                cuts = cuts,
                                traces = traces,
                                diagnostics = listOf(
                                    SolveIssue(
                                        code = "benders-subproblem-objective-missing",
                                        category = SolveIssueCategory.Backend,
                                        message = "CP 模型声明了目标但后端未返回 objective / " +
                                            "The CP model declares an objective but the backend returned no objective"
                                    )
                                )
                            )
                        )
                    }
                    val context = BendersCutContext(master, subproblem, assignment, iteration, options.proofMode)
                    val optimalityCuts = cutOracle.optimalityCuts(result, context)
                    if (optimalityCuts.failed) {
                        return propagate(optimalityCuts)
                    }
                    val newCuts = validateCuts(optimalityCuts.value!!, knownCuts)
                    if (newCuts.failed) {
                        return propagate(newCuts)
                    }
                    val trace = trace(
                        iteration = iteration,
                        masterModel = master,
                        master = masterOutput,
                        subproblem = BendersSubproblemStatus.Feasible,
                        objective = result.output.objective,
                        cutCount = newCuts.value!!.size,
                        conflictSize = 0,
                        started = started
                    )
                    traces += trace
                    when (val notification = notify(trace)) {
                        is Ok -> {}
                        is Failed -> return Failed(notification.error)
                        is Fatal -> return Fatal(notification.errors)
                    }
                    if (newCuts.value!!.isEmpty()) {
                        val hasVerifiedOptimalityCut = cuts.any {
                            it.kind == BendersCutKind.Optimality &&
                                it.validity == BendersCutValidity.Global &&
                                it.proofStatus == ProofStatus.Verified
                        }
                        if (options.proofMode == BendersProofMode.Exact && hasSubproblemObjective) {
                            if (!hasVerifiedOptimalityCut) {
                                return ok(
                                    report(
                                        status = ProblemStatus.Feasible,
                                        termination = TerminationReason.IterationLimit,
                                        proof = ProofStatus.None,
                                        master = masterOutput,
                                        assignment = assignment,
                                        subproblem = result,
                                        cuts = cuts,
                                        traces = traces,
                                        diagnostics = listOf(
                                            SolveIssue(
                                                code = "benders-no-optimality-proof",
                                                category = SolveIssueCategory.Unsupported,
                                                message = "优化型 CP 子问题没有全局有效 optimality proof / " +
                                                    "The optimization CP subproblem has no globally valid optimality proof"
                                            )
                                        )
                                    )
                                )
                            }
                            val objectiveEvaluator = options.completeObjectiveEvaluator
                            if (objectiveEvaluator == null) {
                                return ok(
                                    report(
                                        status = ProblemStatus.Feasible,
                                        termination = TerminationReason.IterationLimit,
                                        proof = ProofStatus.None,
                                        master = masterOutput,
                                        assignment = assignment,
                                        subproblem = result,
                                        cuts = cuts,
                                        traces = traces,
                                        diagnostics = listOf(
                                            SolveIssue(
                                                code = "benders-objective-contract-missing",
                                                category = SolveIssueCategory.Unsupported,
                                                message = "Exact 模式缺少完整 Benders 目标评估契约 / " +
                                                    "Exact mode requires a complete Benders objective evaluator",
                                                details = mapOf(
                                                    "masterObjective" to masterOutput.obj.toString(),
                                                    "subproblemObjective" to (result.output.objective?.toString() ?: "missing")
                                                )
                                            )
                                        )
                                    )
                                )
                            }
                            val completeObjective = objectiveEvaluator(masterOutput, assignment, result.output)
                            if (completeObjective.failed) {
                                return propagate(completeObjective)
                            }
                            val expectedObjective = completeObjective.value!!
                            val convergenceGap = masterConvergenceGap(master, masterOutput)
                            val objectiveGap = (expectedObjective - masterOutput.obj).abs()
                            if (objectiveGap > options.optimalityTolerance) {
                                return ok(
                                    report(
                                        status = ProblemStatus.Feasible,
                                        termination = TerminationReason.IterationLimit,
                                        proof = ProofStatus.None,
                                        master = masterOutput,
                                        assignment = assignment,
                                        subproblem = result,
                                        cuts = cuts,
                                        traces = traces,
                                        diagnostics = listOf(
                                            SolveIssue(
                                                code = "benders-objective-inconsistent",
                                                category = SolveIssueCategory.Unsupported,
                                                message = "Exact 模式要求主问题目标与完整 Benders 目标一致 / " +
                                                    "Exact mode requires the master objective to match the complete Benders objective",
                                                details = mapOf(
                                                    "masterObjective" to masterOutput.obj.toString(),
                                                    "expectedCompleteObjective" to expectedObjective.toString(),
                                                    "subproblemObjective" to (result.output.objective?.toString() ?: "missing"),
                                                    "gap" to objectiveGap.toString(),
                                                    "tolerance" to options.optimalityTolerance.toString()
                                                )
                                            )
                                        )
                                    )
                                )
                            }
                            if (convergenceGap == null || convergenceGap > options.optimalityTolerance) {
                                return ok(
                                    report(
                                        status = ProblemStatus.Feasible,
                                        termination = TerminationReason.IterationLimit,
                                        proof = ProofStatus.None,
                                        master = masterOutput,
                                        assignment = assignment,
                                        subproblem = result,
                                        cuts = cuts,
                                        traces = traces,
                                        diagnostics = listOf(
                                            SolveIssue(
                                                code = "benders-no-convergence-proof",
                                                category = SolveIssueCategory.Unsupported,
                                                message = "Exact 模式缺少满足容差的主问题上下界收敛证明 / " +
                                                    "Exact mode lacks a master incumbent/bound convergence proof within tolerance",
                                                details = mapOf(
                                                    "gap" to (convergenceGap?.toString() ?: "missing"),
                                                    "tolerance" to options.optimalityTolerance.toString()
                                                )
                                            )
                                        )
                                    )
                                )
                            }
                        }
                        val proof = if (options.proofMode == BendersProofMode.Exact) {
                            ProofStatus.Verified
                        } else {
                            ProofStatus.Claimed
                        }
                        return ok(
                            report(
                                status = ProblemStatus.Feasible,
                                termination = TerminationReason.Completed,
                                proof = proof,
                                master = masterOutput,
                                assignment = assignment,
                                subproblem = result,
                                cuts = cuts,
                                traces = traces
                            )
                        )
                    }
                    val added = addCuts(master, newCuts.value!!, cuts, knownCuts)
                    if (added.failed) {
                        return propagate(added)
                    }
                    stallIterations = 0
                }

                is InfeasibleSubproblemResult -> {
                    if (options.proofMode == BendersProofMode.Exact &&
                        result.proofStatus != ProofStatus.Verified
                    ) {
                        return ok(
                            report(
                                status = ProblemStatus.Unknown,
                                termination = TerminationReason.BackendFailure,
                                proof = ProofStatus.None,
                                master = masterOutput,
                                assignment = assignment,
                                subproblem = result,
                                cuts = cuts,
                                traces = traces,
                                diagnostics = listOf(
                                    SolveIssue(
                                        code = "benders-subproblem-not-proven-infeasible",
                                        category = SolveIssueCategory.Backend,
                                        message = "Exact 模式要求 CP 子问题不可行证明 / " +
                                            "Exact mode requires a proven CP subproblem infeasibility certificate"
                                    )
                                )
                            )
                        )
                    }
                    val context = BendersCutContext(master, subproblem, assignment, iteration, options.proofMode)
                    val generated = cutOracle.feasibilityCuts(result, context)
                    if (generated.failed) {
                        return propagate(generated)
                    }
                    val newCuts = validateCuts(generated.value!!, knownCuts)
                    if (newCuts.failed) {
                        return propagate(newCuts)
                    }
                    if (newCuts.value!!.isEmpty()) {
                        return ok(
                            report(
                                status = ProblemStatus.Unknown,
                                termination = TerminationReason.IterationLimit,
                                proof = ProofStatus.None,
                                master = masterOutput,
                                assignment = assignment,
                                subproblem = result,
                                cuts = cuts,
                                traces = traces,
                                diagnostics = listOf(
                                    SolveIssue(
                                        code = "benders-no-feasibility-cut",
                                        category = SolveIssueCategory.Unsupported,
                                        message = "不可行子问题没有可用的全局有效 cut / No globally valid cut is available for the infeasible subproblem"
                                    )
                                )
                            )
                        )
                    }
                    val before = cuts.size
                    val addedResult = addCuts(master, newCuts.value!!, cuts, knownCuts)
                    if (addedResult.failed) {
                        return propagate(addedResult)
                    }
                    val added = cuts.size - before
                    val trace = trace(
                        iteration = iteration,
                        masterModel = master,
                        master = masterOutput,
                        subproblem = BendersSubproblemStatus.Infeasible,
                        objective = null,
                        cutCount = added,
                        conflictSize = result.conflict?.members?.size ?: result.conflict?.constraintIds?.size ?: 0,
                        started = started
                    )
                    traces += trace
                    when (val notification = notify(trace)) {
                        is Ok -> {}
                        is Failed -> return Failed(notification.error)
                        is Fatal -> return Fatal(notification.errors)
                    }
                    if (added == 0) {
                        ++stallIterations
                        if (stallIterations >= options.stallIterationLimit) {
                            return ok(
                                report(
                                    status = ProblemStatus.Unknown,
                                    termination = TerminationReason.IterationLimit,
                                    proof = ProofStatus.None,
                                    master = masterOutput,
                                    assignment = assignment,
                                    subproblem = result,
                                    cuts = cuts,
                                    traces = traces,
                                    diagnostics = listOf(
                                        SolveIssue(
                                            code = "benders-cut-stalled",
                                            category = SolveIssueCategory.Backend,
                                            message = "Benders 割添加后未产生新约束 / Benders cut generation stalled without a new constraint"
                                        )
                                    )
                                )
                            )
                        }
                    } else {
                        stallIterations = 0
                    }
                }

                is UnknownSubproblemResult -> {
                    val trace = trace(
                        iteration = iteration,
                        masterModel = master,
                        master = masterOutput,
                        subproblem = BendersSubproblemStatus.Unknown,
                        objective = null,
                        cutCount = 0,
                        conflictSize = 0,
                        started = started
                    )
                    traces += trace
                    when (val notification = notify(trace)) {
                        is Ok -> {}
                        is Failed -> return Failed(notification.error)
                        is Fatal -> return Fatal(notification.errors)
                    }
                    return ok(
                        report(
                            status = ProblemStatus.Unknown,
                            termination = result.terminationReason,
                            proof = ProofStatus.None,
                            master = masterOutput,
                            assignment = assignment,
                            subproblem = result,
                            cuts = cuts,
                            traces = traces,
                            diagnostics = listOf(
                                SolveIssue(
                                    code = "benders-subproblem-unknown",
                                    category = SolveIssueCategory.Backend,
                                    message = "CP 子问题未得到证明终态 / CP subproblem did not reach a proven terminal state"
                                )
                            )
                        )
                    )
                }
            }
        }
        return ok(
            report(
                status = ProblemStatus.Unknown,
                termination = TerminationReason.IterationLimit,
                proof = ProofStatus.None,
                master = lastMaster,
                assignment = lastAssignment,
                subproblem = lastSubproblem,
                cuts = cuts,
                traces = traces,
                diagnostics = listOf(
                    SolveIssue(
                        code = "benders-iteration-limit",
                        category = SolveIssueCategory.Backend,
                        message = "Benders 达到迭代次数上限 / Benders reached the iteration limit"
                    )
                )
            )
        )
    }

    private fun classifySubproblem(
        output: ConstraintProgrammingSolverOutput,
        assignment: BendersSubproblemAssignment
    ): LogicBasedBendersSubproblemResult {
        return when (output) {
            is ConstraintProgrammingFeasibleOutput -> FeasibleSubproblemResult(assignment, output)
            is ConstraintProgrammingInfeasibleOutput ->
                InfeasibleSubproblemResult(assignment, output.conflict, output.proofStatus)
            is ConstraintProgrammingUnknownOutput -> UnknownSubproblemResult(assignment, output.terminationReason)
        }
    }

    private fun validateCuts(
        cuts: List<BendersMasterCut>,
        knownCuts: Set<String>
    ): Ret<List<BendersMasterCut>> {
        val accepted = ArrayList<BendersMasterCut>()
        for (cut in cuts) {
            if (cut.source.isBlank()) {
                return Failed(ErrorCode.IllegalArgument, "Benders cut source 不能为空 / Benders cut source must not be blank")
            }
            if (cut.inequality.comparison == Comparison.NE ||
                cut.inequality.comparison == Comparison.LT ||
                cut.inequality.comparison == Comparison.GT
            ) {
                return Failed(
                    ErrorCode.Other,
                    "Benders 主问题首版不支持严格或不等关系 cut / Strict or not-equal Benders cuts are unsupported"
                )
            }
            if (options.proofMode == BendersProofMode.Exact &&
                (cut.validity != BendersCutValidity.Global || cut.proofStatus != ProofStatus.Verified)
            ) {
                return Failed(
                    ErrorCode.IllegalArgument,
                    "Exact 模式只能接受已验证的全局有效 cut / Exact mode only accepts verified globally valid cuts"
                )
            }
            if (cut.key !in knownCuts && accepted.none { it.key == cut.key }) {
                accepted += cut
            }
        }
        return ok(accepted)
    }

    private fun addCuts(
        master: LinearMetaModel<Flt64>,
        newCuts: List<BendersMasterCut>,
        allCuts: MutableList<BendersMasterCut>,
        knownCuts: MutableSet<String>
    ): Ret<Int> {
        var added = 0
        newCuts.forEachIndexed { index, cut ->
            val name = cut.name ?: "benders-${allCuts.size + index}-${cut.kind.name.lowercase()}"
            for (auxiliary in cut.auxiliaryVariables) {
                val result = master.add(auxiliary)
                if (result.failed) {
                    return propagate(result)
                }
            }
            val inequalities = listOf(cut.inequality) + cut.additionalInequalities
            for ((constraintIndex, inequality) in inequalities.withIndex()) {
                val constraintName = if (inequalities.size == 1) name else "$name-$constraintIndex"
                val result = master.addConstraint(
                    relation = inequality,
                    name = constraintName,
                    displayName = constraintName
                )
                if (result.failed) {
                    return propagate(result)
                }
            }
            knownCuts += cut.key
            allCuts += cut.copy(name = name)
            ++added
        }
        return ok(added)
    }

    private fun masterConvergenceGap(
        master: LinearMetaModel<Flt64>,
        output: FeasibleSolverOutput<Flt64>
    ): Flt64? {
        val bestBound = output.bestBound ?: return null
        val signedGap = when (master.objectCategory) {
            fuookami.ospf.kotlin.core.model.basic.ObjectCategory.Minimum -> output.obj - bestBound
            fuookami.ospf.kotlin.core.model.basic.ObjectCategory.Maximum -> bestBound - output.obj
        }
        return if (signedGap < Flt64.zero) null else signedGap.abs()
    }

    private fun trace(
        iteration: Int,
        masterModel: LinearMetaModel<Flt64>,
        master: FeasibleSolverOutput<Flt64>,
        subproblem: BendersSubproblemStatus,
        objective: Flt64?,
        cutCount: Int,
        conflictSize: Int,
        started: TimeSource.Monotonic.ValueTimeMark
    ): BendersIterationTrace {
        val convergenceGap = masterConvergenceGap(masterModel, master)
        return BendersIterationTrace(
            iteration = iteration,
            masterObjective = master.obj,
            masterBestBound = master.bestBound,
            masterStatus = master.status,
            subproblemStatus = subproblem,
            subproblemObjective = objective,
            cutCount = cutCount,
            conflictCoreSize = conflictSize,
            elapsed = started.elapsedNow(),
            proofMode = options.proofMode,
            convergenceGap = convergenceGap
        )
    }

    private fun notify(trace: BendersIterationTrace): Try {
        return when (val result = options.progressReporter?.invoke(trace)) {
            null -> ok
            is Ok -> ok
            is Failed -> Failed(result.error)
            is Fatal -> Fatal(result.errors)
        }
    }

    private fun report(
        status: ProblemStatus,
        termination: TerminationReason,
        proof: ProofStatus,
        master: FeasibleSolverOutput<Flt64>?,
        assignment: BendersSubproblemAssignment?,
        subproblem: LogicBasedBendersSubproblemResult?,
        cuts: List<BendersMasterCut>,
        traces: List<BendersIterationTrace>,
        diagnostics: List<SolveIssue> = emptyList()
    ): LogicBasedBendersReport {
        return LogicBasedBendersReport(
            problemStatus = status,
            terminationReason = termination,
            proof = SolveProof(
                status = proof,
                kind = "logic-based-benders"
            ),
            masterOutput = master,
            assignment = assignment,
            subproblemResult = subproblem,
            cuts = cuts.toList(),
            iterations = traces.toList(),
            diagnostics = diagnostics
        )
    }

    private fun validateOptions(): String? {
        if (options.maxIterations <= 0) {
            return "Benders maxIterations 必须为正 / Benders maxIterations must be positive"
        }
        if (options.stallIterationLimit <= 0) {
            return "Benders stallIterationLimit 必须为正 / Benders stallIterationLimit must be positive"
        }
        if (options.optimalityTolerance < Flt64.zero) {
            return "Benders optimalityTolerance 不得为负 / Benders optimalityTolerance must not be negative"
        }
        return null
    }

    private fun cancelledIssue(): SolveIssue {
        return SolveIssue(
            code = "benders-cancelled",
            category = SolveIssueCategory.Backend,
            message = "Benders 求解已取消 / Benders solve was cancelled"
        )
    }
}

/** CP registration pipeline contract. / CP 注册管线契约。 */
interface ConstraintProgrammingPipeline {
    /** Stable pipeline name. / 稳定管线名称。 */
    val name: String

    /** Register constraints and variables. / 注册变量和约束。 */
    fun register(model: ConstraintProgrammingModel): Try

    /** Execute pipeline registration. / 执行管线注册。 */
    operator fun invoke(model: ConstraintProgrammingModel): Try {
        return register(model)
    }
}

/** Apply a list of CP pipelines. / 执行 CP 管线列表。 */
fun List<ConstraintProgrammingPipeline>.registerConstraintProgramming(
    model: ConstraintProgrammingModel
): Try {
    for (pipeline in this) {
        when (val result = pipeline.register(model)) {
            is Ok -> {}
            is Failed -> return Failed(result.error)
            is Fatal -> return Fatal(result.errors)
        }
    }
    return ok
}

/** Benders subproblem pipeline contract. / Benders 子问题管线契约。 */
interface BendersSubproblemPipeline {
    /** Register a CP subproblem with the binding context. / 使用绑定上下文注册 CP 子问题。 */
    fun register(
        model: ConstraintProgrammingModel,
        binding: BendersVariableBinding
    ): Try

    /** Extract a typed solution into domain state. / 将类型化解提取回领域状态。 */
    fun extractSolution(
        solution: fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingSolution
    ): Try {
        return ok
    }
}

private fun AbstractVariableItem<*, *>.bendersStableKey(): String {
    return "${identifier}:${index}"
}

private fun <T> propagate(result: Ret<*>): Ret<T> {
    return when (result) {
        is Failed -> Failed(result.error)
        is Fatal -> Fatal(result.errors)
        else -> Failed(ErrorCode.ApplicationError, "Benders 结果状态无效 / Invalid Benders result state")
    }
}
