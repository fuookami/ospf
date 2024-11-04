package fuookami.ospf.kotlin.core.backend.plugins.cplex

import kotlin.math.*
import kotlin.time.*
import kotlin.time.Duration.Companion.seconds
import kotlinx.coroutines.*
import ilog.concert.*
import ilog.cplex.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.math.ordinary.*
import fuookami.ospf.kotlin.utils.error.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.Solution
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.intermediate_model.*
import fuookami.ospf.kotlin.core.backend.solver.*
import fuookami.ospf.kotlin.core.backend.solver.config.*
import fuookami.ospf.kotlin.core.backend.solver.output.*

class CplexQuadraticSolver(
    override val config: SolverConfig = SolverConfig(),
    private val callBack: CplexSolverCallBack? = null
) : QuadraticSolver {
    override val name = "cplex"

    override suspend fun invoke(
        model: QuadraticTetradModelView,
        statusCallBack: SolvingStatusCallBack?
    ): Ret<SolverOutput> {
        val impl = CplexQuadraticSolverImpl(config, callBack, statusCallBack)
        return impl(model)
    }

    override suspend fun invoke(
        model: QuadraticTetradModelView,
        solutionAmount: UInt64,
        statusCallBack: SolvingStatusCallBack?
    ): Ret<Pair<SolverOutput, List<Solution>>> {
        return if (solutionAmount leq UInt64.one) {
            this(model).map { it to emptyList() }
        } else {
            val results = ArrayList<Solution>()
            val impl = CplexQuadraticSolverImpl(config, callBack.ifNull { CplexSolverCallBack() }.copy()
                .configuration { cplex, _, _ ->
                    if (solutionAmount gr UInt64.one) {
                        cplex.setParam(IloCplex.Param.MIP.Pool.Intensity, 4)
                        cplex.setParam(IloCplex.Param.MIP.Pool.AbsGap, 0.0)
                        cplex.setParam(IloCplex.Param.MIP.Pool.Capacity, solutionAmount.toInt())
                        cplex.setParam(IloCplex.Param.MIP.Pool.Replace, 2)
                        cplex.setParam(IloCplex.Param.MIP.Limits.Populate, solutionAmount.cub().toInt())
                    }
                    ok
                }.solving { cplex, _, _ ->
                    try {
                        cplex.populate()
                        ok
                    } catch (e: IloException) {
                        Failed(Err(ErrorCode.OREngineSolvingException, e.message))
                    }
                }
                .analyzingSolution { cplex, variables, _ ->
                    for (i in 0 until min(solutionAmount.toInt(), cplex.solnPoolNsolns)) {
                        val thisResults = variables.map { Flt64(cplex.getValue(it, i)) }
                        if (!results.any { it.toTypedArray() contentEquals thisResults.toTypedArray() }) {
                            results.add(thisResults)
                        }
                    }
                    ok
                }, statusCallBack
            )
            impl(model).map { Pair(it, results) }
        }
    }
}

