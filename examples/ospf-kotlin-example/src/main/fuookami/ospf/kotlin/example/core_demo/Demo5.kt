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

data object Demo5 {
    data class Cargo(
        val weight: UInt64,
        val value: UInt64
    ) : AutoIndexed(Cargo::class)

    private val cargos = listOf(
        Cargo(UInt64(2U), UInt64(6U)),
        Cargo(UInt64(2U), UInt64(3U)),
        Cargo(UInt64(6U), UInt64(5U)),
        Cargo(UInt64(5U), UInt64(4U)),
        Cargo(UInt64(4U), UInt64(6U))
    )
    private val maxWeight = UInt64(10U)

    private lateinit var x: BinVariable1
    private lateinit var cargoWeight: LinearSymbol
    private lateinit var cargoValue: LinearSymbol

    private val metaModel: LinearMetaModel = LinearMetaModel("demo5")

    private val subProcesses = listOf(
        Demo5::initVariable,
        Demo5::initSymbol,
        Demo5::initObject,
        Demo5::initConstraint,
        Demo5::solve,
        Demo5::analyzeSolution
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
        x = BinVariable1("x", Shape1(cargos.size))
        for (c in cargos) {
            x[c].name = "${x.name}_${c.index}"
        }
        metaModel.add(x)
        return ok
    }

    private suspend fun initSymbol(): Try {
        cargoValue = LinearExpressionSymbol(sum(cargos) { c -> c.value * x[c] }, "value")
        metaModel.add(cargoValue)

        cargoWeight = LinearExpressionSymbol(sum(cargos) { c -> c.weight * x[c] }, "weight")
        metaModel.add(cargoWeight)
        return ok
    }

    private suspend fun initObject(): Try {
        metaModel.maximize(cargoValue,"value")
        return ok
    }

    private suspend fun initConstraint(): Try {
        metaModel.addConstraint(
            cargoWeight leq maxWeight,"weight"
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
        val ret = HashSet<Cargo>()
        for (token in metaModel.tokens.tokens) {
            if (token.result!! eq Flt64.one
                && token.variable.belongsTo(x)
            ) {
                ret.add(cargos[token.variable.vectorView[0]])
            }
        }
        return ok
    }
}
