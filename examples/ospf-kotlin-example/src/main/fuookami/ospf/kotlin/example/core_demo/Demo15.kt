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

data object Demo15 {
    data class CarModel(
        val name: String
    ) : AutoIndexed(CarModel::class)

    data class Replacement(
        val c1: CarModel,
        val c2: CarModel,
        val maximum: Flt64
    )

    data class DistributionCenter(
        val name: String,
        val replacements: List<Replacement>,
        val demands: Map<CarModel, UInt64>
    ) : AutoIndexed(DistributionCenter::class)

    data class Manufacturer(
        val name: String,
        val productivity: Map<CarModel, UInt64>,
        val logisticsCost: Map<DistributionCenter, UInt64>
    ) : AutoIndexed(Manufacturer::class)

    val carModels = listOf(
        CarModel("M1"),
        CarModel("M2"),
        CarModel("M3"),
        CarModel("M4"),
    )

    val distributionCenters = listOf(
        DistributionCenter(
            "丹佛", listOf(
                Replacement(carModels[0], carModels[1], Flt64(0.1)),
                Replacement(carModels[1], carModels[0], Flt64(0.1)),
                Replacement(carModels[2], carModels[3], Flt64(0.2)),
                Replacement(carModels[3], carModels[2], Flt64(0.2))
            ), mapOf(
                carModels[0] to UInt64(700),
                carModels[1] to UInt64(500),
                carModels[2] to UInt64(500),
                carModels[3] to UInt64(600)
            )
        ),
        DistributionCenter(
            "迈阿密", listOf(
                Replacement(carModels[0], carModels[1], Flt64(0.1)),
                Replacement(carModels[1], carModels[0], Flt64(0.1)),
                Replacement(carModels[1], carModels[3], Flt64(0.05)),
                Replacement(carModels[3], carModels[1], Flt64(0.05))
            ), mapOf(
                carModels[0] to UInt64(600),
                carModels[1] to UInt64(500),
                carModels[2] to UInt64(200),
                carModels[3] to UInt64(100)
            )
        )
    )

    val manufacturers = listOf(
        Manufacturer(
            "洛杉矶", mapOf(
                carModels[2] to UInt64(700U),
                carModels[3] to UInt64(300U)
            ), mapOf(
                distributionCenters[0] to UInt64(80),
                distributionCenters[1] to UInt64(215),
            )
        ),
        Manufacturer(
            "底特律", mapOf(
                carModels[0] to UInt64(500U),
                carModels[1] to UInt64(600U),
                carModels[3] to UInt64(400U)
            ), mapOf(
                distributionCenters[0] to UInt64(100),
                distributionCenters[1] to UInt64(108)
            )
        ),
        Manufacturer(
            "新奥尔良", mapOf(
                carModels[0] to UInt64(800U),
                carModels[1] to UInt64(400U)
            ), mapOf(
                distributionCenters[0] to UInt64(102),
                distributionCenters[1] to UInt64(68)
            )
        )
    )

    lateinit var x: UIntVariable3
    lateinit var y: Map<DistributionCenter, PctVariable1>

    lateinit var receive: LinearSymbols2
    lateinit var demand: LinearSymbols2
    lateinit var trans: LinearSymbols2
    lateinit var cost: LinearSymbol

    val metaModel = LinearMetaModel("demo15")

    private val subProcesses = listOf(
        Demo15::initVariable,
        Demo15::initSymbol,
        Demo15::initObject,
        Demo15::initConstraint,
        Demo15::solve,
        Demo15::analyzeSolution
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
        x = UIntVariable3("x", Shape3(manufacturers.size, distributionCenters.size, carModels.size))
        for (m in manufacturers) {
            for (d in distributionCenters) {
                for (c in carModels) {
                    val xi = x[m, d, c]
                    if (m.productivity.containsKey(c)) {
                        xi.name = "x_${m.name}_${d.name}_${c.name}"
                        metaModel.add(xi)
                    } else {
                        xi.range.eq(UInt64.zero)
                    }
                }
            }
        }
        y = distributionCenters.associateWith { d ->
            val y = PctVariable1("y_${d.name}", Shape1(d.replacements.size))
            for ((r, replacement) in d.replacements.withIndex()) {
                val yi = y[r]
                yi.range.leq(replacement.maximum)
                yi.name = "${y.name}_${r}"
            }
            y
        }
        for ((_, yi) in y) {
            metaModel.add(yi)
        }
        return ok
    }