private class CplexQuadraticSolverImpl(
    private val config: SolverConfig,
    private val callBack: CplexSolverCallBack? = null,
    private val statusCallBack: SolvingStatusCallBack? = null
): CplexSolver() {
    lateinit var cplexVars: List<IloNumVar>
    lateinit var cplexConstraint: List<IloRange>
    lateinit var output: SolverOutput

    private var bestObj: Flt64? = null
    private var bestBound: Flt64? = null
    private var bestTime: Duration = Duration.ZERO

    suspend operator fun invoke(model: QuadraticTetradModelView): Ret<SolverOutput> {
        val processes = arrayOf(
            { it.init(model.name) },
            { it.dump(model) },
            CplexQuadraticSolverImpl::configure,
            CplexQuadraticSolverImpl::solve,
            CplexQuadraticSolverImpl::analyzeStatus,
            CplexQuadraticSolverImpl::analyzeSolution
        )
        for (process in processes) {
            when (val result = process(this)) {
                is Failed -> {
                    return Failed(result.error)
                }

                else -> {}
            }
        }
        return Ok(output)
    }

    private suspend fun dump(model: QuadraticTetradModelView): Try {
        cplexVars = model.variables.map {
            cplex.numVar(it.lowerBound.toDouble(), it.upperBound.toDouble(), CplexVariable(it.type).toCplexVar())
        }.toList()

        if (cplex.isMIP && model.variables.any { it.initialResult != null }) {
            val initialSolution = model.variables.withIndex()
                .filter { it.value.initialResult != null }
                .map { Pair(cplexVars[it.index], it.value.initialResult!!.toDouble()) }
            cplex.addMIPStart(
                initialSolution.map { it.first }.toTypedArray(),
                initialSolution.map { it.second }.toDoubleArray()
            )
        }

        val constraints = coroutineScope {
            val promises = model.constraints.indices.map { i ->
                i to async(Dispatchers.Default) {
                    var lb = Flt64.negativeInfinity
                    var ub = Flt64.infinity
                    when (model.constraints.signs[i]) {
                        Sign.GreaterEqual -> {
                            lb = model.constraints.rhs[i]
                        }

                        Sign.LessEqual -> {
                            ub = model.constraints.rhs[i]
                        }

                        Sign.Equal -> {
                            lb = model.constraints.rhs[i]
                            ub = model.constraints.rhs[i]
                        }
                    }
                    val lhs = cplex.lqNumExpr()
                    for (cell in model.constraints.lhs[i]) {
                        if (cell.colIndex2 == null) {
                            lhs.addTerm(cell.coefficient.toDouble(), cplexVars[cell.colIndex1])
                        } else {
                            lhs.addTerm(cell.coefficient.toDouble(), cplexVars[cell.colIndex1], cplexVars[cell.colIndex2!!])
                        }
                    }
                    Triple(lb, lhs, ub)
                }
            }
            promises.map {
                val (lb, lhs, ub) = it.second.await()
                val cplexConstraint = cplex.range(lb.toDouble(), lhs, ub.toDouble(), model.constraints.names[it.first])
                cplex.add(cplexConstraint)
                cplexConstraint
            }
        }
        cplexConstraint = constraints

        val objective = cplex.lqNumExpr()
        for (cell in model.objective.obj) {
            if (cell.colIndex2 == null) {
                objective.addTerm(cell.coefficient.toDouble(), cplexVars[cell.colIndex1])
            } else {
                objective.addTerm(cell.coefficient.toDouble(), cplexVars[cell.colIndex1], cplexVars[cell.colIndex2!!])
            }
        }
        when (model.objective.category) {
            ObjectCategory.Minimum -> {
                cplex.add(cplex.minimize(objective))
            }

            ObjectCategory.Maximum -> {
                cplex.add(cplex.maximize(objective))
            }
        }

        when (val result = callBack?.execIfContain(Point.AfterModeling, cplex, cplexVars, cplexConstraint)) {
            is Failed -> {
                return Failed(result.error)
            }

            else -> {}
        }
        return ok
    }

    private suspend fun configure(): Try {
        cplex.setParam(IloCplex.DoubleParam.TiLim, config.time.toDouble(DurationUnit.SECONDS))
        cplex.setParam(IloCplex.DoubleParam.EpGap, config.gap.toDouble())
        cplex.setParam(IloCplex.IntParam.Threads, config.threadNum.toInt())
        cplex.setParam(IloCplex.IntParam.OptimalityTarget, IloCplex.OptimalityTarget.OptimalGlobal)

        if (config.notImprovementTime != null || callBack?.nativeCallback != null || statusCallBack != null) {
            cplex.use(object : IloCplex.MIPInfoCallback() {
                override fun main() {
                    callBack?.nativeCallback?.invoke(this)

                    val currentObj = Flt64(incumbentObjValue)
                    val currentBound = Flt64(bestObjValue)
                    val currentTime = cplexTime.seconds

                    if (config.notImprovementTime != null) {
                        if (bestObj == null || bestBound == null || currentObj neq bestObj!! || currentBound neq bestBound!!) {
                            bestObj = currentObj
                            bestBound = currentBound
                            bestTime = currentTime
                        } else if (currentTime - bestTime >= config.notImprovementTime!!) {
                            abort()
                        }
                    }

                    statusCallBack?.let {
                        when (it(SolvingStatus(
                            solver = "cplex",
                            obj = currentObj,
                            possibleBestObj = currentBound,
                            gap = (currentObj - currentBound + Flt64.decimalPrecision) / (currentObj + Flt64.decimalPrecision)
                        ))) {
                            is Ok -> {}

                            is Failed -> {
                                abort()
                            }
                        }
                    }
                }
            })
        }

        when (val result = callBack?.execIfContain(Point.Configuration, cplex, cplexVars, cplexConstraint)) {
            is Failed -> {
                return Failed(result.error)
            }

            else -> {}
        }
        return ok
    }

    private suspend fun solve(): Try {
        when (val result = callBack?.execIfContain(Point.Solving, cplex, cplexVars, cplexConstraint)) {
            is Failed -> {
                return Failed(result.error)
            }

            null -> {
                try {
                    cplex.solve()
                } catch (e: IloException) {
                    return Failed(Err(ErrorCode.OREngineSolvingException, e.message))
                }
                return ok
            }

            else -> {}
        }
        return ok
    }

    private suspend fun analyzeSolution(): Try {
        return if (status.succeeded) {
            output = SolverOutput(
                Flt64(cplex.objValue),
                cplexVars.map { Flt64(cplex.getValue(it)) },
                cplex.cplexTime.seconds,
                Flt64(cplex.bestObjValue),
                Flt64(
                    if (cplex.isMIP) {
                        cplex.mipRelativeGap
                    } else {
                        0.0
                    }
                )
            )

            when (val result = callBack?.execIfContain(Point.AnalyzingSolution, cplex, cplexVars, cplexConstraint)) {
                is Failed -> {
                    return Failed(result.error)
                }

                else -> {}
            }
            ok
        } else {
            Failed(Err(status.errCode!!))
        }
    }
}
