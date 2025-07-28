package fuookami.ospf.kotlin.example.framework_demo.demo4.domain.passenger.service.limits

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.domain.task.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.domain.passenger.model.*
import fuookami.ospf.kotlin.utils.functional.get

class PassengerClassChangeMinimization(
    private val passengers: List<FlightPassenger>,
    private val change: PassengerChange,
    private val coefficient: (FlightPassenger, PassengerClass) -> Flt64 = { _, _ -> Flt64.one },
    override val name: String = "passenger_class_change_minimization"
) : CGPipeline {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        val poly = MutableLinearPolynomial()
        for (passenger in passengers) {
            for (cls in PassengerClass.entries) {
                if (passenger.cls == cls) {
                    continue
                }

                poly += coefficient(passenger, cls) * change.passengerClassChange[passenger, cls]!!
            }
        }
        when (val result = model.minimize(
            poly,
            "passenger class change"
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
