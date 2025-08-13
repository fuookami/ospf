package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.payload_maximization

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.dto.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.AircraftContext
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.StowageContext
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.payload_maximization.service.*

class PayloadMaximizationContext {
    lateinit var aggregation: Aggregation

    fun init(
        aircraftContext: AircraftContext,
        stowageContext: StowageContext,
        input: RequestDTO
    ): Try {
        if (!::aggregation.isInitialized) {
            aggregation = Aggregation(
                aircraftModel = aircraftContext.aggregation.aircraftModel,
                payload = stowageContext.aggregation.payload
            )
        }

        return ok
    }

    fun register(
        stowageMode: StowageMode,
        parameter: Parameter,
        model: AbstractLinearMetaModel
    ): Try {
        val generator = PipelineListGenerator(aggregation)
        val pipelines = when (val result = generator.invoke(
            stowageMode = stowageMode,
            parameter = parameter
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
