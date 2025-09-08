package fuookami.ospf.kotlin.example.linear_function

import kotlinx.coroutines.*
import org.junit.jupiter.api.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

class SlackRangeTest {
    @Test
    fun slackRang() {
        val x = RealVar("x")
        x.range.leq(Flt64.two)
        x.range.geq(-Flt64.three)
        val slack = SlackRangeFunction(
            x = x,
            lb = Flt64.five,
            ub = Flt64(6.0),
            name = "slack"
        )

        val model = LinearMetaModel()
        model.add(x)
        model.add(slack)
        model.minimize(slack)

        val solver = ScipLinearSolver()
        val result = runBlocking { solver(model) }
        assert(result.value!!.obj eq Flt64.three)
        assert(result.value!!.solution[0] eq Flt64.two)
    }
}
