package fuookami.ospf.kotlin.example.linear_function

import kotlinx.coroutines.*
import org.junit.jupiter.api.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

class AbsTest {
    @Test
    fun abs() {
        val x = RealVar("x")
        x.range.leq(Flt64.two)
        x.range.geq(-Flt64.three)
        val abs = AbsFunction(x, name = "abs")
        val solver = ScipLinearSolver()

        val model1 = LinearMetaModel()
        model1.add(x)
        model1.add(abs)
        model1.minimize(abs)
        val result1 = runBlocking { solver(model1) }
        assert(result1.value!!.obj eq Flt64.zero)

        val model2 = LinearMetaModel()
        model2.add(x)
        model2.add(abs)
        model2.maximize(abs)
        val result2 = runBlocking { solver(model2) }
        assert(result2.value!!.solution[0] eq -Flt64.three)
        assert(result2.value!!.obj eq Flt64.three)

        val model3 = LinearMetaModel()
        model3.add(x)
        model3.add(abs)
        model3.addConstraint(x geq Flt64.one)
        model3.minimize(abs)
        val result3 = runBlocking { solver(model3) }
        assert((result3.value!!.obj - Flt64.one).toFlt32() eq Flt32.zero)

        val model4 = LinearMetaModel()
        model4.add(x)
        model4.add(abs)
        model4.addConstraint(x geq Flt64.one)
        model4.maximize(abs)
        val result4 = runBlocking { solver(model4) }
        assert(result4.value!!.obj eq Flt64.two)

        val model5 = LinearMetaModel()
        model5.add(x)
        model5.add(abs)
        model5.addConstraint(x leq -Flt64.one)
        model5.minimize(abs)
        val result5 = runBlocking { solver(model5) }
        assert((result5.value!!.obj - Flt64.one).toFlt32() eq Flt32.zero)

        val model6 = LinearMetaModel()
        model6.add(x)
        model6.add(abs)
        model6.addConstraint(x leq -Flt64.one)
        model6.maximize(abs)
        val result6 = runBlocking { solver(model6) }
        assert(result6.value!!.obj eq Flt64.three)

        val model7 = LinearMetaModel()
        model7.add(x)
        model7.add(abs)
        model7.addConstraint(abs eq Flt64.one)
        model7.maximize(x)
        val result7 = runBlocking { solver(model7) }
        assert((result7.value!!.obj - Flt64.one).toFlt32() eq Flt32.zero)

        val model8 = LinearMetaModel()
        model8.add(x)
        model8.add(abs)
        model8.addConstraint(abs eq Flt64.one)
        model8.minimize(x)
        val result8 = runBlocking { solver(model8) }
        assert((result8.value!!.obj + Flt64.one).toFlt32() eq Flt32.zero)

        val model9 = LinearMetaModel()
        model9.add(x)
        model9.add(abs)
        model9.addConstraint(abs geq Flt64.one)
        model9.maximize(x)
        val result9 = runBlocking { solver(model9) }
        assert(result9.value!!.obj eq Flt64.two)

        val model10 = LinearMetaModel()
        model10.add(x)
        model10.add(abs)
        model10.addConstraint(abs geq Flt64.one)
        model10.minimize(x)
        val result10 = runBlocking { solver(model10) }
        assert(result10.value!!.obj eq -Flt64.three)
    }
}
