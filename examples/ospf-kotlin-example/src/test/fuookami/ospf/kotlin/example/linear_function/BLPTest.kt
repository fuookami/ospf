package fuookami.ospf.kotlin.example.linear_function

import kotlinx.coroutines.*
import org.junit.jupiter.api.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.math.geometry.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

class BLPTest {
    @Test
    fun bivariate() {
        val x = URealVar("x")
        val y = URealVar("y")
        x.range.leq(Flt64.two)
        y.range.leq(Flt64.two)

        val blp = BivariateLinearPiecewiseFunction(
            x = x,
            y = y,
            points = listOf(
                point3(),
                point3(x = Flt64.two),
                point3(y = Flt64.two),
                point3(x = Flt64.two, y = Flt64.two),
                point3(x = Flt64.one, y = Flt64.one, z = Flt64.one)
            ),
            name = "z"
        )

        val model = LinearMetaModel()
        model.add(x)
        model.add(y)
        model.add(blp)
        model.maximize(blp)

        val solver = ScipLinearSolver()
        val result = runBlocking { solver(model) }
        assert(result.value!!.solution[0] eq Flt64.one)
        assert(result.value!!.solution[1] eq Flt64.one)
    }
}
