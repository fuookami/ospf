/**
 * SCIP CP solver adapter。 / SCIP CP solver adapter.
 */
@file:OptIn(kotlin.time.ExperimentalTime::class)
package fuookami.ospf.kotlin.core.solver.scip

import kotlin.math.abs
import kotlin.math.roundToLong
import kotlin.time.Duration.Companion.seconds
import kotlinx.coroutines.CancellationException
import jscip.Scip
import jscip.SCIP_Status
import fuookami.ospf.kotlin.core.model.constraint_programming.BooleanLiteral
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingModel
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingModelSnapshot
import fuookami.ospf.kotlin.core.model.constraint_programming.IntervalValue
import fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSession
import fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSolveOptions
import fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSolver
import fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSolution
import fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSolverOutput
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingConflict
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingConflictMinimality
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingFeasibleOutput
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingInfeasibleOutput
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingUnknownOutput
import fuookami.ospf.kotlin.core.solver.output.SolverStatus
import fuookami.ospf.kotlin.core.solver.progress.SolverProgressSnapshot
import fuookami.ospf.kotlin.core.solver.progress.SolverProgressContext
import fuookami.ospf.kotlin.core.solver.progress.SolverStages
import fuookami.ospf.kotlin.core.solver.progress.SolverSubStage
import fuookami.ospf.kotlin.core.solver.report.ConstraintId
import fuookami.ospf.kotlin.core.solver.report.EvidenceMinimality
import fuookami.ospf.kotlin.core.solver.report.EvidenceValidity
import fuookami.ospf.kotlin.core.solver.report.InfeasibilityEvidence
import fuookami.ospf.kotlin.core.solver.report.InfeasibilityEvidenceSource
import fuookami.ospf.kotlin.core.solver.report.InfeasibilityMember
import fuookami.ospf.kotlin.core.solver.report.ProblemStatus
import fuookami.ospf.kotlin.core.solver.report.ProofStatus
import fuookami.ospf.kotlin.core.solver.report.SolveDiagnostics
import fuookami.ospf.kotlin.core.solver.report.SolveProof
import fuookami.ospf.kotlin.core.solver.report.SolveReport
import fuookami.ospf.kotlin.core.solver.report.SolveSolution
import fuookami.ospf.kotlin.core.solver.report.SolutionPresence
import fuookami.ospf.kotlin.core.solver.report.SolveStatistics
import fuookami.ospf.kotlin.core.solver.report.SolverCapabilities
import fuookami.ospf.kotlin.core.solver.report.SolverDescriptor
import fuookami.ospf.kotlin.core.solver.report.SolverModelType
import fuookami.ospf.kotlin.core.solver.report.SolverProvenance
import fuookami.ospf.kotlin.core.solver.report.TerminationReason
import fuookami.ospf.kotlin.core.solver.report.VariableId
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.number.Int64
import fuookami.ospf.kotlin.utils.error.ErrorCode
import fuookami.ospf.kotlin.utils.functional.Failed
import fuookami.ospf.kotlin.utils.functional.Fatal
import fuookami.ospf.kotlin.utils.functional.Ret
import fuookami.ospf.kotlin.utils.functional.ok

