package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.airworthiness_security.model

import java.util.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.math.geometry.*
import fuookami.ospf.kotlin.utils.physics.quantity.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*

interface AbstractEnvelope {
    data class Point(
        val totalWeight: Quantity<Flt64>,
        val index: Quantity<Flt64>
    )

    enum class SideType {
        Left,
        Right
    }

    class Side(
        private val aircraftModel: AircraftModel,
        val name: String,
        val type: SideType,
        val points: List<Point>
    ) {
        val piecewise by lazy {
            UnivariateLinearPiecewiseFunction(
                // 占位以使用里面的插值方法
                x = LinearPolynomial(),
                points = points.map {
                    Point2(
                        it.totalWeight.to(aircraftModel.weightUnit)!!.value,
                        it.index.to(aircraftModel.torqueUnit)!!.value
                    )
                },
                name = "${name}_${type.name.lowercase(Locale.getDefault())}"
            )
        }

        fun piecewise(totalWeight: QuantityLinearIntermediateSymbol): QuantityLinearIntermediateSymbol {
            return Quantity(
                UnivariateLinearPiecewiseFunction(
                    x = LinearPolynomial(totalWeight.to(aircraftModel.weightUnit)!!.value),
                    points = points.map {
                        Point2(
                            it.totalWeight.to(aircraftModel.weightUnit)!!.value,
                            it.index.to(aircraftModel.torqueUnit)!!.value
                        )
                    },
                    name = when (type) {
                        SideType.Left -> "min_index_${name}_${type.name.lowercase(Locale.getDefault())}"
                        SideType.Right -> "max_index_${name}_${type.name.lowercase(Locale.getDefault())}"
                    },
                ),
                aircraftModel.torqueUnit
            )
        }

        operator fun invoke(totalWeight: Quantity<Flt64>): Quantity<Flt64> {
            return Quantity(
                piecewise.y(totalWeight.to(aircraftModel.weightUnit)!!.value)!!,
                aircraftModel.torqueUnit
            )
        }
    }

    val phase: FlightPhase
    val name: String
    val minIndex: QuantityLinearIntermediateSymbol
    val maxIndex: QuantityLinearIntermediateSymbol

    fun register(model: AbstractLinearMetaModel): Try
}

