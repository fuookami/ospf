/*
 * Prometheus 指标端口适配器
 *
 * Prometheus Metrics Port Adapter
 *
 * 该模块提供基于 Prometheus Java Client 的指标收集和暴露实现，支持 Counter、Gauge 和 Histogram 指标类型，
 * 并提供 Prometheus 文本格式的指标抓取端点。
 * This module provides Prometheus Java Client-based metrics collection and exposition implementation,
 * supporting Counter, Gauge and Histogram metric types, and providing Prometheus text format
 * metrics scrape endpoint.
 */

package fuookami.ospf.framework.remote_solver.adapter.prometheus

import fuookami.ospf.framework.remote_solver.port.MetricsPort
import fuookami.ospf.framework.remote_solver.port.MetricsScrapePort
import io.prometheus.client.CollectorRegistry
import io.prometheus.client.Counter
import io.prometheus.client.Gauge
import io.prometheus.client.Histogram
import io.prometheus.client.exporter.common.TextFormat
import java.io.StringWriter
import java.util.concurrent.ConcurrentHashMap

/**
 * Prometheus 指标端口
 *
 * Prometheus Metrics Port
 *
 * 该类实现了 [MetricsPort] 和 [MetricsScrapePort] 接口，使用 Prometheus Java Client 库
 * 提供指标收集和暴露功能。支持三种指标类型：
 * This class implements [MetricsPort] and [MetricsScrapePort] interfaces, using Prometheus
 * Java Client library to provide metrics collection and exposition functionality.
 * Supports three metric types:
 *
 * - **Counter**: 累加型指标，只能增加，适用于计数场景（如请求次数、错误次数）。
 *   Cumulative metric, can only increase, suitable for counting scenarios (e.g., request count, error count).
 * - **Gauge**: 可增可减型指标，适用于表示当前值（如队列长度、内存使用）。
 *   Can increase or decrease metric, suitable for representing current values (e.g., queue length, memory usage).
 * - **Histogram**: 分布型指标，自动创建多个桶进行统计，适用于耗时分布分析。
 *   Distribution metric, automatically creates multiple buckets for statistics, suitable for duration distribution analysis.
 *
 * 指标通过标签（labels）进行维度区分，支持并发安全访问。
 * Metrics are differentiated by labels, supporting concurrent safe access.
 *
 * @param registry Prometheus CollectorRegistry 实例，默认使用全局默认注册表。
 *                 Prometheus CollectorRegistry instance, defaults to global default registry.
 */
