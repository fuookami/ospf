package fuookami.ospf.kotlin.example.linear_function

import kotlinx.coroutines.*
import org.junit.jupiter.api.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

class BTerTest {
    @Test
    fun bter1() {
        val x = RealVar("x")
        x.range.leq(Flt64.two)
        x.range.geq(-Flt64.two)
        val bter = BalanceTernaryzationFunction(x, name = "bter")
        val solver = ScipLinearSolver()

        val model1 = LinearMetaModel()
        model1.add(x)
        model1.add(bter)
        model1.minimize(bter)
        val result1 = runBlocking { solver(model1) }
        assert(result1.value!!.obj eq -Flt64.one)

        val model2 = LinearMetaModel()
        model2.add(x)
        model2.add(bter)
        model2.maximize(bter)
        val result2 = runBlocking { solver(model2) }
        assert(result2.value!!.obj eq Flt64.one)

        val model3 = LinearMetaModel()
        model3.add(x)
        model3.add(bter)
        model3.addConstraint(x geq Flt64.zero)
        model3.minimize(bter)
        val result3 = runBlocking { solver(model3) }
        assert(result3.value!!.obj eq Flt64.zero)

        val model4 = LinearMetaModel()
        model4.add(x)
        model4.add(bter)
        model4.addConstraint(x geq Flt64.zero)
        model4.maximize(bter)
        val result4 = runBlocking { solver(model4) }
        assert(result4.value!!.obj eq Flt64.one)

        val model5 = LinearMetaModel()
        model5.add(x)
        model5.add(bter)
        model5.addConstraint(x leq 0)
        model5.minimize(bter)
        val result5 = runBlocking { solver(model5) }
        assert(result5.value!!.obj eq -Flt64.one)

        val model6 = LinearMetaModel()
        model6.add(x)
        model6.add(bter)
        model6.addConstraint(x leq 0)
        model6.maximize(bter)
        val result6 = runBlocking { solver(model6) }
        assert(result6.value!!.obj eq Flt64.zero)

        val model7 = LinearMetaModel()
        model7.add(x)
        model7.add(bter)
        model7.addConstraint(x leq Flt64(0.3))
        model7.maximize(bter)
        val result7 = runBlocking { solver(model7) }
        assert(result7.value!!.obj eq Flt64.one)

        val model8 = LinearMetaModel()
        model8.add(x)
        model8.add(bter)
        model8.addConstraint(x geq -Flt64(0.3))
        model8.minimize(bter)
        val result8 = runBlocking { solver(model8) }
        assert(result8.value!!.obj eq -Flt64.one)
    }

    @Test
    fun bter2() {
        val x = IntVar("x")
        x.range.leq(Int64.two)
        x.range.geq(-Int64.two)
        val bter = BalanceTernaryzationFunction(x, name = "bter")
        val solver = ScipLinearSolver()

        val model1 = LinearMetaModel()
        model1.add(x)
        model1.add(bter)
        model1.minimize(bter)
        val result1 = runBlocking { solver(model1) }
        assert(result1.value!!.obj eq -Flt64.one)

        val model2 = LinearMetaModel()
        model2.add(x)
        model2.add(bter)
        model2.maximize(bter)
        val result2 = runBlocking { solver(model2) }
        assert(result2.value!!.obj eq Flt64.one)

        val model3 = LinearMetaModel()
        model3.add(x)
        model3.add(bter)
        model3.addConstraint(x geq Flt64.zero)
        model3.minimize(bter)
        val result3 = runBlocking { solver(model3) }
        assert(result3.value!!.obj eq Flt64.zero)

        val model4 = LinearMetaModel()
        model4.add(x)
        model4.add(bter)
        model4.addConstraint(x geq Flt64.zero)
        model4.maximize(bter)
        val result4 = runBlocking { solver(model4) }
        assert(result4.value!!.obj eq Flt64.one)

        val model5 = LinearMetaModel()
        model5.add(x)
        model5.add(bter)
        model5.addConstraint(x leq 0)
        model5.minimize(bter)
        val result5 = runBlocking { solver(model5) }
        assert(result5.value!!.obj eq -Flt64.one)

        val model6 = LinearMetaModel()
        model6.add(x)
        model6.add(bter)
        model6.addConstraint(x leq 0)
        model6.maximize(bter)
        val result6 = runBlocking { solver(model6) }
        assert(result6.value!!.obj eq Flt64.zero)
    }
}
