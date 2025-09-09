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

class OrTest {
    @Test
    fun or() {
        val x = BinVar("x")
        val y = BinVar("y")
        val or = OrFunction(listOf(x, y), "or")
        val solver = ScipLinearSolver()

        val model1 = LinearMetaModel()
        model1.add(x)
        model1.add(y)
        model1.add(or)
        model1.minimize(or)
        val result1 = runBlocking { solver(model1) }
        assert(result1.value!!.obj eq Flt64.zero)
        assert(result1.value!!.solution[0] eq Flt64.zero)
        assert(result1.value!!.solution[1] eq Flt64.zero)

        val model2 = LinearMetaModel()
        model2.add(x)
        model2.add(y)
        model2.add(or)
        model2.addConstraint(or eq Flt64.one)
        model2.minimize(x + y)
        val result2 = runBlocking { solver(model2) }
        assert(result2.value!!.obj eq Flt64.one)
        assert((result2.value!!.solution[0] eq Flt64.one) xor (result2.value!!.solution[1] eq Flt64.one))
    }
}
