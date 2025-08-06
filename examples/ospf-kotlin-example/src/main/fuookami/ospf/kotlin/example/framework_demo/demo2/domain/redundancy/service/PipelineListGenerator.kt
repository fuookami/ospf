package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.redundancy.service

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.redundancy.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.redundancy.service.limits.*

class PipelineListGenerator(
    private val aggregation: Aggregation
) {
    operator fun invoke(
        stowageMode: StowageMode
    ): Ret<PipelineList<AbstractLinearMetaModel>> {
        val pipelines = kotlin.collections.ArrayList<Pipeline<AbstractLinearMetaModel>>()

        pipelines.add(
            ExperimentalLongitudinalBalanceLimit(
                aircraftModel = aggregation.aircraftModel,
                longitudinalBalance = aggregation.experimentalLongitudinalBalance,
                coefficient = {
                    TODO("not implemented yet")
                }
            )
        )

        pipelines.add(
            RedundancyLimit(
                redundancy = aggregation.redundancy,
                coefficient = {
                    TODO("not implemented yet")
                }
            )
        )

        return Ok(pipelines)
    }
}
