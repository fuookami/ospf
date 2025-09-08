package fuookami.ospf.kotlin.example.linear_function

import kotlinx.coroutines.*
import org.junit.jupiter.api.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

class BinTest {
    @Test
    fun bin1() {
        val x = URealVar("x")
        x.range.leq(Flt64.two)
        val bin = BinaryzationFunction(
            x,
            name = "bin"
        )
        val solver = ScipLinearSolver()

        val model1 = LinearMetaModel()
        model1.add(x)
        model1.add(bin)
        model1.minimize(bin)
        val result1 = runBlocking { solver(model1) }
        assert(result1.value!!.obj eq Flt64.zero)

        val model2 = LinearMetaModel()
        model2.add(x)
        model2.add(bin)
        model2.maximize(bin)
        val result2 = runBlocking { solver(model2) }
        assert(result2.value!!.obj eq Flt64.one)

        val model3 = LinearMetaModel()
        model3.add(x)
        model3.add(bin)
        model3.addConstraint(x eq Flt64.zero)
        model3.maximize(bin)
        val result3 = runBlocking { solver(model3) }
        assert(result3.value!!.obj eq Flt64.zero)

        val model4 = LinearMetaModel()
        model4.add(x)
        model4.add(bin)
        model4.addConstraint(x eq Flt64(0.3))
        model4.maximize(bin)
        val result4 = runBlocking { solver(model4) }
        assert(result4.value!!.obj eq Flt64.one)
    }

    @Test
    fun bin2() {
        val x = UIntVar("x")
        x.range.leq(UInt64.two)
        val bin = BinaryzationFunction(x, name = "bin")
        val solver = ScipLinearSolver()

        val model1 = LinearMetaModel()
        model1.add(x)
        model1.add(bin)
        model1.minimize(bin)
        val result1 = runBlocking { solver(model1) }
        assert(result1.value!!.obj eq Flt64.zero)

        val model2 = LinearMetaModel()
        model2.add(x)
        model2.add(bin)
        model2.maximize(bin)
        val result2 = runBlocking { solver(model2) }
        assert(result2.value!!.obj eq Flt64.one)

        val model3 = LinearMetaModel()
        model3.add(x)
        model3.add(bin)
        model3.addConstraint(x eq Flt64.zero)
        model3.maximize(bin)
        val result3 = runBlocking { solver(model3) }
        assert(result3.value!!.obj eq Flt64.zero)

        val model4 = LinearMetaModel()
        model4.add(x)
        model4.add(bin)
        model4.addConstraint(x eq Flt64.zero)
        model4.minimize(bin)
        val result4 = runBlocking { solver(model4) }
        assert(result4.value!!.obj eq Flt64.zero)
    }
}
