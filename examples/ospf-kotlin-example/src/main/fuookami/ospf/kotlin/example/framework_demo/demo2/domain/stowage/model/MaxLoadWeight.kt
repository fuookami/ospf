package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model

import fuookami.ospf.kotlin.utils.math.geometry.*
import fuookami.ospf.kotlin.utils.physics.quantity.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*

class MaxLoadWeight(
    private val aircraftModel: AircraftModel,
    private val fuselage: Fuselage,
    private val items: List<Item>,
    private val positions: List<Position>,
    private val totalWeight: TotalWeight
) {
    lateinit var maxLoadWeight: QuantityLinearIntermediateSymbols1

    /**
     * 注册变量和中间值到模型中
     *
     * @param model             模型
     * @return                  成功与否
     */
    fun register(
        model: AbstractLinearMetaModel
    ): Try {
        if (!::maxLoadWeight.isInitialized) {
            maxLoadWeight = QuantityLinearIntermediateSymbols1("max_load_weight", Shape1(positions.size)) { j, _ ->
                val position = positions[j]
                Quantity(
                    if (position.mlw.segments.isNotEmpty()) {
                        val zfw = totalWeight.computedTotalWeight[FlightPhase.ZeroFuel]
                        if (zfw != null) {
                            // 预配载、全配载模式下，无油总重是个确定值，所以可以通过线性插值直接求出最大载重量
                            LinearExpressionSymbol(
                                LinearPolynomial(position.mlw(zfw).to(aircraftModel.weightUnit)!!.value),
                                name = "max_load_weight_${position}"
                            )
                        } else {
                            if (position.status.unavailable) {
                                // 如果该舱位已经不可用，即已装载或已经被堵住，无需考虑最大载重量
                                // 所以直接使用静态最大载重量占位，无需构建分段线性函数，以节省性能
                                LinearExpressionSymbol(
                                    LinearPolynomial(position.mlw.mlw.to(aircraftModel.weightUnit)!!.value),
                                    name = "max_load_weight_${position}",
                                )
                            } else {
                                // 构建线性插值（一元变量）分段线性函数，映射无油总重与最大载重量的关系
                                UnivariateLinearPiecewiseFunction(
                                    x = LinearPolynomial(totalWeight.estimateTotalWeight[FlightPhase.ZeroFuel]!!.to(aircraftModel.weightUnit)!!.value),
                                    points = position.mlw.points.map {
                                        Point2(it.zfw.to(aircraftModel.weightUnit)!!.value, it.mlw.to(aircraftModel.weightUnit)!!.value)
                                    },
                                    name = "max_load_weight_${position}"
                                )
                            }
                        }
                    } else {
                        LinearExpressionSymbol(
                            LinearPolynomial(position.mlw.mlw.to(aircraftModel.weightUnit)!!.value),
                            name = "max_load_weight_${position}"
                        )
                    },
                    aircraftModel.weightUnit
                )
            }
        }
        when (val result = model.add(maxLoadWeight)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