class PrometheusMetricsPort(
    private val registry: CollectorRegistry = CollectorRegistry.defaultRegistry
) : MetricsPort, MetricsScrapePort {

    /**
     * 指标描述符
     *
     * Metric Descriptor
     *
     * 内部数据类，用于唯一标识一个指标实例，包含指标名称和标签键名列表。
     * Internal data class for uniquely identifying a metric instance,
     * containing metric name and label key names list.
     *
     * @param name 指标名称。
     *             Metric name.
     * @param labelNames 标签键名列表。
     *                   Label key names list.
     */
    private data class MetricDescriptor(
        val name: String,
        val labelNames: List<String>
    )

    private val counters = ConcurrentHashMap<MetricDescriptor, Counter>()
    private val gauges = ConcurrentHashMap<MetricDescriptor, Gauge>()
    private val histograms = ConcurrentHashMap<MetricDescriptor, Histogram>()

    /**
     * 增加计数器指标
     *
     * Increment counter metric
     *
     * 动态创建或获取 Counter 指标实例，并根据标签值增加计数。如果没有标签，
     * 直接增加无标签的计数器；如果有标签，则使用标签值定位对应的计数器子项。
     *
     * Dynamically creates or gets Counter metric instance, and increments count based on label values.
     * If no labels, directly increments unlabeled counter; if has labels, uses label values
     * to locate corresponding counter child.
     *
     * @param name 指标名称，应符合 Prometheus 命名规范（如 remote_solver_task_failed_total）。
     *             Metric name, should follow Prometheus naming convention (e.g., remote_solver_task_failed_total).
     * @param delta 增量值，通常为正数。
     *               Delta value, usually positive.
     * @param tags 标签键值对映射，用于维度区分。
     *              Tag key-value pairs mapping for dimension differentiation.
     */
    override suspend fun increment(name: String, delta: Long, tags: Map<String, String>) {
        val normalizedTags = normalizeTags(tags)
        val descriptor = MetricDescriptor(name = name, labelNames = normalizedTags.keys.toList())
        val counter = counters.computeIfAbsent(descriptor) {
            Counter.build()
                .name(name)
                .help("${name}_counter")
                .labelNames(*descriptor.labelNames.toTypedArray())
                .register(registry)
        }
        if (descriptor.labelNames.isEmpty()) {
            counter.inc(delta.toDouble())
        } else {
            counter.labels(*normalizedTags.values.toTypedArray()).inc(delta.toDouble())
        }
    }

    /**
     * 设置 gauge 指标值
     *
     * Set gauge metric value
     *
     * 动态创建或获取 Gauge 指标实例，并根据标签值设置当前值。如果没有标签，
     * 直接设置无标签的 gauge；如果有标签，则使用标签值定位对应的 gauge 子项。
     *
     * Dynamically creates or gets Gauge metric instance, and sets current value based on label values.
     * If no labels, directly sets unlabeled gauge; if has labels, uses label values
     * to locate corresponding gauge child.
     *
     * @param name 指标名称，应符合 Prometheus 命名规范（如 remote_solver_slice_cost）。
     *             Metric name, should follow Prometheus naming convention (e.g., remote_solver_slice_cost).
     * @param value 指标的当前值。
     *               Current value of the metric.
     * @param tags 标签键值对映射，用于维度区分。
     *              Tag key-value pairs mapping for dimension differentiation.
     */
    override suspend fun gauge(name: String, value: Double, tags: Map<String, String>) {
        val normalizedTags = normalizeTags(tags)
        val descriptor = MetricDescriptor(name = name, labelNames = normalizedTags.keys.toList())
        val gauge = gauges.computeIfAbsent(descriptor) {
            Gauge.build()
                .name(name)
                .help("${name}_gauge")
                .labelNames(*descriptor.labelNames.toTypedArray())
                .register(registry)
        }
        if (descriptor.labelNames.isEmpty()) {
            gauge.set(value)
        } else {
            gauge.labels(*normalizedTags.values.toTypedArray()).set(value)
        }
    }

    /**
     * 记录耗时指标
     *
     * Record timing metric
     *
     * 动态创建或获取 Histogram 指标实例，并记录耗时值。Histogram 会自动创建预定义的桶
     * 进行统计分布分析。如果没有标签，直接记录无标签的 histogram；如果有标签，
     * 则使用标签值定位对应的 histogram 子项。
     *
     * Dynamically creates or gets Histogram metric instance, and records duration value.
     * Histogram automatically creates predefined buckets for statistical distribution analysis.
     * If no labels, directly records unlabeled histogram; if has labels, uses label values
     * to locate corresponding histogram child.
     *
     * @param name 指标名称，应符合 Prometheus 命名规范（如 remote_solver_slice_runtime_ms）。
     *             Metric name, should follow Prometheus naming convention (e.g., remote_solver_slice_runtime_ms).
     * @param durationMs 耗时值（毫秒）。
     *                   Duration value in milliseconds.
     * @param tags 标签键值对映射，用于维度区分。
     *              Tag key-value pairs mapping for dimension differentiation.
     */
    override suspend fun timing(name: String, durationMs: Long, tags: Map<String, String>) {
        val normalizedTags = normalizeTags(tags)
        val descriptor = MetricDescriptor(name = name, labelNames = normalizedTags.keys.toList())
        val histogram = histograms.computeIfAbsent(descriptor) {
            Histogram.build()
                .name(name)
                .help("${name}_duration_ms")
                .labelNames(*descriptor.labelNames.toTypedArray())
                .buckets(1.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0)
                .register(registry)
        }
        if (descriptor.labelNames.isEmpty()) {
            histogram.observe(durationMs.toDouble())
        } else {
            histogram.labels(*normalizedTags.values.toTypedArray()).observe(durationMs.toDouble())
        }
    }

    /**
     * 抓取 Prometheus 格式的指标数据
     *
     * Scrape Prometheus format metrics data
     *
     * 将注册表中的所有指标导出为 Prometheus 文本格式，供 Prometheus 服务器抓取。
     * Exports all metrics in registry to Prometheus text format for Prometheus server to scrape.
     *
     * @return Prometheus 文本格式的指标数据字符串。
     *         Prometheus text format metrics data string.
     */
    override fun scrape(): String {
        val writer = StringWriter()
        TextFormat.write004(writer, registry.metricFamilySamples())
        return writer.toString()
    }

    /**
     * 规范化标签
     *
     * Normalize tags
     *
     * 将标签映射按键名排序，确保相同标签组合在不同调用中产生一致的标签顺序。
     * Prometheus 要求标签顺序一致以正确聚合指标。
     *
     * Sorts tags mapping by key names, ensuring same tag combination produces consistent
     * label order across different calls. Prometheus requires consistent label order
     * for proper metric aggregation.
     *
     * @param tags 原始标签映射。
     *             Original tags mapping.
     * @return 排序后的标签映射。
     *         Sorted tags mapping.
     */
    private fun normalizeTags(tags: Map<String, String>): Map<String, String> =
        tags.toSortedMap()
}