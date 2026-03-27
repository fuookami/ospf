package fuookami.ospf.framework.remote_solver.adapter.inmemory

import fuookami.ospf.framework.remote_solver.contract.runSuspend
import fuookami.ospf.framework.remote_solver.protocol.domain.CheckpointMetadata
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import kotlin.test.Test
import kotlin.test.assertEquals

class InMemoryCheckpointPortRetentionTest {
    @Test
    fun retentionShouldKeepLatestCheckpoints() {
        val checkpointPort = InMemoryCheckpointPort(maxRetainedPerTask = 2)

        runSuspend {
            checkpointPort.save(
                CheckpointMetadata(
                    taskId = "task-1",
                    sliceId = "slice-1",
                    ref = ObjectRef.of(path = "checkpoint/task-1/slice-1", version = "v1"),
                    createdAtEpochMs = 1000L
                )
            )
            checkpointPort.save(
                CheckpointMetadata(
                    taskId = "task-1",
                    sliceId = "slice-2",
                    ref = ObjectRef.of(path = "checkpoint/task-1/slice-2", version = "v2"),
                    createdAtEpochMs = 2000L
                )
            )
            checkpointPort.save(
                CheckpointMetadata(
                    taskId = "task-1",
                    sliceId = "slice-3",
                    ref = ObjectRef.of(path = "checkpoint/task-1/slice-3", version = "v3"),
                    createdAtEpochMs = 3000L
                )
            )

            val checkpoints = checkpointPort.list("task-1")
            assertEquals(2, checkpoints.size)
            assertEquals(listOf("slice-2", "slice-3"), checkpoints.map { it.sliceId.value })
        }
    }
}
