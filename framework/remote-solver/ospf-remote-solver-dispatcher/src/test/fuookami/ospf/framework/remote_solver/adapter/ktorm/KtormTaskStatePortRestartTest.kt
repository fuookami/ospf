@file:OptIn(kotlin.time.ExperimentalTime::class)

package fuookami.ospf.framework.remote_solver.adapter.ktorm

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlinx.coroutines.runBlocking
import fuookami.ospf.framework.remote_solver.domain.TaskState
import fuookami.ospf.framework.remote_solver.protocol.domain.ModelData
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.SolveResult
import fuookami.ospf.framework.remote_solver.protocol.domain.SolverConfig
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskComplexity
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.TimeSensitivity

/**
 * Ktorm persistence reconstruction regression tests.
 * Ktorm 持久化端口重建回归测试。
 */
class KtormTaskStatePortRestartTest {
    /**
     * Verify task configuration and object references survive port recreation.
     * 验证任务配置和对象引用在端口重建后仍可恢复。
     */
    @Test
    fun taskResultAndObjectRefsSurvivePortRecreation() = runBlocking {
        val databaseName = "remote_restart_${System.nanoTime()}"
        val jdbcUrl = "jdbc:h2:mem:$databaseName;MODE=PostgreSQL;DB_CLOSE_DELAY=-1"
        val taskTable = "rs_restart_task_$databaseName"
        val sliceTable = "rs_restart_slice_$databaseName"
        val task = TaskState(
            taskId = "restart-task",
            requestId = "restart-request",
            tenantId = "tenant-restart",
            status = TaskStatus.COMPLETED,
            complexity = TaskComplexity.SIMPLE,
            timeSensitivity = TimeSensitivity.NON_REALTIME,
            priority = 2,
            deadline = null,
            payload = SolvePayload(
                modelData = ModelData.reference(ObjectRef.of("models/restart")),
                config = SolverConfig(
                    timeLimitMs = 1500L,
                    solutionLimit = 3,
                    mipGapTolerance = 0.05,
                    threads = 2,
                    solverParams = mapOf("presolve" to "aggressive")
                )
            ),
            latestResult = SolveResult(
                feasible = true,
                optimal = true,
                objectiveValue = null,
                gap = null,
                elapsedMs = 12L,
                resultRef = ObjectRef.of("tenant-restart/result/restart-task/slice-1", version = "v1", etag = "etag-result"),
                checkpointRef = ObjectRef.of("tenant-restart/checkpoint/restart-task/slice-1", version = "v1", etag = "etag-checkpoint")
            ),
            createdAt = 1000L,
            updatedAt = 2000L,
            budgetScope = "restart-task"
        )

        KtormTaskStatePort(
            jdbcUrl = jdbcUrl,
            taskTableName = taskTable,
            sliceTableName = sliceTable
        ).apply {
            upsertTask(task)
            appendSlice(
                fuookami.ospf.framework.remote_solver.domain.SliceState(
                    sliceId = "slice-1",
                    taskId = task.taskId.value,
                    dispatchId = "dispatch-1",
                    status = SliceStatus.COMPLETED,
                    nodeId = "node-1",
                    quantum = 1000L,
                    checkpointRef = task.latestResult?.checkpointRef,
                    resultRef = task.latestResult?.resultRef
                )
            )
        }

        val reloaded = KtormTaskStatePort(
            jdbcUrl = jdbcUrl,
            taskTableName = taskTable,
            sliceTableName = sliceTable
        )
        val loadedTask = reloaded.getTask(task.taskId)
        assertNotNull(loadedTask)
        assertEquals(task.payload.config, loadedTask.payload.config)
        assertEquals(task.latestResult?.resultRef, loadedTask.latestResult?.resultRef)
        assertEquals(task.latestResult?.checkpointRef, loadedTask.latestResult?.checkpointRef)

        val loadedSlice = reloaded.getSlices(task.taskId).single()
        assertEquals(task.latestResult?.resultRef, loadedSlice.resultRef)
        assertEquals(task.latestResult?.checkpointRef, loadedSlice.checkpointRef)
    }
}
