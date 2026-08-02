@file:OptIn(kotlin.time.ExperimentalTime::class)

package fuookami.ospf.framework.remote_solver.application

import fuookami.ospf.framework.remote_solver.bootstrap.InMemoryRemoteSolverBootstrap
import fuookami.ospf.framework.remote_solver.contract.runSuspend
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.ModelData
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolverErrorCode
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolverException
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskMeta
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskComplexity
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.TimeSensitivity
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
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

    @Test
    fun structuredSolvePayloadArtifactIsDecodedAndRetainsCpFormat() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val facade = RemoteSolverApiFacade(runtime.service, runtime.objectStoragePort)
        val payload = SolvePayload(
            modelData = ModelData.raw(
                bytes = "{}".encodeToByteArray(),
                format = "ospf-cp-snapshot-json"
            ),
            taskMeta = TaskMeta(targetType = "cp")
        )

        runSuspend {
            runtime.objectStoragePort.put(
                path = "tenant-cp/payload",
                bytes = Json.encodeToString(SolvePayload.serializer(), payload).encodeToByteArray()
            )
            val response = facade.submit(
                TaskSubmitRequest(
                    tenantId = "tenant-cp",
                    payloadRef = ObjectRef.of("payload"),
                    taskMeta = TaskMeta(targetType = "cp")
                )
            )
            val task = runtime.service.getTask(response.taskId)
            assertNotNull(task)
            assertEquals("ospf-cp-snapshot-json", task.payload.modelData.format)
            assertEquals(
                "tenant-cp/snapshot/payload",
                task.payload.modelData.ref?.path?.value
            )
            assertEquals(
                "{}",
                runtime.objectStoragePort.get(task.payload.modelData.ref!!)?.decodeToString()
            )
            assertEquals("cp", task.payload.taskMeta.targetType?.value)
        }
    }

    @Test
    fun malformedPayloadArtifactRequiresExplicitLegacyMode() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val facade = RemoteSolverApiFacade(runtime.service, runtime.objectStoragePort)
        runSuspend {
            runtime.objectStoragePort.put(
                path = "tenant-cp/malformed",
                bytes = "not-json".encodeToByteArray()
            )
        }
        val error = assertFailsWith<RemoteSolverException> {
            runSuspend {
                facade.submit(
                    TaskSubmitRequest(
                        tenantId = "tenant-cp",
                        payloadRef = ObjectRef.of("malformed")
                    )
                )
            }
        }
        assertEquals(RemoteSolverErrorCode.INVALID_ARGUMENT, error.code)
    }

    @Test
    fun malformedPayloadArtifactCanUseExplicitLegacyModelChannel() {
        val runtime = InMemoryRemoteSolverBootstrap.create()
        val facade = RemoteSolverApiFacade(runtime.service, runtime.objectStoragePort)
        runSuspend {
            runtime.objectStoragePort.put(
                path = "tenant-legacy/model",
                bytes = "legacy-model-bytes".encodeToByteArray()
            )
            val response = facade.submit(
                TaskSubmitRequest(
                    tenantId = "tenant-legacy",
                    payloadRef = ObjectRef.of("model"),
                    extension = mapOf("payloadMode" to "legacy-model")
                )
            )
            val task = runtime.service.getTask(response.taskId)
            assertNotNull(task)
            assertEquals("tenant-legacy/model", task.payload.modelRef?.path?.value)
            assertEquals("legacy-model", task.payload.extension["payloadMode"])
            assertEquals("true", task.payload.extension["legacyPayloadRef"])
        }
    }
}
