package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.payload_maximization.service

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.payload_maximization.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.payload_maximization.service.limits.*

data class PipelineListGenerator(
    private val aggregation: Aggregation
) {
    operator fun invoke(
        stowageMode: StowageMode,
        parameter: Parameter
    ): Ret<PipelineList<AbstractLinearMetaModel>> {
        val pipelines = ArrayList<Pipeline<AbstractLinearMetaModel>>()

        pipelines.add(
            MaxPayloadLimit(
                aircraftModel = aggregation.aircraftModel,
                payload = aggregation.payload
            )
        )

        return Ok(pipelines)
    }
}
