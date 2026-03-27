@file:OptIn(kotlin.time.ExperimentalTime::class)

/*
 * 任务时间线回放器
 *
 * 本模块提供任务时间线回放功能，
 * 用于重建任务的历史执行过程并检测异常。
 * 支持从事件日志重建或从状态数据重建两种方式。
 *
 * Task Timeline Replayer
 *
 * This module provides task timeline replay functionality,
 * used to reconstruct the historical execution process of a task and detect anomalies.
 * Supports two reconstruction methods: from event log or from state data.
 */
package fuookami.ospf.framework.remote_solver.application

import fuookami.ospf.framework.remote_solver.protocol.domain.CheckpointMetadata
import fuookami.ospf.framework.remote_solver.domain.CostRecord
import fuookami.ospf.framework.remote_solver.domain.EventEnvelopeHeaders
import fuookami.ospf.framework.remote_solver.domain.EventRecord
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolverErrorCode
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolverException
import fuookami.ospf.framework.remote_solver.domain.SliceState
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceStatus
import fuookami.ospf.framework.remote_solver.domain.TaskState
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskId
import fuookami.ospf.framework.remote_solver.protocol.port.CheckpointPort
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import fuookami.ospf.framework.remote_solver.port.CostLedgerPort
import fuookami.ospf.framework.remote_solver.port.TaskEventQueryPort
import fuookami.ospf.framework.remote_solver.port.TaskStatePort
import java.time.Instant
import java.nio.charset.StandardCharsets

/**
 * 任务时间线回放器
 *
 * 用于重建任务的历史执行时间线并检测异常。
 *
 * Task timeline replayer.
 *
 * Used to reconstruct the historical execution timeline of a task and detect anomalies.
 *
 * @param taskStatePort 任务状态端口，用于查询任务和切片状态
 *                      Task state port for querying task and slice states
 * @param checkpointPort 检查点端口，用于查询检查点元数据
 *                       Checkpoint port for querying checkpoint metadata
 * @param costLedgerPort 成本账本端口，用于查询成本记录
 *                       Cost ledger port for querying cost records
 * @param clock 时钟端口，用于获取当前时间
 *              Clock port for getting current time
 * @param taskEventQueryPort 任务事件查询端口（可选），用于从事件日志重建时间线
 *                            Task event query port (optional) for reconstructing timeline from event log
 */
