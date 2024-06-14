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

data object Demo7 {
    data class Store(
        val demand: Flt64
    ) : AutoIndexed(Store::class)

    data class Warehouse(
        val stowage: Flt64,
        val cost: Map<Store, Flt64>
    ) : AutoIndexed(Warehouse::class)

    private val stores = listOf(
        Store(Flt64(200.0)),
        Store(Flt64(400.0)),
        Store(Flt64(600.0)),
        Store(Flt64(300.0))
    )
    private val warehouses = listOf(
        Warehouse(
            Flt64(510.0), mapOf(
                stores[0] to Flt64(12.0),
                stores[1] to Flt64(13.0),
                stores[2] to Flt64(21.0),
                stores[3] to Flt64(7.0)
            )
        ),
        Warehouse(
            Flt64(470.0), mapOf(
                stores[0] to Flt64(14.0),
                stores[1] to Flt64(17.0),
                stores[2] to Flt64(8.0),
                stores[3] to Flt64(18.0)
            )
        ),
        Warehouse(
            Flt64(520.0), mapOf(
                stores[0] to Flt64(10.0),
                stores[1] to Flt64(11.0),
                stores[2] to Flt64(9.0),
                stores[3] to Flt64(15.0)
            )
        )
    )

    private lateinit var x: UIntVariable2

    private lateinit var cost: LinearSymbol
    private lateinit var shipment: LinearSymbols1
    private lateinit var purchase: LinearSymbols1

    private val metaModel: LinearMetaModel = LinearMetaModel("demo7")

    private val subProcesses = listOf(
        Demo7::initVariable,
        Demo7::initSymbol,
        Demo7::initObject,
        Demo7::initConstraint,
        Demo7::solve,
        Demo7::analyzeSolution
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
        x = UIntVariable2("x", Shape2(warehouses.size, stores.size))
        for (w in warehouses) {
            for (s in stores) {
                x[w, s].name = "${x.name}_(${w.index},${s.index})"
            }
        }
        metaModel.add(x)
        return ok
    }

    private suspend fun initSymbol(): Try {
        cost = LinearExpressionSymbol(sum(warehouses.map { w ->
            sum(stores.filter { w.cost.contains(it) }.map { s ->
                w.cost[s]!! * x[w, s]
            })
        }), "cost")
        metaModel.add(cost)

        shipment = LinearSymbols1(
            "shipment",
            Shape1(warehouses.size)
        ) { i, _ ->
            val w = warehouses[i]
            LinearExpressionSymbol(
                sum(stores.filter { w.cost.contains(it) }.map { s -> x[w, s] }),
                "shipment_${w.index}"
            )
        }
        metaModel.add(shipment)

        purchase = LinearSymbols1(
            "purchase",
            Shape1(stores.size)
        ) { i, _ ->
            val s = stores[i]
            LinearExpressionSymbol(
                sum(warehouses.filter { w -> w.cost.contains(s) }.map { w -> x[w, s] }),
                "purchase_${s.index}"
            )
        }
        metaModel.add(purchase)
        return ok
    }

    private suspend fun initObject(): Try {
        metaModel.minimize(LinearPolynomial(cost),"cost")
        return ok
    }

    private suspend fun initConstraint(): Try {
        for(w in warehouses){
            metaModel.addConstraint(
                shipment[w] leq w.stowage,"stowage_${w.index}"
            )
        }

        for(s in stores){
            metaModel.addConstraint(
                purchase[s] geq s.demand ,"demand_${s.index}"
            )
        }
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
        val solution = stores.associateWith { warehouses.associateWith { Flt64.zero }.toMutableMap() }
        for (token in metaModel.tokens.tokens) {
            if (token.result!! geq Flt64.one
                && token.variable.belongsTo(x)
            ) {
                val warehouse = warehouses[token.variable.vectorView[0]]
                val store = stores[token.variable.vectorView[1]]
                solution[store]!![warehouse] = solution[store]!![warehouse]!! + token.result!!
            }
        }
        return ok
    }
}
