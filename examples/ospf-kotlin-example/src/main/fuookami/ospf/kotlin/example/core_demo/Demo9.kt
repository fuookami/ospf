package fuookami.ospf.kotlin.example.core_demo

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.concept.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

/**
 * @see     https://fuookami.github.io/ospf/examples/example9.html
 */
data object Demo9 {
    data class Settlement(
        val x: Flt64,
        val y: Flt64
    ) : AutoIndexed(Settlement::class)

    private val settlements = listOf(
        Settlement(Flt64(9.0), Flt64(2.0)),
        Settlement(Flt64(2.0), Flt64(1.0)),
        Settlement(Flt64(3.0), Flt64(8.0)),
        Settlement(Flt64(3.0), Flt64(-2.0)),
        Settlement(Flt64(5.0), Flt64(9.0)),
        Settlement(Flt64(4.0), Flt64(-2.0))
    )

    private lateinit var x: IntVar
    private lateinit var y: IntVar
    private lateinit var dx: LinearIntermediateSymbols1
    private lateinit var dy: LinearIntermediateSymbols1
    private lateinit var distance: LinearIntermediateSymbols1

    private val metaModel = LinearMetaModel("demo9")

    private val subProcesses = listOf(
        Demo9::initVariable,
        Demo9::initSymbol,
        Demo9::initObject,
        Demo9::initConstraint,
        Demo9::solve,
        Demo9::analyzeSolution
    )

    suspend operator fun invoke(): Try {
        for (process in subProcesses) {
            when (val result = process()) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }
        return ok
    }

    private suspend fun initVariable(): Try {
        x = IntVar("x")
        y = IntVar("y")
        metaModel.add(x)
        metaModel.add(y)
        return ok
    }

    private suspend fun initSymbol(): Try {
        dx = LinearIntermediateSymbols1("dx", Shape1(settlements.size)) { i, _ ->
            SlackFunction(
                type = UInteger,
                x = LinearPolynomial(x),
                y = LinearPolynomial(settlements[i].x),
                name = "dx_$i"
            )
        }
        metaModel.add(dx)

        dy = LinearIntermediateSymbols1("dy", Shape1(settlements.size)) { i, _ ->
            SlackFunction(
                type = UInteger,
                x = LinearPolynomial(y),
                y = LinearPolynomial(settlements[i].y),
                name = "dy_$i"
            )
        }
        metaModel.add(dy)

        distance = LinearIntermediateSymbols1("distance", Shape1(settlements.size)) { i, _ ->
            LinearExpressionSymbol(
                dx[i] + dy[i],
                name = "distance_$i"
            )
        }
        metaModel.add(distance)
        return ok
    }

    private suspend fun initObject(): Try {
        metaModel.minimize(sum(distance[_a]))
        return ok
    }

    private suspend fun initConstraint(): Try {
        return ok
    }

    private suspend fun solve(): Try {
        val solver = ScipLinearSolver()
        when (val ret = solver(metaModel)) {
            is Ok -> {
                metaModel.tokens.setSolution(ret.value.solution)
            }

            is Failed -> {
                return Failed(ret.error)
            }
        }
        return ok
    }

    private suspend fun analyzeSolution(): Try {
        val position = ArrayList<Flt64>()
        for (token in metaModel.tokens.tokens) {
            if (token.variable.belongsTo(x)) {
                position.add(token.result!!)
            }
        }
        for (token in metaModel.tokens.tokens) {
            if (token.variable.belongsTo(y)) {
                position.add(token.result!!)
            }
        }
        return ok
    }
}
