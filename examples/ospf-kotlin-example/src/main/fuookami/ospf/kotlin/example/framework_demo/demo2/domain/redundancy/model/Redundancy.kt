package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.redundancy.model

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.Position

class Redundancy(
    private val aircraftModel: AircraftModel,
    private val flight: Flight,
    private val items: List<Item>,
    private val positions: List<Position>,
    private val stowage: Stowage,
    private val load: Load,
    private val payload: Payload
) {
    lateinit var redundancy: LinearIntermediateSymbol
    lateinit var predicateRedundancy: LinearIntermediateSymbol
    lateinit var redundancySlack: LinearIntermediateSymbol

    val minRedundancy: LinearPolynomial by lazy {
        TODO("not implemented yet")
    }

    val maxRedundancy: LinearPolynomial by lazy {
        TODO("not implemented yet")
    }

    fun register(
        model: AbstractLinearMetaModel
    ): Try {
        if (!::redundancy.isInitialized) {
            val poly = MutableLinearPolynomial()
            for ((i, item) in items.withIndex()) {
                if (!item.location.main) {
                    continue
                }

                when (item.status) {
                    ItemStatus.Reserved -> {
                        poly -= 1
                    }

                    ItemStatus.Optional -> {
                        poly -= stowage.loaded[i]
                    }

                    else -> {}
                }
            }
            redundancy = LinearExpressionSymbol(
                poly,
                "redundancy"
            )
        }
        when (val result = model.add(redundancy)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        if (!::predicateRedundancy.isInitialized) {
            TODO("not implemented yet")
        }
        when (val result = model.add(predicateRedundancy)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        if (!::redundancySlack.isInitialized) {
            redundancySlack = SlackRangeFunction(
                x = LinearPolynomial(redundancy),
                lb = minRedundancy,
                ub = maxRedundancy,
                name = "redundancy_slack"
            )
        }
        when (val result = model.add(redundancySlack)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
