package fuookami.ospf.kotlin.example.framework_demo.demo4.domain.bunch_compilation.model

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.concept.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.domain.task.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.domain.rule.model.*

class FlightLink(
    val links: List<Link>,
    private val compilation: Compilation
) {
    init {
        ManualIndexed.flush<Link>()
        for (link in links) {
            link.setIndexed()
        }
    }

    lateinit var link: LinearExpressionSymbols1
    lateinit var slack: SymbolCombination<AbstractSlackFunction<*>, Shape1>

    fun register(model: AbstractLinearMetaModel): Try {
        if (links.isNotEmpty()) {
            if (!::link.isInitialized) {
                link = LinearExpressionSymbols1(
                    "link",
                    Shape1(links.size)
                ) { k, _ ->
                    LinearExpressionSymbol(
                        LinearPolynomial(),
                        "link_$k"
                    )
                }
            }
            when (val result = model.add(link)) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }

            if (!::slack.isInitialized) {
                slack = SymbolCombination(
                    "link_slack",
                    Shape1(slack.size)
                ) { k, _ ->
                    SlackFunction(
                        x = link[k] + compilation.y[links[k].prevTask] / 2 + compilation.y[links[k].succTask] / 2,
                        threshold = LinearPolynomial(Flt64.one),
                        withPositive = false,
                        constraint = false,
                        name = "link_slack_$k"
                    )
                }
            }
            when (val result = model.add(slack)) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }

        return ok
    }

    fun addColumns(
        iteration: UInt64,
        bunches: List<FlightTaskBunch>,
    ): Try {
        val xi = compilation.x[iteration.toInt()]

        for (link in links) {
            val thisBunches = bunches.filter { it.contains(link.prevTask to link.succTask) }
            if (thisBunches.isNotEmpty()) {
                val thisLink = this.link[link]
                thisLink.flush()
                thisLink.asMutable() += sum(thisBunches.map { xi[it] })
            }
        }

        return ok
    }
}
