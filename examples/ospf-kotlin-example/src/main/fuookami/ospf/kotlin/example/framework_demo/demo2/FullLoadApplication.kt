package fuookami.ospf.kotlin.example.framework_demo.demo2

import fuookami.ospf.kotlin.utils.error.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.dto.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.airworthiness_security.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.soft_security.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac_optimization.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.express_effectiveness.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.loading_effectiveness.*

data object FullLoadAlgorithm {
    suspend operator fun invoke(
        request: RequestDTO,
        runningHeartBeatCallBack: ((RunningHeartBeatDTO) -> Try)? = null,
        finnishHeartBeatCallBack: ((FinnishHeartBeatDTO) -> Unit)? = null,
        withRender: Boolean = false
    ): Pair<ResponseDTO, RenderDTO?> {
        val algo = FullLoadAlgorithmImpl()
        return algo(request, runningHeartBeatCallBack, finnishHeartBeatCallBack, withRender)
    }
}

private class FullLoadAlgorithmImpl {
    private val aircraftContext = AircraftContext()
    private val stowageContext = StowageContext()
    private val macContext = MacContext()
    private val airworthinessSecurityContext = AirworthinessSecurityContext()
    private val softSecurityContext = SoftSecurityContext()
    private val macOptimizationContext =  MacOptimizationContext()
    private val expressEffectivenessContext = ExpressEffectivenessContext()
    private val loadingEffectivenessContext = LoadingEffectivenessContext()

    suspend operator fun invoke(
        request: RequestDTO,
        runningHeartBeatCallBack: ((RunningHeartBeatDTO) -> Try)? = null,
        finnishHeartBeatCallBack: ((FinnishHeartBeatDTO) -> Unit)? = null,
        withRender: Boolean = false
    ): Pair<ResponseDTO, RenderDTO?> {
        val parameter = request.parameter

        when (val result = init(request)) {
            is Ok -> {}

            is Failed -> {
                return ResponseDTO(request, result.error) to null
            }
        }

        val solution = when (aircraftContext.aggregation.aircraftModel.type) {
            AircraftType.B737, AircraftType.B757 -> {
                when (val result = solveWithMILP(
                    parameter = parameter,
                    runningHeartBeatCallBack = runningHeartBeatCallBack
                )) {
                    is Ok -> {
                        result.value
                    }

                    is Failed -> {
                        return ResponseDTO(request, result.error) to null
                    }
                }
            }

            AircraftType.B767 -> {
                TODO("not implemented yet")
            }

            AircraftType.B747 -> {
                TODO("not implemented yet")
            }

            null -> {
                TODO("not implemented yet")
            }
        }

        return when (val result = stowageContext.analyze(
            solution = solution,
            input = request
        )) {
            is Ok -> {
                result.value to if (withRender) {
                    solution.render()
                } else {
                    null
                }
            }

            is Failed -> {
                return ResponseDTO(request, result.error) to null
            }
        }
    }

    private fun init(
        request: RequestDTO
    ): Try {
        when (val result = aircraftContext.init(
            input = request
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        when (val result = stowageContext.init(
            aircraftContext = aircraftContext,
            input = request
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        when (val result = macContext.init(
            aircraftContext = aircraftContext,
            stowageContext = stowageContext,
            input = request
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        when (val result = airworthinessSecurityContext.init(
            aircraftContext = aircraftContext,
            stowageContext = stowageContext,
            macContext = macContext,
            input = request
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        when (val result = softSecurityContext.init(
            aircraftContext = aircraftContext,
            stowageContext = stowageContext,
            input = request
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        when (val result = macOptimizationContext.init(
            aircraftContext = aircraftContext,
            stowageContext = stowageContext,
            macContext = macContext,
            input = request
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        when (val result = expressEffectivenessContext.init(
            aircraftContext = aircraftContext,
            stowageContext = stowageContext,
            input = request
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        when (val result = loadingEffectivenessContext.init(
            aircraftContext = aircraftContext,
            stowageContext = stowageContext,
            input = request
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }

    private suspend fun solveWithMILP(
        parameter: Parameter,
        runningHeartBeatCallBack: ((RunningHeartBeatDTO) -> Try)? = null
    ): Ret<Solution> {
        val model = LinearMetaModel()
        when (val result = register(parameter, model)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        val solver = LinearSolverBuilder()
        val modelSolution = when (val result = solver(model)) {
            is Ok -> {
                result.value
            }

            is Failed -> {
                if (result.error.code == ErrorCode.ORModelNoSolution) {
                    TODO("not implemented yet")
                } else {
                    return Failed(result.error)
                }
            }
        }

        val solution = when (val result = stowageContext.analyze(
            solution = modelSolution.solution,
            model = model
        )) {
            is Ok -> {
                result.value
            }

            is Failed -> {
                return Failed(result.error)
            }
        }

        return Ok(solution)
    }

    private fun register(
        parameter: Parameter,
        model: AbstractLinearMetaModel
    ): Try {
        when (val result = stowageContext.register(
            stowageMode = StowageMode.FullLoad,
            model = model
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        when (val result = macContext.register(
            stowageMode = StowageMode.FullLoad,
            model = model
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        when (val result = airworthinessSecurityContext.register(
            stowageMode = StowageMode.FullLoad,
            model = model
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        when (val result = softSecurityContext.register(
            stowageMode = StowageMode.FullLoad,
            parameter = parameter,
            model = model
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        when (val result = macOptimizationContext.register(
            stowageMode = StowageMode.FullLoad,
            parameter = parameter,
            model = model
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        when (val result = expressEffectivenessContext.register(
            stowageMode = StowageMode.FullLoad,
            parameter = parameter,
            model = model
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        when (val result = loadingEffectivenessContext.register(
            stowageMode = StowageMode.FullLoad,
            parameter = parameter,
            model = model
        )) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }

    private fun solveWithBendersAlgorithm(
        parameter: Parameter,
        runningHeartBeatCallBack: ((RunningHeartBeatDTO) -> Try)? = null
    ): Ret<Solution> {
        TODO("not implemented yet")
    }
}
