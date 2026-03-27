package fuookami.ospf.framework.remote_solver.adapter.ospf

import fuookami.ospf.framework.remote_solver.adapter.infrastructure.SystemClockPort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.UUIDIdGeneratorPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryObjectStoragePort
import fuookami.ospf.framework.remote_solver.contract.runSuspend
import fuookami.ospf.framework.remote_solver.protocol.domain.ExecutionHandle
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceResult
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.SolveResult
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskMeta
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

class OspfSolverExecutionPortTranslationTest {
    @Test
    fun startShouldInjectNormalizedModelTypeExtension() {
        val clock = SystemClockPort()
        val objectStoragePort = InMemoryObjectStoragePort(clock)
        val bridge = RecordingBridge()
        val port = OspfSolverExecutionPort(bridge)
        val payload = SolvePayload(
            modelRef = ObjectRef.of(path = "model/lp"),
            taskMeta = TaskMeta(targetType = "milp")
        )

        runSuspend {
            port.start(
                payload = payload,
                taskId = "task-translate-1",
                sliceId = "slice-translate-1",
                nodeId = "node-1",
                tenantId = "tenant-test"
            )
        }

        val captured = bridge.lastStartPayload
        assertNotNull(captured)
        assertEquals("linear", captured.extension["normalizedModelType"])
    }

    @Test
    fun resumeShouldPreserveExistingExtensionAndInjectNormalizedModelType() {
        val clock = SystemClockPort()
        val objectStoragePort = InMemoryObjectStoragePort(clock)
        val bridge = RecordingBridge()
        val port = OspfSolverExecutionPort(bridge)
        val payload = SolvePayload(
            modelRef = ObjectRef.of(path = "model/miqcp"),
            taskMeta = TaskMeta(targetType = "quadratic"),
            extension = mapOf("traceId" to "trace-123")
        )

        runSuspend {
            port.resume(
                payload = payload,
                checkpoint = ObjectRef.of(path = "checkpoint/task-translate-2"),
                taskId = "task-translate-2",
                sliceId = "slice-translate-2",
                nodeId = "node-2",
                tenantId = "tenant-test"
            )
        }

        val captured = bridge.lastResumePayload
        assertNotNull(captured)
        assertEquals("trace-123", captured.extension["traceId"])
        assertEquals("quadratic", captured.extension["normalizedModelType"])
        assertTrue(bridge.lastResumeCheckpoint != null)
    }

    private class RecordingBridge : OspfExecutionBridge {
        var lastStartPayload: SolvePayload? = null
        var lastResumePayload: SolvePayload? = null
        var lastResumeCheckpoint: ObjectRef? = null

        override suspend fun start(
            payload: SolvePayload,
            taskId: String,
            sliceId: String,
            nodeId: String,
            tenantId: String
        ): ExecutionHandle {
            lastStartPayload = payload
            return ExecutionHandle(
                handleId = "handle-start",
                taskId = taskId,
                sliceId = sliceId,
                nodeId = nodeId,
                startedAtEpochMs = 0L
            )
        }

        override suspend fun resume(
            payload: SolvePayload,
            checkpoint: ObjectRef,
            taskId: String,
            sliceId: String,
            nodeId: String,
            tenantId: String
        ): ExecutionHandle {
            lastResumePayload = payload
            lastResumeCheckpoint = checkpoint
            return ExecutionHandle(
                handleId = "handle-resume",
                taskId = taskId,
                sliceId = sliceId,
                nodeId = nodeId,
                startedAtEpochMs = 0L
            )
        }

        override suspend fun awaitSliceEnd(handle: ExecutionHandle, quantumMs: Long): SliceResult =
            SliceResult(
                sliceId = handle.sliceId.value,
                completed = true,
                feasible = true,
                objectiveValue = 1.0,
                gap = 0.0,
                elapsedMs = quantumMs
            )

        override suspend fun exportCheckpoint(handle: ExecutionHandle): ObjectRef? = null

        override suspend fun fetchFinalResult(handle: ExecutionHandle): SolveResult? = null

        override suspend fun stop(handle: ExecutionHandle): Boolean = true
    }
}
