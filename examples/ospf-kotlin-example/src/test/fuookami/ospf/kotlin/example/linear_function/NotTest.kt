package fuookami.ospf.kotlin.example.linear_function

import kotlinx.coroutines.*
import org.junit.jupiter.api.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

class NotTest {
    @Test
    fun abs1() {
        val x = URealVar("x")
        x.range.leq(Flt64.one)
        val not = NotFunction(x, name = "not")
        val solver = ScipLinearSolver()

        val model1 = LinearMetaModel()
        model1.add(x)
        model1.add(not)
        model1.addConstraint(not)
        model1.maximize(x)
        val result1 = runBlocking { solver(model1) }
        assert(result1.value!!.obj eq Flt64.zero)

        val model2 = LinearMetaModel()
        model2.add(x)
        model2.add(not)
        model2.addConstraint(!not)
        model2.minimize(x)
        val result2 = runBlocking { solver(model2) }
        assert(result2.value!!.obj geq Flt64(1e-6))
    }

    @Test
    fun abs2() {
        val x = BinVar("x")
        val not = NotFunction(x, name = "not")
        val solver = ScipLinearSolver()

        val model1 = LinearMetaModel()
        model1.add(x)
        model1.add(not)
        model1.addConstraint(not)
        model1.maximize(x)
        val result1 = runBlocking { solver(model1) }
        assert(result1.value!!.obj eq Flt64.zero)

        val model2 = LinearMetaModel()
        model2.add(x)
        model2.add(not)
        model2.addConstraint(!not)
        model2.minimize(x)
        val result2 = runBlocking { solver(model2) }
        assert(result2.value!!.obj eq Flt64.one)
    }
}
