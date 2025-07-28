package fuookami.ospf.kotlin.example.framework_demo.demo4.domain.passenger.service.limits

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.domain.task.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.domain.passenger.model.*

class PassengerFlightChangeMinimization(
    private val passengers: List<FlightPassenger>,
    private val change: PassengerChange,
    private val coefficient: (FlightTask, FlightTask, PassengerClass) -> Flt64 = { _, _, _ -> Flt64.one },
    override val name: String = "passenger_flight_change_minimization"
) : CGPipeline {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        val poly = MutableLinearPolynomial()
        for (passer in passengers) {
            for (toFlight in change.toFlights[passer.flight] ?: emptyList()) {
                for (cls in PassengerClass.entries) {
                    poly += coefficient(passer.flight, toFlight, cls) * change.passengerFlightChange[passer, toFlight, cls]!!
                }
            }
        }
        when (val result = model.minimize(
            poly,
            "passenger flight change"
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
