package fuookami.ospf.kotlin.example.framework_demo.demo4.domain.passenger.service.limits

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.domain.task.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.domain.passenger.model.*

class PassengerRouteCancelConstraint(
    private val passengers: List<FlightPassenger>,
    private val cancel: PassengerCancel,
    override val name: String = "passenger_route_cancel_constraint"
) : CGPipeline {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        for (passenger in passengers) {
            val prev = passenger.prev
            if (prev != null) {
                when (val result = model.addConstraint(
                    cancel.passengerCancel[prev] leq cancel.passengerCancel[passenger],
                    name = "passenger_route_cancel_constraint_${passenger}"
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
