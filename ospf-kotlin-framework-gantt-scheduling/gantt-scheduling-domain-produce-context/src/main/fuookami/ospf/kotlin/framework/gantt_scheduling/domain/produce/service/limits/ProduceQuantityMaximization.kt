package fuookami.ospf.kotlin.framework.gantt_scheduling.domain.produce.service.limits

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.gantt_scheduling.domain.task.model.*
import fuookami.ospf.kotlin.framework.gantt_scheduling.domain.produce.model.*

class ProduceQuantityMaximization<
    Args : GanttSchedulingShadowPriceArguments<E, A>,
    E : Executor,
    A : AssignmentPolicy<E>
>(
    private val products: List<Product>,
    private val produce: Produce,
    private val threshold: (Product) -> Flt64 = { Flt64.zero },
    private val coefficient: (Product) -> Flt64 = { Flt64.one },
    override val name: String = "produce_quantity_maximization"
) : AbstractGanttSchedulingCGPipeline<Args, E, A> {
    override fun invoke(model: LinearMetaModel): Try {
        model.maximize(
            sum(products.map {
                val thresholdValue = threshold(it)
                if (thresholdValue eq Flt64.zero) {
                    coefficient(it) * produce.quantity[it]
                } else {
                    val slack = SlackFunction(
                        UContinuous,
                        x = LinearPolynomial(produce.quantity[it]),
                        threshold = LinearPolynomial(thresholdValue),
                        name = "produce_quantity_minimization_threshold_$it"
                    )
                    model.addSymbol(slack)
                    coefficient(it) * slack
                }
            }),
            "produce quantity"
        )
        return ok
    }
}
