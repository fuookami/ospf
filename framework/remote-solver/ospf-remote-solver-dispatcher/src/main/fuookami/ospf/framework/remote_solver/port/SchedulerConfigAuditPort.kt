/*
 * 调度器配置审计端口接口
 *
 * Scheduler Configuration Audit Port Interface
 *
 * 该接口定义了调度器配置审计的核心抽象，支持配置变更记录和快照管理。
 * This interface defines the core abstraction for scheduler configuration auditing,
 * supporting configuration change logging and snapshot management.
 *
 * 调度器配置审计端口用于追踪调度器配置的历史变更，支持审计和回滚。
 * The scheduler config audit port is used to track historical changes to scheduler
 * configuration, supporting auditing and rollback.
 *
 * 功能特性：
 * Features:
 * - 配置变更审计日志 / Configuration change audit log
 * - 配置快照存储和检索 / Configuration snapshot storage and retrieval
 * - 支持热重载变更追踪 / Support for hot reload change tracking
 */
package fuookami.ospf.framework.remote_solver.port

import fuookami.ospf.framework.remote_solver.application.SchedulerHotReloadAuditRecord
import fuookami.ospf.framework.remote_solver.application.SchedulerRuntimeConfig

/**
 * 调度器配置审计端口接口
 *
 * Scheduler Configuration Audit Port Interface
 *
 * 提供调度器配置审计功能的端口接口。
 * Port interface providing scheduler configuration audit capabilities.
 */
interface SchedulerConfigAuditPort {
    /**
     * 追加审计记录
     *
     * Appends an audit record.
     *
     * 将一条配置变更审计记录追加到审计日志中。
     * Appends a configuration change audit record to the audit log.
     *
     * @param record 审计记录 / Audit record to append
     */
    suspend fun append(record: SchedulerHotReloadAuditRecord)

    /**
     * 列出审计记录
     *
     * Lists audit records.
     *
     * 获取最近的审计记录列表，按时间倒序排列。
     * Retrieves a list of recent audit records, sorted by time in descending order.
     *
     * @param limit 最大返回数量（默认：100）/ Maximum number to return (default: 100)
     * @return 审计记录列表 / List of audit records
     */
    suspend fun list(limit: Int = 100): List<SchedulerHotReloadAuditRecord>

    /**
     * 保存配置快照
     *
     * Saves a configuration snapshot.
     *
     * 保存指定版本的调度器运行时配置快照。
     * Saves a snapshot of the scheduler runtime configuration for the specified version.
     *
     * @param version 版本标识符 / Version identifier
     * @param config 调度器运行时配置 / Scheduler runtime configuration
     */
    suspend fun saveSnapshot(version: String, config: SchedulerRuntimeConfig)

    /**
     * 获取配置快照
     *
     * Gets a configuration snapshot.
     *
     * 获取指定版本的调度器运行时配置快照。
     * Retrieves a snapshot of the scheduler runtime configuration for the specified version.
     *
     * @param version 版本标识符 / Version identifier
     * @return 配置快照，如不存在返回 null
     *         Configuration snapshot, or null if not found
     */
    suspend fun getSnapshot(version: String): SchedulerRuntimeConfig?
}