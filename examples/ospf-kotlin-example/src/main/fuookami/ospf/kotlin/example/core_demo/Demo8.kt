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

data object Demo8 {
    data class Product(
        val profit: Flt64
    ) : AutoIndexed(Product::class)

    data class Equipment(
        val amount: UInt64,
        val manHours: Map<Product, Flt64>
    ) : AutoIndexed(Equipment::class)

    private val maxManHours = Flt64(2000)
    private val products = listOf(
        Product(Flt64(123.0)),
        Product(Flt64(94.0)),
        Product(Flt64(105.0)),
        Product(Flt64(132.0)),
        Product(Flt64(118.0)),
    )
    private val equipments = listOf(
        Equipment(
            UInt64(12), mapOf(
                products[0] to Flt64(0.23),
                products[1] to Flt64(0.44),
                products[2] to Flt64(0.17),
                products[3] to Flt64(0.08),
                products[4] to Flt64(0.36),
            )
        ),
        Equipment(
            UInt64(14), mapOf(
                products[0] to Flt64(0.13),
                products[2] to Flt64(0.20),
                products[3] to Flt64(0.37),
                products[4] to Flt64(0.19),
            )
        ),
        Equipment(
            UInt64(8), mapOf(
                products[1] to Flt64(0.25),
                products[2] to Flt64(0.34),
                products[4] to Flt64(0.18),
            )
        ),
        Equipment(
            UInt64(6), mapOf(
                products[0] to Flt64(0.55),
                products[1] to Flt64(0.72),
                products[3] to Flt64(0.61)
            )
        )
    )

    private lateinit var x: UIntVariable1

    private lateinit var profit: LinearIntermediateSymbol
    private lateinit var manHours: LinearIntermediateSymbols1

    private val metaModel = LinearMetaModel("demo8")

    private val subProcesses = listOf(
        Demo8::initVariable,
        Demo8::initSymbol,
        Demo8::initObject,
        Demo8::initConstraint,
        Demo8::solve,
        Demo8::analyzeSolution
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
        for (p in products) {
            x[p].name = "${x.name}_${p.index}"
        }
        metaModel.add(x)
        return ok
    }

    private suspend fun initSymbol(): Try {
        profit = LinearExpressionSymbol(sum(products.map { p ->
            p.profit * x[p]
        }), "profit")
        metaModel.add(profit)

        manHours = LinearIntermediateSymbols1(
            "man_hours",
            Shape1(equipments.size)
        ) { i, _ ->
            val e = equipments[i]
            LinearExpressionSymbol(
                sum(products.mapNotNull { p -> e.manHours[p]?.let { it * x[p] } }),
                "man_hours_${e.index}"
            )
        }
        metaModel.add(manHours)

        return ok
    }

    private suspend fun initObject(): Try {
        metaModel.maximize(profit, "profit")
        return ok
    }

    private suspend fun initConstraint(): Try {
        for (e in equipments) {
            metaModel.addConstraint(
                manHours[e] leq e.amount.toFlt64() * maxManHours,
                "eq_man_hours_${e.index}"
            )
        }
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
            if (token.result!! neq Flt64.one
                && token.variable.belongsTo(x)
            ) {
                ret[products[token.variable.vectorView[0]]] = token.result!!.round().toUInt64()
            }
        }
        return ok
    }
}
