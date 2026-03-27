/*
 * 任务时间线回放模型
 *
 * 本模块定义任务时间线回放相关的数据模型，
 * 包括事件、间隙和报告等。
 *
 * Task Timeline Replay Models
 *
 * This module defines data models related to task timeline replay,
 * including events, gaps, and reports.
 */
package fuookami.ospf.framework.remote_solver.application

/**
 * 任务时间线事件
 *
 * 表示任务生命周期中的一个事件。
 *
 * Task timeline event.
 *
 * Represents an event in the task lifecycle.
 *
 * @param atEpochMs 事件发生时间戳（毫秒）
 *                  Event timestamp in milliseconds
 * @param type 事件类型，如 "TASK_CREATED"、"SLICE_STARTED" 等
 *             Event type, such as "TASK_CREATED", "SLICE_STARTED", etc.
 * @param summary 事件摘要描述
 *                Event summary description
 * @param attributes 事件属性键值对
 *                   Event attribute key-value pairs
 */
data class TaskTimelineEvent(
    val atEpochMs: Long,
    val type: String,
    val summary: String,
    val attributes: Map<String, String> = emptyMap()
)

/**
 * 任务时间线间隙
 *
 * 表示时间线中检测到的异常或不一致。
 *
 * Task timeline gap.
 *
 * Represents an anomaly or inconsistency detected in the timeline.
 *
 * @param severity 严重程度，如 "WARN"、"ERROR"
 *                  Severity level, such as "WARN", "ERROR"
 * @param message 间隙描述信息
 *                Gap description message
 * @param relatedIds 相关实体标识键值对
 *                   Related entity identifier key-value pairs
 */
data class TaskTimelineGap(
    val severity: String,
    val message: String,
    val relatedIds: Map<String, String> = emptyMap()
)

/**
 * 任务时间线报告
 *
 * 包含任务完整时间线信息的报告。
 *
 * Task timeline report.
 *
 * Report containing complete timeline information of a task.
 *
 * @param taskId 任务标识
 *               Task identifier
 * @param generatedAtEpochMs 报告生成时间戳（毫秒）
 *                           Report generation timestamp in milliseconds
 * @param status 任务当前状态
 *               Task current status
 * @param events 时间线事件列表
 *               Timeline event list
 * @param gaps 检测到的间隙列表
 *             Detected gap list
 */
data class TaskTimelineReport(
    val taskId: String,
    val generatedAtEpochMs: Long,
    val status: String,
    val events: List<TaskTimelineEvent>,
    val gaps: List<TaskTimelineGap>
)
