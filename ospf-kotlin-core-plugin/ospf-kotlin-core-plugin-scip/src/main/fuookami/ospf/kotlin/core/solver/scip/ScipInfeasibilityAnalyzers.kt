/** SCIP structured infeasibility analyzers. / SCIP 结构化不可行分析器。 */
@file:OptIn(kotlin.time.ExperimentalTime::class)

package fuookami.ospf.kotlin.core.solver.scip

import kotlin.math.abs
import kotlin.time.Clock
import fuookami.ospf.kotlin.core.model.basic.ConstraintRelation
import fuookami.ospf.kotlin.core.model.intermediate.LinearTriadModelView
import fuookami.ospf.kotlin.core.solver.config.SolverConfig
import fuookami.ospf.kotlin.core.solver.iis.FarkasInfeasibilityAnalyzer
import fuookami.ospf.kotlin.core.solver.iis.IISConfig
import fuookami.ospf.kotlin.core.solver.iis.InfeasibilityAnalyzerCapabilities
import fuookami.ospf.kotlin.core.solver.report.BoundSide
import fuookami.ospf.kotlin.core.solver.report.ConstraintId
import fuookami.ospf.kotlin.core.solver.report.diagnosticConstraintId
import fuookami.ospf.kotlin.core.solver.report.diagnosticVariableId
import fuookami.ospf.kotlin.core.solver.report.EvidenceCompleteness
import fuookami.ospf.kotlin.core.solver.report.EvidenceExactness
import fuookami.ospf.kotlin.core.solver.report.EvidenceMinimality
import fuookami.ospf.kotlin.core.solver.report.EvidenceValidity
import fuookami.ospf.kotlin.core.solver.report.InfeasibilityEvidence
import fuookami.ospf.kotlin.core.solver.report.InfeasibilityEvidenceSource
import fuookami.ospf.kotlin.core.solver.report.InfeasibilityMember
import fuookami.ospf.kotlin.core.solver.report.SolverModelType
import fuookami.ospf.kotlin.core.solver.report.VariableBoundRef
import fuookami.ospf.kotlin.core.solver.value.toSolverDouble
import fuookami.ospf.kotlin.utils.error.ErrorCode
import fuookami.ospf.kotlin.utils.functional.Failed
import fuookami.ospf.kotlin.utils.functional.Fatal
import fuookami.ospf.kotlin.utils.functional.Ok
import fuookami.ospf.kotlin.utils.functional.Ret
import fuookami.ospf.kotlin.utils.functional.Try
import fuookami.ospf.kotlin.utils.functional.ok
import jscip.SCIP_ParamSetting
import jscip.SCIP_Status

private const val FARKAS_TOLERANCE = 1.0e-9

/** Continuous LP Farkas analyzer backed by SCIP's LP dual certificate API. / 基于 SCIP LP 对偶证书 API 的连续 LP Farkas 分析器。 */
internal class ScipFarkasInfeasibilityAnalyzer(
    private val solverConfig: SolverConfig,
    private val iisConfig: IISConfig,
    private val callBack: ScipSolverCallBack?
) : FarkasInfeasibilityAnalyzer<LinearTriadModelView> {
    override val capabilities = InfeasibilityAnalyzerCapabilities(
        modelTypes = setOf(SolverModelType.LP),
        exact = true,
        source = InfeasibilityEvidenceSource.Farkas
    )

    override suspend fun analyze(model: LinearTriadModelView): Ret<InfeasibilityEvidence> {
        if (model.variables.any { it.type.isIntegerType }) {
            return Failed(
                ErrorCode.IllegalArgument,
                "SCIP Farkas 证书仅适用于连续线性模型 / SCIP Farkas certificates require a continuous linear model"
            )
        }
        return ScipLinearFarkasDiagnosticRun(solverConfig, iisConfig, callBack).run(model)
    }
}

