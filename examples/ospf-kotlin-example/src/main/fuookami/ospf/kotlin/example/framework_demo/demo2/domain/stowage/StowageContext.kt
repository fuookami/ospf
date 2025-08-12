package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.dto.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.AircraftContext
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.service.*

internal typealias AircraftAggregation = fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.Aggregation

class StowageContext {
    lateinit var aggregation: Aggregation

    fun init(
        aircraftContext: AircraftContext,
        input: RequestDTO
    ): Try {
        when (val result = AggregationInitializer(
            aircraftAggregation = aircraftContext.aggregation,
            input = input
        )) {
            is Ok -> {
                aggregation = result.value
            }

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
    
    fun register(
        stowageMode: StowageMode,
        model: AbstractLinearMetaModel
    ): Try {
        when (val result = aggregation.register(
            stowageMode = stowageMode,
            model = model
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        val generator = PipelineListGenerator(aggregation)
        val pipelines = when (val result = generator.invoke(stowageMode)) {
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

    fun analyze(
        solution: List<Flt64>,
        model: AbstractLinearMetaModel
    ): Ret<Solution> {
        val analyzer = SolutionAnalyzer(aggregation)
        val stowageSolution = when (val result = analyzer(solution, model)) {
            is Ok -> {
                result.value
            }

            is Failed -> {
                return Failed(result.error)
            }
        }

        return Ok(stowageSolution)
    }
    
    fun analyze(
        solution: Solution,
        input: RequestDTO
    ): Ret<ResponseDTO> {
        TODO("not implemented yet")
    }
}
