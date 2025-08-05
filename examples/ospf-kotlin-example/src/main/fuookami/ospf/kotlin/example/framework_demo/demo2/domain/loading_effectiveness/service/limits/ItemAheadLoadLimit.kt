package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.loading_effectiveness.service.limits

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*

class ItemAheadLoadLimit(
    private val items: List<Item>,
    private val stowage: Stowage,
    private val coefficient: (Item) -> Flt64 = { Flt64.one },
    override val name: String = "item_ahead_load_limit"
) : Pipeline<AbstractLinearMetaModel> {
    companion object {
        private val predicates = listOf(
            // 已到机下
            { item: Item -> item.order?.hardstand != null },
            // 已出库
            { item: Item -> item.order?.hardstand != null || item.order?.carBoard != null },
            // 已复磅
            { item: Item -> item.order?.hardstand != null || item.order?.carBoard != null || item.order?.reweighed != null },
        )
    }

    override fun invoke(model: AbstractLinearMetaModel): Try {
        val predicate = predicates.find { pred -> items.any(pred) }
        if (predicate != null) {
            when (val result = model.minimize(
                sum(items.mapIndexed { i, item ->
                    coefficient(item) * stowage.loaded[i]
                }),
                "item ahead load"
            )) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }

        return ok
    }
}
