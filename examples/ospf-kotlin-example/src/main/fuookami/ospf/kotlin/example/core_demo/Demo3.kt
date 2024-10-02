package fuookami.ospf.kotlin.example.core_demo

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.error.*
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

data object Demo3 {
    data class Product(val minYield: Flt64) : AutoIndexed(Product::class)

    data class Material(
        val cost: Flt64,
        val yieldValue: Map<Product, Flt64>
    ) : AutoIndexed(Material::class)

    private val products = listOf(
        Product(Flt64(15000.0)),
        Product(Flt64(15000.0)),
        Product(Flt64(10000.0))
    )
    private val materials = listOf(
        Material(
            Flt64(115.0), mapOf(
                products[0] to Flt64(30.0),
                products[1] to Flt64(10.0)
            )
        ),
        Material(
            Flt64(97.0), mapOf(
                products[0] to Flt64(15.0),
                products[2] to Flt64(20.0)
            )
        ),
        Material(
            Flt64(82.0), mapOf(
                products[1] to Flt64(25.0),
                products[2] to Flt64(15.0)
            )
        ),
        Material(
            Flt64(76.0), mapOf(
                products[0] to Flt64(15.0),
                products[1] to Flt64(15.0),
                products[2] to Flt64(15.0)
            )
        )
    )

    private lateinit var x: UIntVariable1

    private lateinit var cost: LinearSymbol
    private lateinit var yieldSymbols: LinearSymbols1

    private val metaModel = LinearMetaModel("demo3")

    private val subProcesses = listOf(
        Demo3::initVariable,
        Demo3::initSymbol,
        Demo3::initObject,
        Demo3::initConstraint,
        Demo3::solve,
        Demo3::analyzeSolution
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
        x = UIntVariable1("x", Shape1(materials.size))
        for (c in materials) {
            x[c].name = "${x.name}_${c.index}"
        }
        metaModel.add(x)
        return ok
    }

    private suspend fun initSymbol(): Try {
        cost = LinearExpressionSymbol(sum(materials) { it.cost * x[it] }, "cost")
        metaModel.add(cost)

        yieldSymbols = LinearSymbols1("yield", Shape1(products.size)) { p, _ ->
            val product = products[p]
            LinearExpressionSymbol(
                sum(materials.filter { it.yieldValue.contains(product) }) { m ->
                    m.yieldValue[product]!! * x[m]
                },
                "yieldProduct_${p}"
            )
        }
        metaModel.add(yieldSymbols)

        return ok
    }

    private suspend fun initObject(): Try {
        metaModel.minimize(cost)
        return ok
    }

    private suspend fun initConstraint(): Try {
        for (p in products) {
            metaModel.addConstraint(yieldSymbols[p.index] geq p.minYield)
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
        val ret = HashMap<Material, UInt64>()
        for (token in metaModel.tokens.tokens) {
            if (token.result!! eq Flt64.one
                && token.variable.belongsTo(x)
            ) {
                ret[materials[token.variable.vectorView[0]]] = token.result!!.round().toUInt64()
            }
        }
        return ok
    }
}
