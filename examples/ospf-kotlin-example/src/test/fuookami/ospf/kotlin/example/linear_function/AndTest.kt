package fuookami.ospf.kotlin.example.linear_function

import kotlinx.coroutines.*
import org.junit.jupiter.api.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

class AndTest {
    @Test
    fun and() {
        val x = UIntVar("x")
        x.range.leq(UInt64.one)
        val y = UIntVar("y")
        y.range.leq(UInt64.two)
        val and = AndFunction(
            listOf(x, y),
            "and"
        )
        val solver = ScipLinearSolver()

        val model1 = LinearMetaModel()
        model1.add(x)
        model1.add(y)
        model1.add(and)
        model1.addConstraint(and)
        model1.maximize(x + y)
        val result1 = runBlocking { solver(model1) }
        assert(result1.value!!.obj eq Flt64.three)

        val model2 = LinearMetaModel()
        model2.add(x)
        model2.add(y)
        model2.add(and)
        model2.addConstraint(and)
        model2.minimize(x + y)
        val result2 = runBlocking { solver(model2) }
        assert(result2.value!!.obj eq Flt64.two)
    }
}
