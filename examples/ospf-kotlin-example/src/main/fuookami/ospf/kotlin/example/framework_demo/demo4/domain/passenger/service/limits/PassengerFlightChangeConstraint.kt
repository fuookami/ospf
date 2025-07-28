package fuookami.ospf.kotlin.example.framework_demo.demo4.domain.passenger.service.limits

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.gantt_scheduling.infrastructure.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.domain.task.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.domain.bunch_compilation.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.domain.passenger.model.*
import fuookami.ospf.kotlin.utils.functional.get

class PassengerFlightChangeConstraint(
    private val timeWindow: TimeWindow,
    private val passengers: List<FlightPassenger>,
    private val time: TaskTime,
    private val change: PassengerChange,
    override val name: String = "passenger_flight_change_constraint"
) : CGPipeline {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        for (passenger in passengers) {
            if (passenger.prev != null) {
                for (toFlight in change.toFlights[passenger.flight] ?: emptyList()) {
                    val earliestStartTime = time.estimateEndTime[passenger.prev.flight] + timeWindow.valueOf(passenger.prev.flight.arr.passengerTransferTime)
                    val estCondition = IfFunction(
                        earliestStartTime leq time.estimateStartTime[passenger.flight],
                        "${passenger}_${toFlight}_est"
                    )
                    when (val result = model.add(estCondition)) {
                        is Ok -> {}

                        is Failed -> {
                            return Failed(result.error)
                        }
                    }

                    for (cls in PassengerClass.entries) {
                        when (val result = model.addConstraint(
                            change.passengerFlightChange[passenger, toFlight, cls]!! leq passenger.amount * estCondition,
                            "${name}_${passenger}_${toFlight}"
                        )) {
                            is Ok -> {}

                            is Failed -> {
                                return Failed(result.error)
                            }
                        }
                    }
                }
            }

            val next = passengers.find { it.prev == passenger }
            if (next != null) {
                for (toFlight in change.toFlights[passenger.flight] ?: emptyList()) {
                    val lastestEndTime = time.estimateStartTime[next.flight] - timeWindow.valueOf(next.flight.dep.passengerTransferTime)
                    val eetCondition = IfFunction(
                        time.estimateEndTime[passenger.flight] leq lastestEndTime,
                        "${passenger}_${toFlight}_eet"
                    )
                    when (val result = model.add(eetCondition)) {
                        is Ok -> {}

                        is Failed -> {
                            return Failed(result.error)
                        }
                    }

                    for (cls in PassengerClass.entries) {
                        when (val result = model.addConstraint(
                            change.passengerFlightChange[passenger, toFlight, cls]!! leq passenger.amount * eetCondition,
                            "${name}_${passenger}_${toFlight}"
                        )) {
                            is Ok -> {}

                            is Failed -> {
                                return Failed(result.error)
                            }
                        }
                    }
                }
            }
        }

        return ok
    }
}
