package fuookami.ospf.kotlin.framework.gantt_scheduling.domain.task_compilation.service.limits

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.gantt_scheduling.domain.task.model.*
import fuookami.ospf.kotlin.framework.gantt_scheduling.domain.task_compilation.model.*

class ResourceLessQuantityMinimization<
    Args : GanttSchedulingShadowPriceArguments<E, A>,
    E : Executor,
    A : AssignmentPolicy<E>,
    S : ResourceTimeSlot<R, C>,
    R : Resource<C>,
    C : ResourceCapacity
>(
    private val quantity: ResourceUsage<S, R, C>,
    private val threshold: (S) -> Flt64 = { Flt64.zero },
    private val coefficient: (S) -> Flt64 = { Flt64.one },
    override val name: String = "resource_less_capacity_minimization"
) : AbstractGanttSchedulingCGPipeline<Args, E, A> {
    private val slots = if (quantity.lessEnabled) {
        quantity.timeSlots.filter { it.resourceCapacity.lessEnabled }
    } else {
        emptyList()
    }

    override fun invoke(model: LinearMetaModel): Try {
        if (slots.isNotEmpty()) {
            val cost = MutableLinearPolynomial()
            for (slot in slots) {
                val thresholdValue = threshold(slot)
                val thisCoefficient = coefficient(slot)
                if (thresholdValue eq Flt64.zero) {
                    cost += thisCoefficient * quantity.lessQuantity[slot]
                } else {
                    val slack = SlackFunction(
                        UContinuous,
                        x = LinearPolynomial(quantity.lessQuantity[slot]),
                        threshold = LinearPolynomial(thresholdValue),
                        name = "${quantity.name}_${slot}_over_quantity_threshold"
                    )
                    model.addSymbol(slack)
                    cost += thisCoefficient * slack
                }
            }
            model.minimize(cost, "${quantity.name} less quantity")
        }

        return ok
    }
}
