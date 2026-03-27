/*
 * 链路追踪端口接口
 *
 * Tracing Port Interface
 *
 * 该接口定义了分布式链路追踪的核心抽象，支持创建和追踪调用链。
 * This interface defines the core abstraction for distributed tracing,
 * supporting creation and tracking of call chains.
 *
 * 链路追踪端口用于监控分布式系统中的请求流转，帮助定位性能瓶颈和故障。
 * The tracing port is used to monitor request flow in distributed systems,
 * helping to identify performance bottlenecks and failures.
 *
 * 功能特性：
 * Features:
 * - 创建追踪跨度 / Create trace spans
 * - 获取当前追踪上下文 / Get current trace context
 * - 支持分布式上下文传播 / Support distributed context propagation
 */
package fuookami.ospf.framework.remote_solver.port

/**
 * 链路追踪端口接口
 *
 * Tracing Port Interface
 *
 * 提供链路追踪功能的端口接口。
 * Port interface providing tracing capabilities.
 */
interface TracingPort {
    /**
     * 在追踪跨度中执行代码块
     *
     * Executes a block within a trace span.
     *
     * 创建一个新的追踪跨度，执行代码块，并在完成后自动结束跨度。
     * Creates a new trace span, executes the block, and automatically ends the span when complete.
     *
     * @param T 代码块返回类型 / Block return type
     * @param name 跨度名称 / Span name
     * @param attributes 跨度属性（默认：空）/ Span attributes (default: empty)
     * @param block 要执行的代码块 / Block to execute
     * @return 代码块的返回值 / Return value of the block
     */
    suspend fun <T> inSpan(
        name: String,
        attributes: Map<String, String> = emptyMap(),
        block: suspend () -> T
    ): T

    /**
     * 获取当前追踪 ID
     *
     * Gets the current trace ID.
     *
     * 返回当前活动追踪的 ID，如果没有活动追踪则返回 null。
     * Returns the ID of the currently active trace, or null if no active trace.
     *
     * @return 当前追踪 ID，无活动追踪返回 null
     *         Current trace ID, or null if no active trace
     */
    fun currentTraceId(): String?

    /**
     * 获取当前跨度 ID
     *
     * Gets the current span ID.
     *
     * 返回当前活动跨度的 ID，如果没有活动跨度则返回 null。
     * Returns the ID of the currently active span, or null if no active span.
     *
     * @return 当前跨度 ID，无活动跨度返回 null
     *         Current span ID, or null if no active span
     */
    fun currentSpanId(): String?
}