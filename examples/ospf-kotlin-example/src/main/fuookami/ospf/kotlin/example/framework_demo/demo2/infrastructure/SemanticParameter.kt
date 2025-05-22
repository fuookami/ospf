package fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure

import kotlinx.serialization.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.math.Less
import fuookami.ospf.kotlin.utils.operator.*

@JvmInline
@Serializer
value class AircraftMinorModel(val model: String)

@JvmInline
@Serializer
value class RegNo(val no: String)

@JvmInline
@Serializer
value class FlightNo(val no: String)

@JvmInline
@Serializer
value class IATACode(val code: String)

@JvmInline
@Serializer
value class MAC(val mac: Flt64): PartialOrd<MAC>, Ord<MAC> {
    companion object {
        private val precision = Flt64(1e-3)
    }

    override fun partialOrd(rhs: MAC): Order {
        val less = Less<Flt64, Flt64>(precision)
        return if (less(mac, rhs.mac)) {
            Order.Less()
        } else if (less(rhs.mac, mac)) {
            Order.Greater()
        } else {
            Order.Equal
        }
    }

    override fun toString(): String {
        return String.format("%.1f%%", mac)
    }
}

@JvmInline
@Serializer
value class HorizontalStabilizerAngle(val angle: String)

@JvmInline
@Serializer
value class HorizontalStabilizerThrustDrate(val mod: String)
