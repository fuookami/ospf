/**
 * 并行组合线性求解器 / Parallel Combinatorial Linear Solver
 *
 * 将多个线性求解器并行运行，取第一个或最优结果。 / Runs multiple linear solvers in parallel, taking the first or best result.
*/
package fuookami.ospf.kotlin.framework.solver

import kotlinx.coroutines.*
import org.apache.logging.log4j.kotlin.logger
import fuookami.ospf.kotlin.core.error.SolverNotFoundError
import fuookami.ospf.kotlin.core.model.basic.ObjectCategory
import fuookami.ospf.kotlin.core.model.intermediate.LinearTriadModelView
import fuookami.ospf.kotlin.core.solver.AbstractLinearSolver
import fuookami.ospf.kotlin.core.solver.output.*
import fuookami.ospf.kotlin.core.solver.report.*
import fuookami.ospf.kotlin.core.solver.progress.SolverProgressContext
import fuookami.ospf.kotlin.math.algebra.number.*
import fuookami.ospf.kotlin.utils.error.ErrorCode
import fuookami.ospf.kotlin.utils.functional.*

/**
 * 并行组合线性求解器 / Parallel combinatorial linear solver
 *
 * @property solvers 线性求解器列表（懒加载） / Linear solver list (lazy loaded)
 * @property mode 并行组合模式，默认 Best / Parallel combinatorial mode, default Best
*/
class ParallelCombinatorialLinearSolver(
    private val solvers: List<Lazy<AbstractLinearSolver>>,
    private val mode: ParallelCombinatorialMode = ParallelCombinatorialMode.Best
) : AbstractLinearSolver {
    private val logger = logger()

    companion object {
        /**
         * Construct from an iterable of solvers.
         * 从求解器可迭代集合构造。
         *
         * @param solvers 要组合的求解器 / the solvers to combine
         * @param mode 组合模式，默认 Best / the combinatorial mode, default Best
         * @return 并行组合求解器 / the parallel combinatorial solver
        */
        @JvmName("constructBySolvers")
        operator fun invoke(
            solvers: Iterable<AbstractLinearSolver>,
            mode: ParallelCombinatorialMode = ParallelCombinatorialMode.Best
        ): ParallelCombinatorialLinearSolver {
            return ParallelCombinatorialLinearSolver(solvers.map { lazy { it } }, mode)
        }

        /**
         * Construct from an iterable of solver provider functions.
         * 从求解器提供函数可迭代集合构造。
         *
         * @param solvers 求解器提供函数集合 / the solver provider functions
         * @param mode 组合模式，默认 Best / the combinatorial mode, default Best
         * @return 并行组合求解器 / the parallel combinatorial solver
        */
        @JvmName("constructBySolverExtractors")
        operator fun invoke(
            solvers: Iterable<() -> AbstractLinearSolver>,
            mode: ParallelCombinatorialMode = ParallelCombinatorialMode.Best
        ): ParallelCombinatorialLinearSolver {
            return ParallelCombinatorialLinearSolver(solvers.map { lazy { it() } }, mode)
        }
    }

    override val name by lazy { "ParallelCombinatorial(${solvers.joinToString(",") { it.value.name }})" }

    /**
     * 并行求解并保留全部 backend 尝试。 / Solve in parallel while preserving all backend attempts.
     *
     * @param model 线性模型 / Linear model
     * @param progressContext 进度上下文 / Progress context
     * @return 组合求解报告 / Combinatorial solve report
     */
    suspend fun solveCombinatorialReport(
        model: LinearTriadModelView,
        progressContext: SolverProgressContext? = null
    ): Ret<CombinatorialSolveReport<Flt64>> = coroutineScope {
        when (val validation = model.identityValidation) {
            is Ok -> {}
            is Failed -> return@coroutineScope Failed(validation.error)
            is Fatal -> return@coroutineScope Fatal(validation.errors)
        }
        val attempts = solvers.mapIndexed { index, lazySolver ->
            async(Dispatchers.Default) {
                val solver = lazySolver.value
                val attemptId = SolveAttemptId("parallel-$index")
                when (val result = solver.solveReport(model, progressContext)) {
                    is Ok -> SolveAttemptTrace(
                        attemptId = attemptId,
                        backendId = solver.name,
                        report = result.value
                    )
                    is Failed -> SolveAttemptTrace(
                        attemptId = attemptId,
                        backendId = solver.name,
                        errors = listOf(
                            SolveIssue(
                                code = result.error.code.toString(),
                                category = SolveIssueCategory.Backend,
                                message = result.error.message
                            )
                        )
                    )
                    is Fatal -> SolveAttemptTrace(
                        attemptId = attemptId,
                        backendId = solver.name,
                        errors = result.errors.map { error ->
                            SolveIssue(
                                code = error.code.toString(),
                                category = SolveIssueCategory.Backend,
                                message = error.message
                            )
                        }
                    )
                }
            }
        }.awaitAll()
        val successful = attempts.filter { it.report != null }
        val selected = when {
            successful.isEmpty() -> null
            mode == ParallelCombinatorialMode.First -> successful.first()
            model.objective.category == ObjectCategory.Minimum -> successful.minBy { attempt ->
                attempt.report?.solution?.objective ?: Flt64.infinity
            }
            else -> successful.maxBy { attempt ->
                attempt.report?.solution?.objective ?: Flt64.negativeInfinity
            }
        }
        Ok(
            CombinatorialSolveReport(
                finalReport = selected?.report,
                attempts = attempts,
                selectedAttemptId = selected?.attemptId,
                selectionReason = when {
                    selected == null -> SolveSelectionReason.NoSuccessfulAttempt
                    mode == ParallelCombinatorialMode.First -> SolveSelectionReason.FirstFeasible
                    else -> SolveSelectionReason.BestObjective
                }
            )
        )
    }

    override suspend fun invoke(
        model: LinearTriadModelView,
        solvingStatusCallBack: SolvingStatusCallBack?
    ): Ret<FeasibleSolverOutput<Flt64>> {
        when (val validation = model.identityValidation) {
            is Ok -> {}
            is Failed -> return Failed(validation.error)
            is Fatal -> return Fatal(validation.errors)
        }
        var bestStatus: SolvingStatus? = null
        val lock = Any()

        return when (mode) {
            ParallelCombinatorialMode.First -> {
                var result: FeasibleSolverOutput<Flt64>? = null
                try {
                    coroutineScope {
                        val promises = solvers.mapIndexed { i, solver ->
                            launch(Dispatchers.Default) {
                                when (val ret = solver.value.invoke(model, solvingStatusCallBack?.let {
                                    { status ->
                                        synchronized(lock) {
                                            if (bestStatus == null) {
                                                bestStatus = status.copy(solver = solver.value.name, solverIndex = UInt64(i))
                                                it(bestStatus!!)
                                            } else if (model.objective.category == ObjectCategory.Maximum) {
                                                if (status.obj gr bestStatus!!.obj) {
                                                    bestStatus = status.copy(solver = solver.value.name, solverIndex = UInt64(i))
                                                    it(bestStatus!!)
                                                } else {
                                                    ok
                                                }
                                            } else {
                                                if (status.obj ls bestStatus!!.obj) {
                                                    bestStatus = status.copy(solver = solver.value.name, solverIndex = UInt64(i))
                                                    it(bestStatus!!)
                                                } else {
                                                    ok
                                                }
                                            }
                                        }
                                    }
                                })) {
                                    is Ok -> {
                                        logger.info { "Solver ${solver.value.name} found a solution." }
                                        synchronized(lock) {
                                            result = ret.value
                                            cancel()
                                        }
                                    }

                                    is Failed -> {
                                        logger.warn { "Solver ${solver.value.name} failed with error ${ret.error.code}: ${ret.error.message}" }
                                    }

                                    is Fatal -> {
                                        logger.error { "Solver ${solver.value.name} fatal: ${ret.errors.joinToString { it.message }}" }
                                    }
                                }
                            }
                        }
                        promises.joinAll()
                        if (result != null) {
                            Ok(result!!)
                        } else {
                            Failed(SolverNotFoundError())
                        }
                    }
                } catch (e: Exception) {
                    if (result != null) {
                        Ok(result!!)
                    } else {
                        Failed(ErrorCode.OREngineSolvingException)
                    }
                }
            }

            ParallelCombinatorialMode.Best -> {
                coroutineScope {
                    val promises = solvers.mapIndexed { i, solver ->
                        async(Dispatchers.Default) {
                            val result = solver.value.invoke(model, solvingStatusCallBack?.let {
                                { status ->
                                    synchronized(lock) {
                                        if (bestStatus == null) {
                                            bestStatus = status.copy(solver = solver.value.name, solverIndex = UInt64(i))
                                            it(bestStatus!!)
                                        } else if (model.objective.category == ObjectCategory.Maximum) {
                                            if (status.obj gr bestStatus!!.obj) {
                                                bestStatus = status.copy(solver = solver.value.name, solverIndex = UInt64(i))
                                                it(bestStatus!!)
                                            } else {
                                                ok
                                            }
                                        } else {
                                            if (status.obj ls bestStatus!!.obj) {
                                                bestStatus = status.copy(solver = solver.value.name, solverIndex = UInt64(i))
                                                it(bestStatus!!)
                                            } else {
                                                ok
                                            }
                                        }
                                    }
                                }
                            })
                            when (result) {
                                is Ok -> {
                                    logger.info { "Solver ${solver.value.name} found a solution." }
                                }

                                is Failed -> {
                                    logger.warn { "Solver ${solver.value.name} failed with error ${result.error.code}: ${result.error.message}" }
                                }

                                is Fatal -> {
                                    logger.error { "Solver ${solver.value.name} fatal: ${result.errors.joinToString { it.message }}" }
                                }
                            }
                            result
                        }
                    }
                    val results = promises.awaitAll()
                    val successResults = results.mapNotNull {
                        when (it) {
                            is Ok -> {
                                it.value
                            }

                            is Failed -> {
                                null
                            }

                            is Fatal -> {
                                null
                            }
                        }
                    }
                    if (successResults.isNotEmpty()) {
                        val bestResult = when (model.objective.category) {
                            ObjectCategory.Minimum -> {
                                successResults.minBy { it.obj }
                            }

                            ObjectCategory.Maximum -> {
                                successResults.maxBy { it.obj }
                            }
                        }
                        Ok(bestResult)
                    } else {
                        Failed(SolverNotFoundError())
                    }
                }
            }
        }
    }

    override suspend fun invoke(
        model: LinearTriadModelView,
        solutionAmount: UInt64,
        solvingStatusCallBack: SolvingStatusCallBack?
    ): Ret<Pair<FeasibleSolverOutput<Flt64>, List<List<Flt64>>>> {
        when (val validation = model.identityValidation) {
            is Ok -> {}
            is Failed -> return Failed(validation.error)
            is Fatal -> return Fatal(validation.errors)
        }
        var bestStatus: SolvingStatus? = null
        val lock = Any()

        return when (mode) {
            ParallelCombinatorialMode.First -> {
                var result: Pair<FeasibleSolverOutput<Flt64>, List<List<Flt64>>>? = null
                try {
                    coroutineScope {
                        val promises = solvers.mapIndexed { i, solver ->
                            launch(Dispatchers.Default) {
                                when (val ret = solver.value.invoke(model, solutionAmount, solvingStatusCallBack?.let {
                                    { status ->
                                        synchronized(lock) {
                                            if (bestStatus == null) {
                                                bestStatus = status.copy(solver = solver.value.name, solverIndex = UInt64(i))
                                                it(bestStatus!!)
                                            } else if (model.objective.category == ObjectCategory.Maximum) {
                                                if (status.obj gr bestStatus!!.obj) {
                                                    bestStatus = status.copy(solver = solver.value.name, solverIndex = UInt64(i))
                                                    it(bestStatus!!)
                                                } else {
                                                    ok
                                                }
                                        } else {
                                            if (status.obj ls bestStatus!!.obj) {
                                                    bestStatus = status.copy(solver = solver.value.name, solverIndex = UInt64(i))
                                                    it(bestStatus!!)
                                                } else {
                                                    ok
                                                }
                                            }
                                        }
                                    }
                                })) {
                                    is Ok -> {
                                        logger.info { "Solver ${solver.value.name} found a solution." }
                                        synchronized(lock) {
                                            result = ret.value
                                            cancel()
                                        }
                                    }

                                    is Failed -> {
                                        logger.warn { "Solver ${solver.value.name} failed with error ${ret.error.code}: ${ret.error.message}" }
                                    }

                                    is Fatal -> {
                                        logger.error { "Solver ${solver.value.name} fatal: ${ret.errors.joinToString { it.message }}" }
                                    }
                                }
                            }
                        }
                        promises.joinAll()
                        if (result != null) {
                            Ok(result!!)
                        } else {
                            Failed(SolverNotFoundError())
                        }
                    }
                } catch (e: Exception) {
                    if (result != null) {
                        Ok(result!!)
                    } else {
                        Failed(ErrorCode.OREngineSolvingException)
                    }
                }
            }

            ParallelCombinatorialMode.Best -> {
                coroutineScope {
                    val promises = solvers.mapIndexed { i, solver ->
                        async(Dispatchers.Default) {
                            val result = solver.value.invoke(model, solutionAmount, solvingStatusCallBack?.let {
                                { status ->
                                    synchronized(lock) {
                                        if (bestStatus == null) {
                                            bestStatus = status.copy(solver = solver.value.name, solverIndex = UInt64(i))
                                            it(bestStatus!!)
                                        } else if (model.objective.category == ObjectCategory.Maximum) {
                                            if (status.obj gr bestStatus!!.obj) {
                                                bestStatus = status.copy(solver = solver.value.name, solverIndex = UInt64(i))
                                                it(bestStatus!!)
                                            } else {
                                                ok
                                            }
                                        } else {
                                            if (status.obj ls bestStatus!!.obj) {
                                                bestStatus = status.copy(solver = solver.value.name, solverIndex = UInt64(i))
                                                it(bestStatus!!)
                                            } else {
                                                ok
                                            }
                                        }
                                    }
                                }
                            })
                            when (result) {
                                is Ok -> {
                                    logger.info { "Solver ${solver.value.name} found a solution." }
                                }

                                is Failed -> {
                                    logger.warn { "Solver ${solver.value.name} failed with error ${result.error.code}: ${result.error.message}" }
                                }

                                is Fatal -> {
                                    logger.error { "Solver ${solver.value.name} fatal: ${result.errors.joinToString { it.message }}" }
                                }
                            }
                            result
                        }
                    }
                    val results = promises.awaitAll()
                    val successResults = results.mapNotNull {
                        when (it) {
                            is Ok -> {
                                it.value
                            }

                            is Failed -> {
                                null
                            }

                            is Fatal -> {
                                null
                            }
                        }
                    }
                    if (successResults.isNotEmpty()) {
                        val bestResult = when (model.objective.category) {
                            ObjectCategory.Minimum -> {
                                successResults.minBy { it.first.obj }
                            }

                            ObjectCategory.Maximum -> {
                                successResults.maxBy { it.first.obj }
                            }
                        }
                        Ok(bestResult)
                    } else {
                        Failed(SolverNotFoundError())
                    }
                }
            }
        }
    }
}
