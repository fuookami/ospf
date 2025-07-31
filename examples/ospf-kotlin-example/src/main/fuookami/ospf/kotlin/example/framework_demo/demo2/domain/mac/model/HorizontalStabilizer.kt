package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac.model

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.math.ordinary.*
import fuookami.ospf.kotlin.utils.math.geometry.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.physics.quantity.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure.MAC
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.aircraft.model.*

typealias MACDecision = fuookami.ospf.kotlin.example.framework_demo.demo2.domain.mac.model.MAC

class HorizontalStabilizer(
    private val aircraftModel: AircraftModel,
    val key: Key,
    val points: List<Point>,
    val limit: Limit,
    private val totalWeight: TotalWeight,
    private val mac: MACDecision
) {
    data class Key(
        val angle: HorizontalStabilizerAngle,
        val thrustDrate: HorizontalStabilizerThrustDrate?
    ) {
        override fun toString(): String {
            return angle.toString()
        }
    }

    data class Point(
        val tow: Quantity<Flt64>,
        val mac: MAC,
        val trim: Flt64
    )

    data class Limit(
        val minTrim: Flt64?,
        val maxTrim: Flt64?,
        val warnMinTrim: Flt64?,
        val warnMaxTrim: Flt64?
    )

    private val piecewise by lazy {
        BivariateLinearPiecewiseFunction(
            // 占位以直接使用里面的插值计算方法
            x = LinearPolynomial(),
            y = LinearPolynomial(),
            points = points.map { (tow, mac, trim) ->
                Point3(
                    tow.to(aircraftModel.weightUnit)!!.value,
                    mac.mac,
                    trim
                )
            },
            name = "${key}_trim"
        )
    }

    lateinit var trim: LinearIntermediateSymbol
    lateinit var warnSlack: LinearIntermediateSymbol

    operator fun invoke(tow: Quantity<Flt64>, mac: MAC): Flt64 {
        return piecewise.z(tow.to(aircraftModel.weightUnit)!!.value, mac.mac)!!
    }

    fun register(
        stowageMode: StowageMode,
        model: AbstractLinearMetaModel
    ): Try {
        if (!::trim.isInitialized) {
            val tow = totalWeight.computedTotalWeight[FlightPhase.TakeOff]
            trim = if (tow != null) {
                // 预配载、全配载模式下，起飞总重是个确定值，所以只需要对重心进行线性插值构建基于重心的分段减推力角线性插值函数
                UnivariateLinearPiecewiseFunction(
                    x = LinearPolynomial(mac.mac),
                    points = pointsOf(tow).map { (mac, trim) ->
                        Point2(mac.mac, trim)
                    },
                    name = "${key}_trim"
                )
            } else {
                // 构建线性插值（二元变量）分段线性函数，映射起飞总重、重心与减推力角的关系
                BivariateLinearPiecewiseFunction(
                    x = LinearPolynomial(totalWeight.estimateTotalWeight[FlightPhase.TakeOff]!!.to(aircraftModel.weightUnit)!!.value),
                    y = LinearPolynomial(mac.mac),
                    points = piecewise.points,
                    triangles = piecewise.triangles,
                    name = "${key}_trim"
                )
            }
        }
        when (val result = model.add(trim)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        if (stowageMode.withMacOptimization) {
            if (!::warnSlack.isInitialized) {
                warnSlack = if (limit.warnMinTrim != null && limit.warnMaxTrim != null) {
                    SlackRangeFunction(
                        x = LinearPolynomial(trim),
                        lb = LinearPolynomial(limit.warnMinTrim),
                        ub = LinearPolynomial(limit.warnMaxTrim),
                        name = "${key}_trim_warn_slack"
                    )
                } else if (limit.warnMinTrim != null) {
                    SlackFunction(
                        x = LinearPolynomial(trim),
                        threshold = LinearPolynomial(limit.warnMinTrim),
                        withPositive = false,
                        name = "${key}_trim_warn_slack"
                    )
                } else if (limit.warnMaxTrim != null) {
                    SlackFunction(
                        x = LinearPolynomial(trim),
                        threshold = LinearPolynomial(limit.warnMaxTrim),
                        withPositive = true,
                        name = "${key}_trim_warn_slack"
                    )
                } else {
                    LinearExpressionSymbol(
                        Flt64.zero,
                        name = "${key}_trim_warn_slack"
                    )
                }
            }
            when (val result = model.add(warnSlack)) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }

        return ok
    }

    private fun pointsOf(tow: Quantity<Flt64>): List<Pair<MAC, Flt64>> {
        val equal = Equal<Flt64, Flt64>(Flt64(1e-5))
        val points = ArrayList<Pair<MAC, Flt64>>()
        for (triangle in piecewise.triangles) {
            for (edge in listOf(triangle.e1, triangle.e2, triangle.e3)) {
                val minTow = Quantity(
                    min(
                        edge.from.x,
                        edge.to.x
                    ),
                    aircraftModel.weightUnit
                )
                val maxTow = Quantity(
                    max(
                        edge.from.x,
                        edge.to.x
                    ),
                    aircraftModel.weightUnit
                )
                if ((tow geq minTow)!! && (tow leq maxTow)!!) {
                    val dTOW = (tow.to(aircraftModel.weightUnit)!!.value - edge.from.x) / (edge.to.x - edge.from.x)
                    val mac = dTOW * (edge.to.y - edge.from.y) + edge.from.y
                    val trim = dTOW * (edge.to.z - edge.from.z) + edge.from.z
                    if (points.none { equal(it.first.mac, mac) }) {
                        points.add(MAC(mac) to trim)
                    }
                }
            }
        }
        return points.sortedBy { it.first.mac }
    }
}
