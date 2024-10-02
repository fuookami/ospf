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

data object Demo16 {
    data class Produce(
        val month: UInt64,
        val productivity: UInt64,
        val demand: UInt64
    ) : AutoIndexed(Produce::class)

    val productPrice: Flt64 = Flt64(40.0)
    val delayDeliveryPrice: Flt64 = Flt64(2.0)
    val stowagePrice: Flt64 = Flt64(0.5)

    val produces = listOf(
        Produce(UInt64(3), UInt64(50), UInt64(100)),
        Produce(UInt64(4), UInt64(180), UInt64(200)),
        Produce(UInt64(5), UInt64(280), UInt64(180)),
        Produce(UInt64(6), UInt64(270), UInt64(300))
    )

    lateinit var x: UIntVariable2

    lateinit var produce: LinearSymbols1
    lateinit var supply: LinearSymbols1
    lateinit var delayDeliveryCost: LinearSymbol
    lateinit var stowageCost: LinearSymbol
    lateinit var produceCost: LinearSymbol

    val metaModel = LinearMetaModel("demo16")

    private val subProcesses = listOf(
        Demo16::initVariable,
        Demo16::initSymbol,
        Demo16::initObject,
        Demo16::initConstraint,
        Demo16::solve,
        Demo16::analyzeSolution
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
        x = UIntVariable2("x", Shape2(produces.size, produces.size))
        metaModel.add(x)
        return ok
    }

    private suspend fun initSymbol(): Try {
        produce = LinearSymbols1("produce", Shape1(produces.size)) { i, _ ->
            val p = produces[i]
            LinearExpressionSymbol(sum(x[p, _a]), "produce_${p.month}")
        }
        metaModel.add(produce)
        supply = LinearSymbols1("supply", Shape1(produces.size)) { i, _ ->
            val p = produces[i]
            LinearExpressionSymbol(sum(x[_a, p]), "supply_${p.month}")
        }
        metaModel.add(supply)
        delayDeliveryCost = LinearExpressionSymbol(sum(produces.withIndex().flatMap { (i, _) ->
            produces.withIndex().mapNotNull { (j, _) ->
                if (i < j) {
                    Flt64(j - i) * delayDeliveryPrice * x[j, i]
                } else {
                    null
                }
            }
        }), "delay_delivery_cost")
        metaModel.add(delayDeliveryCost)
        stowageCost = LinearExpressionSymbol(sum(produces.withIndex().flatMap { (i, _) ->
            produces.withIndex().mapNotNull { (j, _) ->
                if (i < j) {
                    Flt64(j - i) * stowagePrice * x[i, j]
                } else {
                    null
                }
            }
        }), "storage_cost")
        metaModel.add(stowageCost)
        produceCost = LinearExpressionSymbol(productPrice * sum(x[_a, _a]), "produce_cost")
        metaModel.add(produceCost)
        return ok
    }

    private suspend fun initObject(): Try {
        metaModel.minimize(delayDeliveryCost + stowageCost + produceCost, "cost")
        return ok
    }

    private suspend fun initConstraint(): Try {
        for (p in produces) {
            metaModel.addConstraint(
                supply[p] geq p.demand,
                "demand_${p.month}"
            )
        }
        for (p in produces) {
            metaModel.addConstraint(
                produce[p] leq p.productivity,
                "productivity_${p.month}"
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
        val produce = HashMap<UInt64, HashMap<UInt64, UInt64>>()
        for (token in metaModel.tokens.tokens) {
            if (token.result!! geq Flt64.one && token.variable belongsTo x) {
                val vector = token.variable.vectorView
                val i = UInt64(vector[0])
                val j = UInt64(vector[1])
                produce.getOrPut(i) { HashMap() }[j] = token.result!!.round().toUInt64()
            }
        }
        return ok
    }
}
