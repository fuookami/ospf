/**
 * 分布式锁端口模块
 *
 * 本模块提供基于 Ktorm 的分布式锁功能，
 * 用于远程求解器调度器的并发控制和资源互斥访问。
 *
 * Distributed Lock Port Module
 *
 * This module provides distributed lock functionality based on Ktorm,
 * used for concurrency control and mutual exclusion access to resources
 * in the remote solver dispatcher.
 */
package fuookami.ospf.framework.remote_solver.adapter.ktorm

import fuookami.ospf.framework.remote_solver.domain.LockLease
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import fuookami.ospf.framework.remote_solver.port.DistributedLockPort
import fuookami.ospf.framework.remote_solver.protocol.port.IdGeneratorPort
import org.ktorm.database.Database
import org.ktorm.dsl.and
import org.ktorm.dsl.delete
import org.ktorm.dsl.eq
import org.ktorm.dsl.greater
import org.ktorm.dsl.insert
import org.ktorm.dsl.lessEq
import org.ktorm.dsl.update
import org.ktorm.dsl.where
import org.ktorm.schema.long
import org.ktorm.schema.varchar
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Ktorm 分布式锁端口
 *
 * 基于 Ktorm 实现的分布式锁端口，提供锁获取、续租和释放功能。
 * 支持基于 TTL 的自动过期机制，确保锁的可靠性。
 *
 * 表结构：remote_solver_lock
 *
 * Ktorm Distributed Lock Port
 *
 * Distributed lock port implemented with Ktorm, providing lock acquisition,
 * renewal, and release functionality. Supports TTL-based automatic expiration
 * mechanism to ensure lock reliability.
 *
 * Table schema: remote_solver_lock
 *
 * @param clock 时钟端口，用于获取当前时间
 * @param idGenerator ID 生成器端口，用于生成租约 ID
 * @param jdbcUrl JDBC 连接 URL
 * @param username 数据库用户名，可选
 * @param password 数据库密码，可选
 * @param tableName 表名称，默认为 remote_solver_lock
 */
class KtormDistributedLockPort(
    private val clock: ClockPort,
    private val idGenerator: IdGeneratorPort,
    private val jdbcUrl: String,
    private val username: String? = null,
    private val password: String? = null,
    tableName: String = "remote_solver_lock"
) : DistributedLockPort {
    private val resolvedTableName = normalizeTableName(tableName)
    private val table = LockTable(resolvedTableName)
    private val initialized = AtomicBoolean(false)
    private val database = Database.connect(
        url = jdbcUrl,
        user = username,
        password = password
    )

    /**
     * 获取锁
     *
     * 尝试获取指定键的分布式锁。如果锁不存在或已过期，则成功获取；
     * 否则获取失败。支持 TTL 自动过期机制。
     *
     * @param key 锁键名
     * @param owner 锁持有者标识
     * @param ttlMs 锁的有效期（毫秒）
     * @return 锁租约，如果获取失败则返回 null
     *
     * Acquires lock
     *
     * Attempts to acquire a distributed lock for the specified key.
     * If the lock does not exist or has expired, acquisition succeeds;
     * otherwise it fails. Supports TTL-based automatic expiration.
     *
     * @param key Lock key name
     * @param owner Lock holder identifier
     * @param ttlMs Lock validity period (milliseconds)
     * @return Lock lease, or null if acquisition fails
     */
    override suspend fun acquire(key: String, owner: String, ttlMs: Long): LockLease? {
        if (ttlMs <= 0L || key.isBlank() || owner.isBlank()) {
            return null
        }
        ensureSchema()
        val now = clock.nowEpochMs()
        val lease = LockLease(
            key = key,
            leaseId = idGenerator.newId("lease"),
            owner = owner,
            expiresAtEpochMs = now + ttlMs
        )

        val inserted = runCatching {
            database.insert(table) {
                set(it.lockKey, lease.key)
                set(it.leaseId, lease.leaseId)
                set(it.ownerId, lease.owner)
                set(it.expiresAtEpochMs, lease.expiresAtEpochMs)
            }
        }.getOrDefault(0)
        if (inserted > 0) {
            return lease
        }

        val updated = database.update(table) {
            set(it.leaseId, lease.leaseId)
            set(it.ownerId, lease.owner)
            set(it.expiresAtEpochMs, lease.expiresAtEpochMs)
            where {
                (it.lockKey eq lease.key) and (it.expiresAtEpochMs lessEq now)
            }
        }
        return if (updated > 0) lease else null
    }

    /**
     * 续租锁
     *
     * 为已持有的锁延长有效期。只有当前持有者才能续租，
     * 且锁必须未过期。
     *
     * @param lease 当前锁租约
     * @param ttlMs 新的有效期（毫秒）
     * @return 新的锁租约，如果续租失败则返回 null
     *
     * Renews lock
     *
     * Extends the validity period of a held lock. Only the current holder
     * can renew, and the lock must not have expired.
     *
     * @param lease Current lock lease
     * @param ttlMs New validity period (milliseconds)
     * @return New lock lease, or null if renewal fails
     */
    override suspend fun renew(lease: LockLease, ttlMs: Long): LockLease? {
        if (ttlMs <= 0L) {
            return null
        }
        ensureSchema()
        val now = clock.nowEpochMs()
        val renewed = lease.copy(expiresAtEpochMs = now + ttlMs)
        val updated = database.update(table) {
            set(it.expiresAtEpochMs, renewed.expiresAtEpochMs)
            where {
                (it.lockKey eq lease.key) and
                    (it.leaseId eq lease.leaseId) and
                    (it.ownerId eq lease.owner) and
                    (it.expiresAtEpochMs greater now)
            }
        }
        return if (updated > 0) renewed else null
    }

    /**
     * 释放锁
     *
     * 释放已持有的分布式锁。只有当前持有者才能释放锁。
     *
     * @param lease 当前锁租约
     * @return 是否成功释放
     *
     * Releases lock
     *
     * Releases a held distributed lock. Only the current holder can release.
     *
     * @param lease Current lock lease
     * @return Whether release succeeded
     */
    override suspend fun release(lease: LockLease): Boolean {
        ensureSchema()
        val deleted = database.delete(table) {
            (it.lockKey eq lease.key) and
                (it.leaseId eq lease.leaseId) and
                (it.ownerId eq lease.owner)
        }
        return deleted > 0
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
                        CREATE TABLE IF NOT EXISTS $resolvedTableName (
                            lock_key VARCHAR(256) PRIMARY KEY,
                            lease_id VARCHAR(256) NOT NULL,
                            owner_id VARCHAR(256) NOT NULL,
                            expires_at_epoch_ms BIGINT NOT NULL
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
     * 锁表定义
     *
     * 定义存储分布式锁信息的数据库表结构。
     *
     * Lock Table Definition
     *
     * Defines the database table structure for storing distributed lock information.
     *
     * @param tableName 表名称
     */
    private class LockTable(tableName: String) : org.ktorm.schema.Table<Nothing>(tableName) {
        val lockKey = varchar("lock_key").primaryKey()
        val leaseId = varchar("lease_id")
        val ownerId = varchar("owner_id")
        val expiresAtEpochMs = long("expires_at_epoch_ms")
    }
}