package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac_optimization.service

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.framework.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac_optimization.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac_optimization.model.MACRange
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac_optimization.service.limits.*

class PipelineListGenerator(
    private val aggregation: Aggregation
) {
    operator fun invoke(
        stowageMode: StowageMode
    ): Ret<PipelineList<AbstractLinearMetaModel>> {
        val pipelines = ArrayList<Pipeline<AbstractLinearMetaModel>>()

        pipelines.add(
            LongitudinalBalanceLimit(
                aircraftModel = aggregation.aircraftModel,
                longitudinalBalance = aggregation.longitudinalBalance,
                coefficient = { macRangeType ->
                    TODO("NOT IMPLEMENTED YET")
                }
            )
        )


        if (aggregation.lateralBalance != null) {
            pipelines.add(
                LateralBalanceLimit(
                    aircraftModel = aggregation.aircraftModel,
                    lateralBalance = aggregation.lateralBalance,
                    coefficient = {
                        TODO("NOT IMPLEMENTED YET")
                    }
                )
            )
        }

        pipelines.add(
            HorizontalStabilizerLimit(
                horizontalStabilizers = aggregation.horizontalStabilizers,
                coefficient = {
                    TODO("NOT IMPLEMENTED YET")
                }
            )
        )

        return Ok(pipelines)
    }
}
