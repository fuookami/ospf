package fuookami.ospf.kotlin.example.linear_function

import kotlinx.coroutines.*
import org.junit.jupiter.api.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

class IfTest {
    @Test
    fun ifTest() {
        val x = URealVar("x")
        x.range.geq(Flt64.two)
        x.range.leq(Flt64.five)
        val condition1 = IfFunction(x geq Flt64.three, name = "c1")
        val condition2 = IfFunction(x geq Flt64.three, epsilon = Flt64.zero, name = "c2")
        val condition3 = IfFunction(x leq Flt64.one, name = "c3")
        val solver = ScipLinearSolver()

        val model1 = LinearMetaModel()
        model1.add(x)
        model1.add(condition1)
        model1.addConstraint(condition1 eq true)
        model1.minimize(x)
        val result1 = runBlocking { solver(model1) }
        assert(result1.value!!.obj eq Flt64.three)

        val model2 = LinearMetaModel()
        model2.add(x)
        model2.add(condition1)
        model2.addConstraint(condition1 eq false)
        model2.maximize(x)
        val result2 = runBlocking { solver(model2) }
        assert(result2.value!!.obj ls Flt64.three)

        val model3 = LinearMetaModel()
        model3.add(x)
        model3.add(condition2)
        model3.addConstraint(condition2 eq false)
        model3.maximize(x)
        val result3 = runBlocking { solver(model3) }
        assert(result3.value!!.obj eq Flt64.three)

        val model4 = LinearMetaModel()
        model4.add(x)
        model4.add(condition3)
        model4.maximize(condition3)
        val result4 = runBlocking { solver(model4) }
        assert(result4.value!!.obj eq Flt64.zero)
    }
}
