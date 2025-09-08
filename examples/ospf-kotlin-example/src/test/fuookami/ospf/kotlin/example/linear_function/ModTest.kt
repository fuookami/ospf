package fuookami.ospf.kotlin.example.linear_function

import kotlinx.coroutines.*
import org.junit.jupiter.api.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

class ModTest {
    @Test
    fun mod() {
        val x = RealVar("x")
        x.range.eq(Flt64.three)
        val mod = ModFunction(x, Flt64(0.7), name = "mod")

        val model = LinearMetaModel()
        model.add(x)
        model.add(mod)
        model.minimize(mod)

        val solver = ScipLinearSolver()
        val result = runBlocking { solver(model) }
        assert(result.value!!.obj eq Flt64(0.2))
    }
}
