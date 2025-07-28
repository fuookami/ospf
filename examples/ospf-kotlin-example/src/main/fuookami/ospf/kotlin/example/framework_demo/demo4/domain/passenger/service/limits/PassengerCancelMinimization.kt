package fuookami.ospf.kotlin.example.framework_demo.demo4.domain.passenger.service.limits

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.domain.task.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.domain.passenger.model.*

class PassengerCancelMinimization(
    private val passengers: List<FlightPassenger>,
    private val cancel: PassengerCancel,
    private val coefficient: (FlightPassenger) -> Flt64 = { _ -> Flt64.one },
    override val name: String = "passenger_cancel_minimization"
) : CGPipeline {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        val poly = MutableLinearPolynomial()
        for (passenger in passengers) {
            poly += coefficient(passenger) * cancel.passengerCancel[passenger]
        }
        when (val result = model.minimize(
            poly,
            "passenger cancel"
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
