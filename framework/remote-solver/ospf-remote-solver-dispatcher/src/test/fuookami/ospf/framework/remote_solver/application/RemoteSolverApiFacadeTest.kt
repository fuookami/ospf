@file:OptIn(kotlin.time.ExperimentalTime::class)

package fuookami.ospf.framework.remote_solver.application

import fuookami.ospf.framework.remote_solver.bootstrap.InMemoryRemoteSolverBootstrap
import fuookami.ospf.framework.remote_solver.contract.runSuspend
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolverErrorCode
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolverException
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskComplexity
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.TimeSensitivity
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

class RemoteSolverApiFacadeTest {
    @Test
    fun submitShouldReturnAcceptedQueuedTask() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val facade = RemoteSolverApiFacade(runtime.service)

        runSuspend {
            val response = facade.submit(
                TaskSubmitRequest(
                    payloadRef = ObjectRef.of(path = "models/api-submit"),
                    complexity = TaskComplexity.SIMPLE,
                    timeSensitivity = TimeSensitivity.NON_REALTIME
                )
            )
            assertTrue(response.accepted)
            assertEquals(TaskStatus.QUEUED, response.status)
            assertTrue(response.taskId.isNotBlank())
        }
    }

    @Test
    fun stopThenResumeShouldMoveTaskToQueued() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val facade = RemoteSolverApiFacade(runtime.service)

        runSuspend {
            val created = facade.submit(
                TaskSubmitRequest(
                    payloadRef = ObjectRef.of(path = "models/api-stop-resume"),
                    complexity = TaskComplexity.SIMPLE,
                    timeSensitivity = TimeSensitivity.NON_REALTIME
                )
            )
            val stopped = facade.stop(created.taskId, reason = "api-stop")
            assertNotNull(stopped)
            assertEquals(TaskStatus.STOPPED, stopped.status)

            val resumed = facade.resume(created.taskId)
            assertNotNull(resumed)
            assertEquals(TaskStatus.QUEUED, resumed.status)
        }
    }

    @Test
    fun blankPayloadPathShouldFailAtObjectRefBoundary() {
        assertFailsWith<IllegalArgumentException> {
            ObjectRef.of(path = "   ")
        }
    }

    @Test
    fun resumeFailedTaskShouldPropagateInvalidStateTransition() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val facade = RemoteSolverApiFacade(runtime.service)

        runSuspend {
            val created = facade.submit(
                TaskSubmitRequest(
                    payloadRef = ObjectRef.of(path = "models/api-resume-failed"),
                    complexity = TaskComplexity.SIMPLE,
                    timeSensitivity = TimeSensitivity.NON_REALTIME
                )
            )
            val stored = runtime.service.getTask(created.taskId)
            assertNotNull(stored)
            runtime.taskStatePort.upsertTask(
                stored.copy(
                    status = TaskStatus.FAILED,
                    updatedAt = kotlin.time.Instant.fromEpochMilliseconds(System.currentTimeMillis())
                )
            )
        }

        val error = assertFailsWith<RemoteSolverException> {
            runSuspend {
                facade.resume(taskId = runtime.taskStatePort.listTasks(setOf(TaskStatus.FAILED), 1).first().taskId.value)
            }
        }
        assertEquals(RemoteSolverErrorCode.INVALID_TASK_STATE_TRANSITION, error.code)
    }

    @Test
    fun schedulerHotReloadAndRollbackShouldWorkViaFacade() {
        val runtime = InMemoryRemoteSolverBootstrap.create(
            config = RemoteSolverConfig(
                schedulerConfigVersion = "v-base",
                schedulerHotReloadEnabled = true
            )
        )
        val facade = RemoteSolverApiFacade(runtime.service)

        runSuspend {
            val hotReload = facade.hotReloadScheduler(
                SchedulerHotReloadRequest(
                    operator = "api-test",
                    changeSet = mapOf("scheduler.simple-task-quantum-ms" to "3100")
                )
            )
            assertTrue(hotReload.version.isNotBlank())

            val rollback = facade.rollbackScheduler(
                SchedulerRollbackRequest(
                    operator = "api-test",
                    targetVersion = "v-base"
                )
            )
            assertTrue(rollback.version.isNotBlank())

            val audits = facade.listSchedulerConfigAudits(limit = 10)
            assertTrue(audits.size >= 2)
        }
    }

    @Test
    fun replayTaskTimelineShouldReturnEvents() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val facade = RemoteSolverApiFacade(runtime.service)

        runSuspend {
            val created = facade.submit(
                TaskSubmitRequest(
                    payloadRef = ObjectRef.of(path = "models/api-replay"),
                    complexity = TaskComplexity.SIMPLE,
                    timeSensitivity = TimeSensitivity.NON_REALTIME
                )
            )
            val report = facade.replayTaskTimeline(created.taskId, limitEvents = 50)
            assertEquals(created.taskId, report.taskId)
            assertTrue(report.events.isNotEmpty())
        }
    }

    @Test
    fun submitWithTenantShouldPrefixModelPathAndBudgetScope() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val facade = RemoteSolverApiFacade(runtime.service)

        runSuspend {
            val response = facade.submit(
                TaskSubmitRequest(
                    tenantId = "tenant-a",
                    payloadRef = ObjectRef.of(path = "models/tenant-model"),
                    budgetScope = "biz-a",
                    complexity = TaskComplexity.SIMPLE,
                    timeSensitivity = TimeSensitivity.NON_REALTIME
                )
            )
            val task = runtime.service.getTask(response.taskId)
            assertNotNull(task)
            assertEquals("tenant-a", task.tenantId.value)
            assertEquals("tenant-a/models/tenant-model", task.payload.modelRef?.path?.value)
            assertEquals("tenant-a:biz-a", task.budgetScope.value)
        }
    }

    @Test
    fun submitWithIllegalTenantShouldFail() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val facade = RemoteSolverApiFacade(runtime.service)

        val error = assertFailsWith<RemoteSolverException> {
            runSuspend {
                facade.submit(
                    TaskSubmitRequest(
                        tenantId = "tenant/a",
                        payloadRef = ObjectRef.of(path = "models/x")
                    )
                )
            }
        }
        assertEquals(RemoteSolverErrorCode.INVALID_ARGUMENT, error.code)
    }
}
