/*
 * 分布式锁端口接口
 *
 * Distributed Lock Port Interface
 *
 * 该接口定义了分布式锁的核心抽象，支持锁的获取、续约和释放。
 * This interface defines the core abstraction for distributed locking,
 * supporting lock acquisition, renewal, and release.
 *
 * 分布式锁用于在分布式环境中实现互斥访问，确保资源操作的原子性。
 * Distributed locks are used to implement mutual exclusion in distributed environments,
 * ensuring atomicity of resource operations.
 *
 * 典型使用场景：
 * Typical use cases:
 * - 防止并发任务重复执行 / Prevent concurrent task duplicate execution
 * - 领导者选举 / Leader election
 * - 资源互斥访问 / Mutual exclusion for resource access
 */
package fuookami.ospf.framework.remote_solver.port

import fuookami.ospf.framework.remote_solver.domain.LockLease

/**
 * 分布式锁端口接口
 *
 * Distributed Lock Port Interface
 *
 * 提供分布式锁功能的端口接口。
 * Port interface providing distributed locking capabilities.
 */
interface DistributedLockPort {
    /**
     * 获取分布式锁
     *
     * Acquires a distributed lock.
     *
     * 尝试获取指定键的分布式锁。如果锁已被其他持有者持有，则返回 null。
     * Attempts to acquire a distributed lock for the specified key.
     * Returns null if the lock is already held by another owner.
     *
     * @param key 锁的唯一标识键 / Unique lock key identifier
     * @param owner 锁持有者标识符 / Lock owner identifier
     * @param ttlMs 锁的生存时间（毫秒）/ Lock time-to-live in milliseconds
     * @return 成功获取时返回锁租约，失败返回 null
     *         Lock lease if acquired successfully, null otherwise
     */
    suspend fun acquire(key: String, owner: String, ttlMs: Long): LockLease?

    /**
     * 续约分布式锁
     *
     * Renews a distributed lock.
     *
     * 延长已持有锁的生存时间。只有锁的当前持有者才能续约。
     * Extends the time-to-live of a held lock.
     * Only the current lock owner can renew the lock.
     *
     * @param lease 要续约的锁租约 / Lock lease to renew
     * @param ttlMs 新的生存时间（毫秒）/ New time-to-live in milliseconds
     * @return 续约成功返回新的锁租约，失败返回 null
     *         New lock lease if renewed successfully, null otherwise
     */
    suspend fun renew(lease: LockLease, ttlMs: Long): LockLease?

    /**
     * 释放分布式锁
     *
     * Releases a distributed lock.
     *
     * 释放持有的锁，允许其他持有者获取。只有锁的当前持有者才能释放。
     * Releases a held lock, allowing other owners to acquire it.
     * Only the current lock owner can release the lock.
     *
     * @param lease 要释放的锁租约 / Lock lease to release
     * @return 释放成功返回 true，失败返回 false
     *         true if released successfully, false otherwise
     */
    suspend fun release(lease: LockLease): Boolean
}