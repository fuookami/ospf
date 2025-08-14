package fuookami.ospf.kotlin.example

import kotlinx.coroutines.*
import org.junit.jupiter.api.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.math.geometry.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

class LinearPiecewiseTest {
    @Test
    fun and1() {
        val x = UIntVar("x")
        x.range.leq(UInt64.one)
        val y = UIntVar("y")
        y.range.leq(UInt64.two)
        val and = AndFunction(listOf(LinearPolynomial(x), LinearPolynomial(y)), "and")
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

    @Test
    fun bin1() {
        val x = URealVar("x")
        x.range.leq(Flt64.two)
        val bin = BinaryzationFunction(LinearPolynomial(x), name = "bin")
        val solver = ScipLinearSolver()

        val model1 = LinearMetaModel()
        model1.add(x)
        model1.add(bin)
        model1.minimize(bin)
        val result1 = runBlocking { solver(model1) }
        assert(result1.value!!.obj eq Flt64.zero)

        val model2 = LinearMetaModel()
        model2.add(x)
        model2.add(bin)
        model2.maximize(bin)
        val result2 = runBlocking { solver(model2) }
        assert(result2.value!!.obj eq Flt64.one)

        val model3 = LinearMetaModel()
        model3.add(x)
        model3.add(bin)
        model3.addConstraint(x eq Flt64.zero)
        model3.maximize(bin)
        val result3 = runBlocking { solver(model3) }
        assert(result3.value!!.obj eq Flt64.zero)

        val model4 = LinearMetaModel()
        model4.add(x)
        model4.add(bin)
        model4.addConstraint(x eq Flt64(0.3))
        model4.maximize(bin)
        val result4 = runBlocking { solver(model4) }
        assert(result4.value!!.obj eq Flt64.one)
    }

    @Test
    fun bin2() {
        val x = UIntVar("x")
        x.range.leq(UInt64.two)
        val bin = BinaryzationFunction(LinearPolynomial(x), name = "bin")
        val solver = ScipLinearSolver()

        val model1 = LinearMetaModel()
        model1.add(x)
        model1.add(bin)
        model1.minimize(bin)
        val result1 = runBlocking { solver(model1) }
        assert(result1.value!!.obj eq Flt64.zero)

        val model2 = LinearMetaModel()
        model2.add(x)
        model2.add(bin)
        model2.maximize(bin)
        val result2 = runBlocking { solver(model2) }
        assert(result2.value!!.obj eq Flt64.one)

        val model3 = LinearMetaModel()
        model3.add(x)
        model3.add(bin)
        model3.addConstraint(x eq Flt64.zero)
        model3.maximize(bin)
        val result3 = runBlocking { solver(model3) }
        assert(result3.value!!.obj eq Flt64.zero)

        val model4 = LinearMetaModel()
        model4.add(x)
        model4.add(bin)
        model4.addConstraint(x eq Flt64.zero)
        model4.minimize(bin)
        val result4 = runBlocking { solver(model4) }
        assert(result4.value!!.obj eq Flt64.zero)
    }

    @Test
    fun bter1() {
        val x = RealVar("x")
        x.range.leq(Flt64.two)
        x.range.geq(-Flt64.two)
        val bter = BalanceTernaryzationFunction(LinearPolynomial(x), name = "bter")
        val solver = ScipLinearSolver()

        val model1 = LinearMetaModel()
        model1.add(x)
        model1.add(bter)
        model1.minimize(bter)
        val result1 = runBlocking { solver(model1) }
        assert(result1.value!!.obj eq -Flt64.one)

        val model2 = LinearMetaModel()
        model2.add(x)
        model2.add(bter)
        model2.maximize(bter)
        val result2 = runBlocking { solver(model2) }
        assert(result2.value!!.obj eq Flt64.one)

        val model3 = LinearMetaModel()
        model3.add(x)
        model3.add(bter)
        model3.addConstraint(x geq Flt64.zero)
        model3.minimize(bter)
        val result3 = runBlocking { solver(model3) }
        assert(result3.value!!.obj eq Flt64.zero)

        val model4 = LinearMetaModel()
        model4.add(x)
        model4.add(bter)
        model4.addConstraint(x geq Flt64.zero)
        model4.maximize(bter)
        val result4 = runBlocking { solver(model4) }
        assert(result4.value!!.obj eq Flt64.one)

        val model5 = LinearMetaModel()
        model5.add(x)
        model5.add(bter)
        model5.addConstraint(x leq 0)
        model5.minimize(bter)
        val result5 = runBlocking { solver(model5) }
        assert(result5.value!!.obj eq -Flt64.one)

        val model6 = LinearMetaModel()
        model6.add(x)
        model6.add(bter)
        model6.addConstraint(x leq 0)
        model6.maximize(bter)
        val result6 = runBlocking { solver(model6) }
        assert(result6.value!!.obj eq Flt64.zero)

        val model7 = LinearMetaModel()
        model7.add(x)
        model7.add(bter)
        model7.addConstraint(x leq Flt64(0.3))
        model7.maximize(bter)
        val result7 = runBlocking { solver(model7) }
        assert(result7.value!!.obj eq Flt64.one)

        val model8 = LinearMetaModel()
        model8.add(x)
        model8.add(bter)
        model8.addConstraint(x geq -Flt64(0.3))
        model8.minimize(bter)
        val result8 = runBlocking { solver(model8) }
        assert(result8.value!!.obj eq -Flt64.one)
    }

    @Test
    fun bter2() {
        val x = IntVar("x")
        x.range.leq(Int64.two)
        x.range.geq(-Int64.two)
        val bter = BalanceTernaryzationFunction(LinearPolynomial(x), name = "bter")
        val solver = ScipLinearSolver()

        val model1 = LinearMetaModel()
        model1.add(x)
        model1.add(bter)
        model1.minimize(bter)
        val result1 = runBlocking { solver(model1) }
        assert(result1.value!!.obj eq -Flt64.one)

        val model2 = LinearMetaModel()
        model2.add(x)
        model2.add(bter)
        model2.maximize(bter)
        val result2 = runBlocking { solver(model2) }
        assert(result2.value!!.obj eq Flt64.one)

        val model3 = LinearMetaModel()
        model3.add(x)
        model3.add(bter)
        model3.addConstraint(x geq Flt64.zero)
        model3.minimize(bter)
        val result3 = runBlocking { solver(model3) }
        assert(result3.value!!.obj eq Flt64.zero)

        val model4 = LinearMetaModel()
        model4.add(x)
        model4.add(bter)
        model4.addConstraint(x geq Flt64.zero)
        model4.maximize(bter)
        val result4 = runBlocking { solver(model4) }
        assert(result4.value!!.obj eq Flt64.one)

        val model5 = LinearMetaModel()
        model5.add(x)
        model5.add(bter)
        model5.addConstraint(x leq 0)
        model5.minimize(bter)
        val result5 = runBlocking { solver(model5) }
        assert(result5.value!!.obj eq -Flt64.one)

        val model6 = LinearMetaModel()
        model6.add(x)
        model6.add(bter)
        model6.addConstraint(x leq 0)
        model6.maximize(bter)
        val result6 = runBlocking { solver(model6) }
        assert(result6.value!!.obj eq Flt64.zero)
    }

    @Test
    fun semi() {
        val model = LinearMetaModel()

        val x = URealVar("x")
        x.range.leq(Flt64.two)
        model.add(x)

        val y = URealVar("y")
        y.range.geq(Flt64.three)
        model.add(y)

        val semi = SemiRealFunction(LinearPolynomial(x - y), name = "semi")
        model.add(semi)

        model.minimize(semi)

        val solver = ScipLinearSolver()
        val result = runBlocking { solver(model) }
        assert(result.value!!.obj eq Flt64.zero)
    }

    @Test
    fun univariate() {
        val model = LinearMetaModel()

        val x = URealVar("x")
        x.range.leq(Flt64.two)
        model.add(x)

        val ulp = UnivariateLinearPiecewiseFunction(
            x = LinearPolynomial(x),
            points = listOf(
                point2(),
                point2(x = Flt64.one, y = Flt64.two),
                point2(x = Flt64.two, y = Flt64.one)
            ),
            name = "y"
        )
        model.add(ulp)

        model.maximize(LinearPolynomial(ulp))

        val solver = ScipLinearSolver()
        val result = runBlocking { solver(model) }
        assert(result.value!!.solution[0] eq Flt64.one)
    }

    @Test
    fun bivariate() {
        val model = LinearMetaModel()

        val x = URealVar("x")
        val y = URealVar("y")
        x.range.leq(Flt64.two)
        y.range.leq(Flt64.two)
        model.add(x)
        model.add(y)

        val blp = BivariateLinearPiecewiseFunction(
            x = LinearPolynomial(x),
            y = LinearPolynomial(y),
            points = listOf(
                point3(),
                point3(x = Flt64.two),
                point3(y = Flt64.two),
                point3(x = Flt64.two, y = Flt64.two),
                point3(x = Flt64.one, y = Flt64.one, z = Flt64.one)
            ),
            name = "z"
        )
        model.add(blp)

        model.maximize(LinearPolynomial(blp))

        val solver = ScipLinearSolver()
        val result = runBlocking { solver(model) }
        assert(result.value!!.solution[0] eq Flt64.one)
        assert(result.value!!.solution[1] eq Flt64.one)
    }
}
