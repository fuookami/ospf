package fuookami.ospf.kotlin.example

import kotlinx.coroutines.*
import org.junit.jupiter.api.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
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
        model.addVar(x)
        model.maximize((Flt64.one - x) * (Flt64.one - x))

        val solver1 = SCIPQuadraticSolver()
        val result1 = runBlocking { solver1(model) }
        assert(result1.value!!.solution[0] eq -Flt64.two)

        val solver2 = CplexQuadraticSolver()
        val result2 = runBlocking { solver2(model) }
        assert(result2.value!!.solution[0] eq -Flt64.two)

//        val solver3 = GurobiQuadraticSolver()
//        val result3 = runBlocking { solver3(model) }
//        assert(result3.value!!.solution[0] eq -Flt64.two)
    }

    @Test
    fun qcp() {
        val x = RealVar("x")
        x.range.leq(Flt64.two)
        x.range.geq(-Flt64.two)

        val model = QuadraticMetaModel()
        model.addVar(x)
        model.maximize(x)
        model.addConstraint(x * x leq Flt64(4.0))

        val solver1 = SCIPQuadraticSolver()
        val result1 = runBlocking { solver1(model) }
        assert(result1.value!!.solution[0] eq Flt64.two)

        val solver2 = CplexQuadraticSolver()
        val result2 = runBlocking { solver2(model) }
        assert(result2.value!!.solution[0] eq Flt64.two)

//        val solver3 = GurobiQuadraticSolver()
//        val result3 = runBlocking { solver3(model) }
//        assert(result3.value!!.solution[0] eq Flt64.two)
    }

    @Test
    fun qcqp() {
        val x = RealVar("x")
        x.range.leq(Flt64.two)
        x.range.geq(-Flt64.two)

        val model = QuadraticMetaModel()
        model.addVar(x)
        model.maximize(x * x)
        model.addConstraint(x * x leq Flt64(4.0))

        model.export("1.opm")

        val himodel = runBlocking { QuadraticModel(model).value!! }
        val limodel = runBlocking { QuadraticTetradModel(himodel) }
        limodel.export("1.lp", ModelFileFormat.LP)

        val solver1 = SCIPQuadraticSolver()
        val result1 = runBlocking { solver1(model) }
        assert(result1.value!!.solution[0] eq Flt64.two)

        val solver2 = CplexQuadraticSolver()
        val result2 = runBlocking { solver2(model) }
        assert(result2.value!!.solution[0] eq Flt64.two)
    }
}
