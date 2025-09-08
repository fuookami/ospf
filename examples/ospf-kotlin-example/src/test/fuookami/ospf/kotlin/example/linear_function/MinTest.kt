package fuookami.ospf.kotlin.example.linear_function

import kotlinx.coroutines.*
import org.junit.jupiter.api.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

class MinTest {
    @Test
    fun min() {
        val x = RealVar("x")
        x.range.leq(Flt64.five)
        x.range.geq(Flt64.three)
        val y = RealVar("y")
        y.range.leq(Flt64.ten)
        y.range.geq(Flt64.two)
        val solver = ScipLinearSolver()

        val maxmin = MaxMinFunction(listOf(x, y), name = "maxmin")
        val model1 = LinearMetaModel()
        model1.add(x)
        model1.add(y)
        model1.add(maxmin)
        model1.minimize(maxmin)
        val result1 = runBlocking { solver(model1) }
        assert(result1.value!!.obj eq Flt64.two)

        val model2 = LinearMetaModel()
        model2.add(x)
        model2.add(y)
        model2.add(maxmin)
        model2.maximize(maxmin)
        val result2 = runBlocking { solver(model2) }
        assert(result2.value!!.obj eq Flt64.five)

        val min = MinFunction(listOf(x, y), name = "min")
        val model3 = LinearMetaModel()
        model3.add(x)
        model3.add(y)
        model3.add(min)
        model3.minimize(min)
        val result3 = runBlocking { solver(model3) }
        assert(result3.value!!.obj ls Flt64.zero)       // -inf

        val model4 = LinearMetaModel()
        model4.add(x)
        model4.add(y)
        model4.add(min)
        model4.maximize(min)
        val result4 = runBlocking { solver(model4) }
        assert(result4.value!!.obj eq Flt64.five)
    }
}
