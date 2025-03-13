package fuookami.ospf.kotlin.example.core_demo

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.concept.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

data object Demo12 {
    data class Product(
        val yield: Flt64,
        val risk: Flt64,
        val premium: Flt64,
        val minPremium: Flt64
    ) : AutoIndexed(Product::class)

    val products = listOf(
        Product(Flt64(0.28), Flt64(0.04), Flt64(0.08), Flt64(103.0)),
        Product(Flt64(0.21), Flt64(0.015), Flt64(0.02), Flt64(198.0)),
        Product(Flt64(0.23), Flt64(0.05), Flt64(0.045), Flt64(52.0)),
        Product(Flt64(0.25), Flt64(0.026), Flt64(0.04), Flt64(40.0)),
        Product(Flt64(0.05), Flt64(0.0), Flt64(0.0), Flt64(0.0))
    )
    val funds = Flt64(1000000.0)

    lateinit var x: UIntVariable1

    lateinit var premium: LinearSymbols1
    lateinit var yield: LinearIntermediateSymbol

    val metaModel = LinearMetaModel("demo12")

    private val subProcesses = listOf(
        Demo12::initVariable,
        Demo12::initSymbol,
        Demo12::initObject,
        Demo12::initConstraint,
        Demo12::solve,
        Demo12::analyzeSolution
    )

    suspend operator fun invoke(): Try {
        for (process in subProcesses) {
            when (val result = process()) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }
        return ok
    }

    private suspend fun initVariable(): Try {
        x = UIntVariable1("x", Shape1(products.size))
        metaModel.add(x)
        return ok
    }

    private suspend fun initSymbol(): Try {
        premium = LinearSymbols1("premium", Shape1(products.size)) { i, _ ->
            val product = products[i]
            MaxFunction(
                listOf(
                    LinearPolynomial(product.premium * x[i]),
                    LinearPolynomial(product.minPremium)
                ),
                "premium_$i"
            )
        }
        metaModel.add(premium)
        yield = LinearExpressionSymbol(
            sum(products.map { p -> (p.yield - p.risk) * x[p] - premium[p] }),
            "yield"
        )
        metaModel.add(yield)
        return ok
    }

    private suspend fun initObject(): Try {
        metaModel.maximize(yield, "yield")
        return ok
    }

    private suspend fun initConstraint(): Try {
        metaModel.addConstraint(
            sum(products.map { p -> x[p] + premium[p] }) eq funds,
            "pay"
        )
        return ok
    }

    private suspend fun solve(): Try {
        val solver = ScipLinearSolver()
        when (val ret = solver(metaModel)) {
            is Ok -> {
                metaModel.tokens.setSolution(ret.value.solution)
            }

            is Failed -> {
                return Failed(ret.error)
            }
        }
        return ok
    }

    private suspend fun analyzeSolution(): Try {
        val ret = HashMap<Product, UInt64>()
        for (token in metaModel.tokens.tokens) {
            if (token.result!! geq Flt64.one && token.variable.belongsTo(x)) {
                val vector = token.variable.vectorView
                val product = products[vector[0]]
                ret[product] = token.result!!.round().toUInt64()
            }
        }
        return ok
    }
}
