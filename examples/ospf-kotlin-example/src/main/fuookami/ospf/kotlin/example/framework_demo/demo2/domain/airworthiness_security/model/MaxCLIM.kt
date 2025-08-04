package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.airworthiness_security.model

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.math.geometry.*
import fuookami.ospf.kotlin.utils.physics.quantity.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*

class MaxCLIM(
    private val aircraftModel: AircraftModel,
    val points: List<Point>,
    private val totalWeight: TotalWeight
) {
    data class Point(
        val tow: Quantity<Flt64>,
        val maxCLIM: Quantity<Flt64>
    )

    private val piecewise by lazy {
        UnivariateLinearPiecewiseFunction(
            // 占位以使用里面的插值方法
            x = LinearPolynomial(),
            points = points.map {
                Point2(
                    it.tow.to(aircraftModel.weightUnit)!!.value,
                    it.maxCLIM.to(aircraftModel.torqueUnit)!!.value
                )
            },
            name = "max_clim"
        )
    }

    operator fun invoke(tow: Quantity<Flt64>): Quantity<Flt64> {
        return Quantity(
            piecewise.y(tow.to(aircraftModel.weightUnit)!!.value)!!,
            aircraftModel.torqueUnit
        )
    }

    lateinit var maxCLIM: QuantityLinearIntermediateSymbol

    fun register(
        model: AbstractLinearMetaModel
    ): Try {
        if (!::maxCLIM.isInitialized) {
            val tow = totalWeight.computedTotalWeight[FlightPhase.TakeOff]
            maxCLIM = if (tow != null) {
                Quantity(
                    LinearExpressionSymbol(
                        this(tow).to(aircraftModel.torqueUnit)!!.value,
                        name = "max_clim"
                    ),
                    aircraftModel.torqueUnit
                )
            } else {
                Quantity(
                    UnivariateLinearPiecewiseFunction(
                        x = LinearPolynomial(totalWeight.estimateTotalWeight[FlightPhase.TakeOff]!!.to(aircraftModel.torqueUnit)!!.value),
                        points = points.map {
                            Point2(
                                it.tow.to(aircraftModel.weightUnit)!!.value,
                                it.maxCLIM.to(aircraftModel.torqueUnit)!!.value
                            )
                        },
                        name = "max_clim"
                    ),
                    aircraftModel.torqueUnit
                )
            }
        }
        when (val result = model.add(maxCLIM)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
