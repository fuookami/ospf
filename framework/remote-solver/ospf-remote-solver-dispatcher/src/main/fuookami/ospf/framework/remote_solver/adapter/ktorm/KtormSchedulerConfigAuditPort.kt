/**
 * 调度器配置审计端口模块
 *
 * 本模块提供基于 Ktorm 的调度器热重载配置审计功能，
 * 用于记录和追踪调度器配置的变更历史，支持版本回滚。
 *
 * Scheduler Configuration Audit Port Module
 *
 * This module provides scheduler hot-reload configuration audit functionality
 * based on Ktorm, used to record and track scheduler configuration change history,
 * supporting version rollback.
 */
package fuookami.ospf.framework.remote_solver.adapter.ktorm

import fuookami.ospf.framework.remote_solver.application.SchedulerHotReloadAuditRecord
import fuookami.ospf.framework.remote_solver.application.SchedulerRuntimeConfig
import fuookami.ospf.framework.remote_solver.port.SchedulerConfigAuditPort
import org.ktorm.database.Database
import org.ktorm.dsl.eq
import org.ktorm.dsl.from
import org.ktorm.dsl.insert
import org.ktorm.dsl.map
import org.ktorm.dsl.select
import org.ktorm.dsl.where
import org.ktorm.schema.long
import org.ktorm.schema.text
import org.ktorm.schema.varchar
import java.net.URLDecoder
import java.net.URLEncoder
import java.nio.charset.StandardCharsets
import java.util.Properties
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Ktorm 调度器配置审计端口
 *
 * 基于 Ktorm 实现的调度器配置审计端口，提供配置变更记录、
 * 配置快照保存和恢复功能。
 *
 * 表结构：
 * - remote_solver_scheduler_audit: 存储配置变更审计记录
 * - remote_solver_scheduler_snapshot: 存储配置快照
 *
 * Ktorm Scheduler Configuration Audit Port
 *
 * Scheduler configuration audit port implemented with Ktorm,
 * providing configuration change recording, snapshot saving
 * and restoration functionality.
 *
 * Table schemas:
 * - remote_solver_scheduler_audit: stores configuration change audit records
 * - remote_solver_scheduler_snapshot: stores configuration snapshots
 *
 * @param jdbcUrl JDBC 连接 URL
 * @param username 数据库用户名，可选
 * @param password 数据库密码，可选
 * @param auditTableName 审计表名称，默认为 remote_solver_scheduler_audit
 * @param snapshotTableName 快照表名称，默认为 remote_solver_scheduler_snapshot
 */
