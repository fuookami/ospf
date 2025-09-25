package fuookami.ospf.kotlin.example.framework_demo.demo1

import kotlin.time.Duration.Companion.seconds
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.math.ordinary.*
import fuookami.ospf.kotlin.utils.operator.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.solver.output.*
import fuookami.ospf.kotlin.core.backend.solver.config.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*
import fuookami.ospf.kotlin.core.backend.plugins.gurobi.*
import fuookami.ospf.kotlin.framework.solver.*
import fuookami.ospf.kotlin.example.framework_demo.demo1.route_context.RouteContext
import fuookami.ospf.kotlin.example.framework_demo.demo1.bandwidth_context.BandwidthContext
import fuookami.ospf.kotlin.example.framework_demo.demo1.infrastructure.*

/**
 * @see     https://fuookami.github.io/ospf/examples/framework-example1.html
 */
class SSP {
    lateinit var routeContext: RouteContext
    lateinit var bandwidthContext: BandwidthContext

    suspend operator fun invoke(input: Input): Ret<Output> {
        when (val result = init(input)) {
            is Failed -> {
                return Failed(result.error)
            }

            is Ok -> {}
        }

        return solveByMILP()
    }

    private suspend fun solveByMILP(): Ret<Output> {
        val model = LinearMetaModel("demo1")
        when (val result = construct(model)) {
            is Failed -> {
                return Failed(result.error)
            }

            is Ok -> {}
        }
        val solver = GurobiLinearSolver()
        val result = when (val result = solver(model)) {
            is Ok -> {
                model.tokens.setSolution(result.value.solution)
                result.value.solution
            }

            is Failed -> {
                return Failed(result.error)
            }
        }
        val solution = when (val result = bandwidthContext.analyze(model, result)) {
            is Ok -> {
                result.value
            }

            is Failed -> {
                return Failed(result.error)
            }
        }

        return Ok(Output(solution.map { list -> list.map { it.id } }))
    }

    private suspend fun solveByBenders(): Ret<Output> {
        val masterModel = LinearMetaModel("demo1-master")
        when (val result = constructMaster(masterModel)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }
        val objVar = RealVar("obj")
        masterModel.add(objVar)

        val subModel = LinearMetaModel("demo1-sub")
        when (val result = construct(subModel)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        val solver = GurobiLinearBendersDecompositionSolver(
            config = SolverConfig(
                notImprovementTime = 5.seconds,
            )
        )
        var lb = Flt64.minimum
        var ub = Flt64.maximum
        var i = UInt64.zero
        var j = UInt64.zero
        var currentGap = Flt64.maximum
        var masterResult: SolverOutput
        var subResult: SolverOutput? = null
        var fixedVariables: Map<AbstractVariableItem<*, *>, Flt64>
        do {
            masterResult = solver.solveMaster("master-${i}", masterModel).value!!
            lb = max(lb, masterResult.obj)
            val masterSolution = when (val result = routeContext.analyze(masterModel, masterResult.solution)) {
                is Ok -> {
                    result.value
                }

                is Failed -> {
                    return Failed(result.error)
                }
            }

            fixedVariables = routeContext.aggregation.graph.nodes.withIndex().flatMap { (i, node) ->
                routeContext.aggregation.services.withIndex().map { (j, service) ->
                    routeContext.aggregation.assignment.x[i, j] to if (masterSolution[service] == node) {
                        Flt64.one
                    } else {
                        Flt64.zero
                    }
                }
            }.toMap()
            val thisSubResult = solver.solveSub("sub-${i}", subModel, objVar, fixedVariables, true).value!!
            if (thisSubResult is LinearBendersDecompositionSolver.LinearFeasibleResult) {
                ub = min(ub, thisSubResult.obj + masterSolution.sumOf { it.key.cost }.toFlt64())
                subResult = thisSubResult.result
            }
            thisSubResult.cuts!!.forEach {
                masterModel.addConstraint(it)
            }
            i += UInt64.one
            val newGap = gap(lb, ub)
            if (subResult == null || newGap ls currentGap) {
                currentGap = newGap
                j = UInt64.zero
            } else {
                j += UInt64.one
            }
            println("lb: ${lb}, ub: ${ub}, gap: $currentGap")
        } while (currentGap gr Flt64.epsilon && j leq UInt64.two)

        val masterSolution = when (val result = routeContext.analyze(masterModel, masterResult.solution)) {
            is Ok -> {
                result.value
            }

            is Failed -> {
                return Failed(result.error)
            }
        }
        val subSolution = when (val result = bandwidthContext.analyze(masterSolution, subModel, subResult!!.solution, fixedVariables.keys)) {
            is Ok -> {
                result.value
            }

            is Failed -> {
                return Failed(result.error)
            }
        }

        return Ok(Output(subSolution.map { list -> list.map { it.id } }))
    }

    private fun init(input: Input): Try {
        routeContext = RouteContext()
        bandwidthContext = BandwidthContext(routeContext)

        when (val result = routeContext.init(input)) {
            is Failed -> {
                return Failed(result.error)
            }

            is Ok -> {}
        }
        when (val result = bandwidthContext.init(input)) {
            is Failed -> {
                return Failed(result.error)
            }

            is Ok -> {}
        }

        return ok
    }

    private fun construct(model: LinearMetaModel): Try {
        when (val result = routeContext.register(model)) {
            is Failed -> {
                return Failed(result.error)
            }

            is Ok -> {}
        }
        when (val result = bandwidthContext.register(model)) {
            is Failed -> {
                return Failed(result.error)
            }

            is Ok -> {}
        }

        when (val result = routeContext.construct(model)) {
            is Failed -> {
                return Failed(result.error)
            }

            is Ok -> {}
        }
        when (val result = bandwidthContext.construct(model)) {
            is Failed -> {
                return Failed(result.error)
            }

            is Ok -> {}
        }

        return ok
    }

    private fun constructMaster(model: LinearMetaModel): Try {
        when (val result = routeContext.register(model)) {
            is Failed -> {
                return Failed(result.error)
            }

            is Ok -> {}
        }

        when (val result = routeContext.construct(model)) {
            is Failed -> {
                return Failed(result.error)
            }

            is Ok -> {}
        }

        return ok
    }

    private fun gap(lb: Flt64, ub: Flt64): Flt64 {
        return if (lb eq ub) {
            Flt64.zero
        } else if (lb eq Flt64.zero || lb == Flt64.minimum || ub == Flt64.maximum) {
            Flt64.infinity
        } else {
            abs((ub - lb) / lb)
        }
    }
}
