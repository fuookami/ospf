/*
 * 规范化指标端口适配器
 *
 * Canonical Metrics Port Adapter
 *
 * 该模块提供指标名称和标签的规范化功能，将内部指标名称转换为 Prometheus 兼容的规范格式。
 * This module provides normalization of metric names and labels, converting internal metric names
 * to Prometheus-compatible canonical format.
 */

package fuookami.ospf.framework.remote_solver.adapter.infrastructure

import fuookami.ospf.framework.remote_solver.port.MetricsPort
import fuookami.ospf.framework.remote_solver.port.MetricsScrapePort

/**
 * 规范化指标端口
 *
 * Canonical Metrics Port
 *
 * 该类实现了 [MetricsPort] 和 [MetricsScrapePort] 接口，负责将内部指标名称和标签
 * 转换为 Prometheus 兼容的规范格式。它包装了另一个 [MetricsPort] 实例，并在调用
 * 底层实现之前进行名称和标签的规范化转换。
 *
 * This class implements [MetricsPort] and [MetricsScrapePort] interfaces, responsible for
 * converting internal metric names and labels to Prometheus-compatible canonical format.
 * It wraps another [MetricsPort] instance and performs name and label normalization before
 * delegating to the underlying implementation.
 *
 * @param delegate 底层的指标端口实现，用于实际记录指标。
 *                 The underlying metrics port implementation used for actual metric recording.
 */
class CanonicalMetricsPort(
    private val delegate: MetricsPort
) : MetricsPort, MetricsScrapePort {

    /**
     * 增加计数器指标
     *
     * Increment counter metric
     *
     * 将内部计数器名称转换为规范格式，并规范化标签键名后调用底层端口。
     * Converts internal counter names to canonical format and normalizes label keys
     * before delegating to the underlying port.
     *
     * @param name 指标名称，如 "task.failed"、"task.stopped" 等。
     *             Metric name, such as "task.failed", "task.stopped", etc.
     * @param delta 增量值，默认为 1。
     *              Delta value, defaults to 1.
     * @param tags 标签键值对映射。
     *             Tag key-value pairs mapping.
     */
    override suspend fun increment(name: String, delta: Long, tags: Map<String, String>) {
        val mappedName = when (name) {
            "task.failed" -> "remote_solver_task_failed_total"
            "task.stopped" -> "remote_solver_task_stopped_total"
            "task.recovered" -> "remote_solver_task_recovered_total"
            else -> name
        }
        delegate.increment(mappedName, delta, normalizeTags(tags))
    }

    /**
     * 设置 gauge 指标值
     *
     * Set gauge metric value
     *
     * 将内部 gauge 名称转换为规范格式，并规范化标签键名后调用底层端口。
     * Converts internal gauge names to canonical format and normalizes label keys
     * before delegating to the underlying port.
     *
     * @param name 指标名称，如 "slice.cost"、"node.performance.score" 等。
     *             Metric name, such as "slice.cost", "node.performance.score", etc.
     * @param value 指标值。
     *              Metric value.
     * @param tags 标签键值对映射。
     *             Tag key-value pairs mapping.
     */
    override suspend fun gauge(name: String, value: Double, tags: Map<String, String>) {
        val mappedName = when (name) {
            "slice.cost" -> "remote_solver_slice_cost"
            "node.performance.score" -> "remote_solver_node_performance_score"
            else -> name
        }
        delegate.gauge(mappedName, value, normalizeTags(tags))
    }

    /**
     * 记录耗时指标
     *
     * Record timing metric
     *
     * 将内部耗时指标名称转换为规范格式，并规范化标签键名后调用底层端口。
     * Converts internal timing metric names to canonical format and normalizes label keys
     * before delegating to the underlying port.
     *
     * @param name 指标名称，如 "slice.runtime.ms" 等。
     *             Metric name, such as "slice.runtime.ms", etc.
     * @param durationMs 耗时（毫秒）。
     *                   Duration in milliseconds.
     * @param tags 标签键值对映射。
     *             Tag key-value pairs mapping.
     */
    override suspend fun timing(name: String, durationMs: Long, tags: Map<String, String>) {
        val mappedName = when (name) {
            "slice.runtime.ms" -> "remote_solver_slice_runtime_ms"
            else -> name
        }
        delegate.timing(mappedName, durationMs, normalizeTags(tags))
    }

    /**
     * 抓取指标数据
     *
     * Scrape metrics data
     *
     * 从底层端口抓取 Prometheus 格式的指标数据。要求底层端口必须支持 [MetricsScrapePort]。
     * Scrapes Prometheus-format metrics data from the underlying port.
     * Requires the underlying port to support [MetricsScrapePort].
     *
     * @return Prometheus 文本格式的指标数据字符串。
     *         Prometheus text format metrics data string.
     * @throws IllegalStateException 如果底层端口不支持抓取功能。
     *                               If the underlying port does not support scrape functionality.
     */
    override fun scrape(): String {
        val scrapeDelegate = delegate as? MetricsScrapePort
            ?: throw IllegalStateException("Delegate metrics port does not support scrape")
        return scrapeDelegate.scrape()
    }

    /**
     * 规范化标签键名
     *
     * Normalize tag keys
     *
     * 将内部标签键名（如 camelCase 格式）转换为 Prometheus 推荐的 snake_case 格式。
     * Converts internal tag keys (such as camelCase format) to Prometheus-recommended snake_case format.
     *
     * @param tags 原始标签映射。
     *             Original tags mapping.
     * @return 规范化后的标签映射。
     *         Normalized tags mapping.
     */
    private fun normalizeTags(tags: Map<String, String>): Map<String, String> =
        tags.entries.associate { (key, value) ->
            val mappedKey = when (key) {
                "nodeId" -> "node_id"
                "taskId" -> "task_id"
                "sliceId" -> "slice_id"
                "tenantId" -> "tenant_id"
                else -> key
            }
            mappedKey to value
        }
}