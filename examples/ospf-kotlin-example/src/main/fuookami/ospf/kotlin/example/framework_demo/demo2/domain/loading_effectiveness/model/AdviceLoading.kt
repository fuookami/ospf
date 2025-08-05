package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.loading_effectiveness.model

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.physics.quantity.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.Position

class AdviceLoading(
    private val aircraftModel: AircraftModel,
    private val positions: List<Position>,
    private val load: Load
) {
    lateinit var amountSlack: LinearIntermediateSymbols1
    lateinit var weightSlack: LinearIntermediateSymbols1

    fun register(
        model: AbstractLinearMetaModel
    ): Try {
        if (!::amountSlack.isInitialized) {
            amountSlack = LinearIntermediateSymbols1("advice_amount_slack", Shape1(positions.size)) { j, _ ->
                val position = positions[j]
                if (position.ala != null) {
                    SlackFunction(
                        x = LinearPolynomial(load.loadAmount[j]),
                        threshold = LinearPolynomial(position.ala!!),
                        withPositive = true,
                        name = "advice_amount_slack_${position}"
                    )
                } else {
                    LinearExpressionSymbol(
                        Flt64.zero,
                        "advice_amount_slack_${position}",
                    )
                }
            }
        }
        when (val result = model.add(amountSlack)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        if (!::weightSlack.isInitialized) {
            weightSlack = LinearIntermediateSymbols1("advice_weight_slack", Shape1(positions.size)) { j, _ ->
                val position = positions[j]
                if (position.alw != null) {
                    SlackFunction(
                        x = LinearPolynomial(load.estimateLoadWeight[j].to(aircraftModel.weightUnit)!!.value),
                        threshold = LinearPolynomial(position.alw!!.to(aircraftModel.weightUnit)!!.value),
                        withPositive = true,
                        name = "advice_weight_slack_${position}"
                    )
                } else {
                    LinearExpressionSymbol(
                        Flt64.zero,
                        "advice_weight_slack_${position}",
                    )
                }
            }
        }
        when (val result = model.add(weightSlack)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
