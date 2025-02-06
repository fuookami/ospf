package fuookami.ospf.kotlin.core.backend.plugins.hexaly

import kotlin.time.*
import kotlinx.datetime.*
import kotlinx.coroutines.*
import com.hexaly.optimizer.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.error.*
import fuookami.ospf.kotlin.utils.concept.*
import fuookami.ospf.kotlin.utils.operator.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.Solution
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.intermediate_model.*
import fuookami.ospf.kotlin.core.backend.solver.*
import fuookami.ospf.kotlin.core.backend.solver.config.*
import fuookami.ospf.kotlin.core.backend.solver.output.*

class HexalyLinearSolver(
    override val config: SolverConfig = SolverConfig(),
    private val callBack: HexalySolverCallBack? = null
) : LinearSolver {
    override val name = "hexaly"

    override suspend operator fun invoke(
        model: LinearTriadModelView,
        statusCallBack: SolvingStatusCallBack?
    ): Ret<SolverOutput> {
        val impl = HexalyLinearSolverImpl(config, callBack, statusCallBack)
        val result = impl(model)
        System.gc()
        return result
    }

    override suspend fun invoke(
        model: LinearTriadModelView,
        solutionAmount: UInt64,
        statusCallBack: SolvingStatusCallBack?
    ): Ret<Pair<SolverOutput, List<Solution>>> {
        return if (solutionAmount leq UInt64.one) {
            this(model).map { it to emptyList() }
        } else {
            val results = ArrayList<Solution>()
            val impl = HexalyLinearSolverImpl(
                config = config,
                callBack = callBack
                    .copyIfNotNullOr { HexalySolverCallBack() }
                    .configuration { hexaly, _, _ ->
                        ok
                    }.analyzingSolution { hexaly, variables, _ ->
                        ok
                    },
                statusCallBack = statusCallBack
            )
            val result = impl(model).map { it to results }
            System.gc()
            return result
        }
    }
}

