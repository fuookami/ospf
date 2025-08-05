package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.express_effectiveness.service

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.express_effectiveness.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.express_effectiveness.service.limits.*

data class PipelineListGenerator(
    private val aggregation: Aggregation
) {
    operator fun invoke(
        stowageMode: StowageMode
    ): Ret<PipelineList<AbstractLinearMetaModel>> {
        val pipelines = ArrayList<Pipeline<AbstractLinearMetaModel>>()

        if (aggregation.absoluteOrder != null) {
            pipelines.add(
                ItemPriorityLimit(
                    items = aggregation.items,
                    positions = aggregation.positions,
                    unloading = aggregation.absoluteOrder,
                    stowage = aggregation.stowage,
                    coefficient = {
                        TODO("not implemented yet")
                    }
                )
            )
        }

        if (aggregation.relativeOrder != null) {
            pipelines.add(
                ItemPriorityReverseLimit(
                    orderedItems = aggregation.relativeOrder.orderedItems,
                    orderedPositions = aggregation.relativeOrder.orderedPositions,
                    unloading = aggregation.relativeOrder,
                    coefficient = { lhs, rhs ->
                        TODO("not implemented yet")
                    }
                )
            )
        }

        return Ok(pipelines)
    }
}