class KtormSchedulerConfigAuditPort(
    private val jdbcUrl: String,
    private val username: String? = null,
    private val password: String? = null,
    auditTableName: String = "remote_solver_scheduler_audit",
    snapshotTableName: String = "remote_solver_scheduler_snapshot"
) : SchedulerConfigAuditPort {
    private val resolvedAuditTableName = normalizeTableName(auditTableName)
    private val resolvedSnapshotTableName = normalizeTableName(snapshotTableName)
    private val auditTable = SchedulerAuditTable(resolvedAuditTableName)
    private val snapshotTable = SchedulerSnapshotTable(resolvedSnapshotTableName)
    private val initialized = AtomicBoolean(false)
    private val database = Database.connect(
        url = jdbcUrl,
        user = username,
        password = password
    )

    /**
     * 添加审计记录
     *
     * 将配置变更审计记录追加到数据库中。
     *
     * @param record 配置变更审计记录
     *
     * Appends an audit record
     *
     * Appends a configuration change audit record to the database.
     *
     * @param record Configuration change audit record
     */
    override suspend fun append(record: SchedulerHotReloadAuditRecord) {
        ensureSchema()
        database.insert(auditTable) {
            set(it.version, record.version)
            set(it.previousVersion, record.previousVersion)
            set(it.operator, record.operator)
            set(it.effectiveAtEpochMs, record.effectiveAtEpochMs)
            set(it.rollbackFromVersion, record.rollbackFromVersion)
            set(it.changeSetEncoded, encodeMap(record.changeSet))
        }
    }

    /**
     * 查询审计记录列表
     *
     * 获取最近指定数量的审计记录，按生效时间排序。
     *
     * @param limit 返回记录的最大数量
     * @return 审计记录列表
     *
     * Lists audit records
     *
     * Retrieves the most recent audit records up to the specified limit,
     * sorted by effective time.
     *
     * @param limit Maximum number of records to return
     * @return List of audit records
     */
    override suspend fun list(limit: Int): List<SchedulerHotReloadAuditRecord> {
        ensureSchema()
        val safeLimit = limit.coerceAtLeast(1)
        val all = database
            .from(auditTable)
            .select(
                auditTable.version,
                auditTable.previousVersion,
                auditTable.operator,
                auditTable.effectiveAtEpochMs,
                auditTable.rollbackFromVersion,
                auditTable.changeSetEncoded
            )
            .map { row ->
                SchedulerHotReloadAuditRecord(
                    version = row[auditTable.version].orEmpty(),
                    previousVersion = row[auditTable.previousVersion].orEmpty(),
                    operator = row[auditTable.operator].orEmpty(),
                    effectiveAtEpochMs = row[auditTable.effectiveAtEpochMs] ?: 0L,
                    rollbackFromVersion = row[auditTable.rollbackFromVersion],
                    changeSet = decodeMap(row[auditTable.changeSetEncoded].orEmpty())
                )
            }
            .sortedBy { it.effectiveAtEpochMs }
        return if (all.size <= safeLimit) all else all.takeLast(safeLimit)
    }

    /**
     * 保存配置快照
     *
     * 将指定版本的运行时配置保存为快照。
     *
     * @param version 配置版本号
     * @param config 运行时配置
     *
     * Saves a configuration snapshot
     *
     * Saves the runtime configuration as a snapshot for the specified version.
     *
     * @param version Configuration version number
     * @param config Runtime configuration
     */
    override suspend fun saveSnapshot(version: String, config: SchedulerRuntimeConfig) {
        ensureSchema()
        database.insert(snapshotTable) {
            set(it.version, version)
            set(it.snapshotProperties, encodeSnapshot(config))
        }
    }

    /**
     * 获取配置快照
     *
     * 获取指定版本的配置快照，用于配置恢复。
     *
     * @param version 配置版本号
     * @return 运行时配置，如果不存在则返回 null
     *
     * Gets a configuration snapshot
     *
     * Retrieves the configuration snapshot for the specified version,
     * used for configuration restoration.
     *
     * @param version Configuration version number
     * @return Runtime configuration, or null if not found
     */
    override suspend fun getSnapshot(version: String): SchedulerRuntimeConfig? {
        ensureSchema()
        val encoded = database
            .from(snapshotTable)
            .select(snapshotTable.snapshotProperties)
            .where { snapshotTable.version eq version }
            .map { row -> row[snapshotTable.snapshotProperties] }
            .lastOrNull()
            ?: return null
        return decodeSnapshot(encoded)
    }

    /**
     * 确保数据库表结构存在
     *
     * 检查并创建必要的数据库表结构，使用双重检查锁定确保线程安全。
     *
     * Ensures database schema exists
     *
     * Checks and creates necessary database table structures,
     * using double-checked locking for thread safety.
     */
    private fun ensureSchema() {
        if (initialized.get()) {
            return
        }
        synchronized(initialized) {
            if (initialized.get()) {
                return
            }
            database.useConnection { connection ->
                connection.createStatement().use { statement ->
                    statement.execute(
                        """
                        CREATE TABLE IF NOT EXISTS $resolvedAuditTableName (
                            version VARCHAR(256) NOT NULL,
                            previous_version VARCHAR(256) NOT NULL,
                            operator VARCHAR(256) NOT NULL,
                            effective_at_epoch_ms BIGINT NOT NULL,
                            rollback_from_version VARCHAR(256),
                            change_set_encoded TEXT NOT NULL
                        )
                        """.trimIndent()
                    )
                    statement.execute(
                        """
                        CREATE TABLE IF NOT EXISTS $resolvedSnapshotTableName (
                            version VARCHAR(256) NOT NULL,
                            snapshot_properties TEXT NOT NULL
                        )
                        """.trimIndent()
                    )
                }
            }
            initialized.set(true)
        }
    }

    /**
     * 规范化表名称
     *
     * 验证并规范化表名称，确保符合数据库命名规范。
     *
     * @param raw 原始表名称
     * @return 规范化后的表名称
     * @throws IllegalArgumentException 如果表名称无效
     *
     * Normalizes table name
     *
     * Validates and normalizes the table name, ensuring it conforms
     * to database naming conventions.
     *
     * @param raw Raw table name
     * @return Normalized table name
     * @throws IllegalArgumentException If table name is invalid
     */
    private fun normalizeTableName(raw: String): String {
        val value = raw.trim()
        require(value.isNotEmpty()) { "tableName must not be blank" }
        require(value.matches(Regex("[A-Za-z0-9_]+"))) { "Invalid tableName: '$raw'" }
        return value
    }

    /**
     * 编码映射为字符串
     *
     * 将键值对映射编码为 URL 编码的字符串格式。
     *
     * @param input 要编码的映射
     * @return 编码后的字符串
     *
     * Encodes map to string
     *
     * Encodes a key-value map to a URL-encoded string format.
     *
     * @param input Map to encode
     * @return Encoded string
     */
    private fun encodeMap(input: Map<String, String>): String =
        input.entries.joinToString("&") { entry ->
            "${urlEncode(entry.key)}=${urlEncode(entry.value)}"
        }

    /**
     * 解码字符串为映射
     *
     * 将 URL 编码的字符串解码为键值对映射。
     *
     * @param encoded 编码的字符串
     * @return 解码后的映射
     *
     * Decodes string to map
     *
     * Decodes a URL-encoded string to a key-value map.
     *
     * @param encoded Encoded string
     * @return Decoded map
     */
    private fun decodeMap(encoded: String): Map<String, String> {
        if (encoded.isBlank()) {
            return emptyMap()
        }
        return encoded.split("&")
            .mapNotNull { part ->
                val idx = part.indexOf('=')
                if (idx <= 0) {
                    return@mapNotNull null
                }
                val key = urlDecode(part.substring(0, idx))
                val value = urlDecode(part.substring(idx + 1))
                key to value
            }
            .toMap()
    }

    /**
     * 编码运行时配置快照
     *
     * 将调度器运行时配置编码为可存储的字符串格式。
     *
     * @param config 运行时配置
     * @return 编码后的字符串
     *
     * Encodes runtime configuration snapshot
     *
     * Encodes the scheduler runtime configuration to a storable string format.
     *
     * @param config Runtime configuration
     * @return Encoded string
     */
    private fun encodeSnapshot(config: SchedulerRuntimeConfig): String {
        val props = Properties().apply {
            setProperty("simpleTaskQuantumMs", config.simpleTaskQuantumMs.toString())
            setProperty("complexTaskQuantumMs", config.complexTaskQuantumMs.toString())
            setProperty("complexTaskQuantumMinMs", config.complexTaskQuantumMinMs.toString())
            setProperty("complexTaskQuantumMaxMs", config.complexTaskQuantumMaxMs.toString())
            setProperty("complexSolveEstimateMs", config.complexSolveEstimateMs.toString())
            setProperty("complexCheckpointEstimateMs", config.complexCheckpointEstimateMs.toString())
            setProperty("complexQuantumAlpha", config.complexQuantumAlpha.toString())
            setProperty("complexQuantumBeta", config.complexQuantumBeta.toString())
            setProperty("complexQuantumPricePenalty", config.complexQuantumPricePenalty.toString())
            setProperty("complexUrgencyWeight", config.complexUrgencyWeight.toString())
            setProperty("complexWaitingAgeWeight", config.complexWaitingAgeWeight.toString())
            setProperty("complexProgressNeedWeight", config.complexProgressNeedWeight.toString())
            setProperty("complexCostSensitivityWeight", config.complexCostSensitivityWeight.toString())
        }
        return props.entries.joinToString("&") { entry ->
            "${urlEncode(entry.key.toString())}=${urlEncode(entry.value.toString())}"
        }
    }

    /**
     * 解码运行时配置快照
     *
     * 将编码的字符串解码为调度器运行时配置。
     *
     * @param encoded 编码的字符串
     * @return 运行时配置，如果解码失败则返回 null
     *
     * Decodes runtime configuration snapshot
     *
     * Decodes an encoded string to scheduler runtime configuration.
     *
     * @param encoded Encoded string
     * @return Runtime configuration, or null if decoding fails
     */
    private fun decodeSnapshot(encoded: String): SchedulerRuntimeConfig? {
        val props = decodeMap(encoded)
        return SchedulerRuntimeConfig(
            simpleTaskQuantumMs = props["simpleTaskQuantumMs"]?.toLongOrNull() ?: return null,
            complexTaskQuantumMs = props["complexTaskQuantumMs"]?.toLongOrNull() ?: return null,
            complexTaskQuantumMinMs = props["complexTaskQuantumMinMs"]?.toLongOrNull() ?: return null,
            complexTaskQuantumMaxMs = props["complexTaskQuantumMaxMs"]?.toLongOrNull() ?: return null,
            complexSolveEstimateMs = props["complexSolveEstimateMs"]?.toLongOrNull() ?: return null,
            complexCheckpointEstimateMs = props["complexCheckpointEstimateMs"]?.toLongOrNull() ?: return null,
            complexQuantumAlpha = props["complexQuantumAlpha"]?.toDoubleOrNull() ?: return null,
            complexQuantumBeta = props["complexQuantumBeta"]?.toDoubleOrNull() ?: return null,
            complexQuantumPricePenalty = props["complexQuantumPricePenalty"]?.toDoubleOrNull() ?: return null,
            complexUrgencyWeight = props["complexUrgencyWeight"]?.toDoubleOrNull() ?: return null,
            complexWaitingAgeWeight = props["complexWaitingAgeWeight"]?.toDoubleOrNull() ?: return null,
            complexProgressNeedWeight = props["complexProgressNeedWeight"]?.toDoubleOrNull() ?: return null,
            complexCostSensitivityWeight = props["complexCostSensitivityWeight"]?.toDoubleOrNull() ?: return null
        )
    }

    /**
     * URL 编码
     *
     * 对字符串进行 URL 编码。
     *
     * @param raw 原始字符串
     * @return 编码后的字符串
     *
     * URL encoding
     *
     * Performs URL encoding on a string.
     *
     * @param raw Raw string
     * @return Encoded string
     */
    private fun urlEncode(raw: String): String =
        URLEncoder.encode(raw, StandardCharsets.UTF_8)

    /**
     * URL 解码
     *
     * 对 URL 编码的字符串进行解码。
     *
     * @param raw 编码的字符串
     * @return 解码后的字符串
     *
     * URL decoding
     *
     * Decodes a URL-encoded string.
     *
     * @param raw Encoded string
     * @return Decoded string
     */
    private fun urlDecode(raw: String): String =
        URLDecoder.decode(raw, StandardCharsets.UTF_8)

    /**
     * 调度器审计表定义
     *
     * 定义存储配置变更审计记录的数据库表结构。
     *
     * Scheduler Audit Table Definition
     *
     * Defines the database table structure for storing configuration
     * change audit records.
     *
     * @param tableName 表名称
     */
    private class SchedulerAuditTable(tableName: String) : org.ktorm.schema.Table<Nothing>(tableName) {
        val version = varchar("version")
        val previousVersion = varchar("previous_version")
        val operator = varchar("operator")
        val effectiveAtEpochMs = long("effective_at_epoch_ms")
        val rollbackFromVersion = varchar("rollback_from_version")
        val changeSetEncoded = text("change_set_encoded")
    }

    /**
     * 调度器快照表定义
     *
     * 定义存储配置快照的数据库表结构。
     *
     * Scheduler Snapshot Table Definition
     *
     * Defines the database table structure for storing configuration snapshots.
     *
     * @param tableName 表名称
     */
    private class SchedulerSnapshotTable(tableName: String) : org.ktorm.schema.Table<Nothing>(tableName) {
        val version = varchar("version")
        val snapshotProperties = text("snapshot_properties")
    }
}