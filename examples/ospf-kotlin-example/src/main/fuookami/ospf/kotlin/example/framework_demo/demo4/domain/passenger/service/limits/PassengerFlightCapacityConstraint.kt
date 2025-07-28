package fuookami.ospf.kotlin.example.framework_demo.demo4.domain.passenger.service.limits

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.domain.task.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.domain.bunch_compilation.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.domain.passenger.model.*
import fuookami.ospf.kotlin.utils.functional.get

class PassengerFlightCapacityConstraint(
    private val flights: List<FlightTask>,
    private val amount: PassengerAmount,
    private val capacity: FlightCapacity,
    override val name: String = "passenger_flight_capacity_constraint"
) : CGPipeline {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        for (flight in flights) {
            for (cls in PassengerClass.entries) {
                when (val result = model.addConstraint(
                    amount.passengerAmount[flight, cls]!! leq capacity.passenger[flight, cls]!!,
                    name = "passenger_flight_capacity_constraint_${flight}_${cls}"
                )) {
                    is Ok -> {}

                    is Failed -> {
                        return Failed(result.error)
                    }
                }
            }
        }

        return ok
    }
}