/** One isolated SCIP LP used for extracting a Farkas certificate. / 用于提取 Farkas 证书的隔离 SCIP LP。 */
private class ScipLinearFarkasDiagnosticRun(
    private val solverConfig: SolverConfig,
    private val iisConfig: IISConfig,
    private val callBack: ScipSolverCallBack?
) : ScipSolver() {
    private lateinit var variables: List<jscip.Variable>
    private lateinit var constraints: List<jscip.Constraint>

    suspend fun run(model: LinearTriadModelView): Ret<InfeasibilityEvidence> {
        val started = Clock.System.now()
        var initialized = false
        return try {
            when (val result = init(model.name)) {
                is Failed -> return Failed(result.error)
                is Fatal -> return Fatal(result.errors)
                is Ok -> initialized = true
            }
            scip.hideOutput(true)
            dump(model).let { result ->
                when (result) {
                    is Failed -> return Failed(result.error)
                    is Fatal -> return Fatal(result.errors)
                    is Ok -> Unit
                }
            }
            when (val result = configure()) {
                is Failed -> return Failed(result.error)
                is Fatal -> return Fatal(result.errors)
                is Ok -> Unit
            }
            scip.solve()
            if (scip.status != SCIP_Status.SCIP_STATUS_INFEASIBLE) {
                return Failed(
                    ErrorCode.ORModelInfeasible,
                    "SCIP Farkas 要求模型状态为不可行 / SCIP Farkas requires an infeasible model"
                )
            }
            val rowIds = linkedSetOf<ConstraintId>()
            val members = linkedSetOf<InfeasibilityMember>()
            constraints.forEachIndexed { index, constraint ->
                val transformed = scip.getTransformedCons(constraint)
                if (transformed != null && abs(scip.getDualFarkasLinear(transformed)) > FARKAS_TOLERANCE) {
                    val id = model.diagnosticConstraintId(index)
                    rowIds += id
                    members += InfeasibilityMember.Constraint(id)
                }
            }
            if (rowIds.isEmpty()) {
                return Failed(
                    ErrorCode.OREngineSolvingException,
                    "SCIP 未返回非零 Farkas 行证书，可能不可行性来自变量界 / " +
                        "SCIP returned no nonzero Farkas row certificate; infeasibility may come from variable bounds"
                )
            }
            val bounds = finiteBounds(model)
            bounds.forEach { members += InfeasibilityMember.VariableBound(it) }
            ok(
                InfeasibilityEvidence(
                    source = InfeasibilityEvidenceSource.Farkas,
                    exactness = EvidenceExactness.Exact,
                    completeness = EvidenceCompleteness.Complete,
                    constraintIds = rowIds,
                    variableBoundIds = bounds.mapTo(linkedSetOf()) { it.variableId },
                    elapsed = Clock.System.now() - started,
                    validity = EvidenceValidity.Verified,
                    minimality = EvidenceMinimality.NotChecked,
                    variableBoundRefs = bounds,
                    members = members,
                    reference = "scip.getDualFarkasLinear"
                )
            )
        } catch (error: Throwable) {
            Failed(
                ErrorCode.OREngineSolvingException,
                "SCIP Farkas 诊断失败：${error.message ?: error::class.simpleName} / " +
                    "SCIP Farkas diagnosis failed: ${error.message ?: error::class.simpleName}"
            )
        } finally {
            if (initialized) {
                try {
                    close()
                } catch (_: Throwable) {
                    // Native cleanup is best effort at the adapter boundary.
                    // 原生资源清理属于 adapter 边界的尽力操作。
                }
            }
        }
    }

    private fun dump(model: LinearTriadModelView): Try {
        variables = model.variables.mapIndexed { index, variable ->
            scip.createVar(
                "diagnostic-var-$index",
                variable.lowerBound.toSolverDouble("variable[$index].lowerBound"),
                variable.upperBound.toSolverDouble("variable[$index].upperBound"),
                0.0,
                ScipVariable(variable.type).toSCIPVar()
            )
        }
        constraints = model.constraints.indices.map { row ->
            var lower = Double.NEGATIVE_INFINITY
            var upper = Double.POSITIVE_INFINITY
            when (model.constraints.signs[row]) {
                ConstraintRelation.GreaterEqual -> lower = model.constraints.rhs[row].toSolverDouble("constraint[$row].rhs")
                ConstraintRelation.LessEqual -> upper = model.constraints.rhs[row].toSolverDouble("constraint[$row].rhs")
                ConstraintRelation.Equal -> {
                    val rhs = model.constraints.rhs[row].toSolverDouble("constraint[$row].rhs")
                    lower = rhs
                    upper = rhs
                }
            }
            val rowVariables = ArrayList<jscip.Variable>()
            val coefficients = ArrayList<Double>()
            model.constraints.sparseLhs.forEachEntry(row) { column, coefficient ->
                rowVariables += variables[column]
                coefficients += coefficient.toSolverDouble("constraint[$row].coefficient[$column]")
            }
            scip.createConsLinear(
                "diagnostic-row-$row",
                rowVariables.toTypedArray(),
                coefficients.toDoubleArray(),
                lower,
                upper
            ).also(scip::addCons)
        }
        return ok
    }

    private suspend fun configure(): Try {
        scip.setRealParam("limits/time", iisConfig.time.toDouble(kotlin.time.DurationUnit.SECONDS))
        scip.setIntParam("parallel/maxnthreads", iisConfig.threadNum.toInt())
        scip.setPresolving(SCIP_ParamSetting.SCIP_PARAMSETTING_OFF, true)
        scip.setMinimize()
        when (val result = callBack?.execIfContain(
            point = Point.Configuration,
            status = null,
            scip = scip,
            variables = variables,
            constraints = constraints
        )) {
            is Failed -> return Failed(result.error)
            is Fatal -> return Fatal(result.errors)
            else -> Unit
        }
        return ok
    }

    private fun finiteBounds(model: LinearTriadModelView): Set<VariableBoundRef> {
        return model.variables.flatMapTo(linkedSetOf()) { variable ->
            buildList {
                if (variable.lowerBound != fuookami.ospf.kotlin.math.algebra.number.Flt64.negativeInfinity) {
                    add(VariableBoundRef(variable.diagnosticVariableId(), BoundSide.Lower))
                }
                if (variable.upperBound != fuookami.ospf.kotlin.math.algebra.number.Flt64.infinity) {
                    add(VariableBoundRef(variable.diagnosticVariableId(), BoundSide.Upper))
                }
            }
        }
    }
}
