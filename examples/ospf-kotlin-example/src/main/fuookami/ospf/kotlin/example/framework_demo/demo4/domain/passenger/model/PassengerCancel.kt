package fuookami.ospf.kotlin.example.framework_demo.demo4.domain.passenger.model

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*

class PassengerCancel(
    private val passengers: List<FlightPassenger>
) {
    lateinit var passengerCancel: UIntVariable1

    fun register(model: AbstractLinearMetaModel): Try {
        if (!::passengerCancel.isInitialized) {
            passengerCancel = UIntVariable1(
                "passenger_cancel",
                Shape1(passengers.size)
            )
            for (passenger in passengers) {
                passengerCancel[passenger].range.leq(passenger.amount)
            }
        }
        when (val result = model.add(passengerCancel)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
