package fuookami.ospf.kotlin.example.linear_function

import kotlinx.coroutines.*
import org.junit.jupiter.api.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

class FloorTest {
    @Test
    fun floor() {
        val x = RealVar("x")
        x.range.leq(Flt64.five)
        x.range.geq(Flt64.two)
        val floor = FloorFunction(x, Flt64(0.7), name = "floor")
        val solver = ScipLinearSolver()

        val model1 = LinearMetaModel()
        model1.add(x)
        model1.add(floor)
        model1.minimize(floor)
        val result1 = runBlocking { solver(model1) }
        assert(result1.value!!.obj eq Flt64.two)
        assert(result1.value!!.solution[0].roundTo(5) leq Flt64(2.1))

        val model2 = LinearMetaModel()
        model2.add(x)
        model2.add(floor)
        model2.maximize(floor)
        val result2 = runBlocking { solver(model2) }
        assert(result2.value!!.obj eq Flt64(7.0))
        assert(result2.value!!.solution[0].roundTo(5) geq Flt64(4.9))
    }
}
