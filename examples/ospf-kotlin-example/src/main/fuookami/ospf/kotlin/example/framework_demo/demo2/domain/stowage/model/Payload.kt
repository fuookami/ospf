package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.physics.quantity.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*

class Payload(
    val plannedPayload: Quantity<Flt64>,
    val maxPayload: Quantity<Flt64>,
    val computedPayload: Quantity<Flt64>?,
    private val aircraftModel: AircraftModel,
    private val items: List<Item>,
    private val positions: List<Position>,
    private val load: Load
) {
    lateinit var mainEstimatePayload: QuantityLinearIntermediateSymbol
    lateinit var lowEstimatePayload: QuantityLinearIntermediateSymbol
    lateinit var estimatePayload: QuantityLinearIntermediateSymbol

    lateinit var mainActualPayload: QuantityLinearIntermediateSymbol
    lateinit var lowActualPayload: QuantityLinearIntermediateSymbol
    lateinit var actualPayload: QuantityLinearIntermediateSymbol

    fun register(
        stowageMode: StowageMode,
        model: AbstractLinearMetaModel
    ): Try {
        if (!::mainEstimatePayload.isInitialized) {
            mainEstimatePayload = Quantity(
                when (stowageMode) {
                    StowageMode.FullLoad -> {
                        // 全配载模式下，业载等于装载物重量之和
                        LinearExpressionSymbol(
                            LinearPolynomial(
                                items.fold(Flt64.zero) { acc, item ->
                                    if (item.location.enabledIn(DeckLocation.Main).ok) {
                                        acc + item.weight.to(aircraftModel.weightUnit)!!.value
                                    } else {
                                        acc
                                    }
                                }
                            ),
                            name = "main_estimate_payload"
                        )
                    }

                    StowageMode.Predistribution, StowageMode.WeightRecommendation -> {
                        val poly = MutableLinearPolynomial()
                        for ((j, position) in positions.withIndex()) {
                            when (position.location.location) {
                                DeckLocation.Main -> {
                                    poly += load.estimateLoadWeight[j].to(aircraftModel.weightUnit)!!.value
                                }

                                DeckLocation.LowForward, DeckLocation.LowAft -> {}
                            }
                        }
                        LinearExpressionSymbol(
                            poly,
                            name = "main_estimate_payload"
                        )
                    }
                },
                aircraftModel.weightUnit
            )
        }
        when (val result = model.add(mainEstimatePayload)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        if (!::lowEstimatePayload.isInitialized) {
            lowEstimatePayload = Quantity(
                when (stowageMode) {
                    StowageMode.FullLoad -> {
                        // 全配载模式下，业载等于装载物重量之和
                        LinearExpressionSymbol(
                            LinearPolynomial(
                                items.fold(Flt64.zero) { acc, item ->
                                    if (item.location.enabledIn(DeckLocation.LowAft).ok || item.location.enabledIn(DeckLocation.LowForward).ok) {
                                        acc + item.weight.to(aircraftModel.weightUnit)!!.value
                                    } else {
                                        acc
                                    }
                                }
                            ),
                            name = "main_estimate_payload"
                        )
                    }

                    StowageMode.Predistribution, StowageMode.WeightRecommendation -> {
                        val poly = MutableLinearPolynomial()
                        for ((j, position) in positions.withIndex()) {
                            when (position.location.location) {
                                DeckLocation.Main -> {}

                                DeckLocation.LowForward, DeckLocation.LowAft -> {
                                    poly += load.estimateLoadWeight[j].to(aircraftModel.weightUnit)!!.value
                                }
                            }
                        }
                        LinearExpressionSymbol(
                            poly,
                            name = "low_estimate_payload"
                        )
                    }
                },
                aircraftModel.weightUnit
            )
        }
        when (val result = model.add(lowEstimatePayload)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        if (!::estimatePayload.isInitialized) {
            estimatePayload = Quantity(
                when (stowageMode) {
                    StowageMode.FullLoad -> {
                        // 全配载模式下，业载等于装载物重量之和
                        LinearExpressionSymbol(
                            LinearPolynomial(
                                items.fold(Flt64.zero) { acc, item ->
                                    acc + item.weight.to(aircraftModel.weightUnit)!!.value
                                }
                            )
                        )
                    }

                    StowageMode.Predistribution -> {
                        // 预配载模式下，业载等于预计业载
                        LinearExpressionSymbol(
                            (computedPayload ?: plannedPayload).to(aircraftModel.weightUnit)!!.value,
                            name = "estimate_payload"
                        )
                    }

                    StowageMode.WeightRecommendation -> {
                        val poly = MutableLinearPolynomial()
                        for ((j, _) in positions.withIndex()) {
                            poly += load.estimateLoadWeight[j].to(aircraftModel.weightUnit)!!.value
                        }
                        LinearExpressionSymbol(
                            poly,
                            name = "estimate_payload"
                        )
                    }
                },
                aircraftModel.weightUnit
            )
        }
        when (val result = model.add(estimatePayload)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        if (!::mainActualPayload.isInitialized) {
            mainActualPayload = Quantity(
                when (stowageMode) {
                    StowageMode.FullLoad -> {
                        // 全配载模式下，实际载等于装载物重量之和
                        LinearExpressionSymbol(
                            LinearPolynomial(
                                items.fold(Flt64.zero) { acc, item ->
                                    if (item.location.enabledIn(DeckLocation.Main).ok) {
                                        acc + item.weight.to(aircraftModel.weightUnit)!!.value
                                    } else {
                                        acc
                                    }
                                }
                            ),
                            name = "main_actual_payload"
                        )
                    }

                    StowageMode.Predistribution, StowageMode.WeightRecommendation -> {
                        val poly = MutableLinearPolynomial()
                        for ((j, position) in positions.withIndex()) {
                            when (position.location.location) {
                                DeckLocation.Main -> {
                                    poly += load.actualLoadWeight[j].to(aircraftModel.weightUnit)!!.value
                                }

                                DeckLocation.LowForward, DeckLocation.LowAft -> {}
                            }
                        }
                        LinearExpressionSymbol(
                            poly,
                            name = "main_actual_payload"
                        )
                    }
                },
                aircraftModel.weightUnit
            )
        }
        when (val result = model.add(mainActualPayload)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        if (!::lowActualPayload.isInitialized) {
            lowActualPayload = Quantity(
                when (stowageMode) {
                    StowageMode.FullLoad -> {
                        // 全配载模式下，实际载等于装载物重量之和
                        LinearExpressionSymbol(
                            LinearPolynomial(
                                items.fold(Flt64.zero) { acc, item ->
                                    if (item.location.enabledIn(DeckLocation.LowAft).ok || item.location.enabledIn(DeckLocation.LowForward).ok) {
                                        acc + item.weight.to(aircraftModel.weightUnit)!!.value
                                    } else {
                                        acc
                                    }
                                }
                            ),
                            name = "main_actual_payload"
                        )
                    }

                    StowageMode.Predistribution, StowageMode.WeightRecommendation -> {
                        val poly = MutableLinearPolynomial()
                        for ((j, position) in positions.withIndex()) {
                            when (position.location.location) {
                                DeckLocation.Main -> {}

                                DeckLocation.LowForward, DeckLocation.LowAft -> {
                                    poly += load.actualLoadWeight[j].to(aircraftModel.weightUnit)!!.value
                                }
                            }
                        }
                        LinearExpressionSymbol(
                            poly,
                            name = "low_actual_payload"
                        )
                    }
                },
                aircraftModel.weightUnit
            )
        }
        when (val result = model.add(lowActualPayload)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        if (!::actualPayload.isInitialized) {
            actualPayload = Quantity(
                when (stowageMode) {
                    StowageMode.FullLoad -> {
                        // 全配载模式下，实际载等于装载物重量之和
                        LinearExpressionSymbol(
                            LinearPolynomial(
                                items.fold(Flt64.zero) { acc, item ->
                                    acc + item.weight.to(aircraftModel.weightUnit)!!.value
                                }
                            )
                        )
                    }

                    StowageMode.Predistribution, StowageMode.WeightRecommendation -> {
                        val poly = MutableLinearPolynomial()
                        for ((j, _) in positions.withIndex()) {
                            poly += load.actualLoadWeight[j].to(aircraftModel.weightUnit)!!.value
                        }
                        LinearExpressionSymbol(
                            poly,
                            name = "actual_payload"
                        )
                    }
                },
                aircraftModel.weightUnit
            )
        }
        when (val result = model.add(actualPayload)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
