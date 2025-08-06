package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.soft_security.service.limits

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.Position

class MainDeckDoorEmptyLimit(
    private val items: List<Item>,
    private val positions: List<Position>,
    private val deck: Deck,
    private val stowage: Stowage,
    private val coefficient: (Item) -> Flt64 = { Flt64.one },
    override val name: String = "main_deck_door_empty_limit"
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        when (val result = model.minimize(
            sum(items.mapIndexed { i, item ->
                coefficient(item) * sum(positions.mapIndexedNotNull { j, position ->
                    if (deck.ubieties(position.base).any { it.type == DoorUbietyType.Beside }) {
                        stowage.stowage[i, j]
                    } else {
                        null
                    }
                })
            }),
            "main deck door empty"
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
