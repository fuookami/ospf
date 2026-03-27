package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.port.SolverExecutionPort
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

abstract class SolverExecutionPortContractTest {
    protected abstract fun createFixture(): TestFixture<SolverExecutionPort>

    @Test
    fun startAndAwaitShouldEventuallyComplete() {
        val fixture = createFixture()
        try {
            runSuspend {
                val handle = fixture.subject.start(
                    payload = samplePayload(),
                    taskId = "task-a",
                    sliceId = "slice-a",
                    nodeId = "node-a",
                    tenantId = "tenant-test"
                )
                assertEquals("task-a", handle.taskId.value)
                assertEquals("slice-a", handle.sliceId.value)
                assertEquals("node-a", handle.nodeId.value)

                var result = fixture.subject.awaitSliceEnd(handle, quantumMs = 4000L)
                var rounds = 1
                while (!result.completed && rounds < 10) {
                    result = fixture.subject.awaitSliceEnd(handle, quantumMs = 4000L)
                    rounds += 1
                }

                assertTrue(result.completed)
                val finalResult = fixture.subject.fetchFinalResult(handle)
                assertNotNull(finalResult)
                assertTrue(finalResult.feasible)
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun resumeShouldContinueFromCheckpointWhenCheckpointAvailable() {
        val fixture = createFixture()
        try {
            runSuspend {
                val firstHandle = fixture.subject.start(
                    payload = samplePayload(),
                    taskId = "task-b",
                    sliceId = "slice-b1",
                    nodeId = "node-b",
                    tenantId = "tenant-test"
                )
                fixture.subject.awaitSliceEnd(firstHandle, quantumMs = 2000L)
                val checkpoint = fixture.subject.exportCheckpoint(firstHandle) ?: return@runSuspend

                val resumedHandle = fixture.subject.resume(
                    payload = samplePayload(snapshotRef = checkpoint),
                    checkpoint = checkpoint,
                    taskId = "task-b",
                    sliceId = "slice-b2",
                    nodeId = "node-b",
                    tenantId = "tenant-test"
                )
                assertEquals("slice-b2", resumedHandle.sliceId.value)

                var result = fixture.subject.awaitSliceEnd(resumedHandle, quantumMs = 6000L)
                var rounds = 1
                while (!result.completed && rounds < 10) {
                    result = fixture.subject.awaitSliceEnd(resumedHandle, quantumMs = 6000L)
                    rounds += 1
                }
                assertTrue(result.completed)
                assertNotNull(fixture.subject.fetchFinalResult(resumedHandle))
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun stopShouldReturnFalseAfterHandleAlreadyRemoved() {
        val fixture = createFixture()
        try {
            runSuspend {
                val handle = fixture.subject.start(
                    payload = samplePayload(),
                    taskId = "task-c",
                    sliceId = "slice-c",
                    nodeId = "node-c",
                    tenantId = "tenant-test"
                )
                val firstStop = fixture.subject.stop(handle)
                val secondStop = fixture.subject.stop(handle)
                assertTrue(firstStop)
                assertFalse(secondStop)
            }
        } finally {
            fixture.close()
        }
    }

    private fun samplePayload(snapshotRef: ObjectRef? = null): SolvePayload =
        SolvePayload(
            modelRef = ObjectRef.of(path = "model/contract"),
            snapshotRef = snapshotRef
        )
}
