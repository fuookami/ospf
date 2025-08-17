package fuookami.ospf.kotlin.example.linear_function

import kotlinx.coroutines.*
import org.junit.jupiter.api.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

class SemiTest {
    @Test
    fun semi() {
        val x = URealVar("x")
        x.range.leq(Flt64.three)
        val y = URealVar("y")
        y.range.geq(Flt64.two)
        y.range.leq(Flt64.five)
        val semi = SemiFunction(
            x - y,
            name = "semi"
        )
        val solver = ScipLinearSolver()

        val model1 = LinearMetaModel()
        model1.add(x)
        model1.add(y)
        model1.add(semi)
        model1.minimize(semi)

        val result1 = runBlocking { solver(model1) }
        assert(result1.value!!.obj eq Flt64.zero)
        assert(result1.value!!.solution[1] geq result1.value!!.solution[0])

        val model2 = LinearMetaModel()
        model2.add(x)
        model2.add(y)
        model2.add(semi)
        model2.maximize(semi)

        val result2 = runBlocking { solver(model2) }
        assert(result2.value!!.obj eq Flt64.one)
        assert(result2.value!!.solution[0] eq Flt64.three)
        assert(result2.value!!.solution[1] eq Flt64.two)
    }
}