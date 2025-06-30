package fuookami.ospf.kotlin.example.framework_demo.demo4.flight_task_context.model

import fuookami.ospf.kotlin.example.framework_demo.demo4.infrastructure.*

enum class AirportType {
    Domestic {
        override val isDomainType: Boolean get() = true
    },
    Regional,
    International;

    open val isDomainType: Boolean get() = false
}

data class Airport(
    val icao: ICAO,
    val type: AirportType,
    val base: Boolean = false
) {
    companion object {
        private val pool = HashMap<ICAO, Airport>()
        val values by pool::values

        operator fun invoke(icao: ICAO): Airport? {
            return pool[icao]
        }
    }

    init {
        pool[icao] = this
    }

    override fun hashCode(): Int {
        assert(icao.code.length == 4 && icao.code.all { it.isUpperCase() })

        var ret = 0
        for (ch in icao.code) {
            ret = ret shl 4
            ret = ret or (ch - 'A')
        }
        return ret
    }

    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (javaClass != other?.javaClass) return false

        other as Airport

        return icao == other.icao
    }

    override fun toString() = "$icao"
}

data class Route(
    val dep: Airport,
    val arr: Airport
)

enum class ArrivalAirportScope {
    Master,
    Alternate
}
