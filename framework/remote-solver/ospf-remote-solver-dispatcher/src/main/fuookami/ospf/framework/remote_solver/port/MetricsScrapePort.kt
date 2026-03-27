/*
 * 指标抓取端口接口
 *
 * Metrics Scrape Port Interface
 *
 * 该接口定义了指标抓取的核心抽象，支持 Prometheus 拉取模式。
 * This interface defines the core abstraction for metrics scraping,
 * supporting Prometheus pull mode.
 *
 * 指标抓取端口用于暴露指标端点，供监控系统定期抓取。
 * The metrics scrape port is used to expose metrics endpoints
 * for monitoring systems to periodically scrape.
 *
 * 使用场景：
 * Use cases:
 * - Prometheus 定期抓取指标 / Prometheus periodically scraping metrics
 * - 手动指标查询 / Manual metrics query
 * - 健康检查端点集成 / Health check endpoint integration
 */
package fuookami.ospf.framework.remote_solver.port

/**
 * 指标抓取端口接口
 *
 * Metrics Scrape Port Interface
 *
 * 提供指标抓取功能的端口接口，返回 Prometheus 格式的指标数据。
 * Port interface providing metrics scraping capabilities,
 * returning metrics data in Prometheus format.
 */
interface MetricsScrapePort {
    /**
     * 抓取指标数据
     *
     * Scrapes metrics data.
     *
     * 以 Prometheus 文本格式返回当前所有指标。
     * Returns all current metrics in Prometheus text format.
     *
     * @return Prometheus 格式的指标文本 / Metrics text in Prometheus format
     */
    fun scrape(): String
}