    private suspend fun initSymbol(): Try {
        receive = LinearSymbols2("receive", Shape2(distributionCenters.size, carModels.size)) { _, v ->
            val d = distributionCenters[v[0]]
            val c = carModels[v[1]]
            LinearExpressionSymbol(sum(x[_a, d, c]), "receive_${d.name}_${c.name}")
        }
        metaModel.add(receive)
        demand = LinearSymbols2("demand", Shape2(distributionCenters.size, carModels.size)) { _, v ->
            val d = distributionCenters[v[0]]
            val c = carModels[v[1]]
            val replacedDemand = if (d.demands[c]?.let { it gr UInt64.zero } == true) {
                sum(d.replacements.withIndex().mapNotNull { (r, replacement) ->
                    if (replacement.c1 != c) {
                        return@mapNotNull null
                    }
                    d.demands[c]!!.toFlt64() * y[d]!![r]
                })
            } else {
                LinearPolynomial(Flt64.zero)
            }
            val replacedToDemand = sum(d.replacements.withIndex().mapNotNull { (r, replacement) ->
                if (replacement.c2 != c) {
                    return@mapNotNull null
                }
                d.demands[replacement.c1]?.let {
                    if (it gr UInt64.zero) {
                        it.toFlt64() * y[d]!![r]
                    } else {
                        null
                    }
                }
            })
            LinearExpressionSymbol((d.demands[c] ?: UInt64.zero) - replacedDemand + replacedToDemand, "demand_${d.name}_${c.name}")
        }
        metaModel.add(demand)
        trans = LinearSymbols2("trans", Shape2(manufacturers.size, carModels.size)) { _, v ->
            val m = manufacturers[v[0]]
            val c = carModels[v[1]]
            LinearExpressionSymbol(sum(x[m, _a, c]), "trans_${m.name}_${c.name}")
        }
        metaModel.add(trans)
        cost = LinearExpressionSymbol(sum(manufacturers.flatMap { m ->
            distributionCenters.flatMap { d ->
                m.logisticsCost[d]?.let {
                    carModels.mapNotNull { c ->
                        if (m.productivity.containsKey(c)) {
                            it * x[m, d, c]
                        } else {
                            null
                        }
                    }
                } ?: emptyList()
            }
        }))
        metaModel.add(cost)
        return ok
    }

    private suspend fun initObject(): Try {
        metaModel.minimize(cost, "cost")
        return ok
    }

    private suspend fun initConstraint(): Try {
        for (d in distributionCenters) {
            for (c in carModels) {
                d.demands[c]?.let {
                    metaModel.addConstraint(
                        receive[d, c] geq it,
                        "demand_${d.name}_${c.name}"
                    )
                }
            }
        }
        for (m in manufacturers) {
            for (c in carModels) {
                m.productivity[c]?.let {
                    metaModel.addConstraint(
                        trans[m, c] geq it,
                        "produce_${m.name}_${c.name}"
                    )
                }
            }
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
        val trans: MutableMap<Manufacturer, MutableMap<Pair<DistributionCenter, CarModel>, UInt64>> = HashMap()
        val replacement: MutableMap<DistributionCenter, MutableMap<Pair<CarModel, CarModel>, UInt64>> = HashMap()
        for (token in metaModel.tokens.tokens) {
            if (token.result!! geq Flt64.one && token.variable belongsTo x) {
                val vector = token.variable.vectorView
                val m = manufacturers[vector[0]]
                val d = distributionCenters[vector[1]]
                val c = carModels[vector[2]]
                trans.getOrPut(m) { hashMapOf() }[d to c] = token.result!!.round().toUInt64()
            }
            for ((_, d) in distributionCenters.withIndex()) {
                val yi = y[d]!!
                if (token.result!! neq Flt64.zero && token.variable belongsTo yi) {
                    val vector = token.variable.vectorView
                    val r = d.replacements[vector[0]]
                    replacement.getOrPut(d) { hashMapOf() }[r.c1 to r.c2] = (token.result!! * d.demands[r.c1]!!.toFlt64()).round().toUInt64()
                }
            }
        }
        return ok
    }
}
