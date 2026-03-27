/*
 * 任务事件查询端口接口
 *
 * Task Event Query Port Interface
 *
 * 该接口定义了任务事件查询的核心抽象，支持按任务 ID 和时间范围检索事件。
 * This interface defines the core abstraction for task event querying,
 * supporting event retrieval by task ID and time range.
 *
 * 任务事件查询端口用于任务时间线回放和调试分析。
 * The task event query port is used for task timeline replay and debugging analysis.
 *
 * 使用场景：
 * Use cases:
 * - 任务执行历史追踪 / Task execution history tracking
 * - 时间线回放分析 / Timeline replay analysis
 * - 问题诊断和调试 / Issue diagnosis and debugging
 */
package fuookami.ospf.framework.remote_solver.port

import fuookami.ospf.framework.remote_solver.domain.EventRecord

/**
 * 任务事件查询端口接口
 *
 * Task Event Query Port Interface
 *
 * 提供任务事件查询功能的端口接口。
 * Port interface providing task event query capabilities.
 */
interface TaskEventQueryPort {
    /**
     * 查询任务事件
     *
     * Queries task events.
     *
     * 根据任务 ID 和时间范围查询相关事件记录。
     * Queries event records by task ID and optional time range.
     *
     * @param taskId 任务唯一标识符 / Unique task identifier
     * @param fromEpochMs 起始时间的毫秒级时间戳（可选）/ Start time in milliseconds (optional)
     * @param toEpochMs 结束时间的毫秒级时间戳（可选）/ End time in milliseconds (optional)
     * @param limit 最大返回数量（默认：500）/ Maximum number to return (default: 500)
     * @return 事件记录列表，按时间排序 / List of event records, sorted by time
     */
    suspend fun queryTaskEvents(
        taskId: String,
        fromEpochMs: Long? = null,
        toEpochMs: Long? = null,
        limit: Int = 500
    ): List<EventRecord>
}