/*
 * 调度决策缓存
 *
 * 本模块提供调度决策缓存层，
 * 用于缓存节点选择决策以提高调度性能。
 * 支持缓存命中/未命中统计、自动过期和手动清除。
 *
 * Scheduling Decision Cache
 *
 * This module provides scheduling decision caching layer,
 * used to cache node selection decisions to improve scheduling performance.
 * Supports cache hit/miss statistics, auto-expiration, and manual clearing.
 */
package fuookami.ospf.framework.remote_solver.application

import fuookami.ospf.framework.remote_solver.domain.NodeState
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskComplexity
import fuookami.ospf.framework.remote_solver.domain.TaskState
import fuookami.ospf.framework.remote_solver.port.MetricsPort
import kotlinx.coroutines.runBlocking
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong

/**
 * 调度决策缓存
 *
 * 用于缓存节点选择决策以提高性能。
 * 缓存任务特征到节点选择的映射，避免重复计算。
 *
 * Caching layer for scheduling decisions.
 *
 * This improves performance by:
 * - Caching node selection decisions for similar tasks
 * - Tracking cache hit/miss rates
 * - Auto-expiring stale cache entries
 *
 * 通过以下方式提高性能：
 * - 缓存相似任务的节点选择决策
 * - 跟踪缓存命中/未命中率
 * - 自动过期陈旧的缓存条目
 *
 * @param metricsPort 指标端口，用于记录缓存统计
 *                    Metrics port for recording cache statistics
 * @param config 缓存配置参数
 *               Cache configuration parameters
 */
