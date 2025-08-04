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

class MinLowPayload(
    private val aircraftModel: AircraftModel,
    val points: List<Point>,
    private val totalWeight: TotalWeight
) {
    data class Point(
        val minLowPayload: Quantity<Flt64>,
        val zfw: Quantity<Flt64>
    )

    private val piecewise by lazy {
        UnivariateLinearPiecewiseFunction(
            // 占位以使用里面的插值方法
            x = LinearPolynomial(),
            points = points.map {
                Point2(
                    it.zfw.to(aircraftModel.weightUnit)!!.value,
                    it.minLowPayload.to(aircraftModel.weightUnit)!!.value
                )
            },
            name = "min_low_payload"
        )
    }

    lateinit var minLowPayload: QuantityLinearIntermediateSymbol

    operator fun invoke(zfw: Quantity<Flt64>): Quantity<Flt64> {
        return Quantity(
            piecewise.y(zfw.to(aircraftModel.weightUnit)!!.value)!!,
            aircraftModel.weightUnit
        )
    }

    fun register(
        model: AbstractLinearMetaModel
    ): Try {
        if (!::minLowPayload.isInitialized) {
            val zfw = totalWeight.computedTotalWeight[FlightPhase.ZeroFuel]
            minLowPayload = if (zfw != null) {
                // 预配载、全配载模式下，无油总重是个确定值，所以可以通过线性插值直接求出最小下货舱业载
                QuantityLinearIntermediateSymbol(
                    LinearExpressionSymbol(
                        this(zfw).to(aircraftModel.weightUnit)!!.value,
                        name = "min_low_payload"
                    ),
                    aircraftModel.weightUnit
                )
            } else {
                QuantityLinearIntermediateSymbol(
                    UnivariateLinearPiecewiseFunction(
                        x = LinearPolynomial(totalWeight.estimateTotalWeight[FlightPhase.ZeroFuel]!!.to(aircraftModel.weightUnit)!!.value),
                        points = points.map {
                            Point2(
                                it.zfw.to(aircraftModel.weightUnit)!!.value,
                                it.minLowPayload.to(aircraftModel.weightUnit)!!.value
                            )
                        },
                        name = "min_low_payload"
                    ),
                    aircraftModel.weightUnit
                )
            }
        }
        when (val result = model.add(minLowPayload)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
