/**
 * 内存指标端口适配器
 *
 * 提供基于内存的指标收集实现，用于测试和非持久化场景。
 * 支持计数器、仪表和计时三种指标类型，使用标签进行指标区分。
 *
 * In-memory metrics port adapter.
 *
 * Provides memory-based metrics collection implementation for testing and non-persistent scenarios.
 * Supports three metric types: counter, gauge, and timing, using tags for metric differentiation.
 */
package fuookami.ospf.framework.remote_solver.adapter.inmemory

import fuookami.ospf.framework.remote_solver.port.MetricsPort
import java.util.concurrent.ConcurrentHashMap

/**
 * 内存指标端口实现
 *
 * 使用内存存储收集的指标数据，支持并发访问和查询。
 *
 * In-memory metrics port implementation.
 *
 * Uses in-memory storage for collected metrics data, supporting concurrent access and queries.
 */
class InMemoryMetricsPort : MetricsPort {
    /**
     * 计数器存储
     *
     * 存储累计计数值，键为指标名称和标签的组合。
     *
     * Counter storage.
     *
     * Stores accumulated counter values, keyed by metric name and tag combination.
     */
    private val counters = ConcurrentHashMap<String, Long>()

    /**
     * 仪表存储
     *
     * 存储当前值，键为指标名称和标签的组合。
     *
     * Gauge storage.
     *
     * Stores current values, keyed by metric name and tag combination.
     */
    private val gauges = ConcurrentHashMap<String, Double>()

    /**
     * 计时存储
     *
     * 存储计时记录列表，键为指标名称和标签的组合。
     *
     * Timing storage.
     *
     * Stores timing record lists, keyed by metric name and tag combination.
     */
    private val timings = ConcurrentHashMap<String, MutableList<Long>>()

    /**
     * 增加计数器值
     *
     * 将指定增量累加到计数器中，如果计数器不存在则创建。
     *
     * Increments counter value.
     *
     * Accumulates the specified delta into the counter, creates if not exists.
     *
     * @param name 指标名称
     *             Metric name
     * @param delta 增量值
     *              Delta value to add
     * @param tags 标签映射，用于区分不同的指标实例
     *             Tag map for differentiating metric instances
     */
    override suspend fun increment(name: String, delta: Long, tags: Map<String, String>) {
        counters.merge(metricKey(name, tags), delta, java.lang.Long::sum)
    }

    /**
     * 设置仪表值
     *
     * 更新仪表的当前值。
     *
     * Sets gauge value.
     *
     * Updates the current value of the gauge.
     *
     * @param name 指标名称
     *             Metric name
     * @param value 当前值
     *              Current value
     * @param tags 标签映射，用于区分不同的指标实例
     *             Tag map for differentiating metric instances
     */
    override suspend fun gauge(name: String, value: Double, tags: Map<String, String>) {
        gauges[metricKey(name, tags)] = value
    }

    /**
     * 记录计时数据
     *
     * 将执行时间添加到对应指标的计时记录列表中。
     *
     * Records timing data.
     *
     * Adds execution time to the timing record list for the corresponding metric.
     *
     * @param name 指标名称
     *             Metric name
     * @param durationMs 执行时间（毫秒）
     *                    Execution time in milliseconds
     * @param tags 标签映射，用于区分不同的指标实例
     *             Tag map for differentiating metric instances
     */
    override suspend fun timing(name: String, durationMs: Long, tags: Map<String, String>) {
        timings.computeIfAbsent(metricKey(name, tags)) { mutableListOf() }.add(durationMs)
    }

    /**
     * 获取计数器值
     *
     * 查询指定指标名称和标签的计数器累计值。
     *
     * Gets counter value.
     *
     * Queries the accumulated counter value for the specified metric name and tags.
     *
     * @param name 指标名称
     *             Metric name
     * @param tags 标签映射，默认为空
     *             Tag map, defaults to empty
     * @return 计数器值，如果不存在则返回0
     *         Counter value, returns 0 if not found
     */
    fun counter(name: String, tags: Map<String, String> = emptyMap()): Long =
        counters[metricKey(name, tags)] ?: 0L

    /**
     * 获取仪表值
     *
     * 查询指定指标名称和标签的仪表当前值。
     *
     * Gets gauge value.
     *
     * Queries the current gauge value for the specified metric name and tags.
     *
     * @param name 指标名称
     *             Metric name
     * @param tags 标签映射，默认为空
     *             Tag map, defaults to empty
     * @return 仪表值，如果不存在则返回null
     *         Gauge value, returns null if not found
     */
    fun gaugeValue(name: String, tags: Map<String, String> = emptyMap()): Double? =
        gauges[metricKey(name, tags)]

    /**
     * 获取计时记录列表
     *
     * 查询指定指标名称和标签的所有计时记录。
     *
     * Gets timing record list.
     *
     * Queries all timing records for the specified metric name and tags.
     *
     * @param name 指标名称
     *             Metric name
     * @param tags 标签映射，默认为空
     *             Tag map, defaults to empty
     * @return 计时记录列表，如果不存在则返回空列表
     *         Timing record list, returns empty list if not found
     */
    fun timingValues(name: String, tags: Map<String, String> = emptyMap()): List<Long> =
        timings[metricKey(name, tags)]?.toList() ?: emptyList()

    /**
     * 生成指标存储键
     *
     * 将指标名称和标签组合成唯一的存储键。
     *
     * Generates metric storage key.
     *
     * Combines metric name and tags into a unique storage key.
     *
     * @param name 指标名称
     *             Metric name
     * @param tags 标签映射
     *             Tag map
     * @return 存储键字符串
     *         Storage key string
     */
    private fun metricKey(name: String, tags: Map<String, String>): String {
        if (tags.isEmpty()) {
            return name
        }
        val suffix = tags.toSortedMap().entries.joinToString("&") { "${it.key}=${it.value}" }
        return "$name?$suffix"
    }
}