class Envelope(
    private val aircraftModel: AircraftModel,
    override val phase: FlightPhase,
    override val name: String,
    val lhsSide: AbstractEnvelope.Side,
    val rhsSide: AbstractEnvelope.Side,
    private val totalWeight: TotalWeight
) : AbstractEnvelope {
    fun minIndexOf(totalWeight: Quantity<Flt64>): Quantity<Flt64> {
        return lhsSide(totalWeight)
    }

    fun maxIndexOf(totalWeight: Quantity<Flt64>): Quantity<Flt64> {
        return rhsSide(totalWeight)
    }

    override lateinit var minIndex: QuantityLinearIntermediateSymbol
    override lateinit var maxIndex: QuantityLinearIntermediateSymbol

    override fun register(
        model: AbstractLinearMetaModel
    ): Try {
        if (!::minIndex.isInitialized) {
            val thisTotalWeight = totalWeight.computedTotalWeight[phase]
            minIndex = if (thisTotalWeight != null) {
                Quantity(
                    LinearExpressionSymbol(
                        minIndexOf(thisTotalWeight).to(aircraftModel.torqueUnit)!!.value,
                        name = "min_index_${name}_${phase.name.lowercase(Locale.getDefault())}"
                    ),
                    aircraftModel.torqueUnit
                )
            } else {
                lhsSide.piecewise(totalWeight.estimateTotalWeight[phase]!!)
            }
        }
        when (val result = model.add(minIndex)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        if (!::maxIndex.isInitialized) {
            val thisTotalWeight = totalWeight.computedTotalWeight[phase]
            maxIndex = if (thisTotalWeight != null) {
                Quantity(
                    LinearExpressionSymbol(
                        maxIndexOf(thisTotalWeight).to(aircraftModel.torqueUnit)!!.value,
                        name = "max_index_${name}_${phase.name.lowercase(Locale.getDefault())}"
                    ),
                    aircraftModel.torqueUnit
                )
            } else {
                rhsSide.piecewise(totalWeight.estimateTotalWeight[phase]!!)
            }
        }
        when (val result = model.add(maxIndex)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}

class ConditionalEnvelope(
    private val aircraftModel: AircraftModel,
    override val phase: FlightPhase,
    override val name: String,
    val lhsSide1: AbstractEnvelope.Side,
    val rhsSide1: AbstractEnvelope.Side,
    val lhsSide2: AbstractEnvelope.Side,
    val rhsSide2: AbstractEnvelope.Side,
    val valueCondition: () -> Boolean?,
    val symbolCondition: (String) -> Either<LinearPolynomial, LinearLogicFunctionSymbol>,
    private val totalWeight: TotalWeight
) : AbstractEnvelope {
    lateinit var condition: LinearIntermediateSymbol
    override lateinit var minIndex: QuantityLinearIntermediateSymbol
    override lateinit var maxIndex: QuantityLinearIntermediateSymbol

    override fun register(
        model: AbstractLinearMetaModel
    ): Try {
        when (valueCondition()) {
            true -> {
                // 取第一种包线
                if (!::minIndex.isInitialized) {
                    val thisTotalWeight = totalWeight.computedTotalWeight[phase]
                    minIndex = if (thisTotalWeight != null) {
                        // 预配载、全配载模式下，总重是个确定值，所以可以通过线性插值直接求出最小指数
                        Quantity(
                            LinearExpressionSymbol(
                                LinearPolynomial(lhsSide1(thisTotalWeight).to(aircraftModel.weightUnit)!!.value),
                                name = "min_index_${name}_${phase.name.lowercase(Locale.getDefault())}"
                            ),
                            aircraftModel.torqueUnit
                        )
                    } else {
                        // 构建线性插值（一元变量）分段线性函数，映射总重与最小指数的关系
                        lhsSide1.piecewise(totalWeight.estimateTotalWeight[phase]!!)
                    }
                }
                when (val result = model.add(minIndex)) {
                    is Ok -> {}

                    is Failed -> {
                        return Failed(result.error)
                    }
                }

                if (!::maxIndex.isInitialized) {
                    val thisTotalWeight = totalWeight.computedTotalWeight[phase]
                    maxIndex = if (thisTotalWeight != null) {
                        // 预配载、全配载模式下，总重是个确定值，所以可以通过线性插值直接求出最大指数
                        Quantity(
                            LinearExpressionSymbol(
                                LinearPolynomial(rhsSide1(thisTotalWeight).to(aircraftModel.weightUnit)!!.value),
                                name = "max_index_${name}_${phase.name.lowercase(Locale.getDefault())}"
                            ),
                            aircraftModel.torqueUnit
                        )
                    } else {
                        // 构建线性插值（一元变量）分段线性函数，映射总重与最大指数的关系
                        rhsSide1.piecewise(totalWeight.estimateTotalWeight[phase]!!)
                    }
                }
                when (val result = model.add(maxIndex)) {
                    is Ok -> {}

                    is Failed -> {
                        return Failed(result.error)
                    }
                }
            }

            false -> {
                // 取第二种包线
                if (!::minIndex.isInitialized) {
                    val thisTotalWeight = totalWeight.computedTotalWeight[phase]
                    minIndex = if (thisTotalWeight != null) {
                        // 预配载、全配载模式下，总重是个确定值，所以可以通过线性插值直接求出最小指数
                        Quantity(
                            LinearExpressionSymbol(
                                LinearPolynomial(lhsSide2(thisTotalWeight).to(aircraftModel.weightUnit)!!.value),
                                name = "min_index_${name}_${phase.name.lowercase(Locale.getDefault())}"
                            ),
                            aircraftModel.torqueUnit
                        )
                    } else {
                        // 构建线性插值（一元变量）分段线性函数，映射总重与最小指数的关系
                        lhsSide2.piecewise(totalWeight.estimateTotalWeight[phase]!!)
                    }
                }
                when (val result = model.add(minIndex)) {
                    is Ok -> {}

                    is Failed -> {
                        return Failed(result.error)
                    }
                }

                if (!::maxIndex.isInitialized) {
                    val thisTotalWeight = totalWeight.computedTotalWeight[phase]
                    maxIndex = if (thisTotalWeight != null) {
                        // 预配载、全配载模式下，总重是个确定值，所以可以通过线性插值直接求出最大指数
                        Quantity(
                            LinearExpressionSymbol(
                                LinearPolynomial(rhsSide2(thisTotalWeight).to(aircraftModel.weightUnit)!!.value),
                                name = "max_index_${name}_${phase.name.lowercase(Locale.getDefault())}"
                            ),
                            aircraftModel.torqueUnit
                        )
                    } else {
                        // 构建线性插值（一元变量）分段线性函数，映射总重与最大指数的关系
                        rhsSide2.piecewise(totalWeight.estimateTotalWeight[phase]!!)
                    }
                }
                when (val result = model.add(maxIndex)) {
                    is Ok -> {}

                    is Failed -> {
                        return Failed(result.error)
                    }
                }
            }

            null -> {
                // 使用条件表达式选择包线
                if (!::condition.isInitialized) {
                    condition = when (val condition = symbolCondition("${name}_${phase.name.lowercase(Locale.getDefault())}_condition")) {
                        is Either.Left -> {
                            LinearExpressionSymbol(
                                condition.value,
                                name = "${name}_${phase.name.lowercase(Locale.getDefault())}_condition"
                            )
                        }

                        is Either.Right -> {
                            condition.value
                        }
                    }
                }
                when (val result = model.add(condition)) {
                    is Ok -> {}

                    is Failed -> {
                        return Failed(result.error)
                    }
                }

                if (!::minIndex.isInitialized) {
                    val thisTotalWeight = totalWeight.computedTotalWeight[phase]
                    val minIndex1 = if (thisTotalWeight != null) {
                        // 预配载、全配载模式下，总重是个确定值，所以可以通过线性插值直接求出最小指数
                        Quantity(
                            LinearExpressionSymbol(
                                LinearPolynomial(lhsSide1(thisTotalWeight).to(aircraftModel.weightUnit)!!.value),
                                name = "min_index_${lhsSide1.name}_${phase.name.lowercase(Locale.getDefault())}"
                            ),
                            aircraftModel.torqueUnit
                        )
                    } else {
                        // 构建线性插值（一元变量）分段线性函数，映射总重与最小指数的关系
                        lhsSide1.piecewise(totalWeight.estimateTotalWeight[phase]!!)
                    }
                    val minIndex2 = if (thisTotalWeight != null) {
                        // 预配载、全配载模式下，总重是个确定值，所以可以通过线性插值直接求出最小指数
                        Quantity(
                            LinearExpressionSymbol(
                                LinearPolynomial(lhsSide2(thisTotalWeight).to(aircraftModel.weightUnit)!!.value),
                                name = "min_index_${lhsSide2.name}_${phase.name.lowercase(Locale.getDefault())}"
                            ),
                            aircraftModel.torqueUnit
                        )
                    } else {
                        // 构建线性插值（一元变量）分段线性函数，映射总重与最小指数的关系
                        lhsSide2.piecewise(totalWeight.estimateTotalWeight[phase]!!)
                    }
                    minIndex = Quantity(
                        IfElseFunction(
                            branch = IfElseFunction.Branch(
                                polynomial = LinearPolynomial(minIndex1.to(aircraftModel.torqueUnit)!!.value),
                                name = "min_index_${name}_${lhsSide1.name}_${phase.name.lowercase(Locale.getDefault())}"
                            ),
                            elseBranch = IfElseFunction.Branch(
                                polynomial = LinearPolynomial(minIndex2.to(aircraftModel.torqueUnit)!!.value),
                                name = "min_index_${name}_${lhsSide2.name}_${phase.name.lowercase(Locale.getDefault())}"
                            ),
                            condition = LinearPolynomial(condition),
                            name = "min_index_${name}_${phase.name.lowercase(Locale.getDefault())}"
                        ),
                        aircraftModel.torqueUnit
                    )
                }
                when (val result = model.add(minIndex)) {
                    is Ok -> {}

                    is Failed -> {
                        return Failed(result.error)
                    }
                }

                if (!::maxIndex.isInitialized) {
                    val thisTotalWeight = totalWeight.computedTotalWeight[phase]
                    val maxIndex1 = if (thisTotalWeight != null) {
                        // 预配载、全配载模式下，总重是个确定值，所以可以通过线性插值直接求出最大指数
                        Quantity(
                            LinearExpressionSymbol(
                                LinearPolynomial(rhsSide1(thisTotalWeight).to(aircraftModel.weightUnit)!!.value),
                                name = "max_index_${rhsSide1.name}_${phase.name.lowercase(Locale.getDefault())}"
                            ),
                            aircraftModel.torqueUnit
                        )
                    } else {
                        // 构建线性插值（一元变量）分段线性函数，映射总重与最大指数的关系
                        rhsSide1.piecewise(totalWeight.estimateTotalWeight[phase]!!)
                    }
                    val maxIndex2 = if (thisTotalWeight != null) {
                        // 预配载、全配载模式下，总重是个确定值，所以可以通过线性插值直接求出最大指数
                        Quantity(
                            LinearExpressionSymbol(
                                LinearPolynomial(rhsSide2(thisTotalWeight).to(aircraftModel.weightUnit)!!.value),
                                name = "max_index_${rhsSide2.name}_${phase.name.lowercase(Locale.getDefault())}"
                            ),
                            aircraftModel.torqueUnit
                        )
                    } else {
                        // 构建线性插值（一元变量）分段线性函数，映射总重与最大指数的关系
                        rhsSide2.piecewise(totalWeight.estimateTotalWeight[phase]!!)
                    }
                    maxIndex = Quantity(
                        IfElseFunction(
                            branch = IfElseFunction.Branch(
                                polynomial = LinearPolynomial(maxIndex1.to(aircraftModel.torqueUnit)!!.value),
                                name = "max_index_${name}_${rhsSide1.name}_${phase.name.lowercase(Locale.getDefault())}"
                            ),
                            elseBranch = IfElseFunction.Branch(
                                polynomial = LinearPolynomial(maxIndex2.to(aircraftModel.torqueUnit)!!.value),
                                name = "max_index_${name}_${rhsSide2.name}_${phase.name.lowercase(Locale.getDefault())}"
                            ),
                            condition = LinearPolynomial(condition),
                            name = "max_index_${name}_${phase.name.lowercase(Locale.getDefault())}"
                        ),
                        aircraftModel.torqueUnit
                    )
                }
                when (val result = model.add(maxIndex)) {
                    is Ok -> {}

                    is Failed -> {
                        return Failed(result.error)
                    }
                }
            }
        }

        return ok
    }
}
