package fuookami.ospf.kotlin.example

import kotlinx.coroutines.*
import org.junit.jupiter.api.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.intermediate_model.*
import fuookami.ospf.kotlin.core.backend.plugins.gurobi.*
import fuookami.ospf.kotlin.core.backend.plugins.cplex.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

class QuadraticTest {
    @Test
    fun qp() {
        val x = RealVar("x")
        x.range.leq(Flt64.two)
        x.range.geq(-Flt64.two)

        val model = QuadraticMetaModel()
        model.add(x)
        model.maximize((Flt64.one - x) * (Flt64.one - x))

        val solver1 = ScipQuadraticSolver()
        val result1 = runBlocking { solver1(model) }
        assert(result1.value!!.solution[0] eq -Flt64.two)

        val solver2 = CplexQuadraticSolver()
        val result2 = runBlocking { solver2(model) }
        assert(result2.value!!.solution[0] eq -Flt64.two)

        val solver3 = GurobiQuadraticSolver()
        val result3 = runBlocking { solver3(model) }
        assert(result3.value!!.solution[0] eq -Flt64.two)
    }

    @Test
    fun qcp() {
        val x = RealVar("x")
        x.range.leq(Flt64.two)
        x.range.geq(-Flt64.two)

        val model = QuadraticMetaModel()
        model.add(x)
        model.maximize(x)
        model.addConstraint(x * x leq Flt64(4.0))

        val solver1 = ScipQuadraticSolver()
        val result1 = runBlocking { solver1(model) }
        assert(result1.value!!.solution[0] eq Flt64.two)

        val solver2 = CplexQuadraticSolver()
        val result2 = runBlocking { solver2(model) }
        assert(result2.value!!.solution[0] eq Flt64.two)

        val solver3 = GurobiQuadraticSolver()
        val result3 = runBlocking { solver3(model) }
        assert(result3.value!!.solution[0] eq Flt64.two)
    }

    @Test
    fun qcqp() {
        val x = RealVar("x")
        x.range.leq(Flt64.two)
        x.range.geq(-Flt64.two)

        val model = QuadraticMetaModel()
        model.add(x)
        model.minimize(x * x)
        model.addConstraint(x * x leq Flt64(4.0))

        val solver1 = ScipQuadraticSolver()
        val result1 = runBlocking { solver1(model) }
        assert(result1.value!!.solution[0] eq Flt64.zero)

        val solver2 = CplexQuadraticSolver()
        val result2 = runBlocking { solver2(model) }
        assert(result2.value!!.solution[0].abs() leq Flt64(1e-5))

        val solver3 = GurobiQuadraticSolver()
        val result3 = runBlocking { solver3(model) }
        assert(result3.value!!.solution[0] eq Flt64.zero)
    }

    @Test
    fun notConvexQcqp() {
        val x = RealVar("x")
        x.range.leq(Flt64.two)
        x.range.geq(-Flt64.two)

        val model = QuadraticMetaModel()
        model.add(x)
        model.maximize(x * x + x)
        model.addConstraint(x * x leq Flt64(4.0))

        val solver1 = ScipQuadraticSolver()
        val result1 = runBlocking { solver1(model) }
        assert(result1.value!!.solution[0] eq Flt64.two)

        val solver2 = CplexQuadraticSolver()
        val result2 = runBlocking { solver2(model) }
        assert(result2.failed)

        val solver3 = GurobiQuadraticSolver()
        val result3 = runBlocking { solver3(model) }
        assert(result3.value!!.solution[0] eq Flt64.two)
    }

    @Test
    fun test() {
        val x = UIntVariable2("x", Shape2(6, 12))
        val price = flatMap("price", 0 until 12, { 5 * x[0, it] + 10 * x[1, it] + 50 * x[2, it] + 100 * x[3, it] + 200 * x[4, it] + 500 * x[5, it] }, { "price_${it.first}" })
        val model = LinearMetaModel()
        model.add(x)
        model.add(price)

        for (i in 0 until 11) {
            model.addConstraint(price[i] eq price[i + 1])
        }
        for (i in 0 until 12) {
            model.addConstraint(x[0, i] geq 1)
        }
        model.addConstraint(sum(x[0, _a]) leq 24)
        model.addConstraint(sum(x[1, _a]) leq 24)
        model.addConstraint(sum(x[2, _a]) leq 20)
        model.addConstraint(sum(x[3, _a]) leq 10)
        model.addConstraint(sum(x[4, _a]) leq 10)
        model.addConstraint(sum(x[5, _a]) leq 10)

        model.maximize(sum(x[_a, _a]))

        val solver = ScipLinearSolver()
        val result = runBlocking {
            solver(model)
        }
        model.setSolution(result.value!!.solution)

        for (i in 0 until 12) {
            val results = (0 until 6).map { model.tokens.find(x[it, i])!!.result!! }
            println(results.joinToString(",") { it.toUInt64().toString() })
        }
    }
}
