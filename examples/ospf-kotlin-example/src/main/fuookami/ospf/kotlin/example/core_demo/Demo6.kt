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

data object Demo6 {
    data class Cargo(
        val weight: UInt64,
        val value: UInt64,
        val amount: UInt64
    ) : AutoIndexed(Cargo::class)

    private val cargos = listOf(
        Cargo(UInt64(1), UInt64(6), UInt64(10)),
        Cargo(UInt64(2), UInt64(10), UInt64(10)),
        Cargo(UInt64(2), UInt64(20), UInt64(10))
    )
    private val maxWeight = UInt64(10)

    private lateinit var x: UIntVariable1

    private lateinit var cargoWeight: LinearSymbol
    private lateinit var cargoValue: LinearSymbol

    private val metaModel = LinearMetaModel("demo6")

    private val subProcesses = listOf(
        Demo6::initVariable,
        Demo6::initSymbol,
        Demo6::initObject,
        Demo6::initConstraint,
        Demo6::solve,
        Demo6::analyzeSolution
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
        x = UIntVariable1("x", Shape1(cargos.size))
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
        for(c in cargos){
            x[c].range.ls(c.amount)
        }

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
        val ret = HashMap<Cargo, UInt64>()
        for (token in metaModel.tokens.tokens) {
            if (token.result!! geq Flt64.one && token.variable.belongsTo(x)) {
                ret[cargos[token.variable.vectorView[0]]] = token.result!!.round().toUInt64()
            }
        }
        return ok
    }
}
