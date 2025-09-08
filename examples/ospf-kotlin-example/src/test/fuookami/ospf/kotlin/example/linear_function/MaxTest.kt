package fuookami.ospf.kotlin.example.linear_function

import kotlinx.coroutines.*
import org.junit.jupiter.api.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

class MaxTest {
    @Test
    fun max() {
        val x = RealVar("x")
        x.range.leq(Flt64.five)
        x.range.geq(Flt64.three)
        val y = RealVar("y")
        y.range.leq(Flt64.ten)
        y.range.geq(Flt64.two)
        val solver = ScipLinearSolver()

        val minmax = MinMaxFunction(listOf(x, y), name = "minmax")
        val model1 = LinearMetaModel()
        model1.add(x)
        model1.add(y)
        model1.add(minmax)
        model1.minimize(minmax)
        val result1 = runBlocking { solver(model1) }
        assert(result1.value!!.obj eq Flt64.three)

        val model2 = LinearMetaModel()
        model2.add(x)
        model2.add(y)
        model2.add(minmax)
        model2.maximize(minmax)
        val result2 = runBlocking { solver(model2) }
        assert(result2.value!!.obj eq Flt64.ten)

        val max = MaxFunction(listOf(x, y), name = "max")
        val model3 = LinearMetaModel()
        model3.add(x)
        model3.add(y)
        model3.add(max)
        model3.minimize(max)
        val result3 = runBlocking { solver(model3) }
        assert(result3.value!!.obj eq Flt64.three)

        val model4 = LinearMetaModel()
        model4.add(x)
        model4.add(y)
        model4.add(max)
        model4.maximize(max)
        val result4 = runBlocking { solver(model4) }
        assert(result4.value!!.obj gr Flt64.ten)    // inf
    }
}