/** SCIP CP 求解器。 / SCIP CP solver. */
class ScipConstraintProgrammingSolver(
    private val sparseDomainLimit: Int = 128,
    private val decompositionLimit: Int = 256
) : ConstraintProgrammingSolver {
    override val descriptor: SolverDescriptor = SolverDescriptor(
        solverId = "scip-cp",
        backendName = "SCIP",
        capabilities = SolverCapabilities(
            modelTypes = setOf(SolverModelType.CP),
            interrupt = true,
            constraintProgrammingFeatures = mapOf(
                fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingFeature.BooleanLogic to fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSupportLevel.Native,
                fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingFeature.Reification to fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSupportLevel.ExactLowering,
                fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingFeature.SparseDomain to fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSupportLevel.ExactLowering,
                fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingFeature.AllDifferent to fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSupportLevel.ExactLowering,
                fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingFeature.Element to fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSupportLevel.ExactLowering,
                fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingFeature.Table to fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSupportLevel.ExactLowering,
                fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingFeature.Interval to fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSupportLevel.ExactLowering,
                fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingFeature.NoOverlap to fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSupportLevel.ExactLowering,
                fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingFeature.Cumulative to fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSupportLevel.Native,
                fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingFeature.Assumption to fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSupportLevel.ExactLowering,
                fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingFeature.ConflictCore to fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSupportLevel.ExactLowering,
                fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingFeature.IncrementalSolve to fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSupportLevel.Unsupported
            )
        )
    )

    override suspend fun solve(
        model: ConstraintProgrammingModel,
        options: ConstraintProgrammingSolveOptions
    ): Ret<ConstraintProgrammingSolverOutput> {
        val session = createSession(model, options)
        if (session.failed) {
            return propagate(session)
        }
        return try {
            session.value!!.solve()
        } finally {
            session.value!!.close()
        }
    }

    override fun createSession(
        model: ConstraintProgrammingModel,
        options: ConstraintProgrammingSolveOptions
    ): Ret<ConstraintProgrammingSession> {
        if (sparseDomainLimit <= 0 || decompositionLimit <= 0) {
            return Failed(ErrorCode.IllegalArgument, "SCIP CP 编译规模上限必须为正 / SCIP CP compilation limits must be positive")
        }
        validateOptions(options)?.let {
            return Failed(ErrorCode.IllegalArgument, it)
        }
        val snapshot = model.snapshot()
        if (snapshot.failed) {
            return propagate(snapshot)
        }
        return ok(
            ScipConstraintProgrammingSession(
                model = model,
                snapshot = snapshot.value!!,
                options = options,
                descriptor = descriptor,
                sparseDomainLimit = sparseDomainLimit,
                decompositionLimit = decompositionLimit
            )
        )
    }

    private fun validateOptions(options: ConstraintProgrammingSolveOptions): String? {
        val timeLimit = options.timeLimit
        if (timeLimit != null && timeLimit.isNegative()) {
            return "SCIP CP timeLimit 不得为负 / SCIP CP timeLimit must not be negative"
        }
        if (options.nodeLimit != null && options.nodeLimit == fuookami.ospf.kotlin.math.algebra.number.UInt64.zero) {
            return "SCIP CP nodeLimit 必须为正 / SCIP CP nodeLimit must be positive"
        }
        if (options.solutionLimit != null && options.solutionLimit == fuookami.ospf.kotlin.math.algebra.number.UInt64.zero) {
            return "SCIP CP solutionLimit 必须为正 / SCIP CP solutionLimit must be positive"
        }
        if (options.conflictShrinkLimit != null &&
            options.conflictShrinkLimit == fuookami.ospf.kotlin.math.algebra.number.UInt64.zero
        ) {
            return "SCIP CP conflictShrinkLimit 必须为正 / SCIP CP conflictShrinkLimit must be positive"
        }
        if (options.conflictActivationIds?.any { it.isBlank() } == true) {
            return "SCIP CP conflictActivationIds 不得包含空 ID / SCIP CP conflictActivationIds must not contain blank IDs"
        }
        options.relativeObjectiveGap?.toDouble()?.let { gap ->
            if (!gap.isFinite() || gap < 0.0 || gap > 1.0) {
                return "SCIP CP relativeObjectiveGap 必须位于 [0, 1] / SCIP CP relativeObjectiveGap must be in [0, 1]"
            }
        }
        options.absoluteObjectiveGap?.toDouble()?.let { gap ->
            if (!gap.isFinite() || gap < 0.0) {
                return "SCIP CP absoluteObjectiveGap 必须为非负有限值 / SCIP CP absoluteObjectiveGap must be finite and non-negative"
            }
        }
        options.randomSeed?.let { seed ->
            if (seed < Int.MIN_VALUE || seed > Int.MAX_VALUE) {
                return "SCIP CP randomSeed 超出 JSCIP 整数参数范围 / SCIP CP randomSeed exceeds the JSCIP integer parameter range"
            }
        }
        return null
    }
}

