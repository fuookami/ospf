package fuookami.ospf.kotlin.example.framework_demo.demo4.rule_context.model

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.infrastructure.*
import fuookami.ospf.kotlin.example.framework_demo.demo4.flight_task_context.model.*

enum class RestrictionType {
    Weak,
    ViolableStrong,
    Strong,
}

sealed interface Restriction {
    val type: RestrictionType

    fun related(aircraft: Aircraft): Boolean

    fun check(task: FlightTask): RestrictionCheckingResult
    fun check(task: FlightTask, aircraft: Aircraft): RestrictionCheckingResult
    fun check(task: FlightTask, recoveryPolicy: FlightTaskAssignment): RestrictionCheckingResult
}

sealed interface RestrictionCheckingResult {
    val restriction: Restriction

    val type get() = restriction.type
}

data class NotMatter(
    override val restriction: Restriction
) : RestrictionCheckingResult

data class Violate(
    override val restriction: Restriction
) : RestrictionCheckingResult

data class NotViolate(
    override val restriction: Restriction
) : RestrictionCheckingResult

data class ViolableViolate(
    override val restriction: Restriction
) : RestrictionCheckingResult

enum class RelationRestrictionCategory {
    BlackList,
    WhiteList
}

class RelationRestriction(
    override val type: RestrictionType,
    val category: RelationRestrictionCategory,
    val dep: Airport,
    val arr: Airport,
    val aircrafts: Set<Aircraft>,
    val weight: Flt64 = Flt64.one,
    val cost: Flt64? = null
) : Restriction {
    override fun related(aircraft: Aircraft): Boolean {
        return aircrafts.contains(aircraft)
    }

    override fun check(task: FlightTask): RestrictionCheckingResult {
        assert(task.isFlight)
        return check(task.dep, task.arr, task.aircraft)
    }

    override fun check(task: FlightTask, aircraft: Aircraft): RestrictionCheckingResult {
        assert(task.isFlight)
        return check(task.dep, task.arr, aircraft)
    }

    override fun check(task: FlightTask, recoveryPolicy: FlightTaskAssignment): RestrictionCheckingResult {
        assert(task.isFlight)
        val dep = recoveryPolicy.route?.dep ?: task.dep
        val arr = recoveryPolicy.route?.arr ?: task.arr
        val aircraft = recoveryPolicy.aircraft ?: task.aircraft
        return check(dep, arr, aircraft)
    }
    
    private fun check(dep: Airport, arr: Airport, aircraft: Aircraft?): RestrictionCheckingResult {
        if (dep != this.dep || arr != this.arr) {
            return NotMatter(this)
        }
        return aircraft?.let { dump(aircrafts.contains(aircraft)) } ?: NotMatter(this)
    }

    private fun violated(hit: Boolean): Boolean {
        return when (category) {
            RelationRestrictionCategory.BlackList -> hit
            RelationRestrictionCategory.WhiteList -> !hit
        }
    }

    private fun dump(hit: Boolean): RestrictionCheckingResult {
        return if (violated(hit)) {
            ViolableViolate(this)
        } else {
            NotViolate(this)
        }
    }
}
