package fuookami.ospf.kotlin.example.framework_demo.demo4.domain.bunch_compilation.model

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.domain.task.model.*

class FlightCapacity(
    private val tasks: List<FlightTask>,
    private val compilation: Compilation,
    val withPassenger: Boolean = tasks.any { it.capacity is AircraftCapacity.Passenger },
    val withCargo: Boolean = tasks.any { it.capacity is AircraftCapacity.Cargo }
) {
    lateinit var passenger: Map<FlightTask, Map<PassengerClass, LinearExpressionSymbol>>
    lateinit var cargo: Map<FlightTask, LinearExpressionSymbol>

    fun register(model: AbstractLinearMetaModel): Try {
        if (withPassenger) {
            if (!::passenger.isInitialized) {
                passenger = tasks
                    .filter { it.capacity is AircraftCapacity.Passenger }
                    .associateWith { task ->
                        PassengerClass.entries.associateWith { cls ->
                            LinearExpressionSymbol(
                                LinearPolynomial(),
                                "${task}_capacity_${cls.name}"
                            )
                        }
                    }
            }

            when (val result = model.add(passenger)) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }

        if (withCargo) {
            if (!::cargo.isInitialized) {
                cargo = tasks
                    .filter { it.capacity is AircraftCapacity.Cargo }
                    .associateWith { task ->
                        LinearExpressionSymbol(
                            LinearPolynomial(),
                            "${task}_capacity"
                        )
                    }
            }

            when (val result = model.add(cargo)) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }

        return ok
    }

    fun addColumns(
        iteration: UInt64,
        bunches: List<FlightTaskBunch>,
    ): Try {
        val xi = compilation.x[iteration.toInt()]

        if (withPassenger) {
            for ((task, thisPassengers) in passenger) {
                for ((cls, thisPassenger) in thisPassengers) {
                    val thisBunches = bunches.filter {
                        val thisTask = it.get(task)
                        thisTask != null && thisTask.aircraft!!.capacity.let { capacity ->
                            when (capacity) {
                                is AircraftCapacity.Passenger -> {
                                    capacity[cls] neq UInt64.zero
                                }

                                else -> {
                                    false
                                }
                            }
                        }
                    }
                    if (thisBunches.isNotEmpty()) {
                        thisPassenger.flush()
                        thisPassenger.asMutable() += sum(thisBunches.map {
                            val thisTask = it.get(task)
                            val thisCapacity = (thisTask!!.aircraft!!.capacity as AircraftCapacity.Passenger)[cls]
                            thisCapacity * xi[it]
                        })
                    }
                }
            }
        }

        if (withCargo) {
            for ((task, thisCargo) in cargo) {
                val thisBunches = bunches.filter {
                    val thisTask = it.get(task)
                    thisTask != null && thisTask.aircraft!!.capacity.let { capacity ->
                        when (capacity) {
                            is AircraftCapacity.Cargo -> {
                                capacity.capacity neq Flt64.zero
                            }

                            else -> {
                                false
                            }
                        }
                    }
                }
                if (thisBunches.isNotEmpty()) {
                    thisCargo.flush()
                    thisCargo.asMutable() += sum(thisBunches.map {
                        val thisTask = it.get(task)
                        val thisCapacity = (thisTask!!.aircraft!!.capacity as AircraftCapacity.Cargo).capacity
                        thisCapacity * xi[it]
                    })
                }
            }
        }

        return ok
    }
}
