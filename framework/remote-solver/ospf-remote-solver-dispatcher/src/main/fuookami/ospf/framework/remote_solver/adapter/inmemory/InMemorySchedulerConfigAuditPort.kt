/**
 * 内存调度器配置审计端口适配器
 *
 * 提供基于内存的调度器热加载配置审计实现，用于测试和非持久化场景。
 * 支持审计记录追加、列表查询和配置快照保存。
 *
 * In-memory scheduler config audit port adapter.
 *
 * Provides memory-based scheduler hot-reload config audit implementation for testing and non-persistent scenarios.
 * Supports audit record appending, list querying, and config snapshot saving.
 */
package fuookami.ospf.framework.remote_solver.adapter.inmemory

import fuookami.ospf.framework.remote_solver.application.SchedulerHotReloadAuditRecord
import fuookami.ospf.framework.remote_solver.application.SchedulerRuntimeConfig
import fuookami.ospf.framework.remote_solver.port.SchedulerConfigAuditPort
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CopyOnWriteArrayList

/**
 * 内存调度器配置审计端口实现
 *
 * 使用内存存储审计记录和配置快照，支持并发访问。
 *
 * In-memory scheduler config audit port implementation.
 *
 * Uses in-memory storage for audit records and config snapshots, supporting concurrent access.
 */
class InMemorySchedulerConfigAuditPort : SchedulerConfigAuditPort {
    /**
     * 审计记录列表
     *
     * 存储所有调度器热加载审计记录。
     *
     * Audit record list.
     *
     * Stores all scheduler hot-reload audit records.
     */
    private val audits = CopyOnWriteArrayList<SchedulerHotReloadAuditRecord>()

    /**
     * 配置快照映射
     *
     * 键为版本号，值为对应的运行时配置。
     *
     * Config snapshot map.
     *
     * Keyed by version, valued by corresponding runtime config.
     */
    private val snapshots = ConcurrentHashMap<String, SchedulerRuntimeConfig>()

    /**
     * 追加审计记录
     *
     * 将新的热加载审计记录添加到审计日志中。
     *
     * Appends audit record.
     *
     * Adds a new hot-reload audit record to the audit log.
     *
     * @param record 审计记录
     *               Audit record
     */
    override suspend fun append(record: SchedulerHotReloadAuditRecord) {
        audits.add(record)
    }

    /**
     * 获取审计记录列表
     *
     * 返回最近指定数量的审计记录，按时间顺序排列。
     *
     * Gets audit record list.
     *
     * Returns the most recent audit records up to the specified limit, sorted chronologically.
     *
     * @param limit 最大返回数量，至少为1
     *              Maximum return count, at least 1
     * @return 审计记录列表
     *         Audit record list
     */
    override suspend fun list(limit: Int): List<SchedulerHotReloadAuditRecord> {
        val safeLimit = limit.coerceAtLeast(1)
        val all = audits.toList()
        if (all.size <= safeLimit) {
            return all
        }
        return all.takeLast(safeLimit)
    }

    /**
     * 保存配置快照
     *
     * 将指定版本的运行时配置保存到快照存储中。
     *
     * Saves config snapshot.
     *
     * Saves the runtime config of the specified version to snapshot storage.
     *
     * @param version 版本号
     *                Version number
     * @param config 运行时配置
     *               Runtime config
     */
    override suspend fun saveSnapshot(version: String, config: SchedulerRuntimeConfig) {
        snapshots[version] = config
    }

    /**
     * 获取配置快照
     *
     * 根据版本号获取对应的运行时配置快照。
     *
     * Gets config snapshot.
     *
     * Retrieves the runtime config snapshot corresponding to the version number.
     *
     * @param version 版本号
     *                Version number
     * @return 运行时配置快照，如果不存在则返回null
     *         Runtime config snapshot, returns null if not found
     */
    override suspend fun getSnapshot(version: String): SchedulerRuntimeConfig? =
        snapshots[version]
}