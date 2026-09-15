/*
 * OSPF execution capability declarations
 */
package fuookami.ospf.framework.remote_solver.adapter.ospf

import fuookami.ospf.framework.remote_solver.protocol.domain.NormalizedModelType

/**
 * How an execution can be preempted between scheduler slices.
 *
 * Native means that the solver owns a reliable interrupt and native search-state
 * checkpoint. Controlled return means that a worker returns at a solver-safe
 * boundary and the next slice rebuilds from a portable checkpoint or warm start.
 */
enum class OspfPreemptionMode {
    NATIVE,
    CONTROLLED_RETURN,
    NON_PREEMPTIBLE
}

/** Recovery mechanism exposed by an OSPF execution adapter. */
enum class OspfResumeMode {
    NONE,
    WARM_START,
    BASIS,
    NATIVE_CHECKPOINT
}

/**
 * Capability evidence published by an OSPF execution bridge.
 *
 * The invariants deliberately make it impossible to construct a native
 * declaration without all of the guarantees required by the scheduler.
 */
data class OspfExecutionCapabilities(
    val supportedModelTypes: Set<NormalizedModelType>,
    val preemptionMode: OspfPreemptionMode,
    val resumeMode: OspfResumeMode,
    val supportsInterrupt: Boolean,
    val supportsPortableCheckpoint: Boolean,
    val supportsNativeCheckpoint: Boolean,
    val supportsWarmStart: Boolean
) {
    init {
        require(preemptionMode != OspfPreemptionMode.NATIVE ||
            (supportsInterrupt && supportsNativeCheckpoint && resumeMode == OspfResumeMode.NATIVE_CHECKPOINT)
        ) {
            "Native preemption requires interrupt, native checkpoint, and native checkpoint resume"
        }
        require(preemptionMode != OspfPreemptionMode.CONTROLLED_RETURN ||
            (supportsPortableCheckpoint && supportsWarmStart && resumeMode == OspfResumeMode.WARM_START)
        ) {
            "Controlled return requires portable checkpoint and warm-start recovery"
        }
        require(resumeMode != OspfResumeMode.NATIVE_CHECKPOINT || supportsNativeCheckpoint) {
            "Native checkpoint resume requires native checkpoint support"
        }
        require(resumeMode != OspfResumeMode.WARM_START || supportsWarmStart) {
            "Warm-start resume requires warm-start support"
        }
    }

    val supportsResume: Boolean
        get() = resumeMode != OspfResumeMode.NONE

    companion object {
        /** CP rebuild-based resume used by the in-process and strict external paths. */
        fun controlledReturn(
            supportedModelTypes: Set<NormalizedModelType> = setOf(NormalizedModelType.CP)
        ): OspfExecutionCapabilities = OspfExecutionCapabilities(
            supportedModelTypes = supportedModelTypes,
            preemptionMode = OspfPreemptionMode.CONTROLLED_RETURN,
            resumeMode = OspfResumeMode.WARM_START,
            supportsInterrupt = false,
            supportsPortableCheckpoint = true,
            supportsNativeCheckpoint = false,
            supportsWarmStart = true
        )

        /** Default for an adapter that cannot prove safe slice recovery. */
        fun nonPreemptible(
            supportedModelTypes: Set<NormalizedModelType> = emptySet()
        ): OspfExecutionCapabilities = OspfExecutionCapabilities(
            supportedModelTypes = supportedModelTypes,
            preemptionMode = OspfPreemptionMode.NON_PREEMPTIBLE,
            resumeMode = OspfResumeMode.NONE,
            supportsInterrupt = false,
            supportsPortableCheckpoint = false,
            supportsNativeCheckpoint = false,
            supportsWarmStart = false
        )
    }
}
