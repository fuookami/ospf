@file:OptIn(kotlin.time.ExperimentalTime::class)

package fuookami.ospf.framework.remote_solver.adapter.ospf

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlin.time.Instant
import kotlinx.coroutines.runBlocking
import fuookami.ospf.framework.remote_solver.protocol.domain.ModelData
import fuookami.ospf.framework.remote_solver.protocol.domain.NormalizedModelType
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectPath
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteProblemStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolutionPresence
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteTerminationReason
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.SolverConfig
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import fuookami.ospf.framework.remote_solver.protocol.port.IdGeneratorPort
import fuookami.ospf.framework.remote_solver.protocol.port.ObjectStoragePort

class OspfExecutionCapabilityTest {
    @Test
    fun inProcessCpDeclaresControlledReturnWithoutNativeRecovery() {
        val bridge = OspfInProcessBridge(FixedClock, FixedIds, EmptyStorage)
        val capabilities = bridge.capabilitiesFor(cpPayload())

        assertEquals(OspfPreemptionMode.CONTROLLED_RETURN, capabilities.preemptionMode)
        assertEquals(OspfResumeMode.WARM_START, capabilities.resumeMode)
        assertTrue(capabilities.supportsPortableCheckpoint)
        assertTrue(capabilities.supportsWarmStart)
        assertFalse(capabilities.supportsInterrupt)
        assertFalse(capabilities.supportsNativeCheckpoint)
        assertEquals(setOf(NormalizedModelType.CP), capabilities.supportedModelTypes)
    }

    @Test
    fun externalNonCpDoesNotClaimRecoverableOrNativeExecution() {
        val bridge = OspfExternalProcessBridge(FixedClock, FixedIds, EmptyStorage)

        assertEquals(OspfPreemptionMode.NON_PREEMPTIBLE, bridge.capabilities.preemptionMode)
        assertEquals(OspfResumeMode.NONE, bridge.capabilities.resumeMode)
        assertFalse(bridge.capabilities.supportsPortableCheckpoint)
        assertFalse(bridge.capabilities.supportsNativeCheckpoint)
        assertEquals(
            OspfPreemptionMode.CONTROLLED_RETURN,
            bridge.capabilitiesFor(cpPayload()).preemptionMode
        )
    }

    @Test
    fun nativeDeclarationRequiresAllRecoveryGuarantees() {
        val invalid = runCatching {
            OspfExecutionCapabilities(
                supportedModelTypes = setOf(NormalizedModelType.CP),
                preemptionMode = OspfPreemptionMode.NATIVE,
                resumeMode = OspfResumeMode.NATIVE_CHECKPOINT,
                supportsInterrupt = true,
                supportsPortableCheckpoint = false,
                supportsNativeCheckpoint = false,
                supportsWarmStart = false
            )
        }

        assertTrue(invalid.isFailure)
    }

    @Test
    fun sliceQuantumUsesRemainingTaskLimitWithoutChangingTaskLimit() {
        val payload = cpPayload().copy(config = SolverConfig(timeLimitMs = 10_000L))
        val executor = OspfCpSnapshotExecutor(EmptyStorage)

        assertEquals(3_000L, executor.effectiveSliceTimeLimitMs(payload, quantumMs = 3_000L))
        assertEquals(
            2_000L,
            executor.effectiveSliceTimeLimitMs(payload, quantumMs = 3_000L, elapsedBeforeMs = 8_000L)
        )
        assertEquals(
            1L,
            executor.effectiveSliceTimeLimitMs(payload, quantumMs = 3_000L, elapsedBeforeMs = 10_000L)
        )
        assertEquals(10_000L, payload.config?.timeLimitMs)
    }

    @Test
    fun inProcessUnsupportedModelReturnsBackendFailure() = runBlocking {
        val bridge = OspfInProcessBridge(FixedClock, FixedIds, EmptyStorage)
        val handle = bridge.start(
            payload = SolvePayload(modelData = ModelData.raw(ByteArray(0), "unsupported-format")),
            taskId = "task-unsupported",
            sliceId = "slice-unsupported",
            nodeId = "node-1",
            tenantId = "tenant-1"
        )

        val slice = bridge.awaitSliceEnd(handle, quantumMs = 1_000L)
        val result = bridge.fetchFinalResult(handle)

        assertFalse(slice.completed)
        assertEquals(RemoteTerminationReason.BACKEND_FAILURE, slice.terminationReason)
        assertEquals(RemoteProblemStatus.UNKNOWN, slice.problemStatus)
        assertEquals(RemoteSolutionPresence.NONE, slice.solutionPresence)
        assertEquals(RemoteTerminationReason.BACKEND_FAILURE, result?.terminationReason)
        assertEquals(RemoteProblemStatus.UNKNOWN, result?.problemStatus)
        assertEquals(RemoteSolutionPresence.NONE, result?.solutionPresence)
    }

    private fun cpPayload(): SolvePayload = SolvePayload(
        modelData = ModelData.raw(ByteArray(0), "ospf-cp-snapshot-json")
    )

    private object FixedClock : ClockPort {
        override fun now(): Instant = Instant.fromEpochMilliseconds(0L)
    }

    private object FixedIds : IdGeneratorPort {
        override fun newId(prefix: String): String = "$prefix-test"
    }

    private object EmptyStorage : ObjectStoragePort {
        override suspend fun put(
            path: ObjectPath,
            bytes: ByteArray,
            metadata: Map<String, String>
        ): ObjectRef = ObjectRef.of(path.value)

        override suspend fun get(ref: ObjectRef): ByteArray? = null

        override suspend fun delete(ref: ObjectRef): Boolean = false

        override suspend fun exists(ref: ObjectRef): Boolean = false
    }
}
