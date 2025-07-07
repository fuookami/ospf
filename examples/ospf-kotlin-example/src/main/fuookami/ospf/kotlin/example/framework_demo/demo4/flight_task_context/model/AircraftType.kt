package fuookami.ospf.kotlin.example.framework_demo.demo4.flight_task_context.model

import kotlin.time.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.infrastructure.*

data class AircraftType(
    val code: AircraftTypeCode
) {
    companion object {
        private val pool = HashMap<AircraftTypeCode, AircraftType>()
        val values by pool::values

        operator fun invoke(code: AircraftTypeCode): AircraftType {
            return pool.getOrPut(code){ AircraftType(code) }
        }
    }

    override fun hashCode(): Int {
        return code.hashCode()
    }

    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (javaClass != other?.javaClass) return false

        other as AircraftType

        return code == other.code
    }
}

operator fun Map<Route, Duration>.get(dep: Airport, arr: Airport): Duration? {
    return this[Route(dep, arr)]
}

data class AircraftMinorType(
    val type: AircraftType,
    val code: AircraftMinorTypeCode,
    val costPerHour: Flt64,
    val routeFlyTime: Map<Route, Duration>,
    val connectionTime: Map<Airport, Duration>,
    val maxFlyTime: Duration? = null
) {
    val maxRouteFlyTime: Duration by lazy { routeFlyTime.asSequence().maxOf { it.value } }
    val maxConnectionTime: Duration by lazy { connectionTime.asSequence().maxOf { it.value } }

    companion object {
        private val pool = HashMap<AircraftMinorTypeCode, AircraftMinorType>()
        val values by pool::values

        operator fun invoke(code: AircraftMinorTypeCode): AircraftMinorType? {
            return pool[code]
        }
    }

    override fun hashCode(): Int {
        return type.hashCode().inv() or code.hashCode()
    }

    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (javaClass != other?.javaClass) return false

        other as AircraftMinorType

        if (type != other.type) return false
        if (code != other.code) return false

        return true
    }

    override fun toString() = "$code"
}