private class HexalyLinearSolverImpl(
    private val config: SolverConfig,
    private val callBack: HexalySolverCallBack? = null,
    private val statusCallBack: SolvingStatusCallBack? = null
) : HexalySolver() {
    private lateinit var hexalyVars: List<HxExpression>
    private lateinit var hexalyConstraints: List<HxExpression>
    private lateinit var hexalyObjective: HxExpression
    private lateinit var output: SolverOutput

    private var bestObj: Flt64? = null
    private var bestBound: Flt64? = null
    private var bestTime: Duration = Duration.ZERO

    suspend operator fun invoke(model: LinearTriadModelView): Ret<SolverOutput> {
        val processes = arrayOf(
            { it.init(model.name, callBack?.creatingEnvironmentFunction) },
            { it.dump(model) },
            HexalyLinearSolverImpl::configure,
            HexalyLinearSolverImpl::solve,
            HexalyLinearSolverImpl::analyzeStatus,
            HexalyLinearSolverImpl::analyzeSolution
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

    private suspend fun dump(model: LinearTriadModelView): Try {
        return try {
            hexalyVars = model.variables.map {
                HexalyVariable(hexalyModel, it.type, it.lowerBound, it.upperBound).toHexalyVariable()
            }

            for ((col, variable) in model.variables.withIndex()) {
                variable.initialResult?.let {
                    hexalyVars[col].setValue(it.toDouble())
                }
            }

            val constraints = coroutineScope {
                val factor = Flt64(model.constraints.size / Runtime.getRuntime().availableProcessors()).lg()!!.floor().toUInt64().toInt()
                val promises = if (factor >= 1) {
                    val segment = pow(UInt64.ten, factor).toInt()
                    (0..(model.constraints.size / segment)).map { i ->
                        async(Dispatchers.Default) {
                            ((i * segment) until minOf(model.constraints.size, (i + 1) * segment)).map { ii ->
                                val lhs = hexalyModel.sum()
                                for (cell in model.constraints.lhs[ii]) {
                                    lhs.addOperands(
                                        hexalyModel.prod(cell.coefficient.toDouble(), hexalyVars[cell.colIndex])
                                    )
                                }
                                ii to lhs
                            }
                        }
                    }
                } else {
                    model.constraints.indices.map { i ->
                        async(Dispatchers.Default) {
                            val lhs = hexalyModel.sum()
                            for (cell in model.constraints.lhs[i]) {
                                lhs.addOperands(
                                    hexalyModel.prod(cell.coefficient.toDouble(), hexalyVars[cell.colIndex])
                                )
                            }
                            listOf(i to lhs)
                        }
                    }
                }
                promises.flatMap { promise ->
                    val result = promise.await().map {
                        val constraint = when (model.constraints.signs[it.first]) {
                            Sign.LessEqual -> {
                                hexalyModel.leq(it.second, model.constraints.rhs[it.first].toDouble())
                            }

                            Sign.Equal -> {
                                hexalyModel.eq(it.second, model.constraints.rhs[it.first].toDouble())
                            }

                            Sign.GreaterEqual -> {
                                hexalyModel.geq(it.second, model.constraints.rhs[it.first].toDouble())
                            }
                        }
                        hexalyModel.constraint(constraint)
                        constraint
                    }
                    System.gc()
                    result
                }
            }
            hexalyConstraints = constraints

            val obj = hexalyModel.sum()
            for (cell in model.objective.obj) {
                obj.addOperands(hexalyModel.prod(cell.coefficient.toDouble(), hexalyVars[cell.colIndex]))
            }
            when (model.objective.category) {
                ObjectCategory.Maximum -> {
                    hexalyModel.maximize(obj)
                }

                ObjectCategory.Minimum -> {
                    hexalyModel.minimize(obj)
                }
            }
            hexalyObjective = obj

            when (val result = callBack?.execIfContain(Point.AfterModeling, optimizer, hexalyVars, hexalyConstraints)) {
                is Failed -> {
                    return Failed(result.error)
                }

                else -> {}
            }
            ok
        } catch (e: HxException) {
            Failed(Err(ErrorCode.OREngineModelingException, e.message))
        } catch (e: Exception) {
            Failed(Err(ErrorCode.OREngineModelingException))
        }
    }

    private suspend fun configure(): Try {
        return try {
            optimizer.param.timeLimit = config.time.toInt(DurationUnit.SECONDS)
            optimizer.param.nbThreads = config.threadNum.toInt()
            optimizer.param.setDoubleObjectiveThreshold(0, config.gap.toDouble())

            if (config.notImprovementTime != null || callBack?.nativeCallback != null || statusCallBack != null) {
                optimizer.addCallback(HxCallbackType.IterationTicked) { optimizer, callBackType ->
                    callBack?.nativeCallback?.invoke(optimizer, callBackType)

                    if (callBackType == HxCallbackType.IterationTicked) {
                        val currentSolution = optimizer.solution
                        val currentObj = Flt64(currentSolution.getDoubleValue(hexalyObjective))
                        val currentBound = Flt64(currentSolution.getDoubleObjectiveBound(0))
                        val currentTime = Clock.System.now() - beginTime!!

                        if (config.notImprovementTime != null) {
                            if (bestObj == null || bestBound == null || currentObj neq bestObj!! || currentBound neq bestBound!!) {
                                bestObj = currentObj
                                bestBound = currentBound
                                bestTime = currentTime
                            } else if (currentTime - bestTime >= config.notImprovementTime!!) {
                                optimizer.stop()
                            }
                        }

                        statusCallBack?.let {
                            when (it(
                                SolvingStatus(
                                    solver = "hexaly",
                                    obj = currentObj,
                                    possibleBestObj = currentBound,
                                    gap = (currentObj - currentBound + Flt64.decimalPrecision) / (currentObj + Flt64.decimalPrecision)
                                )
                            )) {
                                is Ok -> {}

                                is Failed -> {
                                    optimizer.stop()
                                }
                            }
                        }
                    }
                }
            }

            when (val result = callBack?.execIfContain(Point.Configuration, optimizer, hexalyVars, hexalyConstraints)) {
                is Failed -> {
                    return Failed(result.error)
                }

                else -> {}
            }
            ok
        } catch (e: HxException) {
            Failed(Err(ErrorCode.OREngineModelingException, e.message))
        } catch (e: Exception) {
            Failed(Err(ErrorCode.OREngineModelingException))
        }
    }

    private suspend fun analyzeSolution(): Try {
        return try {
            if (status.succeeded) {
                val results = ArrayList<Flt64>()
                for (hexalyVar in hexalyVars) {
                    results.add(Flt64(hexalyVar.doubleValue))
                }
                output = SolverOutput(
                    obj = Flt64(hexalyObjective.doubleValue),
                    solution = results,
                    time = solvingTime!!,
                    possibleBestObj = Flt64(hexalySolution.getDoubleObjectiveBound(0)),
                    gap = Flt64(hexalySolution.getObjectiveGap(0))
                )

                when (val result = callBack?.execIfContain(Point.AnalyzingSolution, optimizer, hexalyVars, hexalyConstraints)) {
                    is Failed -> {
                        return Failed(result.error)
                    }

                    else -> {}
                }
                ok
            } else {
                when (val result = callBack?.execIfContain(Point.AfterFailure, optimizer, hexalyVars, hexalyConstraints)) {
                    is Failed -> {
                        return Failed(result.error)
                    }

                    else -> {}
                }
                Failed(Err(status.errCode!!))
            }
        } catch (e: HxException) {
            Failed(Err(ErrorCode.OREngineModelingException, e.message))
        } catch (e: Exception) {
            Failed(Err(ErrorCode.OREngineModelingException))
        }
    }
}