class SchedulingDecisionCache(
    private val metricsPort: MetricsPort,
    private val config: CacheConfig = CacheConfig()
) {
    private val cache = ConcurrentHashMap<String, CachedDecision>()
    private val hits = AtomicLong(0)
    private val misses = AtomicLong(0)
    private val evictions = AtomicLong(0)

    /**
     * 获取缓存的节点选择决策
     *
     * 如果缓存可用且有效，则返回缓存的节点 ID。
     *
     * Gets a cached node selection decision if available and valid.
     *
     * @param task 待调度任务
     *             Task to schedule
     * @param availableNodes 当前可用节点列表
     *                       Current available nodes
     * @return 缓存的节点 ID，如果无效则返回 null
     *         Cached node ID if valid, null otherwise
     */
    fun getCachedNode(task: TaskState, availableNodes: List<NodeState>): String? {
        if (!config.enabled) return null

        val cacheKey = buildCacheKey(task)
        val cached = cache[cacheKey]

        if (cached == null) {
            misses.incrementAndGet()
            runBlocking { metricsPort.increment("scheduling.cache.miss") }
            return null
        }

        // Check if cached node is still available
        val cachedNode = availableNodes.find { it.nodeId.value == cached.nodeId }
        if (cachedNode == null || cachedNode.availableUnits <= 0) {
            cache.remove(cacheKey)
            evictions.incrementAndGet()
            runBlocking { metricsPort.increment("scheduling.cache.eviction", tags = mapOf("reason" to "node_unavailable")) }
            misses.incrementAndGet()
            runBlocking { metricsPort.increment("scheduling.cache.miss") }
            return null
        }

        // Check if cache entry is expired
        val now = System.currentTimeMillis()
        if (now - cached.timestampEpochMs > config.ttlMs) {
            cache.remove(cacheKey)
            evictions.incrementAndGet()
            runBlocking { metricsPort.increment("scheduling.cache.eviction", tags = mapOf("reason" to "expired")) }
            misses.incrementAndGet()
            runBlocking { metricsPort.increment("scheduling.cache.miss") }
            return null
        }

        // Check if task state has changed significantly
        if (task.status != cached.taskStatus) {
            cache.remove(cacheKey)
            evictions.incrementAndGet()
            runBlocking { metricsPort.increment("scheduling.cache.eviction", tags = mapOf("reason" to "status_changed")) }
            misses.incrementAndGet()
            runBlocking { metricsPort.increment("scheduling.cache.miss") }
            return null
        }

        hits.incrementAndGet()
        runBlocking { metricsPort.increment("scheduling.cache.hit") }
        return cached.nodeId
    }

    /**
     * 缓存节点选择决策
     *
     * Caches a node selection decision.
     *
     * @param task 已调度的任务
     *             Task that was scheduled
     * @param nodeId 选中的节点 ID
     *                Selected node ID
     */
    fun cacheDecision(task: TaskState, nodeId: String) {
        if (!config.enabled) return

        val cacheKey = buildCacheKey(task)
        val cached = CachedDecision(
            nodeId = nodeId,
            taskStatus = task.status,
            timestampEpochMs = System.currentTimeMillis()
        )

        // Evict old entries if cache is full
        if (cache.size >= config.maxSize) {
            evictOldest()
        }

        cache[cacheKey] = cached
        runBlocking { metricsPort.gauge("scheduling.cache.size", cache.size.toDouble()) }
    }

    /**
     * 清除所有缓存决策
     *
     * Clears all cached decisions.
     */
    fun clear() {
        val size = cache.size
        cache.clear()
        evictions.addAndGet(size.toLong())
        runBlocking { metricsPort.increment("scheduling.cache.eviction", tags = mapOf("reason" to "manual_clear")) }
        runBlocking { metricsPort.gauge("scheduling.cache.size", 0.0) }
    }

    /**
     * 获取缓存统计信息
     *
     * Gets cache statistics.
     *
     * @return 缓存统计信息，包含命中数、未命中数、驱逐数、大小和命中率
     *         Cache statistics containing hits, misses, evictions, size, and hit rate
     */
    fun getStats(): CacheStats {
        val total = hits.get() + misses.get()
        val hitRate = if (total > 0) hits.get().toDouble() / total.toDouble() else 0.0
        return CacheStats(
            hits = hits.get(),
            misses = misses.get(),
            evictions = evictions.get(),
            size = cache.size,
            hitRate = hitRate
        )
    }

    private fun buildCacheKey(task: TaskState): String {
        // Cache key based on task characteristics that affect node selection
        val complexity = task.complexity.name
        val solverType = task.payload.taskMeta.solverType ?: "default"
        val budgetTier = budgetTier(task.budgetLimit?.toDouble())
        val priorityTier = priorityTier(task.priority)
        return "$complexity:$solverType:$budgetTier:$priorityTier:${task.tenantId}"
    }

    private fun budgetTier(budgetLimit: Double?): String {
        if (budgetLimit == null) return "unlimited"
        return when {
            budgetLimit < 10.0 -> "low"
            budgetLimit < 100.0 -> "medium"
            else -> "high"
        }
    }

    private fun priorityTier(priority: Int): String {
        return when {
            priority >= 50 -> "high"
            priority >= 20 -> "medium"
            else -> "low"
        }
    }

    private fun evictOldest() {
        val entries = cache.entries.sortedBy { it.value.timestampEpochMs }
        val toEvict = entries.take(config.evictBatchSize)
        toEvict.forEach { entry ->
            cache.remove(entry.key)
            evictions.incrementAndGet()
        }
        runBlocking { metricsPort.increment("scheduling.cache.eviction", tags = mapOf("reason" to "size_limit")) }
    }

    private data class CachedDecision(
        val nodeId: String,
        val taskStatus: fuookami.ospf.framework.remote_solver.protocol.domain.TaskStatus,
        val timestampEpochMs: Long
    )
}

/**
 * 缓存配置
 *
 * Cache configuration.
 *
 * @param enabled 是否启用缓存，默认为 true
 *                Whether cache is enabled, defaults to true
 * @param maxSize 最大缓存大小，默认为 1000
 *                 Maximum cache size, defaults to 1000
 * @param ttlMs 缓存条目 TTL 时间（毫秒），默认为 60000ms（1 分钟）
 *              Cache entry TTL in milliseconds, defaults to 60000ms (1 minute)
 * @param evictBatchSize 驱逐批次大小，默认为 100
 *                        Eviction batch size, defaults to 100
 */
data class CacheConfig(
    val enabled: Boolean = true,
    val maxSize: Int = 1000,
    val ttlMs: Long = 60_000L, // 1 minute default TTL
    val evictBatchSize: Int = 100
)

/**
 * 缓存统计信息
 *
 * Cache statistics.
 *
 * @param hits 缓存命中次数
 *             Cache hit count
 * @param misses 缓存未命中次数
 *               Cache miss count
 * @param evictions 缓存驱逐次数
 *                   Cache eviction count
 * @param size 当前缓存大小
 *             Current cache size
 * @param hitRate 缓存命中率
 *                 Cache hit rate
 */
data class CacheStats(
    val hits: Long,
    val misses: Long,
    val evictions: Long,
    val size: Int,
    val hitRate: Double
)
