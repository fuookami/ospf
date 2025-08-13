package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.recommended_weight_equalization

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.dto.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.AircraftContext
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.StowageContext
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.recommended_weight_equalization.service.*

internal typealias AircraftAggregation = fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.Aggregation
internal typealias StowageAggregation = fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.Aggregation

class RecommendedWeightEqualizationContext {
    lateinit var aggregation: Aggregation

    fun init(
        aircraftContext: AircraftContext,
        stowageContext: StowageContext,
        input: RequestDTO
    ): Try {
        if (!::aggregation.isInitialized) {
            when (val result = AggregationInitializer.invoke(
                aircraftAggregation = aircraftContext.aggregation,
                stowageAggregation = stowageContext.aggregation,
                input = input
            )) {
                is Ok -> {
                    aggregation = result.value
                }

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }

        return ok
    }

    fun register(
        stowageMode: StowageMode,
        model: AbstractLinearMetaModel
    ): Try {
        val generator = PipelineListGenerator(aggregation)
        val pipelines = when (val result = generator.invoke(
            stowageMode = stowageMode
        )) {
            is Ok -> {
                result.value
            }

            is Failed -> {
                return Failed(result.error)
            }
        }

        for (pipeline in pipelines) {
            when (val result = pipeline(model)) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }

        return ok
    }

    fun registerForBendersMP(
        model: AbstractLinearMetaModel
    ): Try {
        TODO("not implemented yet")
    }

    fun registerForBendersSP(
        model: AbstractLinearMetaModel
    ): Try {
        TODO("not implemented yet")
    }

    fun flushForBendersSP(
        model: AbstractLinearMetaModel,
        solution: List<Flt64>
    ): Try {
        TODO("not implemented yet")
    }
}
