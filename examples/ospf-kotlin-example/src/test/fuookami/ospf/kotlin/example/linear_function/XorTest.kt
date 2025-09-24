package fuookami.ospf.kotlin.example.linear_function

import kotlinx.coroutines.*
import org.junit.jupiter.api.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

class XorTest {
    @Test
    fun xor1() {
        val x = BinVar("x")
        val y = BinVar("y")
        val xor = XorFunction(listOf(x, y), name = "xor")
        val solver = ScipLinearSolver()

        val model1 = LinearMetaModel()
        model1.add(x)
        model1.add(y)
        model1.add(xor)
        model1.addConstraint(xor)
        model1.minimize(x + y)
        val result1 = runBlocking { solver(model1) }
        assert(result1.value!!.obj eq Flt64.one)

        val model2 = LinearMetaModel()
        model2.add(x)
        model2.add(y)
        model2.add(xor)
        model2.addConstraint(xor)
        model2.maximize(x + y)
        val result2 = runBlocking { solver(model2) }
        assert(result2.value!!.obj eq Flt64.one)

        val model3 = LinearMetaModel()
        model3.add(x)
        model3.add(y)
        model3.add(xor)
        model3.addConstraint(!xor)
        model3.minimize(x + y)
        val result3 = runBlocking { solver(model3) }
        assert(result3.value!!.obj eq Flt64.zero)

        val model4 = LinearMetaModel()
        model4.add(x)
        model4.add(y)
        model4.add(xor)
        model4.addConstraint(!xor)
        model4.maximize(x + y)
        val result4 = runBlocking { solver(model4) }
        assert(result4.value!!.obj eq Flt64.two)
    }

    @Test
    fun xor2() {
        val x = BinVar("x")
        val y = BinVar("y")
        val z = BinVar("z")
        val xor = XorFunction(listOf(x, y, z), name = "xor")
        val solver = ScipLinearSolver()

        val model1 = LinearMetaModel()
        model1.add(x)
        model1.add(y)
        model1.add(z)
        model1.add(xor)
        model1.addConstraint(xor)
        model1.minimize(x + y + z)
        val result1 = runBlocking { solver(model1) }
        assert(result1.value!!.obj eq Flt64.one)

        val model2 = LinearMetaModel()
        model2.add(x)
        model2.add(y)
        model2.add(z)
        model2.add(xor)
        model2.addConstraint(xor)
        model2.maximize(x + y + z)
        val result2 = runBlocking { solver(model2) }
        assert(result2.value!!.obj eq Flt64.two)

        val model3 = LinearMetaModel()
        model3.add(x)
        model3.add(y)
        model3.add(z)
        model3.add(xor)
        model3.addConstraint(!xor)
        model3.minimize(x + y + z)
        val result3 = runBlocking { solver(model3) }
        assert(result3.value!!.obj eq Flt64.zero)

        val model4 = LinearMetaModel()
        model4.add(x)
        model4.add(y)
        model4.add(z)
        model4.add(xor)
        model4.addConstraint(!xor)
        model4.maximize(x + y + z)
        val result4 = runBlocking { solver(model4) }
        assert(result4.value!!.obj eq Flt64.three)
    }
}