class TaskTimelineReplayer(
    private val taskStatePort: TaskStatePort,
    private val checkpointPort: CheckpointPort,
    private val costLedgerPort: CostLedgerPort,
    private val clock: ClockPort,
    private val taskEventQueryPort: TaskEventQueryPort? = null
) {
    /**
     * 回放任务时间线
     *
     * 重建任务的历史执行时间线，并检测时间线中的异常或不一致。
     *
     * Replays task timeline.
     *
     * Reconstructs the historical execution timeline of a task and detects anomalies or inconsistencies.
     *
     * @param taskId 任务标识
     *               Task identifier
     * @param limitEvents 最大事件数量限制，默认为 500
     *                     Maximum event count limit, defaults to 500
     * @return 任务时间线报告
     *         Task timeline report
     * @throws RemoteSolverException 当任务不存在或参数无效时抛出
     *                                Thrown when task does not exist or parameters are invalid
     */
    suspend fun replay(taskId: String, limitEvents: Int = 500): TaskTimelineReport {
        val normalizedTaskId = taskId.trim()
        if (normalizedTaskId.isEmpty()) {
            throw RemoteSolverException(
                code = RemoteSolverErrorCode.INVALID_ARGUMENT,
                message = "taskId must not be blank"
            )
        }

        val task = taskStatePort.getTask(normalizedTaskId)
            ?: throw RemoteSolverException(
                code = RemoteSolverErrorCode.INVALID_ARGUMENT,
                message = "Task not found: $normalizedTaskId"
            )

        val slices = taskStatePort.getSlices(normalizedTaskId)
        val checkpoints = checkpointPort.list(normalizedTaskId)
        val costRecords = costLedgerPort.listByTask(normalizedTaskId)

        val eventRecords = taskEventQueryPort?.queryTaskEvents(
            taskId = normalizedTaskId,
            fromEpochMs = task.createdAt.toEpochMilliseconds(),
            toEpochMs = null,
            limit = limitEvents
        ) ?: emptyList()
        val sourceGaps = mutableListOf<TaskTimelineGap>()
        val events = if (eventRecords.isNotEmpty()) {
            buildEventsFromEventLog(eventRecords)
        } else {
            if (taskEventQueryPort != null) {
                sourceGaps.add(
                    TaskTimelineGap(
                        severity = "WARN",
                        message = "event log is empty, fallback to state-rebuild timeline",
                        relatedIds = mapOf("taskId" to normalizedTaskId)
                    )
                )
            }
            buildEvents(task, slices, checkpoints, costRecords)
                .sortedBy { it.atEpochMs }
                .let { sorted ->
                    val safeLimit = limitEvents.coerceAtLeast(1)
                    if (sorted.size <= safeLimit) sorted else sorted.takeLast(safeLimit)
                }
        }

        val gaps = detectGaps(task, slices, checkpoints, costRecords) + sourceGaps

        return TaskTimelineReport(
            taskId = normalizedTaskId,
            generatedAtEpochMs = clock.nowEpochMs(),
            status = task.status.name,
            events = events,
            gaps = gaps
        )
    }

    suspend fun replay(taskId: TaskId, limitEvents: Int = 500): TaskTimelineReport =
        replay(taskId.value, limitEvents)

    private fun buildEventsFromEventLog(records: List<EventRecord>): List<TaskTimelineEvent> =
        records.sortedBy { it.createdAtEpochMs }.map { record ->
            val payloadText = runCatching { String(record.payload, StandardCharsets.UTF_8) }.getOrDefault("<binary>")
            TaskTimelineEvent(
                atEpochMs = record.createdAtEpochMs,
                type = record.headers[EventEnvelopeHeaders.EVENT_TYPE] ?: "EVENT_LOG",
                summary = "topic=${record.topic} key=${record.key} eventId=${record.eventId}",
                attributes = mapOf(
                    "source" to "event_log",
                    "topic" to record.topic,
                    "key" to record.key,
                    "eventId" to record.eventId,
                    "schemaVersion" to record.schemaVersion.toString(),
                    "deliveryAttempt" to record.deliveryAttempt.toString(),
                    "traceId" to (record.headers[EventEnvelopeHeaders.TRACE_ID] ?: ""),
                    "spanId" to (record.headers[EventEnvelopeHeaders.SPAN_ID] ?: ""),
                    "tenantId" to (record.headers[EventEnvelopeHeaders.TENANT_ID] ?: ""),
                    "payload" to payloadText
                )
            )
        }

    /**
     * 将时间线报告渲染为文本格式
     *
     * Renders timeline report as text format.
     *
     * @param report 时间线报告
     *               Timeline report
     * @return 文本格式的时间线报告
     *         Text format timeline report
     */
    fun renderText(report: TaskTimelineReport): String {
        val sb = StringBuilder()
        sb.appendLine("taskId=${report.taskId}")
        sb.appendLine("status=${report.status}")
        sb.appendLine("generatedAtEpochMs=${report.generatedAtEpochMs}")
        sb.appendLine("generatedAt=${formatInstant(report.generatedAtEpochMs)}")
        sb.appendLine("events=${report.events.size}")
        sb.appendLine("gaps=${report.gaps.size}")
        sb.appendLine("----------------------------------------")
        report.events.forEachIndexed { index, event ->
            sb.appendLine(
                "${index + 1}. at=${formatInstant(event.atEpochMs)} (${event.atEpochMs}) type=${event.type} summary=${event.summary}"
            )
            if (event.attributes.isNotEmpty()) {
                sb.appendLine("   attributes=" + event.attributes.entries.joinToString(",") { "${it.key}=${it.value}" })
            }
        }
        if (report.gaps.isNotEmpty()) {
            sb.appendLine("----------------------------------------")
            sb.appendLine("gaps:")
            report.gaps.forEachIndexed { index, gap ->
                sb.appendLine("${index + 1}. severity=${gap.severity} message=${gap.message}")
                if (gap.relatedIds.isNotEmpty()) {
                    sb.appendLine("   related=" + gap.relatedIds.entries.joinToString(",") { "${it.key}=${it.value}" })
                }
            }
        }
        return sb.toString()
    }

    private fun buildEvents(
        task: TaskState,
        slices: List<SliceState>,
        checkpoints: List<CheckpointMetadata>,
        costRecords: List<CostRecord>
    ): List<TaskTimelineEvent> {
        val events = mutableListOf<TaskTimelineEvent>()
        events.add(
            TaskTimelineEvent(
                atEpochMs = task.createdAt.toEpochMilliseconds(),
                type = "TASK_CREATED",
                summary = "status=${task.status.name}",
                attributes = mapOf(
                    "complexity" to task.complexity.name,
                    "timeSensitivity" to task.timeSensitivity.name,
                    "priority" to task.priority.toString()
                )
            )
        )
        if (task.updatedAt != task.createdAt) {
            events.add(
                TaskTimelineEvent(
                    atEpochMs = task.updatedAt.toEpochMilliseconds(),
                    type = "TASK_UPDATED",
                    summary = "status=${task.status.name}",
                    attributes = mapOf(
                        "assignedNodeId" to (task.assignedNodeId?.value ?: "")
                    )
                )
            )
        }

        slices.forEach { slice ->
            val startedAt = slice.startedAt
            if (startedAt != null) {
                events.add(
                    TaskTimelineEvent(
                        atEpochMs = startedAt.toEpochMilliseconds(),
                        type = "SLICE_STARTED",
                        summary = "sliceId=${slice.sliceId} nodeId=${slice.nodeId?.value ?: ""} quantumMs=${slice.quantum.inWholeMilliseconds}",
                        attributes = mapOf(
                            "sliceId" to slice.sliceId.value,
                            "dispatchId" to slice.dispatchId.value,
                            "nodeId" to (slice.nodeId?.value ?: ""),
                            "status" to slice.status.name
                        )
                    )
                )
            }
            val finishedAt = slice.finishedAt
            if (finishedAt != null) {
                events.add(
                    TaskTimelineEvent(
                        atEpochMs = finishedAt.toEpochMilliseconds(),
                        type = "SLICE_FINISHED",
                        summary = "sliceId=${slice.sliceId} status=${slice.status.name}",
                        attributes = mapOf(
                            "sliceId" to slice.sliceId.value,
                            "status" to slice.status.name,
                            "error" to (slice.error ?: "")
                        )
                    )
                )
            }
            slice.checkpointRef?.let { ref ->
                val at = finishedAt ?: startedAt ?: task.updatedAt
                events.add(
                    TaskTimelineEvent(
                        atEpochMs = at.toEpochMilliseconds(),
                        type = "SLICE_CHECKPOINT_REF",
                        summary = "sliceId=${slice.sliceId} checkpointPath=${ref.path.value}",
                        attributes = mapOf("sliceId" to slice.sliceId.value, "checkpointPath" to ref.path.value)
                    )
                )
            }
            slice.resultRef?.let { ref ->
                val at = finishedAt ?: startedAt ?: task.updatedAt
                events.add(
                    TaskTimelineEvent(
                        atEpochMs = at.toEpochMilliseconds(),
                        type = "SLICE_RESULT_REF",
                        summary = "sliceId=${slice.sliceId} resultPath=${ref.path.value}",
                        attributes = mapOf("sliceId" to slice.sliceId.value, "resultPath" to ref.path.value)
                    )
                )
            }
        }

        checkpoints.forEach { cp ->
            events.add(
                TaskTimelineEvent(
                    atEpochMs = cp.createdAtEpochMs,
                    type = "CHECKPOINT_SAVED",
                    summary = "sliceId=${cp.sliceId.value} checkpointPath=${cp.ref.path.value}",
                    attributes = mapOf(
                        "sliceId" to cp.sliceId.value,
                        "checkpointPath" to cp.ref.path.value
                    )
                )
            )
        }

        costRecords.forEach { cost ->
            events.add(
                TaskTimelineEvent(
                    atEpochMs = cost.createdAtEpochMs,
                    type = "COST_RECORDED",
                    summary = "sliceId=${cost.sliceId} nodeId=${cost.nodeId} totalCost=${cost.totalCost}",
                    attributes = mapOf(
                        "sliceId" to cost.sliceId,
                        "nodeId" to cost.nodeId,
                        "totalCost" to cost.totalCost.toString(),
                        "runtimeMs" to cost.runtimeMs.toString()
                    )
                )
            )
        }

        task.latestResult?.let { result ->
            val reasonCode = result.extension["reasonCode"]
            val at = task.updatedAt.toEpochMilliseconds()
            if (task.status in setOf(TaskStatus.COMPLETED, TaskStatus.FAILED, TaskStatus.STOPPED)) {
                events.add(
                    TaskTimelineEvent(
                        atEpochMs = at,
                        type = "TASK_TERMINAL",
                        summary = "status=${task.status.name} reasonCode=${reasonCode ?: ""}",
                        attributes = mapOf(
                            "status" to task.status.name,
                            "reasonCode" to (reasonCode ?: ""),
                            "feasible" to result.feasible.toString(),
                            "optimal" to result.optimal.toString()
                        )
                    )
                )
            }
        }

        return events
    }

    private fun detectGaps(
        task: TaskState,
        slices: List<SliceState>,
        checkpoints: List<CheckpointMetadata>,
        costRecords: List<CostRecord>
    ): List<TaskTimelineGap> {
        val gaps = mutableListOf<TaskTimelineGap>()

        if (task.updatedAt < task.createdAt) {
            gaps.add(
                TaskTimelineGap(
                    severity = "ERROR",
                    message = "task.updatedAt < task.createdAt"
                )
            )
        }

        slices.forEach { slice ->
            val startedAt = slice.startedAt
            val finishedAt = slice.finishedAt
            if (slice.status == SliceStatus.RUNNING && startedAt == null) {
                gaps.add(
                    TaskTimelineGap(
                        severity = "WARN",
                        message = "slice is RUNNING but startedAtEpochMs is null",
                        relatedIds = mapOf("sliceId" to slice.sliceId.value)
                    )
                )
            }
            if (finishedAt != null && startedAt != null && finishedAt < startedAt) {
                gaps.add(
                    TaskTimelineGap(
                        severity = "ERROR",
                        message = "slice.finishedAt < slice.startedAt",
                        relatedIds = mapOf("sliceId" to slice.sliceId.value)
                    )
                )
            }
            if (slice.status in setOf(SliceStatus.COMPLETED, SliceStatus.FAILED, SliceStatus.SUSPENDED) && finishedAt == null) {
                gaps.add(
                    TaskTimelineGap(
                        severity = "WARN",
                        message = "slice is terminal but finishedAtEpochMs is null",
                        relatedIds = mapOf("sliceId" to slice.sliceId.value, "status" to slice.status.name)
                    )
                )
            }
        }

        val checkpointSliceIds = checkpoints.map { it.sliceId.value }.toSet()
        val costSliceIds = costRecords.map { it.sliceId }.toSet()
        val sliceIds = slices.map { it.sliceId.value }.toSet()

        (checkpointSliceIds - sliceIds).forEach { sliceId ->
            gaps.add(
                TaskTimelineGap(
                    severity = "WARN",
                    message = "checkpoint exists but slice not found in task state",
                    relatedIds = mapOf("sliceId" to sliceId)
                )
            )
        }
        (costSliceIds - sliceIds).forEach { sliceId ->
            gaps.add(
                TaskTimelineGap(
                    severity = "WARN",
                    message = "cost record exists but slice not found in task state",
                    relatedIds = mapOf("sliceId" to sliceId)
                )
            )
        }

        if (task.status == TaskStatus.FAILED) {
            val reasonCode = task.latestResult?.extension?.get("reasonCode")
            if (reasonCode.isNullOrBlank()) {
                gaps.add(
                    TaskTimelineGap(
                        severity = "WARN",
                        message = "task failed but latestResult.extension['reasonCode'] is missing"
                    )
                )
            }
        }

        return gaps
    }

    private fun formatInstant(epochMs: Long): String =
        runCatching { Instant.ofEpochMilli(epochMs).toString() }.getOrDefault(epochMs.toString())
}
