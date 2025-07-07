package fuookami.ospf.kotlin.example.framework_demo.demo4.flight_task_context.model

import kotlinx.datetime.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.infrastructure.*
import fuookami.ospf.kotlin.framework.gantt_scheduling.domain.task.model.*

data class Aircraft(
    val regNo: AircraftRegisterNumber,
    val minorType: AircraftMinorType,
    val capacity: AircraftCapacity
) : Executor(regNo.no, regNo.no) {
    internal lateinit var _usability: AircraftUsability
    val usability get() = _usability

    val type by minorType::type
    val costPerHour by minorType::costPerHour
    val routeFlyTime by minorType::routeFlyTime
    val maxFlyTime by minorType::maxFlyTime
    val maxRouteFlyTime by minorType::maxRouteFlyTime
    val connectionTime by minorType::connectionTime
    val maxConnectionTime by minorType::maxConnectionTime

    companion object {
        private val pool = HashMap<AircraftRegisterNumber, Aircraft>()
        val values by pool::values

        operator fun invoke(regNo: AircraftRegisterNumber): Aircraft? {
            return pool[regNo]
        }
    }

    init {
        pool[regNo] = this
    }

    fun capacity(cls: PassengerClass): UInt64 {
        return when (val capacity = this.capacity) {
            is AircraftCapacity.Passenger -> { capacity[cls] }
            else -> { UInt64.zero }
        }
    }

    override fun toString() = "$regNo"
}

class AircraftUsability(
    lastTask: FlightTask?,
    val location: Airport,
    enabledTime: Instant,
    val flightCyclePeriods: List<FlightCyclePeriod> = emptyList()
) : ExecutorInitialUsability<FlightTask, Aircraft, FlightTaskAssignment>(lastTask, enabledTime) {
    fun overFlightHourTimes(time: Instant, flightHour: FlightHour): UInt64 {
        return UInt64(flightCyclePeriods.count {
            time < it.expirationTime && !it.enabled(flightHour)
        })
    }

    fun overFlightCycleTimes(time: Instant, flightCycle: FlightCycle): UInt64 {
        return UInt64(flightCyclePeriods.count {
            time < it.expirationTime && !it.enabled(flightCycle)
        })
    }

    fun overFlightHour(time: Instant, flightHour: FlightHour): FlightHour {
        var ret = FlightHour.zero
        for (period in flightCyclePeriods) {
            if (time >= period.expirationTime) {
                continue
            }
            ret += period.overFlightHour(flightHour)
        }
        return ret
    }

    fun overFlightCycle(time: Instant, flightCycle: FlightCycle): FlightCycle {
        var ret = FlightCycle.zero
        for (period in flightCyclePeriods) {
            if (time >= period.expirationTime) {
                continue
            }
            ret += period.overFlightCycle(flightCycle)
        }
        return ret
    }
}
