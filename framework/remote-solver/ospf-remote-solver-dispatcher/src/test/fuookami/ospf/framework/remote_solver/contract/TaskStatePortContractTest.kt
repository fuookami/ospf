@file:OptIn(kotlin.time.ExperimentalTime::class)

package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.domain.SliceState
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceId
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskComplexity
import fuookami.ospf.framework.remote_solver.domain.TaskState
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.TimeSensitivity
import fuookami.ospf.framework.remote_solver.port.TaskStatePort
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

abstract class TaskStatePortContractTest {
    protected abstract fun createFixture(): TestFixture<TaskStatePort>

    @Test
    fun upsertAndGetTask() {
        val fixture = createFixture()
        try {
            runSuspend {
                val task = sampleTask(taskId = "task-a", priority = 3)
                fixture.subject.upsertTask(task)

                val loaded = fixture.subject.getTask(task.taskId)
                assertNotNull(loaded)
                assertEquals(task.taskId, loaded.taskId)
                assertEquals(task.status, loaded.status)
                assertEquals(task.priority, loaded.priority)
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun getTaskByRequestIdShouldReturnMatchedTask() {
        val fixture = createFixture()
        try {
            runSuspend {
                val task = sampleTask(taskId = "task-a2")
                fixture.subject.upsertTask(task)

                val loaded = fixture.subject.getTaskByRequestId(task.requestId)
                assertNotNull(loaded)
                assertEquals(task.taskId, loaded.taskId)
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun getTaskByRequestIdAndTenantIdShouldReturnMatchedTask() {
        val fixture = createFixture()
        try {
            runSuspend {
                val task = sampleTask(taskId = "task-tenant-req", tenantId = "tenant-a")
                fixture.subject.upsertTask(task)

                val loaded = fixture.subject.getTaskByRequestId("tenant-a", task.requestId.value)
                assertNotNull(loaded)
                assertEquals(task.taskId, loaded.taskId)
                assertEquals("tenant-a", loaded.tenantId.value)
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun differentTenantsCanUseSameRequestId() {
        val fixture = createFixture()
        try {
            runSuspend {
                val sharedRequestId = "shared-request-id"
                val taskA = sampleTask(taskId = "task-tenant-a", tenantId = "tenant-a", requestId = sharedRequestId)
                val taskB = sampleTask(taskId = "task-tenant-b", tenantId = "tenant-b", requestId = sharedRequestId)

                fixture.subject.upsertTask(taskA)
                fixture.subject.upsertTask(taskB)

                val loadedA = fixture.subject.getTaskByRequestId("tenant-a", sharedRequestId)
                val loadedB = fixture.subject.getTaskByRequestId("tenant-b", sharedRequestId)

                assertNotNull(loadedA)
                assertNotNull(loadedB)
                assertEquals("task-tenant-a", loadedA.taskId.value)
                assertEquals("task-tenant-b", loadedB.taskId.value)
                assertEquals("tenant-a", loadedA.tenantId.value)
                assertEquals("tenant-b", loadedB.tenantId.value)
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun compareAndSetUsesExpectedStatusSet() {
        val fixture = createFixture()
        try {
            runSuspend {
                val task = sampleTask(taskId = "task-b", status = TaskStatus.QUEUED)
                fixture.subject.upsertTask(task)

                val fail = fixture.subject.compareAndSet(
                    taskId = task.taskId,
                    from = setOf(TaskStatus.RUNNING),
                    to = TaskStatus.SUSPENDED,
                    updatedAt = kotlin.time.Instant.fromEpochMilliseconds(2000L)
                )
                assertTrue(!fail)

                val success = fixture.subject.compareAndSet(
                    taskId = task.taskId,
                    from = setOf(TaskStatus.QUEUED, TaskStatus.ACCEPTED),
                    to = TaskStatus.RUNNING,
                    updatedAt = kotlin.time.Instant.fromEpochMilliseconds(3000L)
                )
                assertTrue(success)

                val loaded = fixture.subject.getTask(task.taskId)
                assertNotNull(loaded)
                assertEquals(TaskStatus.RUNNING, loaded.status)
                assertEquals(3000L, loaded.updatedAtEpochMs)
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun listTasksShouldOrderByPriorityThenCreatedTime() {
        val fixture = createFixture()
        try {
            runSuspend {
                fixture.subject.upsertTask(sampleTask(taskId = "task-c1", priority = 1, createdAt = 1000L))
                fixture.subject.upsertTask(sampleTask(taskId = "task-c2", priority = 5, createdAt = 3000L))
                fixture.subject.upsertTask(sampleTask(taskId = "task-c3", priority = 5, createdAt = 2000L))

                val list = fixture.subject.listTasks(setOf(TaskStatus.QUEUED), limit = 3)
                assertEquals(3, list.size)
                assertEquals("task-c3", list[0].taskId.value)
                assertEquals("task-c2", list[1].taskId.value)
                assertEquals("task-c1", list[2].taskId.value)
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun appendAndUpdateSlice() {
        val fixture = createFixture()
        try {
            runSuspend {
                val task = sampleTask(taskId = "task-d")
                fixture.subject.upsertTask(task)

                val slice = SliceState(
                    sliceId = "slice-1",
                    taskId = task.taskId.value,
                    dispatchId = "dispatch-1",
                    status = SliceStatus.PLANNED,
                    nodeId = "node-1",
                    quantum = 1000L
                )
                fixture.subject.appendSlice(slice)

                val updated = slice.copy(
                    status = SliceStatus.RUNNING,
                    startedAt = kotlin.time.Instant.fromEpochMilliseconds(5000L)
                )
                fixture.subject.updateSlice(updated)

                val slices = fixture.subject.getSlices(task.taskId)
                assertEquals(1, slices.size)
                assertEquals(SliceStatus.RUNNING, slices[0].status)
                assertEquals(5000L, slices[0].startedAtEpochMs)
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun appendSliceShouldBeIdempotentBySliceAndDispatchId() {
        val fixture = createFixture()
        try {
            runSuspend {
                val task = sampleTask(taskId = "task-e")
                fixture.subject.upsertTask(task)

                val base = SliceState(
                    sliceId = "slice-e1",
                    taskId = task.taskId.value,
                    dispatchId = "dispatch-e1",
                    status = SliceStatus.PLANNED,
                    nodeId = "node-e",
                    quantum = 1200L
                )
                fixture.subject.appendSlice(base)
                fixture.subject.appendSlice(base.copy(status = SliceStatus.RUNNING))
                fixture.subject.appendSlice(base.copy(sliceId = SliceId.of("slice-e2")))

                val slices = fixture.subject.getSlices(task.taskId)
                assertEquals(1, slices.size)
                assertEquals("slice-e1", slices[0].sliceId.value)
                assertEquals("dispatch-e1", slices[0].dispatchId.value)
            }
        } finally {
            fixture.close()
        }
    }

    private fun sampleTask(
        taskId: String,
        status: TaskStatus = TaskStatus.QUEUED,
        priority: Int = 1,
        createdAt: Long = 1000L,
        tenantId: String = "default",
        requestId: String? = null
    ): TaskState =
        TaskState(
            taskId = taskId,
            requestId = requestId ?: "req-$taskId",
            tenantId = tenantId,
            status = status,
            complexity = TaskComplexity.SIMPLE,
            timeSensitivity = TimeSensitivity.NON_REALTIME,
            priority = priority,
            deadline = null,
            payload = SolvePayload(modelRef = ObjectRef.of(path = "models/$taskId")),
            createdAt = createdAt,
            updatedAt = createdAt,
            budgetScope = taskId
        )
}
