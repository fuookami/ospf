package fuookami.ospf.kotlin.example.linear_function

import kotlinx.coroutines.*
import org.junit.jupiter.api.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

class MaskingTest {
    @Test
    fun masking() {
        val x = RealVar("x")
        x.range.leq(Flt64.two)
        x.range.geq(-Flt64.three)
        val mask = BinVar("mask")
        val masking = MaskingFunction(
            x = x,
            mask = mask,
            name = "masking"
        )
        val solver = ScipLinearSolver()

        val model1 = LinearMetaModel()
        model1.add(x)
        model1.add(mask)
        model1.add(masking)
        model1.minimize(masking)

        val result1 = runBlocking { solver(model1) }
        assert(result1.value!!.obj eq -Flt64.three)
        assert(result1.value!!.solution[0] eq -Flt64.three)
        assert(result1.value!!.solution[1] eq Flt64.one)

        val model2 = LinearMetaModel()
        model2.add(x)
        model2.add(mask)
        model2.add(masking)
        model2.addConstraint(mask eq false)
        model2.minimize(masking)

        val result2 = runBlocking { solver(model2) }
        assert(result2.value!!.obj eq Flt64.zero)
    }
}
