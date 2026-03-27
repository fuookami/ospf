/**
 * 内存分布式锁端口适配器
 *
 * 提供基于内存的分布式锁实现，用于测试和非持久化场景。
 * 支持锁获取、续租和释放，基于TTL实现锁过期机制。
 *
 * In-memory distributed lock port adapter.
 *
 * Provides memory-based distributed lock implementation for testing and non-persistent scenarios.
 * Supports lock acquisition, renewal, and release, with TTL-based lock expiration mechanism.
 */
package fuookami.ospf.framework.remote_solver.adapter.inmemory

import fuookami.ospf.framework.remote_solver.domain.LockLease
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import fuookami.ospf.framework.remote_solver.port.DistributedLockPort
import fuookami.ospf.framework.remote_solver.protocol.port.IdGeneratorPort
import java.util.concurrent.ConcurrentHashMap

/**
 * 内存分布式锁端口实现
 *
 * 使用内存存储锁租约，支持TTL过期和租约验证。
 *
 * In-memory distributed lock port implementation.
 *
 * Uses in-memory storage for lock leases, supporting TTL expiration and lease verification.
 *
 * @param clock 时钟端口，用于获取当前时间戳
 *              Clock port for obtaining current timestamps
 * @param idGenerator ID生成器端口，用于生成租约ID
 *                    ID generator port for generating lease IDs
 */
class InMemoryDistributedLockPort(
    private val clock: ClockPort,
    private val idGenerator: IdGeneratorPort
) : DistributedLockPort {
    /**
     * 锁租约存储映射
     *
     * 键为锁键名，值为租约信息。
     *
     * Lock lease storage map.
     *
     * Keyed by lock key, valued by lease information.
     */
    private val leases = ConcurrentHashMap<String, LockLease>()

    /**
     * 获取锁
     *
     * 尝试获取指定键的分布式锁。如果锁已被其他持有者占用且未过期，则返回null。
     *
     * Acquires lock.
     *
     * Attempts to acquire a distributed lock for the specified key. If the lock is held by another owner
     * and hasn't expired, returns null.
     *
     * @param key 锁键名
     *            Lock key name
     * @param owner 持有者标识
     *              Owner identifier
     * @param ttlMs 锁生存时间（毫秒）
     *              Lock TTL in milliseconds
     * @return 锁租约，如果获取失败则返回null
     *         Lock lease, returns null if acquisition fails
     */
    override suspend fun acquire(key: String, owner: String, ttlMs: Long): LockLease? {
        if (ttlMs <= 0L) {
            return null
        }
        synchronized(leases) {
            val now = clock.nowEpochMs()
            val existing = leases[key]
            if (existing != null && existing.expiresAtEpochMs > now) {
                return null
            }
            val lease = LockLease(
                key = key,
                leaseId = idGenerator.newId("lease"),
                owner = owner,
                expiresAtEpochMs = now + ttlMs
            )
            leases[key] = lease
            return lease
        }
    }

    /**
     * 续租锁
     *
     * 延长锁租约的有效期。只有当前持有者且租约未过期时才能续租。
     *
     * Renews lock lease.
     *
     * Extends the validity period of a lock lease. Only the current owner with an unexpired lease can renew.
     *
     * @param lease 当前租约
     *              Current lease
     * @param ttlMs 新的生存时间（毫秒）
     *              New TTL in milliseconds
     * @return 续租后的新租约，如果续租失败则返回null
     *         New lease after renewal, returns null if renewal fails
     */
    override suspend fun renew(lease: LockLease, ttlMs: Long): LockLease? {
        if (ttlMs <= 0L) {
            return null
        }
        synchronized(leases) {
            val current = leases[lease.key] ?: return null
            if (current.leaseId != lease.leaseId || current.owner != lease.owner) {
                return null
            }
            if (current.expiresAtEpochMs <= clock.nowEpochMs()) {
                leases.remove(lease.key)
                return null
            }
            val renewed = current.copy(expiresAtEpochMs = clock.nowEpochMs() + ttlMs)
            leases[lease.key] = renewed
            return renewed
        }
    }

    /**
     * 释放锁
     *
     * 释放持有的锁租约。只有当前持有者才能释放锁。
     *
     * Releases lock.
     *
     * Releases the held lock lease. Only the current owner can release the lock.
     *
     * @param lease 要释放的租约
     *              Lease to release
     * @return 是否成功释放
     *         Whether release was successful
     */
    override suspend fun release(lease: LockLease): Boolean {
        synchronized(leases) {
            val current = leases[lease.key] ?: return false
            if (current.leaseId != lease.leaseId || current.owner != lease.owner) {
                return false
            }
            leases.remove(lease.key)
            return true
        }
    }
}