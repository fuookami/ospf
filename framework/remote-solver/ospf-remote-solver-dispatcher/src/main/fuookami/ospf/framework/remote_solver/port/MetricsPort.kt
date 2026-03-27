/*
 * 指标端口接口
 *
 * Metrics Port Interface
 *
 * 该接口定义了指标收集的核心抽象，支持计数器、仪表和计时器指标。
 * This interface defines the core abstraction for metrics collection,
 * supporting counter, gauge, and timing metrics.
 *
 * 指标端口用于监控系统性能和业务指标，支持与 Prometheus 等监控系统集成。
 * The metrics port is used for monitoring system performance and business metrics,
 * supporting integration with monitoring systems like Prometheus.
 *
 * 指标类型：
 * Metrics types:
 * - Counter（计数器）：累计递增的计数值 / Cumulative incrementing count value
 * - Gauge（仪表）：可增可减的瞬时值 / Instantaneous value that can increase or decrease
 * - Timing（计时器）：操作耗时的测量值 / Duration measurement for operations
 */
package fuookami.ospf.framework.remote_solver.port

/**
 * 指标端口接口
 *
 * Metrics Port Interface
 *
 * 提供指标收集功能的端口接口。
 * Port interface providing metrics collection capabilities.
 */
interface MetricsPort {
    /**
     * 递增计数器指标
     *
     * Increments a counter metric.
     *
     * 将指定计数器增加指定的增量值。计数器是只增不减的累计指标。
     * Increments the specified counter by the given delta value.
     * Counters are cumulative metrics that only increase.
     *
     * @param name 指标名称 / Metric name
     * @param delta 增量值（默认：1）/ Delta value (default: 1)
     * @param tags 标签键值对（默认：空）/ Tag key-value pairs (default: empty)
     */
    suspend fun increment(
        name: String,
        delta: Long = 1,
        tags: Map<String, String> = emptyMap()
    )

    /**
     * 设置仪表指标值
     *
     * Sets a gauge metric value.
     *
     * 将指定仪表设置为当前值。仪表是可增可减的瞬时指标。
     * Sets the specified gauge to the current value.
     * Gauges are instantaneous metrics that can increase or decrease.
     *
     * @param name 指标名称 / Metric name
     * @param value 当前值 / Current value
     * @param tags 标签键值对（默认：空）/ Tag key-value pairs (default: empty)
     */
    suspend fun gauge(
        name: String,
        value: Double,
        tags: Map<String, String> = emptyMap()
    )

    /**
     * 记录计时指标
     *
     * Records a timing metric.
     *
     * 记录操作的执行耗时。用于性能监控和分析。
     * Records the execution duration of an operation.
     * Used for performance monitoring and analysis.
     *
     * @param name 指标名称 / Metric name
     * @param durationMs 持续时间（毫秒）/ Duration in milliseconds
     * @param tags 标签键值对（默认：空）/ Tag key-value pairs (default: empty)
     */
    suspend fun timing(
        name: String,
        durationMs: Long,
        tags: Map<String, String> = emptyMap()
    )
}