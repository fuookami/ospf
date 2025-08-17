package fuookami.ospf.kotlin.example.linear_function

import kotlinx.coroutines.*
import org.junit.jupiter.api.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.math.geometry.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

class ULPTest {
    @Test
    fun univariate() {
        val x = URealVar("x")
        x.range.leq(Flt64.two)

        val ulp = UnivariateLinearPiecewiseFunction(
            x = x,
            points = listOf(
                point2(),
                point2(x = Flt64.one, y = Flt64.two),
                point2(x = Flt64.two, y = Flt64.one)
            ),
            name = "y"
        )

        val model = LinearMetaModel()
        model.add(x)
        model.add(ulp)
        model.maximize(ulp)

        val solver = ScipLinearSolver()
        val result = runBlocking { solver(model) }
        assert(result.value!!.obj eq Flt64.two)
        assert(result.value!!.solution[0] eq Flt64.one)
    }
}
