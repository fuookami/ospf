package fuookami.ospf.kotlin.example.framework_demo.demo4.domain.passenger.model

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.domain.task.model.*

class PassengerAmount(
    private val flights: List<FlightTask>,
    private val passengers: Map<FlightTask, List<FlightPassenger>>,
    private val cancel: PassengerCancel,
    private val change: PassengerChange
) {
    lateinit var passengerAmount: Map<FlightTask, Map<PassengerClass, LinearIntermediateSymbol>>

    fun register(model: AbstractLinearMetaModel): Try {
        if (!::passengerAmount.isInitialized) {
            passengerAmount = flights.associateWith { flight ->
                PassengerClass.entries.associateWith { cls ->
                    val poly = MutableLinearPolynomial()
                    for (passenger in (passengers[flight] ?: emptyList())) {
                        if (passenger.cls == cls) {
                            poly += passenger.amount
                        }
                        poly -= cancel.passengerCancel[passenger]
                        if (passenger.cls == cls) {
                            poly -= sum(change.passengerClassChange[passenger, _a])
                            poly -= sum(change.passengerFlightChange[passenger, _a, _a])
                        } else {
                            poly += change.passengerClassChange[passenger, cls]!!
                        }
                    }
                    for (passenger in passengers.values.flatten()) {
                        val toFlights = change.toFlights[passenger.flight] ?: emptyList()
                        if (toFlights.contains(flight)) {
                            poly += change.passengerFlightChange[passenger, flight, cls]!!
                        }
                    }
                    LinearExpressionSymbol(
                        poly,
                        "passenger_amount_${flight}_${cls}"
                    )
                }
            }
        }
        when (val result = model.add(passengerAmount)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