private class ScipConstraintProgrammingSession(
    override val model: ConstraintProgrammingModel,
    private val snapshot: ConstraintProgrammingModelSnapshot,
    override val options: ConstraintProgrammingSolveOptions,
    private val descriptor: SolverDescriptor,
    private val sparseDomainLimit: Int,
    private val decompositionLimit: Int
) : ScipSolver(), ConstraintProgrammingSession {

    private var closed = false
    private var nativeInitialized = false
    private var compiled: ScipConstraintProgrammingCompiledModel? = null

    override val isClosed: Boolean
        get() = closed

    override suspend fun solve(
        assumptions: List<BooleanLiteral>,
        fixedValues: Map<VariableId, Int64>,
        hints: ConstraintProgrammingSolution?
    ): Ret<ConstraintProgrammingSolverOutput> {
        if (closed) {
            return Failed(ErrorCode.ApplicationStopped, "SCIP CP session 已关闭 / SCIP CP session is closed")
        }
        if (options.cancellationToken?.isCancellationRequested == true) {
            return ok(ConstraintProgrammingUnknownOutput(TerminationReason.Cancelled))
        }
        val started = System.nanoTime()
        closeCurrentNativeModel()
        val initialized = initNative()
        if (initialized.failed) {
            return propagate(initialized)
        }
        val configured = configure()
        if (configured.failed) {
            closeCurrentNativeModel()
            return propagate(configured)
        }
        val buildProgress = reportProgress(
            stage = SolverStages.ModelBuilding,
            progressInStage = 0,
            overallProgress = 0,
            subStage = SolverSubStage(
                key = "compile",
                defaultTemplate = "编译 CP 模型",
                messageKey = "i18n.ospf.substage.cp_compile"
            )
        )
        if (buildProgress.failed) {
            closeCurrentNativeModel()
            return propagate(buildProgress)
        }
        val compiler = ScipConstraintProgrammingCompiler(
            scip = scip,
            snapshot = snapshot,
            sparseDomainLimit = sparseDomainLimit,
            decompositionLimit = decompositionLimit
        )
        val compiledResult = compiler.compile(
            assumptions = assumptions,
            fixedValues = fixedValues,
            diagnosticMode = options.collectConflict || options.conflictActivationIds != null,
            activeActivationIds = options.conflictActivationIds
        )
        if (compiledResult.failed) {
            compiler.cleanup()
            closeCurrentNativeModel()
            return propagate(compiledResult)
        }
        compiled = compiledResult.value
        val compiledProgress = reportProgress(
            stage = SolverStages.ModelBuilding,
            progressInStage = 100,
            overallProgress = 30,
            subStage = SolverSubStage(
                key = "compiled",
                defaultTemplate = "CP 模型编译完成",
                messageKey = "i18n.ospf.substage.cp_compiled"
            )
        )
        if (compiledProgress.failed) {
            closeCurrentNativeModel()
            return propagate(compiledProgress)
        }
        val hintResult = applyHint(hints)
        if (hintResult.failed) {
            closeCurrentNativeModel()
            return propagate(hintResult)
        }
        val cancellationWatcher = startCancellationWatcher()
        return try {
            val solvingProgress = reportProgress(
                stage = SolverStages.MILP,
                progressInStage = 0,
                overallProgress = 30,
                subStage = SolverSubStage(
                    key = "search",
                    defaultTemplate = "SCIP 搜索中",
                    messageKey = "i18n.ospf.substage.cp_search"
                )
            )
            if (solvingProgress.failed) {
                propagate(solvingProgress)
            } else if (options.cancellationToken?.isCancellationRequested == true) {
                unknown(started, TerminationReason.Cancelled)
            } else {
                scip.solve()
                val extracted = extract(started, assumptions)
                if (extracted.failed) {
                    propagate(extracted)
                } else {
                    val result = extracted.value!!
                    val completedProgress = reportProgress(
                        stage = SolverStages.PostProcessing,
                        progressInStage = 100,
                        overallProgress = 100,
                        subStage = SolverSubStage(
                            key = "completed",
                            defaultTemplate = "SCIP 求解完成",
                            messageKey = "i18n.ospf.substage.cp_completed"
                        )
                    )
                    if (completedProgress.failed) {
                        propagate(completedProgress)
                    } else if (result is ConstraintProgrammingInfeasibleOutput &&
                        options.shrinkConflict &&
                        (assumptions.isNotEmpty() || result.conflict?.activationIds?.isNotEmpty() == true)
                    ) {
                        closeCurrentNativeModel()
                        shrinkConflict(result, assumptions)
                    } else {
                        ok(result)
                    }
                }
            }
        } catch (_: CancellationException) {
            unknown(started, TerminationReason.Cancelled)
        } catch (error: Throwable) {
            Failed(
                ErrorCode.OREngineSolvingException,
                "SCIP CP 求解失败：${error.message ?: error::class.simpleName} / " +
                    "SCIP CP solve failed: ${error.message ?: error::class.simpleName}"
            )
        } finally {
            stopCancellationWatcher(cancellationWatcher)
            closeCurrentNativeModel()
        }
    }

    override fun close() {
        if (closed) {
            return
        }
        closed = true
        closeCurrentNativeModel()
        if (nativeInitialized) {
            super.close()
            nativeInitialized = false
        }
    }

    private suspend fun initNative(): Ret<Unit> {
        if (nativeInitialized) {
            return ok(Unit)
        }
        val explicit = System.getProperty("ospf.scip.library")
        if (!ScipSolver.loadedLibrary) {
            try {
                if (!explicit.isNullOrBlank()) {
                    System.load(explicit)
                } else {
                    System.loadLibrary("jscip")
                }
                ScipSolver.loadedLibrary = true
            } catch (error: Throwable) {
                return Failed(
                    ErrorCode.SolverNotFound,
                    "无法加载 JSCIP 原生库：${error.message ?: error::class.simpleName} / " +
                        "Unable to load the JSCIP native library: ${error.message ?: error::class.simpleName}"
                )
            }
        }
        val result = init(snapshot.name)
        if (result.failed) {
            return propagate(result)
        }
        nativeInitialized = true
        return ok(Unit)
    }

    private fun configure(): Ret<Unit> {
        return try {
            scip.hideOutput(!options.logEnabled)
            options.timeLimit?.let {
                scip.setRealParam(
                    "limits/time",
                    it.inWholeNanoseconds.toDouble() / 1_000_000_000.0
                )
            }
            options.nodeLimit?.let { scip.setLongintParam("limits/nodes", it.toLong()) }
            options.solutionLimit?.let { scip.setLongintParam("limits/solutions", it.toLong()) }
            options.randomSeed?.let { scip.setIntParam("randomization/randomseed", it.toInt()) }
            if (options.deterministic) {
                scip.setIntParam("parallel/maxnthreads", 1)
            }
            options.relativeObjectiveGap?.let { scip.setRealParam("limits/gap", it.toDouble()) }
            options.absoluteObjectiveGap?.let { scip.setRealParam("limits/absgap", it.toDouble()) }
            ok(Unit)
        } catch (error: Throwable) {
            Failed(
                ErrorCode.OREngineModelingException,
                "SCIP CP 参数配置失败：${error.message ?: error::class.simpleName} / " +
                    "SCIP CP parameter configuration failed: ${error.message ?: error::class.simpleName}"
            )
        }
    }

    private fun extract(
        started: Long,
        assumptions: List<BooleanLiteral>
    ): Ret<ConstraintProgrammingSolverOutput> {
        val status = scip.status
        val reason = ScipConstraintProgrammingStatusMapper.terminationReason(status)
        val solution = scip.bestSol
        if (status == SCIP_Status.SCIP_STATUS_INFEASIBLE) {
            val conflict = if (options.collectConflict) {
                val activationEntries = compiled?.activations.orEmpty()
                val members = activationEntries.values.mapTo(linkedSetOf()) { it.member }
                val constraintIds = members.mapNotNullTo(linkedSetOf()) {
                    (it as? InfeasibilityMember.Constraint)?.id
                }
                ConstraintProgrammingConflict(
                    constraintIds = if (constraintIds.isEmpty() && assumptions.isEmpty()) {
                        snapshot.constraints.mapTo(linkedSetOf()) { it.id }
                    } else constraintIds,
                    variableIds = assumptions.mapNotNullTo(linkedSetOf()) { it.variableId },
                    assumptions = assumptions,
                    members = members,
                    activationIds = activationEntries.keys,
                    activationMembers = activationEntries.mapValues { it.value.member },
                    validity = EvidenceValidity.Verified,
                    message = if (assumptions.isEmpty()) {
                        "SCIP returned a proven infeasible model / SCIP 已证明模型不可行"
                    } else {
                        "All active assumptions are a proven conflict seed / 所有活动 assumption 构成已证明冲突种子"
                    }
                )
            } else {
                null
            }
            return ok(
                ConstraintProgrammingInfeasibleOutput(
                    conflict = conflict,
                    proofStatus = ProofStatus.Verified,
                    report = report(
                        started = started,
                        status = ProblemStatus.Infeasible,
                        reason = reason,
                        presence = SolutionPresence.None,
                        solution = null,
                        proof = ProofStatus.Verified,
                        diagnostics = conflict?.let(::conflictDiagnostics) ?: SolveDiagnostics()
                    )
                )
            )
        }
        if (solution == null) {
            val problemStatus = when (status) {
                SCIP_Status.SCIP_STATUS_UNBOUNDED -> ProblemStatus.Unbounded
                SCIP_Status.SCIP_STATUS_INFORUNBD -> ProblemStatus.InfeasibleOrUnbounded
                else -> ProblemStatus.Unknown
            }
            return ok(ConstraintProgrammingUnknownOutput(reason, report(started, problemStatus, reason, SolutionPresence.None, null, ProofStatus.None)))
        }
        val extracted = extractSolution(solution)
        if (extracted.failed) {
            return propagate(extracted)
        }
        val optimal = status == SCIP_Status.SCIP_STATUS_OPTIMAL
        val objective = if (snapshot.objectives.isEmpty()) null else Flt64(scip.getSolOrigObj(solution))
        val bound = if (snapshot.objectives.isEmpty()) null else Flt64(scip.dualbound)
        return ok(
            ConstraintProgrammingFeasibleOutput(
                solution = extracted.value!!,
                objective = objective,
                bestBound = bound,
                status = if (optimal) SolverStatus.Optimal else SolverStatus.Feasible,
                proofStatus = if (optimal) ProofStatus.Verified else ProofStatus.None,
                report = report(
                    started,
                    ProblemStatus.Feasible,
                    reason,
                    if (optimal) SolutionPresence.Optimal else SolutionPresence.Incumbent,
                    extracted.value,
                    if (optimal) ProofStatus.Verified else ProofStatus.None,
                    objective
                )
            )
        )
    }

    private fun extractSolution(solution: jscip.Solution): Ret<ConstraintProgrammingSolution> {
        val values = LinkedHashMap<VariableId, Int64>()
        val compiledModel = compiled
            ?: return Failed(ErrorCode.ApplicationError, "SCIP CP 编译模型缺失 / Compiled SCIP CP model is missing")
        for ((id, variable) in compiledModel.variables) {
            val raw = scip.getSolVal(solution, variable)
            val rounded = raw.roundToLong()
            if (abs(raw - rounded.toDouble()) > 1e-6) {
                return Failed(ErrorCode.ORSolutionInvalid, "SCIP CP 返回非整数解 / SCIP CP returned a non-integer value")
            }
            values[id] = Int64(rounded)
        }
        val intervals = LinkedHashMap<fuookami.ospf.kotlin.core.model.constraint_programming.IntervalId, IntervalValue>()
        for ((id, interval) in compiledModel.intervals) {
            val start = scip.getSolVal(solution, interval.start).roundToLong()
            val end = scip.getSolVal(solution, interval.end).roundToLong()
            intervals[id] = IntervalValue(Int64(start), Int64(interval.size.toLong()), Int64(end))
        }
        return ok(ConstraintProgrammingSolution(values = values, intervals = intervals))
    }

    private fun unknown(started: Long, reason: TerminationReason): Ret<ConstraintProgrammingSolverOutput> {
        return ok(ConstraintProgrammingUnknownOutput(reason, report(started, ProblemStatus.Unknown, reason, SolutionPresence.None, null, ProofStatus.None)))
    }

    private fun report(
        started: Long,
        status: ProblemStatus,
        reason: TerminationReason,
        presence: SolutionPresence,
        solution: ConstraintProgrammingSolution?,
        proof: ProofStatus,
        objective: Flt64? = null,
        diagnostics: SolveDiagnostics<Int64> = SolveDiagnostics()
    ): SolveReport<Int64> {
        return SolveReport(
            problemStatus = status,
            terminationReason = reason,
            solutionPresence = presence,
            solution = solution?.let {
                SolveSolution(
                    it.asList(snapshot.variables.map { variable -> variable.id }),
                    objective = objective?.takeIf { it == it.round() }?.toInt64()
                )
            },
            proof = SolveProof(status = proof, kind = "scip-cp"),
            statistics = SolveStatistics(solveTime = (scip.solvingTime).seconds),
            diagnostics = diagnostics,
            provenance = SolverProvenance(descriptor = descriptor, deterministic = options.deterministic)
        )
    }

    private fun conflictDiagnostics(conflict: ConstraintProgrammingConflict): SolveDiagnostics<Int64> {
        val minimality = when (conflict.minimality) {
            ConstraintProgrammingConflictMinimality.Irreducible -> EvidenceMinimality.Irreducible
            ConstraintProgrammingConflictMinimality.Partial -> EvidenceMinimality.Partial
            ConstraintProgrammingConflictMinimality.NotChecked -> EvidenceMinimality.NotChecked
        }
        val completeness = when (conflict.minimality) {
            ConstraintProgrammingConflictMinimality.Partial ->
                fuookami.ospf.kotlin.core.solver.report.EvidenceCompleteness.Partial

            ConstraintProgrammingConflictMinimality.Irreducible,
            ConstraintProgrammingConflictMinimality.NotChecked ->
                fuookami.ospf.kotlin.core.solver.report.EvidenceCompleteness.Complete
        }
        val members = linkedSetOf<InfeasibilityMember>().apply {
            addAll(conflict.members)
            conflict.constraintIds.forEach { add(InfeasibilityMember.Constraint(it)) }
        }
        val boundRefs = members.mapNotNullTo(linkedSetOf()) {
            (it as? InfeasibilityMember.VariableBound)?.ref
        }
        val domainRefs = members.mapNotNullTo(linkedSetOf()) {
            (it as? InfeasibilityMember.VariableDomain)?.ref
        }
        return SolveDiagnostics(
            infeasibilityEvidence = InfeasibilityEvidence(
                source = InfeasibilityEvidenceSource.ConstraintConflict,
                exactness = when {
                    conflict.validity == EvidenceValidity.Unknown ->
                        fuookami.ospf.kotlin.core.solver.report.EvidenceExactness.Unknown

                    conflict.minimality == ConstraintProgrammingConflictMinimality.Irreducible ->
                        fuookami.ospf.kotlin.core.solver.report.EvidenceExactness.Irreducible

                    else -> fuookami.ospf.kotlin.core.solver.report.EvidenceExactness.Exact
                },
                completeness = if (conflict.validity == EvidenceValidity.Unknown) {
                    fuookami.ospf.kotlin.core.solver.report.EvidenceCompleteness.Partial
                } else {
                    completeness
                },
                constraintIds = conflict.constraintIds,
                variableBoundIds = boundRefs.mapTo(linkedSetOf()) { it.variableId },
                variableBoundRefs = boundRefs,
                variableDomainRefs = domainRefs,
                validity = conflict.validity,
                minimality = minimality,
                members = members,
                assumptionIds = conflict.variableIds,
                verificationChecks = conflict.verificationChecks,
                terminationReason = conflict.terminationReason
            )
        )
    }

    private fun applyHint(hints: ConstraintProgrammingSolution?): Ret<Unit> {
        if (hints == null || (hints.values.isEmpty() && hints.intervals.isEmpty())) {
            return ok(Unit)
        }
        val compiledModel = compiled
            ?: return Failed(ErrorCode.ApplicationError, "SCIP CP 编译模型缺失 / Compiled SCIP CP model is missing")
        val definitions = snapshot.variables.associateBy { it.id }
        for ((id, value) in hints.values) {
            val definition = definitions[id]
                ?: return Failed(
                    ErrorCode.DataNotFound,
                    "SCIP CP hint 引用了未知变量：$id / SCIP CP hint references an unknown variable: $id"
                )
            if (!definition.domain.contains(value)) {
                return Failed(
                    ErrorCode.ORSolutionInvalid,
                    "SCIP CP hint 超出变量值域：$id=$value / SCIP CP hint is outside the variable domain: $id=$value"
                )
            }
            if (abs(value.toLong().toDouble()) > MAX_EXACT_DOUBLE_INTEGER) {
                return Failed(
                    ErrorCode.ORSolutionInvalid,
                    "SCIP CP hint 超出 double 精确整数范围 / SCIP CP hint exceeds SCIP's exact integer range"
                )
            }
        }
        for (id in hints.intervals.keys) {
            if (id !in compiledModel.intervals) {
                return Failed(
                    ErrorCode.DataNotFound,
                    "SCIP CP hint 引用了未知 interval：$id / SCIP CP hint references an unknown interval: $id"
                )
            }
        }
        return try {
            val solution = scip.createPartialSol()
            for ((id, value) in hints.values) {
                scip.setSolVal(solution, compiledModel.variables[id]!!, value.toLong().toDouble())
            }
            for ((id, intervalValue) in hints.intervals) {
                val interval = compiledModel.intervals[id]!!
                scip.setSolVal(solution, interval.start, intervalValue.start.toLong().toDouble())
                scip.setSolVal(solution, interval.end, intervalValue.end.toLong().toDouble())
            }
            if (!scip.addSolFree(solution)) {
                Failed(
                    ErrorCode.ORSolutionInvalid,
                    "SCIP 拒绝 CP hint / SCIP rejected the CP hint"
                )
            } else {
                ok(Unit)
            }
        } catch (error: Throwable) {
            Failed(
                ErrorCode.OREngineModelingException,
                "SCIP CP hint 注入失败：${error.message ?: error::class.simpleName} / " +
                    "SCIP CP hint injection failed: ${error.message ?: error::class.simpleName}"
            )
        }
    }

    private fun reportProgress(
        stage: fuookami.ospf.kotlin.core.solver.progress.SolverStage,
        progressInStage: Int,
        overallProgress: Int,
        subStage: SolverSubStage
    ): Ret<Unit> {
        val context: SolverProgressContext = options.progressContext ?: return ok(Unit)
        val result = context.report(
            SolverProgressSnapshot(
                stage = stage,
                subStage = subStage,
                progressInStage = progressInStage,
                overallProgress = overallProgress
            )
        )
        return if (result.failed) {
            propagate(result)
        } else {
            ok(Unit)
        }
    }

    private fun startCancellationWatcher(): Thread? {
        val token = options.cancellationToken ?: return null
        val watcher = Thread {
            try {
                while (!Thread.currentThread().isInterrupted && nativeInitialized) {
                    if (token.isCancellationRequested) {
                        try {
                            scip.interruptSolve()
                        } catch (_: Throwable) {
                            // Native interruption is best effort; SCIP status remains authoritative.
                        }
                        return@Thread
                    }
                    Thread.sleep(CANCELLATION_POLL_MILLIS)
                }
            } catch (_: InterruptedException) {
                // Normal watcher shutdown after solve completion.
            }
        }.apply {
            isDaemon = true
            name = "ospf-scip-cp-cancellation"
            start()
        }
        return watcher
    }

    private fun stopCancellationWatcher(watcher: Thread?) {
        watcher ?: return
        watcher.interrupt()
        try {
            watcher.join(CANCELLATION_JOIN_MILLIS)
        } catch (_: InterruptedException) {
            Thread.currentThread().interrupt()
        }
    }

    private suspend fun shrinkConflict(
        initial: ConstraintProgrammingInfeasibleOutput,
        assumptions: List<BooleanLiteral>
    ): Ret<ConstraintProgrammingSolverOutput> {
        val initialConflict = initial.conflict ?: return ok(initial)
        val initialActivationIds = initialConflict.activationIds.toList()
        val maximumChecks = options.conflictShrinkLimit?.toLong()
            ?.takeIf { it > 0L }
            ?: (initialActivationIds.size + assumptions.size).toLong()
        val active = initialActivationIds.toMutableList()
        val activeAssumptions = assumptions.toMutableList()
        var checks = 0L
        var partial = false
        var lastReason: TerminationReason? = null
        for (candidate in initialActivationIds) {
            if (checks >= maximumChecks) {
                partial = true
                break
            }
            val trial = active.filterNot { it == candidate }
            val result = checkInfeasible(trial, assumptions)
            ++checks
            when (result) {
                is Failed,
                is Fatal -> partial = true
                is fuookami.ospf.kotlin.utils.functional.Ok -> when (val value = result.value) {
                    is ConstraintProgrammingInfeasibleOutput -> {
                        if (value.proofStatus == ProofStatus.Verified) {
                            active.remove(candidate)
                            lastReason = value.report?.terminationReason
                        } else {
                            // A non-verified deletion result cannot justify shrinking the conflict.
                            // 未经证明的删除结果不能作为缩减 conflict 的依据。
                            partial = true
                            lastReason = value.report?.terminationReason ?: TerminationReason.BackendFailure
                        }
                    }
                    is ConstraintProgrammingUnknownOutput -> {
                        partial = true
                        lastReason = value.terminationReason
                    }
                    is ConstraintProgrammingFeasibleOutput -> {
                        lastReason = value.report?.terminationReason
                    }
                    else -> {}
                }
            }
            if (partial) {
                break
            }
        }
        if (!partial) {
            for (candidate in assumptions) {
                if (checks >= maximumChecks) {
                    partial = true
                    break
                }
                val trial = activeAssumptions.filterNot { it == candidate }
                val result = checkInfeasible(active, trial)
                ++checks
                when (result) {
                    is Failed,
                    is Fatal -> partial = true
                    is fuookami.ospf.kotlin.utils.functional.Ok -> when (val value = result.value) {
                        is ConstraintProgrammingInfeasibleOutput -> {
                            if (value.proofStatus == ProofStatus.Verified) {
                                activeAssumptions.remove(candidate)
                                lastReason = value.report?.terminationReason
                            } else {
                                // A non-verified deletion result cannot justify shrinking the conflict.
                                // 未经证明的删除结果不能作为缩减 conflict 的依据。
                                partial = true
                                lastReason = value.report?.terminationReason ?: TerminationReason.BackendFailure
                            }
                        }
                        is ConstraintProgrammingUnknownOutput -> {
                            partial = true
                            lastReason = value.terminationReason
                        }
                        is ConstraintProgrammingFeasibleOutput -> {
                            lastReason = value.report?.terminationReason
                        }
                        else -> {}
                    }
                }
                if (partial) {
                    break
                }
            }
        }
        val minimality = if (partial) {
            ConstraintProgrammingConflictMinimality.Partial
        } else {
            ConstraintProgrammingConflictMinimality.Irreducible
        }
        val conflict = initialConflict.copy(
            assumptions = activeAssumptions,
            variableIds = activeAssumptions.mapNotNullTo(linkedSetOf()) { it.variableId },
            activationIds = active.toSet(),
            members = active.mapNotNullTo(linkedSetOf()) { initialConflict.activationMembers[it] },
            constraintIds = active.mapNotNullTo(linkedSetOf()) {
                (initialConflict.activationMembers[it] as? InfeasibilityMember.Constraint)?.id
            },
            minimality = minimality,
            verificationChecks = Flt64(checks.toDouble()).toUInt64(),
            terminationReason = lastReason ?: TerminationReason.Completed
        )
        val finalVerification = checkInfeasible(active, activeAssumptions)
        ++checks
        val (verified, finalReason) = when (finalVerification) {
            is fuookami.ospf.kotlin.utils.functional.Ok -> when (val value = finalVerification.value) {
                is ConstraintProgrammingInfeasibleOutput -> {
                    (value.proofStatus == ProofStatus.Verified) to
                        (value.report?.terminationReason ?: TerminationReason.Completed)
                }

                is ConstraintProgrammingUnknownOutput -> false to value.terminationReason
                is ConstraintProgrammingFeasibleOutput -> false to
                    (value.report?.terminationReason ?: TerminationReason.Completed)

                else -> false to TerminationReason.BackendFailure
            }

            is Failed,
            is Fatal -> false to TerminationReason.BackendFailure
        }
        if (!verified) {
            partial = true
        }
        val finalConflict = conflict.copy(
            minimality = if (partial || !verified) {
                ConstraintProgrammingConflictMinimality.Partial
            } else {
                ConstraintProgrammingConflictMinimality.Irreducible
            },
            validity = if (verified) EvidenceValidity.Verified else EvidenceValidity.Unknown,
            verificationChecks = Flt64(checks.toDouble()).toUInt64(),
            terminationReason = finalReason
        )
        return ok(
            initial.copy(
                conflict = finalConflict,
                report = initial.report?.copy(diagnostics = conflictDiagnostics(finalConflict))
            )
        )
    }

    private suspend fun checkInfeasible(
        activeActivationIds: List<String>,
        assumptions: List<BooleanLiteral>
    ): Ret<ConstraintProgrammingSolverOutput> {
        val solver = ScipConstraintProgrammingSolver(
            sparseDomainLimit = sparseDomainLimit,
            decompositionLimit = decompositionLimit
        )
        val sessionResult = solver.createSession(
            model,
            options.copy(
                collectConflict = false,
                shrinkConflict = false,
                conflictShrinkLimit = null,
                conflictActivationIds = activeActivationIds.toSet(),
                progressContext = null
            )
        )
        if (sessionResult.failed) {
            return propagate(sessionResult)
        }
        val session = sessionResult.value!!
        return try {
            session.solve(assumptions = assumptions)
        } finally {
            session.close()
        }
    }

    private fun closeCurrentNativeModel() {
        // JSCIP 求解结束后不允许释放原始变量和约束；直接释放整个 problem，下一轮重新创建实例。
        // JSCIP does not permit releasing original variables and constraints after solve; free the whole problem and rebuild it next round.
        if (nativeInitialized) {
            try {
                super.close()
            } catch (_: Throwable) {
                // Native cleanup is best effort at the adapter boundary.
            }
            nativeInitialized = false
        }
        compiled = null
    }

    private companion object {
        const val CANCELLATION_POLL_MILLIS = 10L
        const val CANCELLATION_JOIN_MILLIS = 100L
        const val MAX_EXACT_DOUBLE_INTEGER = 9_007_199_254_740_991.0
    }
}

private fun <T> propagate(result: Ret<*>): Ret<T> {
    return when (result) {
        is Failed -> Failed(result.error)
        is Fatal -> Fatal(result.errors)
        else -> Failed(ErrorCode.ApplicationError, "SCIP CP 结果状态无效 / Invalid SCIP CP result state")
    }
}
