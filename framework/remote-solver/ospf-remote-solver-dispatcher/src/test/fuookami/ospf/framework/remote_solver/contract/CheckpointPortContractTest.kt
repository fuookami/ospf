package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.protocol.domain.CheckpointMetadata
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.port.CheckpointPort
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

abstract class CheckpointPortContractTest {
    protected abstract fun createFixture(): TestFixture<CheckpointPort>

    @Test
    fun saveAndLoadLatest() {
        val fixture = createFixture()
        try {
            runSuspend {
                fixture.subject.save(
                    CheckpointMetadata(
                        taskId = "task-1",
                        sliceId = "slice-1",
                        ref = ObjectRef.of(path = "checkpoint/task-1/slice-1", version = "v1"),
                        createdAtEpochMs = 1000L
                    )
                )
                fixture.subject.save(
                    CheckpointMetadata(
                        taskId = "task-1",
                        sliceId = "slice-2",
                        ref = ObjectRef.of(path = "checkpoint/task-1/slice-2", version = "v2"),
                        createdAtEpochMs = 2000L
                    )
                )

                val latest = fixture.subject.latest("task-1")
                assertNotNull(latest)
                assertEquals("slice-2", latest.sliceId.value)
                assertEquals("v2", latest.ref.version?.value)
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun listReturnsSortedCheckpoints() {
        val fixture = createFixture()
        try {
            runSuspend {
                fixture.subject.save(
                    CheckpointMetadata(
                        taskId = "task-2",
                        sliceId = "slice-2",
                        ref = ObjectRef.of(path = "checkpoint/task-2/slice-2", version = "v2"),
                        createdAtEpochMs = 2000L
                    )
                )
                fixture.subject.save(
                    CheckpointMetadata(
                        taskId = "task-2",
                        sliceId = "slice-1",
                        ref = ObjectRef.of(path = "checkpoint/task-2/slice-1", version = "v1"),
                        createdAtEpochMs = 1000L
                    )
                )

                val list = fixture.subject.list("task-2")
                assertEquals(2, list.size)
                assertTrue(list[0].createdAtEpochMs <= list[1].createdAtEpochMs)
                assertEquals("slice-1", list[0].sliceId.value)
                assertEquals("slice-2", list[1].sliceId.value)
            }
        } finally {
            fixture.close()
        }
    }
}
