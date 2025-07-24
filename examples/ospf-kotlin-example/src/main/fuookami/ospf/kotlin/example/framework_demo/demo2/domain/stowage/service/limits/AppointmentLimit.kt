package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.service.limits

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.error.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*

class AppointmentLimit(
    private val items: List<Item>,
    private val positions: List<Position>,
    private val appointment: Appointment,
    private val stowage: Stowage,
    override val name: String = "appointment_limit"
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        for ((i, item) in items.withIndex()) {
            val thisAppointment = appointment[item]
            if (thisAppointment != null) {
                for ((j, position) in positions.withIndex()) {
                    if (position != thisAppointment && Stowage.stowageNeeded(item, position)) {
                        when (val result = model.addConstraint(
                            stowage.stowage[i, j] eq false,
                            "${name}_${item}_${position}"
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
        for ((j, position) in positions.withIndex()) {
            val appointments = appointment[position]
            if (appointments.usize == position.mla) {
                for ((i, item) in items.withIndex()) {
                    if (item !in appointments && Stowage.stowageNeeded(item, position)) {
                        when (val result = model.addConstraint(
                            stowage.stowage[i, j] eq false,
                            "${name}_${item}_${position}"
                        )) {
                            is Ok -> {}

                            is Failed -> {
                                return Failed(result.error)
                            }
                        }
                    }
                }
            } else {
                return Failed(
                    ErrorCode.ApplicationFailed,
                    "指定舱位 $position 的装载物有 ${appointments.usize} 个，但该舱位最多装载 ${position.mla} 个货物"
                )
            }
        }

        return ok
    }
}
