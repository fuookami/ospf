package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.soft_security.service.limits

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.Position

class EmptyHatedLimit(
    private val positions: List<Position>,
    private val load: Load,
    private val coefficient: (Position) -> Flt64 = { Flt64.one },
    override val name: String = "empty_hated_limit"
) : Pipeline<AbstractLinearMetaModel> {
    override fun invoke(model: AbstractLinearMetaModel): Try {
        when (val result = model.minimize(
            sum(positions.mapIndexedNotNull { j, position ->
                if (position.type.contains(PositionTypeCode.EmptyHated)) {
                    coefficient(position) * (Flt64.one - load.full[j])
                } else {
                    null
                }
            }),
            "empty hated"
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
