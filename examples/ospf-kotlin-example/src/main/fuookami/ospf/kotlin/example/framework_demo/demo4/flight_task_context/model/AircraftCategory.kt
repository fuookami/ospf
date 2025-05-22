package fuookami.ospf.kotlin.example.framework_demo.demo4.flight_task_context.model

import fuookami.ospf.kotlin.utils.math.*

enum class AircraftCategory {
    Passenger,
    Cargo
}

sealed class AircraftCapacity {
    class Passenger(
        private val capacity: Map<PassengerClass, UInt64>
    ) : AircraftCapacity() {
        val total = UInt64(capacity.values.sumOf { it.toInt() }.toULong())

        operator fun get(cls: PassengerClass) = capacity[cls] ?: UInt64.zero

        fun enabled(payload: Map<PassengerClass, UInt64>) = payload.asSequence().all { this[it.key] >= it.value }

        override val category get() = AircraftCategory.Passenger
    }

    class Cargo(
        val capacity: Flt64
    ) : AircraftCapacity() {
        override val category get() = AircraftCategory.Cargo

        fun enabled(payload: Flt64) = capacity geq payload
    }

    abstract val category: AircraftCategory
}
