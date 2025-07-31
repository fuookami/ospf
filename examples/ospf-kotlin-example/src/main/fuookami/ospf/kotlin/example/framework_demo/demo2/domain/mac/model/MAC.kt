package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac.model

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

class MAC(
    private val aircraftModel: AircraftModel,
    private val formula: Formula,
    private val totalWeight: TotalWeight,
    private val torque: Torque
) {
    lateinit var mac: LinearIntermediateSymbol

    fun register(
        model: AbstractLinearMetaModel
    ): Try {
        if (!::mac.isInitialized) {
            val tow = totalWeight.computedTotalWeight[FlightPhase.TakeOff]
            mac = if (tow != null) {
                // 预配载、全配载模式下，起飞总重是个确定值，所以重心与起飞指数是线性相关，可以通过重心指数计算公式计算得到重心的表达式
                val index = torque.index[FlightPhase.TakeOff]!!
                LinearExpressionSymbol(
                    formula.mac(Quantity(LinearPolynomial(index.value), index.unit), tow),
                    "mac"
                )
            } else {
                // 使用线性插值构建采样点
                val points = ArrayList<Point3>()
                // todo: 使用线性插值构建采样点
                // 构建线性插值（二元变量）分段线性函数，映射起飞总重、起飞指数与重心的关系
                BivariateLinearPiecewiseFunction(
                    x = LinearPolynomial(totalWeight.estimateTotalWeight[FlightPhase.TakeOff]!!.to(aircraftModel.weightUnit)!!.value),
                    y = LinearPolynomial(torque.index[FlightPhase.TakeOff]!!.to(aircraftModel.torqueUnit)!!.value),
                    points = points,
                    name = "mac"
                )
            }
        }
        when (val result = model.add(mac)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
