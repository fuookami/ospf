package fuookami.ospf.kotlin.example.core_demo

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.concept.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

data object Demo1 {
    data class Company(
        val capital: Flt64,
        val liability: Flt64,
        val profit: Flt64
    ) : AutoIndexed(Company::class)

    private val companies: List<Company> = listOf(
        Company(Flt64(3.48), Flt64(1.28), Flt64(5400.0)),
        Company(Flt64(5.62), Flt64(2.53), Flt64(2300.0)),
        Company(Flt64(7.33), Flt64(1.02), Flt64(4600.0)),
        Company(Flt64(6.27), Flt64(3.55), Flt64(3300.0)),
        Company(Flt64(2.14), Flt64(0.53), Flt64(980.0))
    )
    private val minCapital: Flt64 = Flt64(10.0)
    private val maxLiability: Flt64 = Flt64(5.0)

    private lateinit var x: BinVariable1
    private lateinit var capital: LinearExpressionSymbol
    private lateinit var liability: LinearExpressionSymbol
    private lateinit var profit: LinearExpressionSymbol

    private val metaModel: LinearMetaModel = LinearMetaModel("demo1")

    private val subProcesses = listOf(
        Demo1::initVariable,
        Demo1::initSymbol,
        Demo1::initObject,
        Demo1::initConstraint,
        Demo1::solve,
        Demo1::analyzeSolution
    )

    suspend operator fun invoke(): Try {
        for (process in subProcesses) {
            when (val result = process()) {
                is Failed -> {
                    return Failed(result.error)
                }

                else -> {}
            }
        }
        return ok
    }

    private suspend fun initVariable(): Try {
        x = BinVariable1("x", Shape1(companies.size))
        for (c in companies) {
            x[c].name = "${x.name}_${c.index}"
        }
        metaModel.addVars(x)
        return ok
    }

    private suspend fun initSymbol(): Try {
        capital = LinearExpressionSymbol(sum(companies) { it.capital * x[it] }, "capital")
        metaModel.addSymbol(capital)

        liability = LinearExpressionSymbol(sum(companies) { it.liability * x[it] }, "liability")
        metaModel.addSymbol(liability)

        profit = LinearExpressionSymbol(sum(companies) { it.profit * x[it] }, "profit")
        metaModel.addSymbol(profit)
        return ok
    }

    private suspend fun initObject(): Try {
        metaModel.maximize(profit)
        return ok
    }

    private suspend fun initConstraint(): Try {
        metaModel.addConstraint(capital geq minCapital)
        metaModel.addConstraint(liability leq maxLiability)
        return ok
    }

    private suspend fun solve(): Try {
        val solver = SCIPLinearSolver()
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
        val ret = ArrayList<Company>()
        for (token in metaModel.tokens.tokens) {
            if (token.result!! eq Flt64.one) {
                ret.add(companies[token.variable.index])
            }
        }
        return ok
    }
}
